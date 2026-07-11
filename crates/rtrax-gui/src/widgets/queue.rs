//! Queue side panel: the play order, current track highlighted, click to
//! jump. Interactions come back to the app as an `Action`.

use crate::theme::Theme;
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
            let label = RichText::new("shuffle")
                .monospace()
                .size(11.0)
                .color(if shuffle { theme.fill } else { theme.dim });
            if ui.selectable_label(shuffle, label).clicked() {
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
                if ui.selectable_label(is_current, text).clicked() && !is_current {
                    action = Some(Action::Play(path.clone()));
                }
            }
        });
    action
}
