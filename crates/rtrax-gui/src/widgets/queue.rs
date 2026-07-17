//! Queue side panel: the play order, current track highlighted, click to
//! jump. Interactions come back to the app as an `Action`.

use crate::theme::Theme;
use crate::widgets::icons::{toggle_icon_button, Icon};
use eframe::egui::{self, RichText};
use rtrax_core::playlist::Playlist;
use std::path::{Path, PathBuf};

pub enum Action {
    Play(PathBuf),
    ToggleShuffle,
}

pub fn show(
    ui: &mut egui::Ui,
    queue: &Playlist,
    current: Option<&Path>,
    shuffle: bool,
    reveal_current: bool,
    theme: &Theme,
) -> Option<Action> {
    let mut action = None;
    let current_idx = current.and_then(|c| queue.position(c));

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("queue · {}", queue.len()))
                .color(theme.dim)
                .monospace(),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if toggle_icon_button(ui, Icon::Shuffle, shuffle, "shuffle play order (z)", theme)
                .clicked()
            {
                action = Some(Action::ToggleShuffle);
            }
        });
    });
    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (idx, path) in queue.entries.iter().enumerate() {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string());
                let is_current = Some(idx) == current_idx;
                let text = RichText::new(name)
                    .monospace()
                    .size(12.0)
                    .color(if is_current {
                        theme.fill
                    } else {
                        theme.fg.gamma_multiply(0.8)
                    });
                let response = ui.selectable_label(is_current, text);
                if is_current && reveal_current {
                    // Put the playing track a third of the way down the
                    // viewport; the scroll clamp turns this into "just make
                    // it visible" near the ends of the list.
                    let clip = ui.clip_rect();
                    let target_y = clip.top() + clip.height() / 3.0;
                    ui.scroll_with_delta(egui::vec2(0.0, target_y - response.rect.top()));
                }
                if response.clicked() && !is_current {
                    action = Some(Action::Play(path.clone()));
                }
            }
        });
    action
}
