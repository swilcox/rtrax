//! The bottom visualization strip: spectrum analyzer bands on the left,
//! master L/R output meters on the right.

use crate::theme;
use crate::widgets::meters::bar;
use eframe::egui::{self, Align2, CornerRadius, FontId, Pos2, Rect};
use rtrax_core::meters::MasterMeter;

const MASTER_W: f32 = 190.0;
const GAP: f32 = 10.0;

/// Draw the strip into `rect`; returns the band count that fits the spectrum
/// area so the caller can resize its `Spectrum`.
pub fn band_count_for(rect: Rect) -> usize {
    (((rect.width() - MASTER_W - GAP) / 14.0) as usize).clamp(16, 64)
}

pub fn show(painter: &egui::Painter, rect: Rect, bands: &[f32], master: &MasterMeter) {
    let spectrum_rect = Rect::from_min_max(
        rect.min,
        Pos2::new((rect.max.x - MASTER_W - GAP).max(rect.min.x), rect.max.y),
    );
    let master_rect = Rect::from_min_max(Pos2::new(rect.max.x - MASTER_W, rect.min.y), rect.max);

    draw_spectrum(painter, spectrum_rect, bands);
    draw_master(painter, master_rect, master);
}

fn draw_spectrum(painter: &egui::Painter, rect: Rect, bands: &[f32]) {
    painter.rect_filled(rect, CornerRadius::same(4), theme::PANEL);
    if bands.is_empty() || rect.width() < 8.0 {
        return;
    }
    let inset = 3.0;
    let band_width = (rect.width() - inset * 2.0) / bands.len() as f32;
    let max_height = rect.height() - inset * 2.0;
    for (i, &value) in bands.iter().enumerate() {
        let height = value.clamp(0.0, 1.0) * max_height;
        if height < 1.0 {
            continue;
        }
        let x0 = rect.left() + inset + i as f32 * band_width;
        let bar_rect = Rect::from_min_max(
            Pos2::new(x0 + 1.0, rect.bottom() - inset - height),
            Pos2::new(x0 + band_width - 1.0, rect.bottom() - inset),
        );
        let t = i as f32 / bands.len() as f32;
        painter.rect_filled(
            bar_rect,
            CornerRadius::same(1),
            theme::lerp_color(theme::CYAN, theme::GREEN, t),
        );
    }
}

/// Master meter: horizontal L/R bars sharing the channel-meter bar style
/// (green fill, magenta hot zone, cyan peak-hold tick).
fn draw_master(painter: &egui::Painter, rect: Rect, master: &MasterMeter) {
    painter.rect_filled(rect, CornerRadius::same(4), theme::PANEL);
    let inset = 10.0;
    let label_w = 16.0;
    let bar_h = 8.0;
    let mid = rect.center().y;
    for (i, (label, env)) in [("L", master.left), ("R", master.right)]
        .into_iter()
        .enumerate()
    {
        let y = if i == 0 { mid - bar_h - 3.0 } else { mid + 3.0 };
        painter.text(
            Pos2::new(rect.left() + inset, y + bar_h * 0.5),
            Align2::LEFT_CENTER,
            label,
            FontId::monospace(11.0),
            theme::DIM,
        );
        bar(
            painter,
            Rect::from_min_size(
                Pos2::new(rect.left() + inset + label_w, y),
                egui::vec2(rect.width() - inset * 2.0 - label_w, bar_h),
            ),
            env,
        );
    }
}
