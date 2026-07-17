//! Per-channel level meters: one row per channel, L over R horizontal bars
//! with peak-hold markers, in a vertical scroll area.

use crate::theme::Theme;
use eframe::egui::{self, Align2, CornerRadius, FontId, Pos2, Rect, Sense};
use rtrax_core::meters::{ChannelMeters, Envelope};

const ROW_H: f32 = 20.0;
const BAR_H: f32 = 6.0;
const LABEL_W: f32 = 26.0;

pub fn show(ui: &mut egui::Ui, meters: &ChannelMeters, theme: &Theme) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for ch in 0..meters.len() {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_H), Sense::hover());
                let painter = ui.painter();
                painter.text(
                    Pos2::new(rect.left() + LABEL_W - 6.0, rect.center().y),
                    Align2::RIGHT_CENTER,
                    format!("{:02}", ch + 1),
                    FontId::monospace(11.0),
                    theme.dim,
                );
                let (left, right) = meters.channel(ch);
                let bar_left = rect.left() + LABEL_W;
                let bar_w = (rect.right() - 4.0 - bar_left).max(1.0);
                let mid = rect.center().y;
                bar(
                    painter,
                    Rect::from_min_size(
                        Pos2::new(bar_left, mid - BAR_H - 1.0),
                        egui::vec2(bar_w, BAR_H),
                    ),
                    left,
                    theme,
                );
                bar(
                    painter,
                    Rect::from_min_size(Pos2::new(bar_left, mid + 1.0), egui::vec2(bar_w, BAR_H)),
                    right,
                    theme,
                );
            }
        });
}

/// One horizontal bar: recessed track, three-zone gradient fill
/// (low → mid → hot, like the TUI meters), peak-hold tick.
///
/// The zones matter for themes whose `meter_low` sits close to the track
/// color (e.g. muted palettes): the mid/high zones carry the contrast.
pub fn bar(painter: &egui::Painter, rect: Rect, env: Envelope, theme: &Theme) {
    // Recess the track toward the background so even a dark fill separates.
    let track = crate::theme::lerp_color(theme.bg, theme.track, 0.55);
    painter.rect_filled(rect, CornerRadius::same(2), track);

    let level = env.smoothed.clamp(0.0, 1.0);
    if level * rect.width() >= 1.0 {
        let zones = [
            (0.0, 0.60, theme.fill),
            (0.60, 0.85, theme.peak),
            (0.85, 1.0, theme.meter_hot),
        ];
        for (z0, z1, color) in zones {
            if level <= z0 {
                break;
            }
            let x0 = rect.left() + rect.width() * z0;
            let x1 = rect.left() + rect.width() * level.min(z1);
            // Round only the outer ends of the whole bar; zone boundaries
            // meet as square butt-joints so the fill reads as one bar.
            let first = z0 == 0.0;
            let last = level <= z1;
            let radius = 2;
            let corner = CornerRadius {
                nw: if first { radius } else { 0 },
                sw: if first { radius } else { 0 },
                ne: if last { radius } else { 0 },
                se: if last { radius } else { 0 },
            };
            painter.rect_filled(
                Rect::from_min_max(Pos2::new(x0, rect.top()), Pos2::new(x1, rect.bottom())),
                corner,
                color,
            );
        }
    }

    let peak = env.peak.clamp(0.0, 1.0);
    if peak > 0.01 {
        let x = rect.left() + rect.width() * peak;
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(x - 1.0, rect.top()),
                Pos2::new(x + 1.0, rect.bottom()),
            ),
            CornerRadius::ZERO,
            theme.fg,
        );
    }
}
