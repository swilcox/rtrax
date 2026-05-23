//! Pattern view. Renders the rows surrounding the current row with the
//! current row centered, dim cells for empty data, and per-token coloring.
//!
//! When `PatternView::stack > 1`, the inner area is split into N vertically-
//! stacked lanes. Each lane is a complete pattern view (full row context,
//! own centered current row) but shows only its slice of channels — lane 0
//! gets channels 0..K, lane 1 gets K..2K, etc. The same pattern rows appear
//! in every lane so the eye reads them as time-aligned bands.

use crate::state::pattern::PatternWindow;
use crate::state::SharedState;
use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use std::sync::atomic::Ordering;

/// User-controlled pattern view settings.
#[derive(Clone, Copy, Debug)]
pub struct PatternView {
    /// How many vertically-stacked lanes the channel band is split into. Each
    /// lane shows the same row window but a different slice of channels. Must
    /// be >= 1.
    pub stack: u8,
    /// When true, cells display only note + instrument (6 chars) instead of
    /// the full note/inst/volume/effect (14 chars).
    pub compact: bool,
}

impl Default for PatternView {
    fn default() -> Self {
        Self {
            stack: 1,
            compact: false,
        }
    }
}

impl PatternView {
    pub fn cycle_stack(&mut self) {
        self.stack = match self.stack {
            1 => 2,
            2 => 4,
            _ => 1,
        };
    }

    pub fn toggle_compact(&mut self) {
        self.compact = !self.compact;
    }
}

const FULL_CELL_W: usize = 14; // "C-5 01 v40 A20"
const COMPACT_CELL_W: usize = 6; // "C-5 01"
const SEP_W: usize = 1;
const ROW_LABEL_W: usize = 4; // "▶ 23 "
const FULL_EMPTY: &str = "... .. .. ...";
const COMPACT_EMPTY: &str = "... ..";

pub fn render(
    f: &mut Frame,
    area: Rect,
    state: &SharedState,
    theme: &Theme,
    focused: bool,
    view: PatternView,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if focused {
            theme.border_focus
        } else {
            theme.border
        }))
        .title(Span::styled(pattern_title(view), theme.dim_style()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let pattern = state.current_pattern.load(Ordering::Relaxed);
    let row = state.current_row.load(Ordering::Relaxed);
    let window = state
        .pattern_cache
        .lock()
        .map(|cache| cache.window(pattern, row))
        .unwrap_or_default();

    if window.rows.is_empty() || window.channel_count == 0 {
        let msg = Paragraph::new(Line::from(Span::styled(
            "no pattern data — load a module to begin",
            theme.dim_style(),
        )));
        f.render_widget(msg, inner);
        return;
    }

    // Cap stack to the number of channels so we never render an empty lane.
    let lanes = (view.stack.max(1) as usize).min(window.channel_count);
    let cell_w = if view.compact {
        COMPACT_CELL_W
    } else {
        FULL_CELL_W
    };
    let empty_cell = if view.compact {
        COMPACT_EMPTY
    } else {
        FULL_EMPTY
    };

    let channels_per_lane = window.channel_count.div_ceil(lanes);
    let inner_w = inner.width as usize;
    let available_chan_w = inner_w.saturating_sub(ROW_LABEL_W);
    let max_cells_per_lane = (available_chan_w / (cell_w + SEP_W)).max(1);

    let lane_height = ((inner.height as usize) / lanes).max(1) as u16;
    let bottom = inner.y.saturating_add(inner.height);

    for lane_idx in 0..lanes {
        let lane_y = inner
            .y
            .saturating_add((lane_height as usize * lane_idx) as u16);
        if lane_y >= bottom {
            break;
        }
        let remaining = bottom - lane_y;
        let lane_h = if lane_idx + 1 == lanes {
            remaining // last lane absorbs any leftover rows
        } else {
            lane_height.min(remaining)
        };
        if lane_h == 0 {
            continue;
        }
        let lane_rect = Rect::new(inner.x, lane_y, inner.width, lane_h);

        let ch_start = lane_idx * channels_per_lane;
        let ch_end = (ch_start + channels_per_lane).min(window.channel_count);
        let cells_in_lane = (ch_end - ch_start).min(max_cells_per_lane);

        // Header (channel-range label + divider) only when we're actually
        // stacking, and only when the lane is tall enough to leave a row
        // behind.
        let show_header = lanes > 1 && lane_h >= 2;

        render_lane(
            f,
            lane_rect,
            &window,
            ch_start,
            cells_in_lane,
            cell_w,
            empty_cell,
            show_header,
            theme,
        );
    }
}

fn pattern_title(view: PatternView) -> String {
    let mut s = String::from(" pattern");
    if view.stack > 1 {
        s.push_str(&format!(" ×{}", view.stack));
    }
    if view.compact {
        s.push_str(" compact");
    }
    s.push(' ');
    s
}

#[allow(clippy::too_many_arguments)]
fn render_lane(
    f: &mut Frame,
    area: Rect,
    window: &PatternWindow,
    ch_start: usize,
    cells_in_lane: usize,
    cell_w: usize,
    empty_cell: &str,
    show_header: bool,
    theme: &Theme,
) {
    let mut row_area = area;
    if show_header {
        let header_rect = Rect::new(area.x, area.y, area.width, 1);
        let header = lane_header_line(ch_start, cells_in_lane, area.width as usize, theme);
        f.render_widget(Paragraph::new(header), header_rect);
        row_area = Rect::new(area.x, area.y + 1, area.width, area.height - 1);
    }

    let visible_rows = row_area.height as usize;
    if visible_rows == 0 {
        return;
    }
    let total_rows = window.rows.len();
    let center = window.current_index;
    let half = visible_rows / 2;
    let start = center.saturating_sub(half);
    let end = (start + visible_rows).min(total_rows);

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
        let marker = if is_current { "▶" } else { " " };
        let mut spans: Vec<Span> = Vec::with_capacity(cells_in_lane * (cell_w + 1) + 2);
        spans.push(Span::styled(format!("{marker} "), prefix_style));
        spans.push(Span::styled(row_label, theme.dim_style()));

        for ci in 0..cells_in_lane {
            let channel = ch_start + ci;
            if ci > 0 {
                spans.push(Span::styled("│", theme.dim_style()));
            }
            let cell = row
                .cells
                .get(channel)
                .map(|s| s.as_str())
                .unwrap_or_default();
            let source: &str = if cell.trim().is_empty() {
                empty_cell
            } else {
                cell
            };
            for (idx, ch) in source.chars().take(cell_w).enumerate() {
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
    f.render_widget(para, row_area);
}

fn lane_header_line(
    ch_start: usize,
    cells_in_lane: usize,
    width: usize,
    theme: &Theme,
) -> Line<'static> {
    let label = if cells_in_lane == 0 {
        String::new()
    } else if cells_in_lane == 1 {
        format!(" ch {} ", ch_start + 1)
    } else {
        format!(" ch {}-{} ", ch_start + 1, ch_start + cells_in_lane)
    };
    // Leading dashes line up before the label; trailing dashes fill the rest.
    let lead = 2usize;
    let label_len = label.chars().count();
    let trail = width.saturating_sub(lead + label_len);
    let dim = theme.dim_style();
    let mut spans: Vec<Span> = Vec::with_capacity(3);
    spans.push(Span::styled("─".repeat(lead), dim));
    if !label.is_empty() {
        spans.push(Span::styled(label, dim));
    }
    if trail > 0 {
        spans.push(Span::styled("─".repeat(trail), dim));
    }
    Line::from(spans)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_stack_progresses_1_2_4_1() {
        let mut v = PatternView::default();
        assert_eq!(v.stack, 1);
        v.cycle_stack();
        assert_eq!(v.stack, 2);
        v.cycle_stack();
        assert_eq!(v.stack, 4);
        v.cycle_stack();
        assert_eq!(v.stack, 1);
    }

    #[test]
    fn toggle_compact_flips() {
        let mut v = PatternView::default();
        assert!(!v.compact);
        v.toggle_compact();
        assert!(v.compact);
        v.toggle_compact();
        assert!(!v.compact);
    }

    #[test]
    fn title_reflects_active_modifiers() {
        assert_eq!(pattern_title(PatternView::default()), " pattern ");
        assert_eq!(
            pattern_title(PatternView {
                stack: 2,
                compact: false
            }),
            " pattern ×2 "
        );
        assert_eq!(
            pattern_title(PatternView {
                stack: 4,
                compact: true
            }),
            " pattern ×4 compact "
        );
    }
}
