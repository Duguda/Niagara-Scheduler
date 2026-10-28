//! Tauri commands: the API the web frontend calls through `invoke`.
//! Argument names are camelCase on the JavaScript side (`groupId` → `group_id`).

use chrono::NaiveDate;
use tauri::State;
use tauri::ipc::{InvokeBody, Request};

use crate::AppState;
use crate::backup::{self, Manifest};
use crate::bookmarks::{Bookmark, BookmarkFolder};
use crate::commissions::{Commission, CommissionClient};
use crate::diary::DiaryEntry;
use crate::expenses::Expense;
use crate::health::{self, Meal, Workout};
use crate::db::{ActivitySummary, Activity, DDay, Event, Memo, MemoGroup, PomodoroSession, Scope, Tag, Todo, TodoGroup};
use crate::tracker::{TrackerSettings, TrackerStatus};
use crate::reading::Book;

type CmdResult<T> = Result<T, String>;

fn with_db<T>(state: &State<AppState>, f: impl FnOnce(&crate::db::Db) -> crate::db::DbResult<T>) -> CmdResult<T> {
    let db = state.db.lock().map_err(|_| "database lock poisoned".to_string())?;
    f(&db).map_err(|e| e.to_string())
}

// ---- window -----------------------------------------------------------------

/// Paints the native title bar in the app theme's colors (`0xRRGGBB`).
/// Windows 11 only; elsewhere (and on older Windows) this does nothing.
#[tauri::command]
pub fn set_titlebar_colors(window: tauri::WebviewWindow, caption: u32, text: u32, dark: bool) -> CmdResult<()> {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::c_void;
        #[link(name = "dwmapi")]
        unsafe extern "system" {
            fn DwmSetWindowAttribute(hwnd: isize, attribute: u32, value: *const c_void, size: u32) -> i32;
        }
        const USE_IMMERSIVE_DARK_MODE: u32 = 20;
        const BORDER_COLOR: u32 = 34;
        const CAPTION_COLOR: u32 = 35;
        const TEXT_COLOR: u32 = 36;
        // COLORREF is 0x00BBGGRR.
        let colorref = |rgb: u32| ((rgb & 0xff) << 16) | (rgb & 0xff00) | ((rgb >> 16) & 0xff);
        let hwnd = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
        let set = |attribute: u32, value: u32| unsafe {
            DwmSetWindowAttribute(hwnd, attribute, &value as *const u32 as *const c_void, 4);
        };
        set(USE_IMMERSIVE_DARK_MODE, dark as u32);
        set(CAPTION_COLOR, colorref(caption));
        set(BORDER_COLOR, colorref(caption));
        set(TEXT_COLOR, colorref(text));
    }
    #[cfg(not(target_os = "windows"))]
    let _ = (window, caption, text, dark);
    Ok(())
}

// ---- settings ---------------------------------------------------------------

#[tauri::command]
pub fn get_setting(state: State<AppState>, key: String) -> CmdResult<Option<String>> {
    with_db(&state, |db| db.setting(&key))
}

#[tauri::command]
pub fn set_setting(state: State<AppState>, key: String, value: String) -> CmdResult<()> {
    with_db(&state, |db| db.set_setting(&key, &value))
}

// ---- events -------------------------------------------------------------------

#[tauri::command]
pub fn events_between(state: State<AppState>, from: NaiveDate, to: NaiveDate) -> CmdResult<Vec<Event>> {
    with_db(&state, |db| db.events_between(from, to))
}

#[tauri::command]
pub fn save_event(state: State<AppState>, event: Event, scope: Option<Scope>) -> CmdResult<i64> {
    if event.end < event.start {
        return Err("The event must end after it starts.".into());
    }
    with_db(&state, |db| db.save_event(&event, scope.unwrap_or(Scope::All)))
}

/// Deletes an event (or one occurrence of a repeating event) and returns the stored row for undo.
#[tauri::command]
pub fn delete_event(state: State<AppState>, id: i64, occurrence: Option<NaiveDate>, scope: Option<Scope>) -> CmdResult<Option<Event>> {
    with_db(&state, |db| db.delete_event(id, occurrence, scope.unwrap_or(Scope::All)))
}

#[tauri::command]
pub fn restore_occurrence(state: State<AppState>, id: i64, day: NaiveDate) -> CmdResult<()> {
    with_db(&state, |db| db.restore_occurrence(id, day))
}

// ---- tags ---------------------------------------------------------------------

#[tauri::command]
pub fn tags(state: State<AppState>) -> CmdResult<Vec<Tag>> {
    with_db(&state, |db| db.tags())
}

#[tauri::command]
pub fn save_tag(state: State<AppState>, mut tag: Tag) -> CmdResult<i64> {
    tag.name = tag.name.trim().to_owned();
    tag.category = tag.category.trim().to_owned();
    if tag.name.is_empty() {
        return Err("a tag needs a name".into());
    }
    with_db(&state, |db| db.save_tag(&tag))
}

#[tauri::command]
pub fn delete_tag(state: State<AppState>, id: i64) -> CmdResult<()> {
    with_db(&state, |db| db.delete_tag(id))
}

// ---- d-days -------------------------------------------------------------------

#[tauri::command]
pub fn ddays(state: State<AppState>) -> CmdResult<Vec<DDay>> {
    with_db(&state, |db| db.ddays())
}

#[tauri::command]
pub fn save_dday(state: State<AppState>, mut dday: DDay) -> CmdResult<()> {
    dday.image = dday.image.filter(|name| crate::images::is_safe_name(name));
    if !DDay::SHAPES.contains(&dday.shape.as_str()) {
        dday.shape = DDay::SHAPES[0].into();
    }
    with_db(&state, |db| db.save_dday(&dday))?;
    // A replaced or removed cover image is no longer referenced: delete the file.
    remove_unused_images(state)
}

#[tauri::command]
pub fn reorder_ddays(state: State<AppState>, ids: Vec<i64>) -> CmdResult<()> {
    with_db(&state, |db| db.reorder_ddays(&ids))
}

#[tauri::command]
pub fn delete_dday(state: State<AppState>, id: i64) -> CmdResult<()> {
    with_db(&state, |db| db.delete_dday(id))?;
    remove_unused_images(state)
}

/// Optimizes an uploaded image (sent as raw bytes) and stores it. Returns the file name.
/// The `purpose` header (`cover`, `card`, `header` or `sticker`) sets the size and format.
/// The file is kept only once something references it; see `remove_unused_images`.
#[tauri::command]
pub async fn import_image(state: State<'_, AppState>, request: Request<'_>) -> CmdResult<String> {
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err("expected raw image bytes".into());
    };
    let bytes = bytes.clone();
    let purpose = request
        .headers()
        .get("purpose")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| serde_json::from_value(serde_json::Value::String(v.to_owned())).ok())
        .unwrap_or_default();
    let dir = state.images_dir.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (file, ext) = crate::images::optimize(&bytes, purpose).map_err(|e| e.to_string())?;
        crate::images::store(&dir, &file, ext).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Deletes stored images no D-Day uses (e.g. after the D-Day editor was cancelled).
#[tauri::command]
pub fn remove_unused_images(state: State<AppState>) -> CmdResult<()> {
    let keep = with_db(&state, |db| db.referenced_images())?;
    crate::images::collect_garbage(&state.images_dir, &keep);
    Ok(())
}

// ---- todos --------------------------------------------------------------------

#[tauri::command]
pub fn todo_groups(state: State<AppState>) -> CmdResult<Vec<TodoGroup>> {
    with_db(&state, |db| db.todo_groups())
}

#[tauri::command]
pub fn add_todo_group(state: State<AppState>, name: String, color: u32) -> CmdResult<i64> {
    with_db(&state, |db| db.add_todo_group(name.trim(), color))
}

#[tauri::command]
pub fn update_todo_group(state: State<AppState>, group: TodoGroup) -> CmdResult<()> {
    with_db(&state, |db| db.update_todo_group(&group))
}

#[tauri::command]
pub fn delete_todo_group(state: State<AppState>, id: i64) -> CmdResult<()> {
    with_db(&state, |db| db.delete_todo_group(id))
}

#[tauri::command]
pub fn todos(state: State<AppState>) -> CmdResult<Vec<Todo>> {
    with_db(&state, |db| db.todos())
}

#[tauri::command]
pub fn add_todo(
    state: State<AppState>,
    group_id: Option<i64>,
    parent_id: Option<i64>,
    title: String,
    due: Option<NaiveDate>,
    due_time: Option<String>,
) -> CmdResult<i64> {
    with_db(&state, |db| db.add_todo(group_id, parent_id, title.trim(), due, due_time.as_deref()))
}

#[tauri::command]
pub fn update_todo(state: State<AppState>, todo: Todo) -> CmdResult<()> {
    with_db(&state, |db| db.update_todo(&todo))
}

#[tauri::command]
pub fn set_todo_done(state: State<AppState>, id: i64, done: bool) -> CmdResult<()> {
    with_db(&state, |db| db.set_todo_done(id, done))
}

#[tauri::command]
pub fn delete_todo(state: State<AppState>, id: i64) -> CmdResult<()> {
    with_db(&state, |db| db.delete_todo(id))
}

// ---- memos --------------------------------------------------------------------

#[tauri::command]
pub fn memos(state: State<AppState>) -> CmdResult<Vec<Memo>> {
    with_db(&state, |db| db.memos())
}

#[tauri::command]
pub fn create_memo(state: State<AppState>, title: String, group_id: Option<i64>) -> CmdResult<i64> {
    with_db(&state, |db| db.create_memo(&title, group_id))
}

#[tauri::command]
pub fn set_memo_group(state: State<AppState>, id: i64, group_id: Option<i64>) -> CmdResult<()> {
    with_db(&state, |db| db.set_memo_group(id, group_id))
}

#[tauri::command]
pub fn memo_groups(state: State<AppState>) -> CmdResult<Vec<MemoGroup>> {
    with_db(&state, |db| db.memo_groups())
}

#[tauri::command]
pub fn save_memo_group(state: State<AppState>, group: MemoGroup) -> CmdResult<i64> {
    let name = group.name.trim();
    if name.is_empty() {
        return Err("A group needs a name.".into());
    }
    with_db(&state, |db| db.save_memo_group(&MemoGroup { name: name.into(), ..group }))
}

#[tauri::command]
pub fn delete_memo_group(state: State<AppState>, id: i64) -> CmdResult<()> {
    with_db(&state, |db| db.delete_memo_group(id))
}

#[tauri::command]
pub fn update_memo(state: State<AppState>, id: i64, title: String, body: String) -> CmdResult<()> {
    with_db(&state, |db| db.update_memo(id, &title, &body))
}

#[tauri::command]
pub fn delete_memo(state: State<AppState>, id: i64) -> CmdResult<()> {
    with_db(&state, |db| db.delete_memo(id))
}

// ---- pomodoro -----------------------------------------------------------------

#[tauri::command]
pub fn save_pomodoro_session(
    state: State<AppState>,
    id: Option<i64>,
    started_at: i64,
    ended_at: i64,
    focused_secs: i64,
    completed: bool,
    label: String,
) -> CmdResult<i64> {
    if ended_at < started_at || focused_secs < 0 {
        return Err("invalid session".into());
    }
    // Focus time can't be longer than the session itself.
    let focused_secs = focused_secs.min(ended_at - started_at);
    with_db(&state, |db| db.save_pomodoro_session(id, started_at, ended_at, focused_secs, completed, label.trim()))
}

#[tauri::command]
pub fn pomodoro_sessions(state: State<AppState>, from_ts: i64, to_ts: i64) -> CmdResult<Vec<PomodoroSession>> {
    with_db(&state, |db| db.pomodoro_sessions(from_ts, to_ts))
}

// ---- working time -------------------------------------------------------------

#[tauri::command]
pub fn activity_summary(state: State<AppState>, days: Vec<NaiveDate>, title_limit: usize) -> CmdResult<ActivitySummary> {
    with_db(&state, |db| db.activity_summary(&days, title_limit))
}

#[tauri::command]
pub fn activity_timeline(state: State<AppState>, day: NaiveDate) -> CmdResult<Vec<Activity>> {
    with_db(&state, |db| db.activity_timeline(day))
}

#[tauri::command]
pub fn tracker_status(state: State<AppState>) -> TrackerStatus {
    state.tracker.status()
}

/// Turns the Working time feature on or off (Settings → features), starting or stopping window reads.
#[tauri::command]
pub fn set_tracker_enabled(state: State<AppState>, enabled: bool) {
    state.tracker.enabled.store(enabled, std::sync::atomic::Ordering::Relaxed);
}

#[tauri::command]
pub fn tracker_settings(state: State<AppState>) -> TrackerSettings {
    state.tracker.settings()
}

#[tauri::command]
pub fn set_tracker_settings(state: State<AppState>, mut settings: TrackerSettings) -> CmdResult<()> {
    settings.tracked_apps.retain(|a| !a.trim().is_empty());
    settings.tracked_apps.sort();
    settings.tracked_apps.dedup();
    with_db(&state, |db| settings.save(db))?;
    state.tracker.apply(settings);
    Ok(())
}

// ---- bookmarks ------------------------------------------------------------------

#[tauri::command]
pub fn bookmark_folders(state: State<AppState>) -> CmdResult<Vec<BookmarkFolder>> {
    with_db(&state, |db| db.bookmark_folders())
}

#[tauri::command]
pub fn save_bookmark_folder(state: State<AppState>, folder: BookmarkFolder) -> CmdResult<i64> {
    let name = folder.name.trim();
    if name.is_empty() {
        return Err("A folder needs a name.".into());
    }
    // A folder cannot be moved inside itself.
    if folder.id != 0
        && let Some(parent) = folder.parent_id
        && with_db(&state, |db| db.folder_is_within(parent, folder.id))?
    {
        return Err("folder_cycle".into());
    }
    with_db(&state, |db| db.save_bookmark_folder(&BookmarkFolder { name: name.into(), ..folder }))
}

#[tauri::command]
pub fn delete_bookmark_folder(state: State<AppState>, id: i64) -> CmdResult<()> {
    with_db(&state, |db| db.delete_bookmark_folder(id))
}

#[tauri::command]
pub fn bookmarks(state: State<AppState>) -> CmdResult<Vec<Bookmark>> {
    with_db(&state, |db| db.bookmarks())
}

#[tauri::command]
pub fn save_bookmark(state: State<AppState>, bookmark: Bookmark) -> CmdResult<i64> {
    let url = bookmark.url.trim();
    if url.is_empty() || url.to_ascii_lowercase().starts_with("javascript:") {
        return Err("invalid_url".into());
    }
    with_db(&state, |db| db.save_bookmark(&Bookmark { url: url.into(), title: bookmark.title.trim().into(), ..bookmark }))
}

#[tauri::command]
pub fn reorder_bookmarks(state: State<AppState>, ids: Vec<i64>) -> CmdResult<()> {
    with_db(&state, |db| db.reorder_bookmarks(&ids))
}

#[tauri::command]
pub fn delete_bookmark(state: State<AppState>, id: i64) -> CmdResult<Option<Bookmark>> {
    with_db(&state, |db| db.delete_bookmark(id))
}

// ---- Obsidian vault ------------------------------------------------------------------

fn vault_root(state: &State<AppState>) -> CmdResult<std::path::PathBuf> {
    let root = with_db(state, |db| db.setting(crate::obsidian::VAULT_KEY))?;
    root.filter(|r| !r.trim().is_empty()).map(std::path::PathBuf::from).ok_or_else(|| "no vault".to_string())
}

#[tauri::command]
pub async fn obsidian_notes(state: State<'_, AppState>) -> CmdResult<Vec<crate::obsidian::NoteMeta>> {
    let root = vault_root(&state)?;
    tauri::async_runtime::spawn_blocking(move || crate::obsidian::list(&root).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn obsidian_read(state: State<AppState>, path: String) -> CmdResult<String> {
    crate::obsidian::read(&vault_root(&state)?, &path)
}

/// `obsidian://open?path=…` link for a note, which opens it in the Obsidian app.
#[tauri::command]
pub fn obsidian_open_url(state: State<AppState>, path: String) -> CmdResult<String> {
    let full = crate::obsidian::absolute(&vault_root(&state)?, &path)?;
    let text = full.to_string_lossy();
    // Windows canonical paths start with `\\?\`, which Obsidian does not understand.
    let text = text.strip_prefix(r"\\?\").unwrap_or(&text);
    let encoded: String = text
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect();
    Ok(format!("obsidian://open?path={encoded}"))
}

/// Writes a memo into the vault (see `obsidian::export_memo`) and returns the note's path.
#[tauri::command]
pub fn export_memo_to_obsidian(state: State<AppState>, id: i64) -> CmdResult<String> {
    let root = vault_root(&state)?;
    let (memo, dir) = with_db(&state, |db| {
        let memo = db.memos()?.into_iter().find(|m| m.id == id);
        let dir = db.setting(crate::obsidian::EXPORT_DIR_KEY)?;
        Ok((memo, dir))
    })?;
    let memo = memo.ok_or("memo not found")?;
    let dir = dir.filter(|d| !d.trim().is_empty()).unwrap_or_else(|| crate::obsidian::DEFAULT_EXPORT_DIR.into());
    crate::obsidian::export_memo(
        &root,
        &dir,
        &crate::obsidian::MemoNote { id: memo.id, title: &memo.title, body: &memo.body, created_at: memo.created_at, updated_at: memo.updated_at },
    )
}

// ---- morning brief ------------------------------------------------------------------

/// Rewrites `brief.json` now and returns where it is.
#[tauri::command]
pub fn refresh_brief(state: State<AppState>) -> CmdResult<String> {
    let db = state.db.lock().map_err(|_| "database lock poisoned".to_string())?;
    crate::brief::refresh(&db, &state.storage.brief).map(|p| p.to_string_lossy().into_owned())
}

// ---- cloze study ---------------------------------------------------------------------

#[derive(serde::Serialize)]
pub struct ClozeSource {
    /// `ob:<vault path>` or `memo:<id>`.
    pub source: String,
    pub title: String,
    /// Folder for notes, memo group for memos ('' when none).
    pub place: String,
    /// Number of bold spans (blanks).
    pub blanks: usize,
    pub due: i64,
    pub weak: i64,
    pub seen: i64,
}

/// Notes and memos tagged `#암기` that have `**bold**` blanks, with today's review counts.
#[tauri::command]
pub async fn cloze_sources(state: State<'_, AppState>) -> CmdResult<Vec<ClozeSource>> {
    let today = crate::cloze::today();
    let (memos, groups, overview, vault) = with_db(&state, |db| {
        Ok((db.memos()?, db.memo_groups()?, db.cloze_overview(today)?, db.setting(crate::obsidian::VAULT_KEY)?))
    })?;
    let stats = crate::cloze::by_source(overview);
    let group_name: std::collections::HashMap<i64, String> = groups.into_iter().map(|g| (g.id, g.name)).collect();
    let with_stats = |source: String, title: String, place: String, blanks: usize| {
        let s = stats.get(&source);
        ClozeSource {
            title,
            place,
            blanks,
            due: s.map_or(0, |s| s.due),
            weak: s.map_or(0, |s| s.weak),
            seen: s.map_or(0, |s| s.seen),
            source,
        }
    };
    let mut out: Vec<ClozeSource> = memos
        .into_iter()
        .filter_map(|m| {
            if !crate::cloze::has_study_tag(&m.body) {
                return None;
            }
            let blanks = crate::cloze::count_bold(&m.body);
            let place = m.group_id.and_then(|g| group_name.get(&g).cloned()).unwrap_or_default();
            (blanks > 0).then(|| with_stats(format!("memo:{}", m.id), m.title, place, blanks))
        })
        .collect();
    if let Some(root) = vault.filter(|v| !v.trim().is_empty()) {
        let root = std::path::PathBuf::from(root);
        let notes = tauri::async_runtime::spawn_blocking(move || crate::obsidian::study_notes(&root))
            .await
            .map_err(|e| e.to_string())?
            .unwrap_or_default();
        out.extend(notes.into_iter().map(|(n, blanks)| with_stats(format!("ob:{}", n.path), n.title, n.folder, blanks)));
    }
    Ok(out)
}

#[tauri::command]
pub fn cloze_stats(state: State<AppState>, source: String) -> CmdResult<Vec<crate::cloze::ClozeStat>> {
    with_db(&state, |db| db.cloze_stats(&source))
}

/// Saves a ○ (`right: true`) or ✕ for one blank.
#[tauri::command]
pub fn cloze_record(state: State<AppState>, source: String, key: String, right: bool) -> CmdResult<crate::cloze::ClozeStat> {
    if !key.starts_with(&format!("{source}|")) {
        return Err("invalid blank".into());
    }
    with_db(&state, |db| db.cloze_record(&source, &key, right, crate::cloze::today()))
}

#[tauri::command]
pub fn cloze_reset(state: State<AppState>, source: String) -> CmdResult<()> {
    with_db(&state, |db| db.cloze_reset(&source))
}

// ---- storage -------------------------------------------------------------------------

#[derive(serde::Serialize)]
pub struct StorageInfo {
    pub base: String,
    pub data: String,
    pub brief: String,
    pub portable: bool,
}

/// Where Niagara keeps its files, for Settings.
#[tauri::command]
pub fn storage_info(state: State<AppState>) -> StorageInfo {
    let s = &state.storage;
    let text = |p: &std::path::Path| p.to_string_lossy().into_owned();
    StorageInfo { base: text(&s.base), data: text(&s.data), brief: text(&s.brief), portable: s.portable }
}

// ---- reading log ------------------------------------------------------------------

#[tauri::command]
pub fn books(state: State<AppState>) -> CmdResult<Vec<Book>> {
    with_db(&state, |db| db.books())
}

#[tauri::command]
pub fn save_book(state: State<AppState>, book: Book) -> CmdResult<i64> {
    let title = book.title.trim();
    if title.is_empty() {
        return Err("title required".into());
    }
    if !Book::STATUSES.contains(&book.status.as_str()) {
        return Err("invalid book".into());
    }
    let book = Book {
        title: title.into(),
        author: book.author.trim().into(),
        publisher: book.publisher.trim().into(),
        image: book.image.filter(|name| crate::images::is_safe_name(name)),
        rating: book.rating.min(Book::MAX_RATING),
        // Can't have read past the last page.
        current_page: if book.total_pages > 0 { book.current_page.min(book.total_pages) } else { book.current_page },
        ..book
    };
    let id = with_db(&state, |db| db.save_book(&book))?;
    // A replaced or removed cover is no longer referenced: delete the file.
    remove_unused_images(state)?;
    Ok(id)
}

#[tauri::command]
pub fn delete_book(state: State<AppState>, id: i64) -> CmdResult<Option<Book>> {
    with_db(&state, |db| db.delete_book(id))
}

// ---- expenses -------------------------------------------------------------------

#[tauri::command]
pub fn expenses_between(state: State<AppState>, from: NaiveDate, to: NaiveDate) -> CmdResult<Vec<Expense>> {
    with_db(&state, |db| db.expenses_between(from, to))
}

#[tauri::command]
pub fn save_expense(state: State<AppState>, expense: Expense) -> CmdResult<i64> {
    if !expense.amount.is_finite() || expense.amount <= 0.0 {
        return Err("invalid_amount".into());
    }
    with_db(&state, |db| db.save_expense(&expense))
}

#[tauri::command]
pub fn delete_expense(state: State<AppState>, id: i64) -> CmdResult<Option<Expense>> {
    with_db(&state, |db| db.delete_expense(id))
}

// ---- commissions ----------------------------------------------------------------

#[tauri::command]
pub fn commissions(state: State<AppState>) -> CmdResult<Vec<Commission>> {
    with_db(&state, |db| db.commissions())
}

#[tauri::command]
pub fn save_commission(state: State<AppState>, commission: Commission) -> CmdResult<i64> {
    let title = commission.title.trim();
    if title.is_empty() {
        return Err("title required".into());
    }
    if !Commission::STATUSES.contains(&commission.status.as_str()) {
        return Err("invalid commission".into());
    }
    let money = |n: f64| if n.is_finite() && n > 0.0 { n } else { 0.0 };
    let commission = Commission {
        title: title.into(),
        kind: commission.kind.trim().chars().take(40).collect(),
        memo: commission.memo.trim().into(),
        price: money(commission.price),
        payments: commission
            .payments
            .into_iter()
            .filter(|p| money(p.amount) > 0.0)
            .collect(),
        revisions: commission.revisions.clamp(0, 999),
        image: commission.image.filter(|name| crate::images::is_safe_name(name)),
        ..commission
    };
    let id = with_db(&state, |db| db.save_commission(&commission))?;
    // A replaced or removed image is no longer referenced: delete the file.
    remove_unused_images(state)?;
    Ok(id)
}

#[tauri::command]
pub fn delete_commission(state: State<AppState>, id: i64) -> CmdResult<Option<Commission>> {
    with_db(&state, |db| db.delete_commission(id))
}

#[tauri::command]
pub fn commission_clients(state: State<AppState>) -> CmdResult<Vec<CommissionClient>> {
    with_db(&state, |db| db.commission_clients())
}

#[tauri::command]
pub fn save_commission_client(state: State<AppState>, client: CommissionClient) -> CmdResult<i64> {
    let name = client.name.trim();
    if name.is_empty() {
        return Err("name required".into());
    }
    let client = CommissionClient {
        name: name.chars().take(60).collect(),
        contact: client.contact.trim().into(),
        memo: client.memo.trim().into(),
        ..client
    };
    with_db(&state, |db| db.save_commission_client(&client))
}

#[tauri::command]
pub fn delete_commission_client(state: State<AppState>, id: i64) -> CmdResult<()> {
    with_db(&state, |db| db.delete_commission_client(id))
}

// ---- diary ----------------------------------------------------------------------

#[tauri::command]
pub fn diary_between(state: State<AppState>, from: NaiveDate, to: NaiveDate) -> CmdResult<Vec<DiaryEntry>> {
    with_db(&state, |db| db.diary_between(from, to))
}

#[tauri::command]
pub fn save_diary_entry(state: State<AppState>, entry: DiaryEntry) -> CmdResult<()> {
    let pick = |value: &str, allowed: &[&str]| if allowed.contains(&value) { value.to_string() } else { allowed[0].to_string() };
    let safe = crate::images::is_safe_name;
    let entry = DiaryEntry {
        mood: if DiaryEntry::MOODS.contains(&entry.mood.as_str()) { entry.mood.clone() } else { String::new() },
        layout: pick(&entry.layout, &DiaryEntry::LAYOUTS),
        paper: pick(&entry.paper, &DiaryEntry::PAPERS),
        line: entry.line.trim().chars().take(200).collect(),
        photo: entry.photo.filter(|name| safe(name)),
        stickers: entry.stickers.into_iter().filter(|s| safe(&s.image)).take(60).collect(),
        ..entry
    };
    with_db(&state, |db| db.save_diary_entry(&entry))?;
    // A replaced photo or removed sticker may no longer be used anywhere.
    remove_unused_images(state)
}

#[tauri::command]
pub fn delete_diary_entry(state: State<AppState>, date: NaiveDate) -> CmdResult<Option<DiaryEntry>> {
    with_db(&state, |db| db.delete_diary_entry(date))
}

// ---- diet and exercise -----------------------------------------------------------

#[tauri::command]
pub fn meals_between(state: State<AppState>, from: NaiveDate, to: NaiveDate) -> CmdResult<Vec<Meal>> {
    with_db(&state, |db| db.meals_between(from, to))
}

#[tauri::command]
pub fn save_meal(state: State<AppState>, meal: Meal) -> CmdResult<i64> {
    let name = meal.name.trim();
    if name.is_empty() {
        return Err("name required".into());
    }
    if !Meal::SLOTS.contains(&meal.slot.as_str()) || !(0..=health::MAX_KCAL).contains(&meal.kcal) {
        return Err("invalid meal".into());
    }
    with_db(&state, |db| db.save_meal(&Meal { name: name.into(), ..meal }))
}

#[tauri::command]
pub fn delete_meal(state: State<AppState>, id: i64) -> CmdResult<Option<Meal>> {
    with_db(&state, |db| db.delete_meal(id))
}

#[tauri::command]
pub fn workouts_between(state: State<AppState>, from: NaiveDate, to: NaiveDate) -> CmdResult<Vec<Workout>> {
    with_db(&state, |db| db.workouts_between(from, to))
}

#[tauri::command]
pub fn save_workout(state: State<AppState>, workout: Workout) -> CmdResult<i64> {
    if workout.kind.trim().is_empty()
        || !(1..=health::MAX_MINUTES).contains(&workout.minutes)
        || !(0..=health::MAX_KCAL).contains(&workout.kcal)
    {
        return Err("invalid workout".into());
    }
    with_db(&state, |db| db.save_workout(&Workout { note: workout.note.trim().into(), ..workout }))
}

#[tauri::command]
pub fn delete_workout(state: State<AppState>, id: i64) -> CmdResult<Option<Workout>> {
    with_db(&state, |db| db.delete_workout(id))
}

// ---- backup, restore and reset -----------------------------------------------------

/// Saves everything (database, D-Day images, settings) to a backup file at `path`.
#[tauri::command]
pub async fn export_data(state: State<'_, AppState>, path: String) -> CmdResult<Manifest> {
    let scratch = state.data_dir.join("export-snapshot.tmp");
    let counts = with_db(&state, |db| Ok(backup::prepare_export(db, &scratch)))??;
    let images_dir = state.images_dir.clone();
    tauri::async_runtime::spawn_blocking(move || backup::write_backup(&scratch, &images_dir, counts, std::path::Path::new(&path)))
        .await
        .map_err(|e| e.to_string())?
}

/// Reads a backup's summary so the user can confirm before importing.
#[tauri::command]
pub fn inspect_backup(path: String) -> CmdResult<Manifest> {
    backup::inspect(std::path::Path::new(&path))
}

/// Replaces all data with a backup, then restarts the app to load it.
#[tauri::command]
pub fn import_data(app: tauri::AppHandle, state: State<AppState>, path: String) -> CmdResult<()> {
    backup::stage_import(std::path::Path::new(&path), &state.data_dir)?;
    app.restart()
}

/// Deletes all data, then restarts the app with a fresh start.
#[tauri::command]
pub fn reset_all_data(app: tauri::AppHandle, state: State<AppState>) -> CmdResult<()> {
    backup::stage_reset(&state.data_dir)?;
    app.restart()
}
