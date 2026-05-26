//! M3U playlist — a list of module file paths with an optional backing file.
//!
//! The format is plain-text M3U: one path per line, lines starting with `#`
//! are treated as comments/metadata and ignored when reading. Relative paths
//! in a loaded file are resolved against the playlist's parent directory.

use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct Playlist {
    pub entries: Vec<PathBuf>,
    /// Backing file path, if this playlist was loaded from or targeted at a file.
    pub path: Option<PathBuf>,
}

impl Playlist {
    /// Build an in-memory playlist from a list of paths (no backing file).
    pub fn from_files(files: Vec<PathBuf>) -> Self {
        let entries = files.into_iter().map(make_absolute).collect();
        Self {
            entries,
            path: None,
        }
    }

    /// Load an M3U file. Comment lines (`#`-prefixed) are skipped.
    /// Relative paths are resolved against the playlist file's parent directory.
    pub fn load(path: PathBuf) -> Result<Self> {
        let text = fs::read_to_string(&path)
            .with_context(|| format!("reading playlist {}", path.display()))?;
        let base = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let entries = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| {
                let p = PathBuf::from(l);
                if p.is_absolute() {
                    p
                } else {
                    base.join(p)
                }
            })
            .collect();
        Ok(Self {
            entries,
            path: Some(path),
        })
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn first(&self) -> Option<&PathBuf> {
        self.entries.first()
    }

    /// Entry immediately after `current` in the list, or `None`.
    pub fn next_after(&self, current: &Path) -> Option<PathBuf> {
        let idx = self.entries.iter().position(|e| paths_equal(e, current))?;
        self.entries.get(idx + 1).cloned()
    }

    /// Entry immediately before `current` in the list, or `None`.
    pub fn prev_before(&self, current: &Path) -> Option<PathBuf> {
        let idx = self.entries.iter().position(|e| paths_equal(e, current))?;
        idx.checked_sub(1)
            .and_then(|i| self.entries.get(i))
            .cloned()
    }
}

/// Check whether the playlist file at `playlist_path` already contains `entry`.
/// Returns `false` if the file does not exist or cannot be read — callers should
/// treat that as "not a duplicate" and proceed with the append.
pub fn file_contains(entry: &Path, playlist_path: &Path) -> bool {
    let Ok(text) = fs::read_to_string(playlist_path) else {
        return false;
    };
    let base = playlist_path
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .any(|l| {
            let p = PathBuf::from(l);
            let abs = if p.is_absolute() { p } else { base.join(p) };
            paths_equal(&abs, entry)
        })
}

/// Append a single path to a playlist file, creating it (with `#EXTM3U` header)
/// if it does not yet exist. Also creates parent directories as needed.
pub fn append_to_file(entry: &Path, playlist_path: &Path) -> Result<()> {
    if let Some(parent) = playlist_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating playlist directory {}", parent.display()))?;
    }
    let needs_header = !playlist_path.exists();
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(playlist_path)
        .with_context(|| format!("opening playlist {}", playlist_path.display()))?;
    if needs_header {
        writeln!(f, "#EXTM3U")?;
    }
    writeln!(f, "{}", entry.display())?;
    Ok(())
}

/// Default path for the user's persistent playlist.
pub fn default_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("rtrax").join("playlist.m3u"))
}

fn make_absolute(p: PathBuf) -> PathBuf {
    if p.is_absolute() {
        p
    } else {
        std::env::current_dir().map(|d| d.join(&p)).unwrap_or(p)
    }
}

/// Path equality that handles minor casing/normalization differences on
/// case-insensitive file systems; for our purposes a simple == is fine on
/// Linux, and on macOS we lower-case both sides.
fn paths_equal(a: &Path, b: &Path) -> bool {
    a == b || a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // ── from_files ──────────────────────────────────────────────────────────

    #[test]
    fn from_files_absolute_stays_absolute() {
        let abs = PathBuf::from("/tmp/song.mod");
        let pl = Playlist::from_files(vec![abs.clone()]);
        assert_eq!(pl.entries[0], abs);
        assert!(pl.path.is_none());
    }

    #[test]
    fn from_files_relative_becomes_absolute() {
        let rel = PathBuf::from("song.mod");
        let pl = Playlist::from_files(vec![rel]);
        assert!(pl.entries[0].is_absolute());
    }

    #[test]
    fn is_empty_reflects_entries() {
        let empty = Playlist::from_files(vec![]);
        assert!(empty.is_empty());
        let nonempty = Playlist::from_files(vec![PathBuf::from("/a.mod")]);
        assert!(!nonempty.is_empty());
    }

    // ── load ────────────────────────────────────────────────────────────────

    #[test]
    fn load_skips_comments_and_blank_lines() {
        let dir = tempfile::tempdir().unwrap();
        let pl_path = dir.path().join("list.m3u");
        fs::write(
            &pl_path,
            "#EXTM3U\n# a comment\n\n/abs/song.mod\n\n# another\n/abs/other.xm\n",
        )
        .unwrap();

        let pl = Playlist::load(pl_path).unwrap();
        assert_eq!(pl.entries.len(), 2);
        assert_eq!(pl.entries[0], PathBuf::from("/abs/song.mod"));
        assert_eq!(pl.entries[1], PathBuf::from("/abs/other.xm"));
    }

    #[test]
    fn load_resolves_relative_paths_against_playlist_dir() {
        let dir = tempfile::tempdir().unwrap();
        let pl_path = dir.path().join("list.m3u");
        fs::write(&pl_path, "song.mod\n").unwrap();

        let pl = Playlist::load(pl_path.clone()).unwrap();
        assert_eq!(pl.entries[0], dir.path().join("song.mod"));
        assert_eq!(pl.path, Some(pl_path));
    }

    #[test]
    fn load_keeps_absolute_paths() {
        let dir = tempfile::tempdir().unwrap();
        let pl_path = dir.path().join("list.m3u");
        fs::write(&pl_path, "/music/song.mod\n").unwrap();

        let pl = Playlist::load(pl_path).unwrap();
        assert_eq!(pl.entries[0], PathBuf::from("/music/song.mod"));
    }

    // ── navigation ──────────────────────────────────────────────────────────

    fn three_entry_playlist() -> Playlist {
        Playlist::from_files(vec![
            PathBuf::from("/a.mod"),
            PathBuf::from("/b.mod"),
            PathBuf::from("/c.mod"),
        ])
    }

    #[test]
    fn next_after_returns_following_entry() {
        let pl = three_entry_playlist();
        assert_eq!(
            pl.next_after(Path::new("/a.mod")),
            Some(pl.entries[1].clone())
        );
        assert_eq!(
            pl.next_after(Path::new("/b.mod")),
            Some(pl.entries[2].clone())
        );
    }

    #[test]
    fn next_after_returns_none_at_last_entry() {
        let pl = three_entry_playlist();
        assert_eq!(pl.next_after(&pl.entries[2].clone()), None);
    }

    #[test]
    fn next_after_returns_none_when_not_found() {
        let pl = three_entry_playlist();
        assert_eq!(pl.next_after(Path::new("/not-in-list.mod")), None);
    }

    #[test]
    fn prev_before_returns_preceding_entry() {
        let pl = three_entry_playlist();
        assert_eq!(
            pl.prev_before(Path::new("/b.mod")),
            Some(pl.entries[0].clone())
        );
        assert_eq!(
            pl.prev_before(Path::new("/c.mod")),
            Some(pl.entries[1].clone())
        );
    }

    #[test]
    fn prev_before_returns_none_at_first_entry() {
        let pl = three_entry_playlist();
        assert_eq!(pl.prev_before(&pl.entries[0].clone()), None);
    }

    #[test]
    fn prev_before_returns_none_when_not_found() {
        let pl = three_entry_playlist();
        assert_eq!(pl.prev_before(Path::new("/not-in-list.mod")), None);
    }

    #[test]
    fn first_returns_first_entry() {
        let pl = three_entry_playlist();
        assert_eq!(pl.first(), Some(&pl.entries[0]));
        let empty = Playlist::from_files(vec![]);
        assert!(empty.first().is_none());
    }

    // ── append_to_file ──────────────────────────────────────────────────────

    #[test]
    fn append_to_file_creates_with_extm3u_header() {
        let dir = tempfile::tempdir().unwrap();
        let pl_path = dir.path().join("new.m3u");

        append_to_file(Path::new("/song.mod"), &pl_path).unwrap();

        let text = fs::read_to_string(&pl_path).unwrap();
        assert!(text.starts_with("#EXTM3U\n"), "missing header: {text:?}");
        assert!(text.contains("/song.mod"));
    }

    #[test]
    fn append_to_file_no_duplicate_header_on_second_append() {
        let dir = tempfile::tempdir().unwrap();
        let pl_path = dir.path().join("list.m3u");

        append_to_file(Path::new("/a.mod"), &pl_path).unwrap();
        append_to_file(Path::new("/b.mod"), &pl_path).unwrap();

        let text = fs::read_to_string(&pl_path).unwrap();
        assert_eq!(
            text.matches("#EXTM3U").count(),
            1,
            "header duplicated:\n{text}"
        );
        assert!(text.contains("/a.mod"));
        assert!(text.contains("/b.mod"));
    }

    // ── file_contains ───────────────────────────────────────────────────────

    #[test]
    fn file_contains_returns_false_for_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let pl_path = dir.path().join("absent.m3u");
        assert!(!file_contains(Path::new("/song.mod"), &pl_path));
    }

    #[test]
    fn file_contains_detects_absolute_entry() {
        let dir = tempfile::tempdir().unwrap();
        let pl_path = dir.path().join("list.m3u");
        fs::write(&pl_path, "#EXTM3U\n/music/song.mod\n").unwrap();

        assert!(file_contains(Path::new("/music/song.mod"), &pl_path));
        assert!(!file_contains(Path::new("/music/other.mod"), &pl_path));
    }

    #[test]
    fn file_contains_resolves_relative_entries_against_playlist_dir() {
        let dir = tempfile::tempdir().unwrap();
        let pl_path = dir.path().join("list.m3u");
        fs::write(&pl_path, "song.mod\n").unwrap();

        let abs = dir.path().join("song.mod");
        assert!(file_contains(&abs, &pl_path));
    }

    #[test]
    fn file_contains_skips_comments_and_blank_lines() {
        let dir = tempfile::tempdir().unwrap();
        let pl_path = dir.path().join("list.m3u");
        fs::write(&pl_path, "#EXTM3U\n# /commented.mod\n\n/real.mod\n").unwrap();

        assert!(!file_contains(Path::new("/commented.mod"), &pl_path));
        assert!(file_contains(Path::new("/real.mod"), &pl_path));
    }

    #[test]
    fn append_to_file_creates_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let pl_path = dir.path().join("nested").join("dir").join("list.m3u");

        append_to_file(Path::new("/song.mod"), &pl_path).unwrap();

        assert!(pl_path.exists());
    }
}
