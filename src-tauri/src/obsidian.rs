//! Obsidian vault access.
//!
//! The user picks a vault folder in the Notes page. Niagara lists and reads the Markdown
//! notes inside it (read-only), and can write a memo into it as a new note. Every path
//! coming from the webview is relative to the vault and is checked to stay inside it.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use chrono::{Local, TimeZone};
use serde::Serialize;

/// Setting from before 0.11 that stored the Obsidian vault; it now moves into the note folders.
pub const VAULT_KEY: &str = "obsidian.vault";
/// Setting that stores the vault-relative folder memos are written to.
pub const EXPORT_DIR_KEY: &str = "obsidian.exportDir";
pub const DEFAULT_EXPORT_DIR: &str = "Niagara";
/// Setting: the note folders the Notes page reads and edits (JSON list of absolute paths).
pub const READ_DIRS_KEY: &str = "notes.readDirs";
/// Setting: the note folder a memo is sent to with "Send to notes" (absolute path).
pub const MEMO_DIR_KEY: &str = "notes.memoDir";

pub fn parse_read_dirs(json: Option<&str>) -> Vec<String> {
    json.and_then(|j| serde_json::from_str::<Vec<String>>(j).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|d| d.trim().to_string())
        .filter(|d| !d.is_empty())
        .collect()
}

/// How a note from an extra read folder is grouped: `<folder name>/<sub-folder>`.
pub fn root_label(root: &str, folder: &str) -> String {
    let name = Path::new(root).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| root.to_string());
    if folder.is_empty() { name } else { format!("{name}/{folder}") }
}

/// Big vaults are fine, but the list stays bounded.
const MAX_NOTES: usize = 5000;
/// Notes larger than this are not opened (they are almost never hand-written notes).
const MAX_NOTE_BYTES: u64 = 2 * 1024 * 1024;
/// Front matter key that marks a note written from a memo, so re-sending updates it.
const MEMO_KEY: &str = "niagara_memo";

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct NoteMeta {
    /// The note folder it was found in (that folder's path).
    pub root: String,
    /// Path inside the vault with `/` separators, e.g. `10 공부/SQL.md`.
    pub path: String,
    /// File name without `.md`, which is the note's title in Obsidian.
    pub title: String,
    /// Folder part of `path` (`''` for the vault root).
    pub folder: String,
    /// Last modified, Unix seconds.
    pub modified: i64,
    pub size: u64,
}

/// Dot folders (`.obsidian`, `.trash`, `.git`) hold settings and deleted notes, not notes.
fn skipped_dir(name: &str) -> bool {
    name.starts_with('.') || name == "node_modules"
}

fn is_markdown(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("md"))
}

/// Notes Niagara can read: Markdown (`.md`) and plain text (`.txt`).
fn is_note(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("txt"))
}

fn rel_string(rel: &Path) -> String {
    rel.components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Every Markdown note in the vault, most recently changed first.
pub fn list(root: &Path) -> io::Result<Vec<NoteMeta>> {
    if !root.is_dir() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "vault folder not found"));
    }
    let mut notes = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else { continue };
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if kind.is_dir() {
                if !skipped_dir(&name) {
                    stack.push(path);
                }
            } else if kind.is_file() && is_note(&path) {
                let Ok(meta) = entry.metadata() else { continue };
                let Ok(rel) = path.strip_prefix(root) else { continue };
                let rel_path = rel_string(rel);
                let folder = rel.parent().map(rel_string).unwrap_or_default();
                let title = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                let modified = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                notes.push(NoteMeta { root: String::new(), path: rel_path, title, folder, modified, size: meta.len() });
            }
        }
    }
    notes.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| a.path.cmp(&b.path)));
    notes.truncate(MAX_NOTES);
    Ok(notes)
}

/// Turns a vault-relative path from the webview into a real path inside the vault.
/// Rejects absolute paths, `..`, and anything that resolves (through links) outside it.
fn resolve(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let rel_path = Path::new(rel);
    if rel.is_empty() || rel_path.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err("invalid note path".into());
    }
    let root = root.canonicalize().map_err(|_| "vault folder not found".to_string())?;
    let full = root.join(rel_path).canonicalize().map_err(|_| "note not found".to_string())?;
    if !full.starts_with(&root) {
        return Err("invalid note path".into());
    }
    Ok(full)
}

/// The text of one note.
pub fn read(root: &Path, rel: &str) -> Result<String, String> {
    let path = resolve(root, rel)?;
    if !is_note(&path) {
        return Err("not a note".into());
    }
    let size = fs::metadata(&path).map_err(|e| e.to_string())?.len();
    if size > MAX_NOTE_BYTES {
        return Err("note is too large to open".into());
    }
    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Notes tagged for recall practice that have `**bold**` blanks, with how many, most recent first.
pub fn study_notes(root: &Path) -> io::Result<Vec<(NoteMeta, usize)>> {
    Ok(list(root)?
        .into_iter()
        .filter(|n| n.size <= MAX_NOTE_BYTES && n.path.to_ascii_lowercase().ends_with(".md"))
        .filter_map(|n| {
            let text = fs::read(root.join(&n.path)).ok()?;
            let text = String::from_utf8_lossy(&text);
            // Memos copied into the vault are studied as memos, not twice.
            if !crate::cloze::has_study_tag(&text) || is_any_memo_note(&text) {
                return None;
            }
            // A tagged note with no bold still counts: it can be studied in the n-reading mode.
            Some((n, crate::cloze::count_bold(&text)))
        })
        .collect())
}

/// Last-modified time (Unix seconds) of a file, 0 when unknown.
fn modified_secs(path: &Path) -> i64 {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Saves a note's text. `expected_modified` is the time the editor loaded it: if the file changed
/// since (edited in another program), nothing is written and `Err("changed")` comes back.
/// Returns the new modified time.
pub fn write(root: &Path, rel: &str, text: &str, expected_modified: Option<i64>) -> Result<i64, String> {
    let path = resolve(root, rel)?;
    if !is_note(&path) {
        return Err("not a note".into());
    }
    if let Some(expected) = expected_modified
        && modified_secs(&path) > expected
    {
        return Err("changed".into());
    }
    let tmp = path.with_extension("niagara-tmp");
    fs::write(&tmp, text).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(modified_secs(&path))
}

/// Creates an empty note `<folder>/<title>.md` (numbered if the name is taken) and returns its path.
pub fn create(root: &Path, folder: &str, title: &str) -> Result<String, String> {
    let folder = folder.trim().trim_matches('/');
    if Path::new(folder).components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err("invalid folder".into());
    }
    if !root.is_dir() {
        return Err("note folder not found".into());
    }
    let dir = root.join(folder);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stem = file_stem_for(title);
    let path = (1..1000)
        .map(|n| dir.join(if n == 1 { format!("{stem}.md") } else { format!("{stem} {n}.md") }))
        .find(|p| !p.exists())
        .ok_or("too many notes with this name")?;
    fs::write(&path, "").map_err(|e| e.to_string())?;
    Ok(rel_string(path.strip_prefix(root).map_err(|e| e.to_string())?))
}

/// Switches a note between `.txt` and `.md` (the text stays as it is) and returns the new path.
pub fn convert(root: &Path, rel: &str) -> Result<String, String> {
    let path = resolve(root, rel)?;
    let to = match path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() {
        Some("txt") => "md",
        Some("md") => "txt",
        _ => return Err("not a note".into()),
    };
    let target = path.with_extension(to);
    if target.exists() {
        return Err("exists".into());
    }
    fs::rename(&path, &target).map_err(|e| e.to_string())?;
    let canon_root = root.canonicalize().map_err(|e| e.to_string())?;
    Ok(rel_string(target.strip_prefix(&canon_root).map_err(|e| e.to_string())?))
}

/// Absolute path of a note, for "Open in Obsidian".
pub fn absolute(root: &Path, rel: &str) -> Result<PathBuf, String> {
    resolve(root, rel)
}

/// A file name Windows, macOS and Obsidian all accept. Obsidian also dislikes `# ^ [ ] |`.
pub fn file_stem_for(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| if matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '#' | '^' | '[' | ']') || c.is_control() { ' ' } else { c })
        .collect();
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut stem: String = collapsed.chars().take(100).collect();
    // Windows drops trailing dots and spaces, which would make the name change under us.
    while stem.ends_with('.') || stem.ends_with(' ') {
        stem.pop();
    }
    if stem.is_empty() || stem.starts_with('.') { format!("Untitled{stem}") } else { stem }
}

fn memo_marker(id: i64) -> String {
    format!("{MEMO_KEY}: {id}")
}

/// True when the note was written from any Niagara memo.
fn is_any_memo_note(text: &str) -> bool {
    let Some(rest) = text.strip_prefix("---") else { return false };
    rest.lines().skip(1).take_while(|l| l.trim() != "---").any(|l| l.trim_start().starts_with(MEMO_KEY))
}

/// True when the note's front matter says it was written from memo `id`.
fn is_memo_note(text: &str, id: i64) -> bool {
    let Some(rest) = text.strip_prefix("---") else { return false };
    let marker = memo_marker(id);
    rest.lines().skip(1).take_while(|l| l.trim() != "---").any(|l| l.trim() == marker)
}

fn timestamp(ts: i64) -> String {
    Local.timestamp_opt(ts, 0).single().map(|t| t.format("%Y-%m-%dT%H:%M").to_string()).unwrap_or_default()
}

/// Markdown files in the memo folder and its group sub-folders (two levels are enough).
fn memo_files(base: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut dirs = vec![(base.to_path_buf(), 0)];
    while let Some((dir, depth)) = dirs.pop() {
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && depth < 2 && !skipped_dir(&entry.file_name().to_string_lossy()) {
                dirs.push((path, depth + 1));
            } else if path.is_file() && is_markdown(&path) {
                out.push(path);
            }
        }
    }
    out
}

pub struct MemoNote<'a> {
    pub id: i64,
    pub title: &'a str,
    pub body: &'a str,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Writes a memo into `<vault>/<dir>` and returns the note's vault-relative path.
///
/// Sending the same memo again updates the note it wrote before (found by its front matter),
/// renaming it if the memo's title changed and the new name is free. Notes the user wrote
/// themselves are never overwritten: a clashing name gets a number instead.
pub fn export_memo(root: &Path, dir: &str, group: Option<&str>, memo: &MemoNote) -> Result<String, String> {
    let dir = dir.trim().trim_matches('/');
    let dir_path = Path::new(dir);
    if dir_path.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err("invalid folder".into());
    }
    if !root.is_dir() {
        return Err("vault folder not found".into());
    }
    // The memo folder; a memo in a group goes into a sub-folder named after the group.
    let base_dir = root.join(dir_path);
    let target_dir = match group.map(file_stem_for) {
        Some(g) if !g.is_empty() => base_dir.join(g),
        _ => base_dir.clone(),
    };
    fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;

    let stem = file_stem_for(memo.title);
    let content = format!(
        "---\n{}\ncreated: {}\nupdated: {}\nsource: Niagara Scheduler\n---\n\n{}\n",
        memo_marker(memo.id),
        timestamp(memo.created_at),
        timestamp(memo.updated_at),
        memo.body.trim_end()
    );

    // A note this memo wrote earlier, anywhere in the memo folder (it may have changed group).
    let previous = memo_files(&base_dir).into_iter().find(|p| fs::read_to_string(p).is_ok_and(|text| is_memo_note(&text, memo.id)));

    // Windows and macOS file names ignore letter case, so "Plan.md" and "plan.md" are one file.
    let same_file = |a: &Path, b: &Path| a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase();
    let mut path = None;
    for n in 1..1000 {
        let name = if n == 1 { format!("{stem}.md") } else { format!("{stem} {n}.md") };
        let candidate = target_dir.join(name);
        if previous.as_deref().is_some_and(|p| same_file(p, &candidate)) || !candidate.exists() {
            path = Some(candidate);
            break;
        }
    }
    let path = path.ok_or("too many notes with this name")?;
    if let Some(old) = previous.filter(|old| *old != path) {
        fs::rename(&old, &path).map_err(|e| e.to_string())?;
    }
    fs::write(&path, content).map_err(|e| e.to_string())?;
    let rel = path.strip_prefix(root).map_err(|e| e.to_string())?;
    Ok(rel_string(rel))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn vault() -> PathBuf {
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!("niagara_vault_{}_{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("10 공부")).unwrap();
        fs::create_dir_all(dir.join(".obsidian")).unwrap();
        fs::write(dir.join("Home.md"), "# Home\n[[SQL]]").unwrap();
        fs::write(dir.join("10 공부/SQL.md"), "JOIN").unwrap();
        fs::write(dir.join("10 공부/diagram.png"), b"png").unwrap();
        fs::write(dir.join("10 공부/lecture.txt"), "plain notes").unwrap();
        fs::write(dir.join(".obsidian/workspace.md"), "settings").unwrap();
        dir
    }

    fn memo<'a>(id: i64, title: &'a str, body: &'a str) -> MemoNote<'a> {
        MemoNote { id, title, body, created_at: 0, updated_at: 0 }
    }

    #[test]
    fn lists_markdown_notes_and_skips_settings_folders() {
        let root = vault();
        let mut paths: Vec<_> = list(&root).unwrap().into_iter().map(|n| (n.path, n.title, n.folder)).collect();
        assert_eq!(parse_read_dirs(Some(r#"["D:\\A", " ", "E:\\B"]"#)), vec!["D:\\A".to_string(), "E:\\B".to_string()]);
        assert_eq!(root_label("/x/Lecture", ""), "Lecture");
        assert_eq!(root_label("/x/Lecture", "w1"), "Lecture/w1");
        paths.sort();
        assert_eq!(
            paths,
            vec![
                ("10 공부/SQL.md".into(), "SQL".into(), "10 공부".into()),
                ("10 공부/lecture.txt".into(), "lecture".into(), "10 공부".into()),
                ("Home.md".into(), "Home".into(), String::new())
            ]
        );
    }

    #[test]
    fn reads_only_inside_the_vault() {
        let root = vault();
        assert_eq!(read(&root, "10 공부/SQL.md").unwrap(), "JOIN");
        assert_eq!(read(&root, "10 공부/lecture.txt").unwrap(), "plain notes");
        assert!(read(&root, "../outside.md").is_err());
        assert!(read(&root, "/etc/passwd").is_err());
        assert!(read(&root, "10 공부/diagram.png").is_err());
        assert!(read(&root, "missing.md").is_err());
    }

    #[test]
    fn notes_can_be_created_edited_and_switched_between_txt_and_md() {
        let root = vault();
        let rel = create(&root, "20 메모", "새 노트").unwrap();
        assert_eq!(rel, "20 메모/새 노트.md");
        assert_eq!(create(&root, "20 메모", "새 노트").unwrap(), "20 메모/새 노트 2.md");
        assert!(create(&root, "../out", "x").is_err());

        let t = write(&root, &rel, "# 제목", None).unwrap();
        assert_eq!(read(&root, &rel).unwrap(), "# 제목");
        // Changed elsewhere after the editor loaded it: refused.
        assert_eq!(write(&root, &rel, "old view", Some(t - 10)), Err("changed".into()));
        assert!(write(&root, &rel, "fresh", Some(t)).is_ok());

        assert_eq!(convert(&root, "10 공부/lecture.txt").unwrap(), "10 공부/lecture.md");
        assert_eq!(convert(&root, "10 공부/lecture.md").unwrap(), "10 공부/lecture.txt");
        assert_eq!(read(&root, "10 공부/lecture.txt").unwrap(), "plain notes");
        assert_eq!(convert(&root, "10 공부/SQL.md").ok(), Some("10 공부/SQL.txt".into()));
        fs::write(root.join("10 공부/SQL.md"), "dup").unwrap();
        assert_eq!(convert(&root, "10 공부/SQL.txt"), Err("exists".into()));
        assert!(write(&root, "../escape.md", "x", None).is_err());
    }

    #[test]
    fn export_creates_then_updates_and_renames_its_own_note() {
        let root = vault();
        let rel = export_memo(&root, "Niagara", None, &memo(7, "회의: 계획", "첫 내용")).unwrap();
        assert_eq!(rel, "Niagara/회의 계획.md");
        let text = fs::read_to_string(root.join(&rel)).unwrap();
        assert!(text.starts_with("---\nniagara_memo: 7\n"));
        assert!(text.ends_with("첫 내용\n"));

        // Same memo again: same file, new content.
        let again = export_memo(&root, "Niagara", None, &memo(7, "회의: 계획", "고친 내용")).unwrap();
        assert_eq!(again, rel);
        assert!(fs::read_to_string(root.join(&rel)).unwrap().contains("고친 내용"));

        // Title changed: the note is renamed, not duplicated.
        let renamed = export_memo(&root, "Niagara", None, &memo(7, "새 제목", "x")).unwrap();
        assert_eq!(renamed, "Niagara/새 제목.md");
        assert!(!root.join(&rel).exists());

        // Moved into a group: the same note moves into the group's sub-folder.
        let grouped = export_memo(&root, "Niagara", Some("자격증"), &memo(7, "새 제목", "x")).unwrap();
        assert_eq!(grouped, "Niagara/자격증/새 제목.md");
        assert!(!root.join("Niagara/새 제목.md").exists());
    }

    #[test]
    fn export_never_overwrites_a_users_own_note() {
        let root = vault();
        let rel = export_memo(&root, "10 공부", None, &memo(1, "SQL", "from memo")).unwrap();
        assert_eq!(rel, "10 공부/SQL 2.md");
        assert_eq!(fs::read_to_string(root.join("10 공부/SQL.md")).unwrap(), "JOIN");
        assert!(export_memo(&root, "../escape", None, &memo(2, "x", "y")).is_err());
    }

    #[test]
    fn file_names_are_safe() {
        assert_eq!(file_stem_for("a/b:c?"), "a b c");
        assert_eq!(file_stem_for("  "), "Untitled");
        assert_eq!(file_stem_for("end..."), "end");
        assert_eq!(file_stem_for(".hidden"), "Untitled.hidden");
    }
}
