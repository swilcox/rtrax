//! Vector-drawn transport icons. Painted with painter primitives instead of
//! text so we get the canonical shapes crisp at any scale, independent of
//! what glyphs the bundled fonts happen to cover.

use crate::theme;
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Response, Sense, Shape, Stroke, Vec2};

#[derive(Clone, Copy)]
pub enum Icon {
    Play,
    Pause,
    Stop,
    Prev,
    Next,
    Rewind,
    FastForward,
}

pub fn icon_button(ui: &mut egui::Ui, icon: Icon, tooltip: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(34.0, 24.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let bg = if response.is_pointer_button_down_on() {
            theme::TRACK.gamma_multiply(1.5)
        } else if response.hovered() {
            theme::TRACK
        } else {
            theme::PANEL
        };
        painter.rect_filled(rect, CornerRadius::same(5), bg);
        let color = if response.hovered() {
            theme::FG
        } else {
            theme::FG.gamma_multiply(0.8)
        };
        draw_icon(
            painter,
            Rect::from_center_size(rect.center(), Vec2::splat(11.0)),
            icon,
            color,
        );
    }
    response.on_hover_text(tooltip)
}

fn draw_icon(painter: &egui::Painter, r: Rect, icon: Icon, color: Color32) {
    let tri_right = |rect: Rect| {
        Shape::convex_polygon(
            vec![
                rect.left_top(),
                Pos2::new(rect.right(), rect.center().y),
                rect.left_bottom(),
            ],
            color,
            Stroke::NONE,
        )
    };
    let tri_left = |rect: Rect| {
        Shape::convex_polygon(
            vec![
                rect.right_top(),
                Pos2::new(rect.left(), rect.center().y),
                rect.right_bottom(),
            ],
            color,
            Stroke::NONE,
        )
    };
    let bar = |rect: Rect| Shape::rect_filled(rect, CornerRadius::same(1), color);

    match icon {
        Icon::Play => {
            painter.add(tri_right(r));
        }
        Icon::Pause => {
            let w = r.width() * 0.34;
            painter.add(bar(Rect::from_min_max(
                r.left_top(),
                Pos2::new(r.left() + w, r.bottom()),
            )));
            painter.add(bar(Rect::from_min_max(
                Pos2::new(r.right() - w, r.top()),
                r.right_bottom(),
            )));
        }
        Icon::Stop => {
            painter.add(bar(r.shrink(0.5)));
        }
        Icon::Prev => {
            let bar_w = 2.0;
            painter.add(bar(Rect::from_min_max(
                r.left_top(),
                Pos2::new(r.left() + bar_w, r.bottom()),
            )));
            painter.add(tri_left(Rect::from_min_max(
                Pos2::new(r.left() + bar_w + 1.0, r.top()),
                r.right_bottom(),
            )));
        }
        Icon::Next => {
            let bar_w = 2.0;
            painter.add(tri_right(Rect::from_min_max(
                r.left_top(),
                Pos2::new(r.right() - bar_w - 1.0, r.bottom()),
            )));
            painter.add(bar(Rect::from_min_max(
                Pos2::new(r.right() - bar_w, r.top()),
                r.right_bottom(),
            )));
        }
        Icon::Rewind => {
            let half = r.width() * 0.58;
            painter.add(tri_left(Rect::from_min_max(
                r.left_top(),
                Pos2::new(r.left() + half, r.bottom()),
            )));
            painter.add(tri_left(Rect::from_min_max(
                Pos2::new(r.right() - half, r.top()),
                r.right_bottom(),
            )));
        }
        Icon::FastForward => {
            let half = r.width() * 0.58;
            painter.add(tri_right(Rect::from_min_max(
                r.left_top(),
                Pos2::new(r.left() + half, r.bottom()),
            )));
            painter.add(tri_right(Rect::from_min_max(
                Pos2::new(r.right() - half, r.top()),
                r.right_bottom(),
            )));
        }
    }
}
