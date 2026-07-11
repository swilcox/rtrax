//! Pattern view: the rows around the current row, current row centered and
//! highlighted, per-token coloring (note/instrument/volume/effect).
//!
//! Unlike the TUI (which stacks channel lanes to fit a fixed-width terminal),
//! the GUI shows all channels in one band inside a horizontal scroll area.
//! Each visible row is laid out as a single `LayoutJob` with one colored
//! section per token, so wide modules stay cheap to paint.

use crate::theme;
use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense};
use rtrax_core::state::pattern::PatternRow;
use rtrax_core::state::SharedState;
use std::sync::atomic::Ordering;

/// Cells come out of libopenmpt as `"C-5 01 v40 A20"` — fixed 14-char layout.
const CELL_W: usize = 14;
const EMPTY_CELL: &str = "... .. .. ...";
/// `(start, end, colorizer)` char ranges within a cell. Gaps between tokens
/// are attached to the token before them so section count stays low.
const TOKENS: &[(usize, usize)] = &[(0, 4), (4, 7), (7, 11), (11, 14)];
const TOKEN_COLORS: &[Color32] = &[theme::NOTE, theme::INSTRUMENT, theme::VOLUME, theme::EFFECT];
/// `"> 123 "` marker + row number.
const LABEL_CHARS: usize = 6;

pub fn show(ui: &mut egui::Ui, state: &SharedState) {
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
            theme::DIM,
        );
        return;
    }

    let font = FontId::monospace(13.0);
    let probe = ui.painter().layout_job(LayoutJob::simple_singleline(
        "0".into(),
        font.clone(),
        theme::FG,
    ));
    let char_w = probe.size().x;
    let row_h = probe.size().y + 3.0;

    let visible = ((rect.height() / row_h).floor() as usize).clamp(1, window.rows.len());
    let center = window.current_index;
    let start = center.saturating_sub(visible / 2);
    let end = (start + visible).min(window.rows.len());

    let row_chars = LABEL_CHARS + window.channel_count * (CELL_W + 1);
    let content_w = (row_chars as f32 * char_w + 8.0).max(rect.width());

    egui::ScrollArea::horizontal()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let (content_rect, _) =
                ui.allocate_exact_size(egui::vec2(content_w, rect.height()), Sense::hover());
            let painter = ui.painter();
            let used_h = (end - start) as f32 * row_h;
            let top = content_rect.top() + ((content_rect.height() - used_h) * 0.5).max(0.0);

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
                        theme::CURRENT_ROW_BG,
                    );
                }
                let job = row_job(pattern_row, window.channel_count, is_current, &font);
                let galley = painter.layout_job(job);
                painter.galley(
                    Pos2::new(content_rect.left() + 4.0, y + 1.5),
                    galley,
                    theme::FG,
                );
            }
        });
}

fn row_job(row: &PatternRow, channels: usize, is_current: bool, font: &FontId) -> LayoutJob {
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

    let mut job = LayoutJob::default();
    let label = if row.row_index < 0 {
        " ".repeat(LABEL_CHARS)
    } else {
        let marker = if is_current { '>' } else { ' ' };
        format!("{marker} {:>3} ", row.row_index)
    };
    let label_color = if is_current {
        theme::MAGENTA
    } else {
        theme::DIM
    };
    job.append(&label, 0.0, fmt(label_color));

    for ch in 0..channels {
        job.append("│", 0.0, fmt(tone(theme::TRACK.gamma_multiply(3.0))));
        let cell = row
            .cells
            .get(ch)
            .map(String::as_str)
            .filter(|c| !c.trim().is_empty())
            .unwrap_or(EMPTY_CELL);
        // Pad/truncate to the fixed cell width; chars (not bytes) so an odd
        // non-ASCII cell can't split a code point.
        let chars: Vec<char> = cell
            .chars()
            .chain(std::iter::repeat(' '))
            .take(CELL_W)
            .collect();
        for (t, &(a, b)) in TOKENS.iter().enumerate() {
            let text: String = chars[a..b].iter().collect();
            let color = if text.trim_matches(['.', ' ']).is_empty() {
                theme::DIM.gamma_multiply(0.8)
            } else {
                TOKEN_COLORS[t]
            };
            job.append(&text, 0.0, fmt(tone(color)));
        }
    }
    job
}
