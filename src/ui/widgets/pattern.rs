//! Pattern view. Renders the rows surrounding the current row with the
//! current row centered, dim cells for empty data, and per-token coloring.

use crate::state::pattern::PatternWindow;
use crate::state::SharedState;
use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn render(f: &mut Frame, area: Rect, state: &SharedState, theme: &Theme, focused: bool) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if focused {
            theme.border_focus
        } else {
            theme.border
        }))
        .title(Span::styled(" pattern ", theme.dim_style()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let window = match state.pattern_window.try_lock() {
        Ok(g) => g.clone(),
        Err(_) => PatternWindow::default(),
    };

    if window.rows.is_empty() || window.channel_count == 0 {
        let msg = Paragraph::new(Line::from(Span::styled(
            "no pattern data — load a module to begin",
            theme.dim_style(),
        )));
        f.render_widget(msg, inner);
        return;
    }

    let visible_rows = inner.height as usize;
    let total_rows = window.rows.len();
    let center = window.current_index;
    let half = visible_rows / 2;
    let start = center.saturating_sub(half);
    let end = (start + visible_rows).min(total_rows);

    // How many channels we can show in the visible width. Cell width depends
    // on libopenmpt's natural format, but we use a safe upper bound of 14
    // chars + 1 separator.
    let cell_w = 14usize;
    let sep_w = 1usize;
    let row_label_w = 4usize; // "▶ 23 "
    let available_chan_w = inner.width as usize - row_label_w.min(inner.width as usize);
    let max_channels = (available_chan_w / (cell_w + sep_w)).max(1);
    let channels_shown = window.channel_count.min(max_channels);

    let mut lines: Vec<Line> = Vec::with_capacity(end - start);
    for (i, row) in window.rows[start..end].iter().enumerate() {
        let absolute_i = start + i;
        let is_current = absolute_i == center;
        let row_label = if row.row_index < 0 {
            "    ".to_string()
        } else {
            format!("{:>3} ", row.row_index)
        };
        let prefix_style = if is_current {
            theme.accent_style()
        } else {
            theme.dim_style()
        };
        let mut spans: Vec<Span> = Vec::with_capacity(channels_shown * 2 + 2);
        let marker = if is_current { "▶" } else { " " };
        spans.push(Span::styled(format!("{marker} "), prefix_style));
        spans.push(Span::styled(row_label, theme.dim_style()));

        for (ci, cell) in row.cells.iter().take(channels_shown).enumerate() {
            if ci > 0 {
                spans.push(Span::styled("│", theme.dim_style()));
            }
            let s: &str = if cell.trim().is_empty() {
                "... .. .. ..."
            } else {
                cell.as_str()
            };
            for (idx, ch) in s.chars().enumerate() {
                let style = classify(ch, idx, theme, is_current);
                spans.push(Span::styled(ch.to_string(), style));
            }
        }
        let line = if is_current {
            Line::from(spans).style(Style::default().bg(theme.current_row_bg))
        } else {
            Line::from(spans)
        };
        lines.push(line);
    }

    let para = Paragraph::new(lines);
    f.render_widget(para, inner);
}

/// Cheap per-character classifier. libopenmpt's `get_formatted(0,false)` lays
/// out a cell like `C-5 01 v40 A20` — the first 3 chars are note, next 2 inst,
/// then volume effect (3), then effect (3). Spaces between them. We split on
/// position rather than parsing.
fn classify(ch: char, idx: usize, theme: &Theme, current: bool) -> Style {
    if ch == '.' || ch == ' ' {
        return Style::default().fg(theme.fg_dim);
    }
    let color = match idx {
        0..=2 => theme.note,
        3 => theme.fg_dim,
        4..=5 => theme.instrument,
        6 => theme.fg_dim,
        7..=9 => theme.volume,
        10 => theme.fg_dim,
        11..=13 => theme.effect,
        _ => theme.fg,
    };
    let mut s = Style::default().fg(color);
    if current {
        s = s.add_modifier(Modifier::BOLD);
    }
    s
}
