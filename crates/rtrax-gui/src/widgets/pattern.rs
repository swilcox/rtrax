//! Pattern view: the rows around the current row, current row centered and
//! highlighted, per-token coloring (note/instrument/volume/effect), a small
//! channel-number header, and gap-free channel separator lines drawn with the
//! painter (not text glyphs, which leave vertical gaps between rows).
//!
//! Wide modules are handled in three stages: the font auto-shrinks a little
//! to fit, compact cells (note+instrument only) drop the width further, and
//! a horizontal scroll area is the final fallback. Each visible row is one
//! `LayoutJob` with a colored section per token, so painting stays cheap.

use crate::theme::Theme;
use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke};
use rtrax_core::state::pattern::PatternRow;
use rtrax_core::state::SharedState;
use std::sync::atomic::Ordering;

/// Full cells come out of libopenmpt as `"C-5 01 v40 A20"` — fixed 14 chars.
const FULL_CELL_W: usize = 14;
const FULL_EMPTY: &str = "... .. .. ...";
const FULL_TOKENS: &[(usize, usize)] = &[(0, 4), (4, 7), (7, 11), (11, 14)];
/// Compact cells show note + instrument only: `"C-5 01"`.
const COMPACT_CELL_W: usize = 6;
const COMPACT_EMPTY: &str = "... ..";
const COMPACT_TOKENS: &[(usize, usize)] = &[(0, 4), (4, 6)];
/// `"> 123 "` marker + row number.
const LABEL_CHARS: usize = 6;

const BASE_FONT: f32 = 13.0;
const MIN_FONT: f32 = 10.0;
const HEADER_H: f32 = 16.0;

pub fn show(ui: &mut egui::Ui, state: &SharedState, theme: &Theme, compact: bool) {
    let pattern = state.current_pattern.load(Ordering::Relaxed);
    let row = state.current_row.load(Ordering::Relaxed);
    let window = state
        .pattern_cache
        .lock()
        .map(|cache| cache.window(pattern, row))
        .unwrap_or_default();

    let rect = ui.available_rect_before_wrap();
    if window.rows.is_empty() || window.channel_count == 0 {
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            "no pattern data — load a module to begin",
            FontId::monospace(14.0),
            theme.dim,
        );
        return;
    }

    let (cell_w, empty_cell, tokens) = if compact {
        (COMPACT_CELL_W, COMPACT_EMPTY, COMPACT_TOKENS)
    } else {
        (FULL_CELL_W, FULL_EMPTY, FULL_TOKENS)
    };
    let row_chars = LABEL_CHARS + window.channel_count * (cell_w + 1);

    // Shrink the font (down to a floor) until the full channel band fits;
    // beyond that the horizontal scroll area takes over.
    let probe_char_w = char_width(ui, BASE_FONT);
    let required = row_chars as f32 * probe_char_w + 8.0;
    let scale = (rect.width() / required).clamp(MIN_FONT / BASE_FONT, 1.0);
    let font = FontId::monospace((BASE_FONT * scale * 2.0).round() / 2.0);
    let char_w = char_width(ui, font.size);
    let row_h = ui
        .painter()
        .layout_job(LayoutJob::simple_singleline(
            "0".into(),
            font.clone(),
            theme.fg,
        ))
        .size()
        .y
        + 3.0;

    let rows_h = rect.height() - HEADER_H;
    let visible = ((rows_h / row_h).floor() as usize).clamp(1, window.rows.len());
    let center = window.current_index;
    let start = center.saturating_sub(visible / 2);
    let end = (start + visible).min(window.rows.len());

    let content_w = (row_chars as f32 * char_w + 8.0).max(rect.width());
    let left_pad = 4.0;

    egui::ScrollArea::horizontal()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let (content_rect, _) =
                ui.allocate_exact_size(egui::vec2(content_w, rect.height()), Sense::hover());
            let painter = ui.painter();

            // x of the character column where channel `ch`'s block begins.
            let channel_x = |ch: usize| {
                content_rect.left() + left_pad + (LABEL_CHARS + ch * (cell_w + 1)) as f32 * char_w
            };

            // Channel-number header row.
            let header_font = FontId::monospace((font.size - 3.0).max(8.0));
            for ch in 0..window.channel_count {
                let center_x = channel_x(ch) + (cell_w as f32 * char_w) * 0.5;
                painter.text(
                    Pos2::new(center_x, content_rect.top() + HEADER_H * 0.5),
                    Align2::CENTER_CENTER,
                    format!("{:02}", ch + 1),
                    header_font.clone(),
                    theme.dim,
                );
            }

            let rows_top = content_rect.top() + HEADER_H;
            let used_h = (end - start) as f32 * row_h;
            let top = rows_top + ((content_rect.bottom() - rows_top - used_h) * 0.5).max(0.0);

            for (i, pattern_row) in window.rows[start..end].iter().enumerate() {
                let y = top + i as f32 * row_h;
                let is_current = start + i == center;
                if is_current {
                    painter.rect_filled(
                        Rect::from_min_size(
                            Pos2::new(content_rect.left(), y),
                            egui::vec2(content_rect.width(), row_h),
                        ),
                        egui::CornerRadius::same(2),
                        theme.current_row_bg,
                    );
                }
                let job = row_job(
                    pattern_row,
                    window.channel_count,
                    is_current,
                    &font,
                    theme,
                    cell_w,
                    empty_cell,
                    tokens,
                );
                let galley = painter.layout_job(job);
                painter.galley(
                    Pos2::new(content_rect.left() + left_pad, y + 1.5),
                    galley,
                    theme.fg,
                );
            }

            // Continuous channel separators, drawn over the full band height
            // (header included) so there are no per-row gaps.
            let sep_stroke = Stroke::new(1.0, theme.track);
            for ch in 0..window.channel_count + 1 {
                let x = (channel_x(ch) - char_w * 0.5).round() + 0.5;
                painter.vline(
                    x,
                    egui::Rangef::new(content_rect.top() + 2.0, top + used_h),
                    sep_stroke,
                );
            }
        });
}

fn char_width(ui: &egui::Ui, font_size: f32) -> f32 {
    ui.painter()
        .layout_job(LayoutJob::simple_singleline(
            "0".into(),
            FontId::monospace(font_size),
            Color32::WHITE,
        ))
        .size()
        .x
}

#[allow(clippy::too_many_arguments)]
fn row_job(
    row: &PatternRow,
    channels: usize,
    is_current: bool,
    font: &FontId,
    theme: &Theme,
    cell_w: usize,
    empty_cell: &str,
    tokens: &[(usize, usize)],
) -> LayoutJob {
    let fmt = |color: Color32| TextFormat {
        font_id: font.clone(),
        color,
        ..Default::default()
    };
    // Non-current rows are dimmed toward the background so the playing row
    // carries the visual weight (monospace has no bold variant to lean on).
    let tone = |color: Color32| {
        if is_current {
            color
        } else {
            color.gamma_multiply(0.62)
        }
    };
    let token_colors = [theme.note, theme.instrument, theme.volume, theme.effect];

    let mut job = LayoutJob::default();
    let label = if row.row_index < 0 {
        " ".repeat(LABEL_CHARS)
    } else {
        let marker = if is_current { '>' } else { ' ' };
        format!("{marker} {:>3} ", row.row_index)
    };
    let label_color = if is_current { theme.accent } else { theme.dim };
    job.append(&label, 0.0, fmt(label_color));

    for ch in 0..channels {
        let cell = row
            .cells
            .get(ch)
            .map(String::as_str)
            .filter(|c| !c.trim().is_empty())
            .unwrap_or(empty_cell);
        // Pad/truncate to the fixed cell width; chars (not bytes) so an odd
        // non-ASCII cell can't split a code point.
        let chars: Vec<char> = cell
            .chars()
            .chain(std::iter::repeat(' '))
            .take(cell_w)
            .collect();
        for (t, &(a, b)) in tokens.iter().enumerate() {
            let text: String = chars[a..b].iter().collect();
            let color = if text.trim_matches(['.', ' ']).is_empty() {
                theme.dim.gamma_multiply(0.8)
            } else {
                token_colors[t]
            };
            job.append(&text, 0.0, fmt(tone(color)));
        }
        // The separator column is blank text; the painter draws a continuous
        // vertical line through it after the rows are laid down.
        job.append(" ", 0.0, fmt(theme.dim));
    }
    job
}
