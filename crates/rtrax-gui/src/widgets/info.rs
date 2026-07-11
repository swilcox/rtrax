//! Live "now playing" list for the right pane (the TUI's `i` view): one row
//! per channel showing the most-recently-seen instrument number + name, read
//! from `SharedState::last_instrument` each frame. Names come from
//! libopenmpt's instrument list first, then fall back to the sample list
//! (MOD/S3M often have only samples; XM/IT have both). Instrument numbers
//! are hex to match the pattern view's instrument column.

use crate::theme::Theme;
use eframe::egui::{self, RichText};
use rtrax_core::state::SharedState;
use std::sync::atomic::Ordering;

pub fn show(ui: &mut egui::Ui, state: &SharedState, theme: &Theme) {
    let n = state.num_channels.load(Ordering::Relaxed).max(0) as usize;
    let instruments = state
        .instrument_names
        .lock()
        .map(|v| v.clone())
        .unwrap_or_default();
    let samples = state
        .sample_names
        .lock()
        .map(|v| v.clone())
        .unwrap_or_default();

    ui.add_space(6.0);
    ui.label(
        RichText::new(format!("now playing · {n} ch"))
            .color(theme.dim)
            .monospace(),
    );
    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            for ch in 0..n {
                let inst = state.last_instrument(ch);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{:02}", ch + 1))
                            .color(theme.dim)
                            .monospace()
                            .size(12.0),
                    );
                    if inst <= 0 {
                        // No instrument event on this channel yet.
                        ui.label(
                            RichText::new("·· (idle)")
                                .color(theme.dim.gamma_multiply(0.8))
                                .monospace()
                                .size(12.0),
                        );
                    } else {
                        ui.label(
                            RichText::new(format!("{inst:02X}"))
                                .color(theme.instrument)
                                .monospace()
                                .size(12.0),
                        );
                        ui.add(
                            egui::Label::new(
                                RichText::new(resolve_name(inst, &instruments, &samples))
                                    .color(theme.fg.gamma_multiply(0.9))
                                    .monospace()
                                    .size(12.0),
                            )
                            .truncate(),
                        );
                    }
                });
            }
        });
}

fn resolve_name(instrument_1based: i32, instruments: &[String], samples: &[String]) -> String {
    let idx = (instrument_1based - 1).max(0) as usize;
    if let Some(name) = instruments.get(idx) {
        if !name.trim().is_empty() {
            return name.trim().to_string();
        }
    }
    if let Some(name) = samples.get(idx) {
        if !name.trim().is_empty() {
            return name.trim().to_string();
        }
    }
    "—".to_string()
}
