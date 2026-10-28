//! Where Niagara keeps its files.
//!
//! Everything lives under one base folder, sorted by who reads it:
//!
//! ```text
//! <base>\data\    the app's own files: database, images (only Niagara touches these)
//! <base>\brief\   files for other tools: brief.json and brief.schema.json
//! ```
//!
//! On Windows release builds the base is the folder the program is installed in, when that
//! folder can be written (e.g. `D:\Program Files\Niagara Scheduler`). The uninstaller only
//! removes the files it installed, so `data` and `brief` survive uninstalls and updates.
//! Otherwise (a read-only install folder such as `C:\Program Files`, other systems, development)
//! the base is the OS app-data folder. Data that already exists always decides, so the location
//! never flips between runs.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::backup::DB_FILE;

pub const DATA_DIR: &str = "data";
pub const BRIEF_DIR: &str = "brief";

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Storage {
    pub base: PathBuf,
    /// Database, images and staged imports.
    pub data: PathBuf,
    /// Files written for other tools to read.
    pub brief: PathBuf,
    /// True when `base` is the install folder.
    pub portable: bool,
}

impl Storage {
    fn under(base: &Path, portable: bool) -> Self {
        Storage { base: base.to_path_buf(), data: base.join(DATA_DIR), brief: base.join(BRIEF_DIR), portable }
    }
}

/// True when a file can be created (and removed) in `dir`.
fn writable(dir: &Path) -> bool {
    if fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".niagara-write-test");
    let ok = fs::write(&probe, b"").is_ok();
    let _ = fs::remove_file(&probe);
    ok
}

/// Picks the folders. `install_dir` is `None` when the install folder must not be used.
pub fn choose(install_dir: Option<&Path>, app_dir: &Path) -> Storage {
    let in_app_dir = Storage::under(app_dir, false);
    // Builds before 0.7 kept the database directly in the app-data folder.
    let legacy = app_dir.join(DB_FILE).exists() && !in_app_dir.data.exists();
    if legacy {
        return Storage { data: app_dir.to_path_buf(), ..in_app_dir };
    }
    if let Some(install) = install_dir {
        let portable = Storage::under(install, true);
        let installed_data_exists = portable.data.is_dir();
        let app_data_exists = in_app_dir.data.join(DB_FILE).exists();
        if installed_data_exists || (!app_data_exists && writable(&portable.data)) {
            return portable;
        }
    }
    in_app_dir
}

/// The folders for this run. `NORA_DATA_DIR` (development) points straight at the data folder.
pub fn resolve(app_dir: &Path) -> Storage {
    if let Some(dir) = std::env::var_os("NORA_DATA_DIR") {
        let data = PathBuf::from(dir);
        return Storage { base: data.clone(), brief: data.join(BRIEF_DIR), data, portable: false };
    }
    let install_dir = if cfg!(all(windows, not(debug_assertions))) {
        std::env::current_exe().ok().and_then(|exe| exe.parent().map(Path::to_path_buf))
    } else {
        None
    };
    choose(install_dir.as_deref(), app_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn temp(tag: &str) -> PathBuf {
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!("niagara_storage_{tag}_{}_{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_fresh_install_keeps_data_next_to_the_program() {
        let (install, app) = (temp("inst"), temp("app"));
        let s = choose(Some(&install), &app);
        assert!(s.portable);
        assert_eq!(s.data, install.join("data"));
        assert_eq!(s.brief, install.join("brief"));
        assert!(s.data.is_dir(), "created so the choice sticks");
    }

    #[test]
    fn existing_app_data_wins_over_an_empty_install_folder() {
        let (install, app) = (temp("inst"), temp("app"));
        fs::create_dir_all(app.join("data")).unwrap();
        fs::write(app.join("data").join(DB_FILE), b"").unwrap();
        let s = choose(Some(&install), &app);
        assert!(!s.portable);
        assert_eq!(s.data, app.join("data"));
        assert!(!install.join("data").exists(), "nothing created next to the program");
    }

    #[test]
    fn data_next_to_the_program_keeps_being_used() {
        let (install, app) = (temp("inst"), temp("app"));
        fs::create_dir_all(install.join("data")).unwrap();
        fs::create_dir_all(app.join("data")).unwrap();
        fs::write(app.join("data").join(DB_FILE), b"").unwrap();
        assert!(choose(Some(&install), &app).portable);
    }

    #[test]
    fn without_an_install_folder_everything_goes_to_app_data() {
        let app = temp("app");
        let s = choose(None, &app);
        assert_eq!(s, Storage::under(&app, false));
    }

    #[test]
    fn databases_from_older_builds_are_kept_where_they_are() {
        let (install, app) = (temp("inst"), temp("app"));
        fs::write(app.join(DB_FILE), b"").unwrap();
        let s = choose(Some(&install), &app);
        assert!(!s.portable);
        assert_eq!(s.data, app);
        assert_eq!(s.brief, app.join("brief"));
    }
}
