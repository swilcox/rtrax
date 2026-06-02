//! Queue view. Shown in the left panel when rtrax is in "play the playlist"
//! mode: it lists the active playlist's entries, marks the now-playing track,
//! and carries its own selection cursor so Enter can jump straight to a track.

use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;
use std::path::{Path, PathBuf};

/// Short display label for a queue entry — the file name, falling back to the
/// full path if there's no file name component.
fn label_for(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

pub fn render(
    f: &mut Frame,
    area: Rect,
    entries: &[PathBuf],
    now_playing: Option<&Path>,
    selected: usize,
    theme: &Theme,
    focused: bool,
) {
    let title = format!(" queue · {} tracks ", entries.len());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if focused {
            theme.border_focus
        } else {
            theme.border
        }))
        .title(Span::styled(title, theme.dim_style()));

    let items: Vec<ListItem> = entries
        .iter()
        .map(|path| {
            let is_playing = now_playing.is_some_and(|np| paths_equal(np, path));
            // A leading marker column keeps now-playing and other rows aligned.
            let (marker, style) = if is_playing {
                (
                    "♪ ",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ("  ", Style::default().fg(theme.fg))
            };
            ListItem::new(Line::from(vec![
                Span::styled(marker, style),
                Span::styled(label_for(path), style),
            ]))
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

    let mut state = ListState::default();
    if !entries.is_empty() {
        state.select(Some(selected.min(entries.len() - 1)));
    }
    f.render_stateful_widget(list, area, &mut state);
}

/// Mirror of the playlist's path comparison: exact match, or case-insensitive
/// on case-insensitive file systems.
fn paths_equal(a: &Path, b: &Path) -> bool {
    a == b || a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_uses_file_name() {
        assert_eq!(label_for(Path::new("/music/songs/cool.xm")), "cool.xm");
        assert_eq!(label_for(Path::new("bare.mod")), "bare.mod");
    }

    #[test]
    fn paths_equal_is_case_insensitive() {
        assert!(paths_equal(
            Path::new("/A/Song.XM"),
            Path::new("/a/song.xm")
        ));
        assert!(!paths_equal(Path::new("/a/one.xm"), Path::new("/a/two.xm")));
    }
}
