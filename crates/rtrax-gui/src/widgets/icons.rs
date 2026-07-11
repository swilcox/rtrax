//! Vector-drawn transport icons. Painted with painter primitives instead of
//! text so we get the canonical shapes crisp at any scale, independent of
//! what glyphs the bundled fonts happen to cover.

use crate::theme::Theme;
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
    /// List box with a play arrow at the top line — the queue.
    Queue,
    /// Partial piano keyboard — live instruments pane.
    Instruments,
    /// Eighth note — song info / message window.
    SongInfo,
    /// Two arrows squeezing a center line — compact pattern cells.
    Compact,
    /// Paint palette with theme-colored dots — theme cycler.
    Palette,
}

pub fn icon_button(ui: &mut egui::Ui, icon: Icon, tooltip: &str, theme: &Theme) -> Response {
    paint_button(ui, icon, tooltip, theme, false)
}

/// An icon button carrying an on/off state: the icon lights up in the theme's
/// fill color while active.
pub fn toggle_icon_button(
    ui: &mut egui::Ui,
    icon: Icon,
    on: bool,
    tooltip: &str,
    theme: &Theme,
) -> Response {
    paint_button(ui, icon, tooltip, theme, on)
}

fn paint_button(ui: &mut egui::Ui, icon: Icon, tooltip: &str, theme: &Theme, on: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(34.0, 24.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let bg = if response.is_pointer_button_down_on() {
            theme.track.gamma_multiply(1.5)
        } else if response.hovered() || on {
            theme.track
        } else {
            theme.panel
        };
        painter.rect_filled(rect, CornerRadius::same(5), bg);
        let color = if on {
            theme.fill
        } else if response.hovered() {
            theme.fg
        } else {
            theme.fg.gamma_multiply(0.8)
        };
        draw_icon(
            painter,
            Rect::from_center_size(rect.center(), Vec2::splat(12.0)),
            icon,
            color,
            theme,
        );
    }
    response.on_hover_text(tooltip)
}

fn draw_icon(painter: &egui::Painter, r: Rect, icon: Icon, color: Color32, theme: &Theme) {
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
        Icon::Queue => {
            // List box; the top line is shortened to make room for a tiny
            // play arrow pointing at it.
            let stroke = Stroke::new(1.2, color);
            painter.rect_stroke(r, CornerRadius::same(2), stroke, egui::StrokeKind::Inside);
            let inset = 2.8;
            let ys = [r.top() + 3.4, r.center().y, r.bottom() - 3.4];
            let arrow_w = 3.2;
            // Top line: arrow + shortened line.
            painter.add(tri_right(Rect::from_min_max(
                Pos2::new(r.left() + inset, ys[0] - 1.8),
                Pos2::new(r.left() + inset + arrow_w, ys[0] + 1.8),
            )));
            painter.hline(
                egui::Rangef::new(r.left() + inset + arrow_w + 1.2, r.right() - inset),
                ys[0],
                stroke,
            );
            for &y in &ys[1..] {
                painter.hline(
                    egui::Rangef::new(r.left() + inset, r.right() - inset),
                    y,
                    stroke,
                );
            }
        }
        Icon::Instruments => {
            // Three white piano keys with two black keys on the boundaries.
            let stroke = Stroke::new(1.2, color);
            painter.rect_stroke(r, CornerRadius::same(2), stroke, egui::StrokeKind::Inside);
            let black_bottom = r.top() + r.height() * 0.55;
            for i in 1..3 {
                let x = r.left() + r.width() * i as f32 / 3.0;
                // Black key hanging from the top…
                painter.add(bar(Rect::from_min_max(
                    Pos2::new(x - 1.3, r.top() + 1.0),
                    Pos2::new(x + 1.3, black_bottom),
                )));
                // …with the white-key divider continuing below it.
                painter.vline(x, egui::Rangef::new(black_bottom, r.bottom() - 1.0), stroke);
            }
        }
        Icon::SongInfo => {
            // Eighth note: head, stem, flag.
            let head = Pos2::new(r.left() + 3.4, r.bottom() - 2.6);
            let stem_x = head.x + 2.0;
            painter.circle_filled(head, 2.6, color);
            painter.add(bar(Rect::from_min_max(
                Pos2::new(stem_x - 0.6, r.top() + 1.0),
                Pos2::new(stem_x + 0.6, head.y),
            )));
            painter.add(Shape::convex_polygon(
                vec![
                    Pos2::new(stem_x, r.top() + 1.0),
                    Pos2::new(stem_x + 4.6, r.top() + 3.6),
                    Pos2::new(stem_x, r.top() + 5.4),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Compact => {
            // Two arrows squeezing a center line.
            let gap = 2.2;
            painter.add(bar(Rect::from_min_max(
                Pos2::new(r.center().x - 0.6, r.top() + 1.0),
                Pos2::new(r.center().x + 0.6, r.bottom() - 1.0),
            )));
            painter.add(tri_right(Rect::from_min_max(
                Pos2::new(r.left(), r.center().y - 3.2),
                Pos2::new(r.center().x - gap, r.center().y + 3.2),
            )));
            painter.add(tri_left(Rect::from_min_max(
                Pos2::new(r.center().x + gap, r.center().y - 3.2),
                Pos2::new(r.right(), r.center().y + 3.2),
            )));
        }
        Icon::Palette => {
            // Palette disc with three theme-colored paint dots.
            painter.circle_stroke(r.center(), r.width() * 0.46, Stroke::new(1.3, color));
            let cx = r.center();
            let dots = [
                (Pos2::new(cx.x - 2.4, cx.y - 1.4), theme.instrument),
                (Pos2::new(cx.x + 2.4, cx.y - 1.4), theme.volume),
                (Pos2::new(cx.x, cx.y + 2.6), theme.effect),
            ];
            for (pos, dot_color) in dots {
                painter.circle_filled(pos, 1.5, dot_color);
            }
        }
    }
}
