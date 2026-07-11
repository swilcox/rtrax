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

/// One horizontal bar: dim track, themed fill going hot above the threshold,
/// peak-hold tick.
pub fn bar(painter: &egui::Painter, rect: Rect, env: Envelope, theme: &Theme) {
    const HOT: f32 = 0.85;
    painter.rect_filled(rect, CornerRadius::same(2), theme.track);

    let level = env.smoothed.clamp(0.0, 1.0);
    if level * rect.width() >= 1.0 {
        let fill_w = rect.width() * level.min(HOT);
        painter.rect_filled(
            Rect::from_min_size(rect.min, egui::vec2(fill_w, rect.height())),
            CornerRadius::same(2),
            theme.fill,
        );
        if level > HOT {
            let hot_x = rect.left() + rect.width() * HOT;
            painter.rect_filled(
                Rect::from_min_max(
                    Pos2::new(hot_x, rect.top()),
                    Pos2::new(rect.left() + rect.width() * level, rect.bottom()),
                ),
                CornerRadius::same(2),
                theme.meter_hot,
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
            theme.peak,
        );
    }
}
