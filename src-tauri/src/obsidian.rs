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

/// Setting that stores the vault folder (absolute path).
pub const VAULT_KEY: &str = "obsidian.vault";
/// Setting that stores the vault-relative folder memos are written to.
pub const EXPORT_DIR_KEY: &str = "obsidian.exportDir";
pub const DEFAULT_EXPORT_DIR: &str = "Niagara";

/// Big vaults are fine, but the list stays bounded.
const MAX_NOTES: usize = 5000;
/// Notes larger than this are not opened (they are almost never hand-written notes).
const MAX_NOTE_BYTES: u64 = 2 * 1024 * 1024;
/// Front matter key that marks a note written from a memo, so re-sending updates it.
const MEMO_KEY: &str = "niagara_memo";

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct NoteMeta {
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
            } else if kind.is_file() && is_markdown(&path) {
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
                notes.push(NoteMeta { path: rel_path, title, folder, modified, size: meta.len() });
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
    if !is_markdown(&path) {
        return Err("not a Markdown note".into());
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
        .filter(|n| n.size <= MAX_NOTE_BYTES)
        .filter_map(|n| {
            let text = fs::read(root.join(&n.path)).ok()?;
            let text = String::from_utf8_lossy(&text);
            if !crate::cloze::has_study_tag(&text) {
                return None;
            }
            let count = crate::cloze::count_bold(&text);
            (count > 0).then_some((n, count))
        })
        .collect())
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

/// True when the note's front matter says it was written from memo `id`.
fn is_memo_note(text: &str, id: i64) -> bool {
    let Some(rest) = text.strip_prefix("---") else { return false };
    let marker = memo_marker(id);
    rest.lines().skip(1).take_while(|l| l.trim() != "---").any(|l| l.trim() == marker)
}

fn timestamp(ts: i64) -> String {
    Local.timestamp_opt(ts, 0).single().map(|t| t.format("%Y-%m-%dT%H:%M").to_string()).unwrap_or_default()
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
pub fn export_memo(root: &Path, dir: &str, memo: &MemoNote) -> Result<String, String> {
    let dir = dir.trim().trim_matches('/');
    let dir_path = Path::new(dir);
    if dir_path.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err("invalid folder".into());
    }
    if !root.is_dir() {
        return Err("vault folder not found".into());
    }
    let target_dir = root.join(dir_path);
    fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;

    let stem = file_stem_for(memo.title);
    let content = format!(
        "---\n{}\ncreated: {}\nupdated: {}\nsource: Niagara Scheduler\n---\n\n{}\n",
        memo_marker(memo.id),
        timestamp(memo.created_at),
        timestamp(memo.updated_at),
        memo.body.trim_end()
    );

    // A note this memo wrote earlier, if any.
    let previous = fs::read_dir(&target_dir)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_markdown(p))
        .find(|p| fs::read_to_string(p).is_ok_and(|text| is_memo_note(&text, memo.id)));

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
        paths.sort();
        assert_eq!(
            paths,
            vec![("10 공부/SQL.md".into(), "SQL".into(), "10 공부".into()), ("Home.md".into(), "Home".into(), String::new())]
        );
    }

    #[test]
    fn reads_only_inside_the_vault() {
        let root = vault();
        assert_eq!(read(&root, "10 공부/SQL.md").unwrap(), "JOIN");
        assert!(read(&root, "../outside.md").is_err());
        assert!(read(&root, "/etc/passwd").is_err());
        assert!(read(&root, "10 공부/diagram.png").is_err());
        assert!(read(&root, "missing.md").is_err());
    }

    #[test]
    fn export_creates_then_updates_and_renames_its_own_note() {
        let root = vault();
        let rel = export_memo(&root, "Niagara", &memo(7, "회의: 계획", "첫 내용")).unwrap();
        assert_eq!(rel, "Niagara/회의 계획.md");
        let text = fs::read_to_string(root.join(&rel)).unwrap();
        assert!(text.starts_with("---\nniagara_memo: 7\n"));
        assert!(text.ends_with("첫 내용\n"));

        // Same memo again: same file, new content.
        let again = export_memo(&root, "Niagara", &memo(7, "회의: 계획", "고친 내용")).unwrap();
        assert_eq!(again, rel);
        assert!(fs::read_to_string(root.join(&rel)).unwrap().contains("고친 내용"));

        // Title changed: the note is renamed, not duplicated.
        let renamed = export_memo(&root, "Niagara", &memo(7, "새 제목", "x")).unwrap();
        assert_eq!(renamed, "Niagara/새 제목.md");
        assert!(!root.join(&rel).exists());
    }

    #[test]
    fn export_never_overwrites_a_users_own_note() {
        let root = vault();
        let rel = export_memo(&root, "10 공부", &memo(1, "SQL", "from memo")).unwrap();
        assert_eq!(rel, "10 공부/SQL 2.md");
        assert_eq!(fs::read_to_string(root.join("10 공부/SQL.md")).unwrap(), "JOIN");
        assert!(export_memo(&root, "../escape", &memo(2, "x", "y")).is_err());
    }

    #[test]
    fn file_names_are_safe() {
        assert_eq!(file_stem_for("a/b:c?"), "a b c");
        assert_eq!(file_stem_for("  "), "Untitled");
        assert_eq!(file_stem_for("end..."), "end");
        assert_eq!(file_stem_for(".hidden"), "Untitled.hidden");
    }
}
