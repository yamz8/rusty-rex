//! Keeps the high score on disk between runs.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Where the high score lives, following the XDG base directory spec:
/// `$XDG_DATA_HOME/rusty-rex/high-score`, falling back to
/// `~/.local/share/rusty-rex/high-score`. Takes the variables as arguments so
/// tests don't have to change the real environment.
pub fn high_score_path(xdg_data_home: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    // The spec says a relative XDG_DATA_HOME is invalid and must be ignored.
    let data = xdg_data_home
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| home.map(|h| PathBuf::from(h).join(".local/share")))?;
    Some(data.join("rusty-rex").join("high-score"))
}

/// The path for the current user, or None when even HOME is unset.
pub fn default_high_score_path() -> Option<PathBuf> {
    high_score_path(std::env::var_os("XDG_DATA_HOME"), std::env::var_os("HOME"))
}

/// Reads the saved high score. A missing or garbled file counts as no high
/// score yet rather than an error, so a bad file can't stop the game starting.
pub fn load(path: &Path) -> u32 {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

/// Writes to a temporary file and renames it over the old one, so a crash
/// mid-write can't leave a half-written score behind.
pub fn save(path: &Path, score: u32) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, format!("{score}\n"))?;
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh directory per test, so tests can run in parallel.
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("rusty-rex-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn prefers_xdg_data_home() {
        let path = high_score_path(Some("/data".into()), Some("/home/rex".into()));
        assert_eq!(path, Some(PathBuf::from("/data/rusty-rex/high-score")));
    }

    #[test]
    fn falls_back_to_local_share() {
        let expected = Some(PathBuf::from("/home/rex/.local/share/rusty-rex/high-score"));
        assert_eq!(high_score_path(None, Some("/home/rex".into())), expected);
        assert_eq!(
            high_score_path(Some("relative".into()), Some("/home/rex".into())),
            expected
        );
    }

    #[test]
    fn no_home_means_nowhere_to_save() {
        assert_eq!(high_score_path(None, None), None);
    }

    #[test]
    fn saves_and_loads_creating_the_directory() {
        let dir = scratch("round-trip");
        let path = dir.join("rusty-rex").join("high-score");
        save(&path, 1234).unwrap();
        assert_eq!(load(&path), 1234);
        save(&path, 99).unwrap();
        assert_eq!(load(&path), 99);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_or_garbled_files_load_as_zero() {
        let dir = scratch("garbled");
        let path = dir.join("high-score");
        assert_eq!(load(&path), 0);
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, "not a number").unwrap();
        assert_eq!(load(&path), 0);
        fs::remove_dir_all(dir).unwrap();
    }
}
