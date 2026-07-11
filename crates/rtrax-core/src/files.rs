//! Module-file discovery: which files are playable, and directory scans.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Extensions libopenmpt handles that we surface in browsers/queues. Not the
/// full libopenmpt list — just the formats worth showing by default.
pub const MODULE_EXTS: &[&str] = &["mod", "xm", "it", "s3m", "mtm", "mptm", "stm", "ult"];

/// Whether `path` looks like a module file, by extension (case-insensitive).
pub fn is_module(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(OsStr::to_str) else {
        return false;
    };
    let ext = ext.to_ascii_lowercase();
    MODULE_EXTS.contains(&ext.as_str())
}

/// All module files directly inside `dir` (non-recursive), sorted by
/// case-insensitive file name. Unreadable directories yield an empty list.
pub fn modules_in_dir(dir: &Path) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = read
        .flatten()
        .map(|ent| ent.path())
        .filter(|p| p.is_file() && is_module(p))
        .collect();
    files.sort_by_key(|p| {
        p.file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    });
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn module_extension_matching_is_case_insensitive() {
        assert!(is_module(Path::new("song.xm")));
        assert!(is_module(Path::new("SONG.XM")));
        assert!(is_module(Path::new("song.It")));
        assert!(!is_module(Path::new("song.txt")));
        assert!(!is_module(Path::new("song")));
    }

    #[test]
    fn modules_in_dir_filters_and_sorts() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["b.xm", "A.mod", "notes.txt", "c.s3m"] {
            fs::write(dir.path().join(name), b"").unwrap();
        }
        fs::create_dir(dir.path().join("sub.xm")).unwrap(); // dir, not a file

        let got: Vec<String> = modules_in_dir(dir.path())
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(got, vec!["A.mod", "b.xm", "c.s3m"]);
    }

    #[test]
    fn modules_in_dir_unreadable_is_empty() {
        assert!(modules_in_dir(Path::new("/definitely/not/here")).is_empty());
    }
}
