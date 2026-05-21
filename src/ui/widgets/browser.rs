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

    /// Walk forward in the file list (skipping directories) and return the
    /// next module path, if any.
    pub fn next_module(&mut self, after: Option<&Path>) -> Option<PathBuf> {
        let start = match after {
            Some(p) => self.entries.iter().position(|e| !e.is_dir && e.path == p),
            None => self.state.selected(),
        };
        let start = start.unwrap_or(0);
        let n = self.entries.len();
        for step in 1..=n {
            let idx = (start + step) % n;
            let e = &self.entries[idx];
            if !e.is_dir {
                self.state.select(Some(idx));
                return Some(e.path.clone());
            }
        }
        None
    }

    pub fn prev_module(&mut self, before: Option<&Path>) -> Option<PathBuf> {
        let start = match before {
            Some(p) => self.entries.iter().position(|e| !e.is_dir && e.path == p),
            None => self.state.selected(),
        };
        let start = start.unwrap_or(0);
        let n = self.entries.len();
        for step in 1..=n {
            let idx = (start + n - step) % n;
            let e = &self.entries[idx];
            if !e.is_dir {
                self.state.select(Some(idx));
                return Some(e.path.clone());
            }
        }
        None
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
    let title = format!(" browser · {} ", browser.root.display());
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
