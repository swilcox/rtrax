//! File browser. Lists module files in the current directory, plus a recent
//! files / playlist area. Loaded paths are returned to the App.

use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

const MODULE_EXTS: &[&str] = &["mod", "xm", "it", "s3m", "mtm", "mptm", "stm", "ult"];

pub struct Browser {
    pub root: PathBuf,
    pub entries: Vec<Entry>,
    pub state: ListState,
    /// Play order over the *module* entries (indices into `entries`, skipping
    /// directories). Natural order when not shuffled. `next_module` /
    /// `prev_module` step through this.
    order: Vec<usize>,
    /// Whether `order` is shuffled. Preserved across directory changes.
    shuffle: bool,
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub is_dir: bool,
    pub label: String,
}

impl Browser {
    pub fn new(root: PathBuf) -> Self {
        let mut b = Self {
            root,
            entries: Vec::new(),
            state: ListState::default(),
            order: Vec::new(),
            shuffle: false,
        };
        b.refresh();
        b.state
            .select(if b.entries.is_empty() { None } else { Some(0) });
        b
    }

    pub fn refresh(&mut self) {
        self.entries.clear();
        if let Some(parent) = self.root.parent() {
            self.entries.push(Entry {
                path: parent.to_path_buf(),
                is_dir: true,
                label: "..".into(),
            });
        }
        let read = match std::fs::read_dir(&self.root) {
            Ok(rd) => rd,
            Err(_) => return,
        };
        let mut dirs: Vec<Entry> = Vec::new();
        let mut files: Vec<Entry> = Vec::new();
        for ent in read.flatten() {
            let path = ent.path();
            let is_dir = ent.file_type().map(|f| f.is_dir()).unwrap_or(false);
            if is_dir {
                if path
                    .file_name()
                    .map(|n| n.to_string_lossy().starts_with('.'))
                    .unwrap_or(false)
                {
                    continue;
                }
                dirs.push(Entry {
                    label: path
                        .file_name()
                        .map(|n| format!("{}/", n.to_string_lossy()))
                        .unwrap_or_default(),
                    path,
                    is_dir: true,
                });
            } else if is_module(&path) {
                files.push(Entry {
                    label: path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    path,
                    is_dir: false,
                });
            }
        }
        dirs.sort_by(|a, b| a.label.cmp(&b.label));
        files.sort_by_key(|a| a.label.to_lowercase());
        self.entries.extend(dirs);
        self.entries.extend(files);
        self.rebuild_order();
    }

    /// Rebuild the module play order from the current entries, honoring the
    /// shuffle flag. Called after every directory refresh and on toggle.
    fn rebuild_order(&mut self) {
        let file_indices: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| !e.is_dir)
            .map(|(i, _)| i)
            .collect();
        self.order = if self.shuffle {
            let mut rng = crate::rng::Rng::from_clock();
            crate::rng::permutation(file_indices.len(), &mut rng)
                .into_iter()
                .map(|i| file_indices[i])
                .collect()
        } else {
            file_indices
        };
    }

    /// Toggle shuffle for folder playback; rebuilds the order in place.
    pub fn set_shuffle(&mut self, on: bool) {
        self.shuffle = on;
        self.rebuild_order();
    }

    pub fn is_shuffled(&self) -> bool {
        self.shuffle
    }

    pub fn select_delta(&mut self, delta: i32) {
        if self.entries.is_empty() {
            return;
        }
        let cur = self.state.selected().unwrap_or(0) as i32;
        let new = (cur + delta).rem_euclid(self.entries.len() as i32) as usize;
        self.state.select(Some(new));
    }

    pub fn selected(&self) -> Option<&Entry> {
        self.state.selected().and_then(|i| self.entries.get(i))
    }

    /// Next module in play order (shuffled or natural), wrapping around the
    /// folder. Returns `None` only when the folder has no modules.
    pub fn next_module(&mut self, after: Option<&Path>) -> Option<PathBuf> {
        self.step_module(after, 1)
    }

    /// Previous module in play order, wrapping around the folder.
    pub fn prev_module(&mut self, before: Option<&Path>) -> Option<PathBuf> {
        self.step_module(before, -1)
    }

    /// Step `dir` (+1 / -1) positions through the module play order, anchored on
    /// `from` (the playing track) or the current selection, and select + return
    /// the landed module.
    fn step_module(&mut self, from: Option<&Path>, dir: i32) -> Option<PathBuf> {
        let len = self.order.len();
        if len == 0 {
            return None;
        }
        // Where are we in the play order? Prefer the playing track, else the
        // current selection, else the start.
        let entry_idx = match from {
            Some(p) => self.entries.iter().position(|e| !e.is_dir && e.path == p),
            None => self.state.selected(),
        };
        let pos = entry_idx
            .and_then(|ei| self.order.iter().position(|&x| x == ei))
            .unwrap_or(0);
        let next_pos = (pos as i32 + dir).rem_euclid(len as i32) as usize;
        let idx = self.order[next_pos];
        self.state.select(Some(idx));
        Some(self.entries[idx].path.clone())
    }

    /// Activate the current selection. Returns a path if a module was chosen;
    /// returns `None` if a directory was entered or nothing was selected.
    pub fn activate(&mut self) -> Option<PathBuf> {
        let entry = self.selected()?.clone();
        if entry.is_dir {
            self.root = entry.path;
            self.refresh();
            self.state.select(if self.entries.is_empty() {
                None
            } else {
                Some(0)
            });
            None
        } else {
            Some(entry.path)
        }
    }
}

fn is_module(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(OsStr::to_str) else {
        return false;
    };
    let ext = ext.to_ascii_lowercase();
    MODULE_EXTS.contains(&ext.as_str())
}

pub fn render(f: &mut Frame, area: Rect, browser: &mut Browser, theme: &Theme, focused: bool) {
    let title = if browser.is_shuffled() {
        format!(" browser · {} · ⤮ shuffle ", browser.root.display())
    } else {
        format!(" browser · {} ", browser.root.display())
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if focused {
            theme.border_focus
        } else {
            theme.border
        }))
        .title(Span::styled(title, theme.dim_style()));

    let items: Vec<ListItem> = browser
        .entries
        .iter()
        .map(|e| {
            let style = if e.is_dir {
                Style::default().fg(theme.instrument)
            } else {
                Style::default().fg(theme.fg)
            };
            ListItem::new(Line::from(Span::styled(e.label.clone(), style)))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(theme.current_row_bg)
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut browser.state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("rtrax-{name}-{stamp}"));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn entry(label: &str, is_dir: bool) -> Entry {
        Entry {
            path: PathBuf::from(label),
            is_dir,
            label: label.to_string(),
        }
    }

    #[test]
    fn module_extension_matching_is_case_insensitive() {
        assert!(is_module(Path::new("song.XM")));
        assert!(is_module(Path::new("song.mod")));
        assert!(!is_module(Path::new("song.txt")));
        assert!(!is_module(Path::new("song")));
    }

    #[test]
    fn refresh_lists_dirs_first_then_modules() {
        let root = temp_root("browser-refresh");
        fs::create_dir(root.join("z-dir")).unwrap();
        fs::create_dir(root.join("a-dir")).unwrap();
        fs::create_dir(root.join(".hidden")).unwrap();
        fs::write(root.join("b.xm"), []).unwrap();
        fs::write(root.join("A.MOD"), []).unwrap();
        fs::write(root.join("notes.txt"), []).unwrap();

        let browser = Browser::new(root.clone());
        let labels: Vec<&str> = browser
            .entries
            .iter()
            .map(|entry| entry.label.as_str())
            .collect();

        assert_eq!(labels, ["..", "a-dir/", "z-dir/", "A.MOD", "b.xm"]);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn selection_wraps_in_both_directions() {
        let mut browser = Browser {
            root: PathBuf::from("."),
            entries: vec![entry("a.xm", false), entry("b.xm", false)],
            state: ListState::default(),
            order: vec![0, 1],
            shuffle: false,
        };
        browser.state.select(Some(0));

        browser.select_delta(-1);
        assert_eq!(browser.state.selected(), Some(1));

        browser.select_delta(1);
        assert_eq!(browser.state.selected(), Some(0));
    }

    #[test]
    fn next_and_previous_skip_directories() {
        let mut browser = Browser {
            root: PathBuf::from("."),
            entries: vec![
                entry("dir", true),
                entry("a.xm", false),
                entry("b.xm", false),
            ],
            state: ListState::default(),
            order: vec![1, 2],
            shuffle: false,
        };

        assert_eq!(
            browser.next_module(Some(Path::new("a.xm"))),
            Some(PathBuf::from("b.xm"))
        );
        assert_eq!(
            browser.prev_module(Some(Path::new("a.xm"))),
            Some(PathBuf::from("b.xm"))
        );
    }
}
