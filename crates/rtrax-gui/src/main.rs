//! rtrax-gui — native GUI frontend (egui/eframe) for the rtrax engine.
//!
//! Proof-of-architecture scaffold. Drives playback exactly like the TUI does:
//! poll `SharedState` atomics each frame, drain the FFT ring into `Spectrum`,
//! and send `Command`s to the audio thread. No queue/browser/pattern view yet.

use anyhow::Result;
use clap::Parser;
use eframe::egui;
use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText};
use rtrax_core::audio::command::Command;
use rtrax_core::audio::{self, AudioHandle, FFT_RING_CAPACITY, FFT_RING_RATE_HZ};
use rtrax_core::fft::Spectrum;
use rtrax_core::state::SharedState;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tracing_subscriber::EnvFilter;

/// Modern-minimal tracker palette, mirroring the TUI's default theme:
/// low-saturation greens/cyans on near-black, magenta accents.
const BG: Color32 = Color32::from_rgb(0x0e, 0x12, 0x14);
const PANEL: Color32 = Color32::from_rgb(0x12, 0x17, 0x1a);
const TRACK: Color32 = Color32::from_rgb(0x1e, 0x28, 0x2b);
const FG: Color32 = Color32::from_rgb(0xcf, 0xe6, 0xd8);
const DIM: Color32 = Color32::from_rgb(0x5a, 0x6e, 0x66);
const GREEN: Color32 = Color32::from_rgb(0x66, 0xd9, 0xa5);
const CYAN: Color32 = Color32::from_rgb(0x5a, 0xc8, 0xc8);
const MAGENTA: Color32 = Color32::from_rgb(0xc6, 0x78, 0xa8);

/// UI tick. The audio thread runs independently; this only paces redraws.
const FRAME_TIME: Duration = Duration::from_millis(33);

#[derive(Parser)]
#[command(
    name = "rtrax-gui",
    version,
    about = "GUI MOD/XM/IT/S3M/MTM module player"
)]
struct Cli {
    /// Module file to play on startup. Files can also be dropped onto the window.
    file: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let state = Arc::new(SharedState::new());
    let (fft_tx, fft_rx) = rtrb::RingBuffer::<f32>::new(FFT_RING_CAPACITY);
    let audio = audio::start(state.clone(), fft_tx)?;

    let mut app = GuiApp::new(state, audio, fft_rx);
    if let Some(path) = cli.file.as_deref() {
        app.load_path(path);
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("rtrax")
            .with_inner_size([920.0, 540.0])
            .with_min_inner_size([560.0, 360.0]),
        ..Default::default()
    };
    eframe::run_native(
        "rtrax",
        options,
        Box::new(move |cc| {
            apply_theme(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
    .map_err(|err| anyhow::anyhow!("eframe: {err}"))
}

fn apply_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG;
    visuals.window_fill = BG;
    visuals.override_text_color = Some(FG);
    visuals.selection.bg_fill = GREEN.gamma_multiply(0.55);
    visuals.slider_trailing_fill = true;
    ctx.set_visuals(visuals);
}

struct GuiApp {
    state: Arc<SharedState>,
    audio: AudioHandle,
    fft_rx: rtrb::Consumer<f32>,
    spectrum: Spectrum,
    volume_millibel: i32,
    /// Load-error text shown in the transport bar until the next load.
    notice: Option<String>,
}

impl GuiApp {
    fn new(state: Arc<SharedState>, audio: AudioHandle, fft_rx: rtrb::Consumer<f32>) -> Self {
        Self {
            state,
            audio,
            fft_rx,
            spectrum: Spectrum::new(FFT_RING_RATE_HZ as f32, 48),
            volume_millibel: 0,
            notice: None,
        }
    }

    fn load_path(&mut self, path: &Path) {
        match audio::load_module(path) {
            Ok(loaded) => {
                audio::publish_loaded_metadata(&self.state, &loaded);
                self.audio.send(Command::Load(loaded.module));
                self.notice = None;
            }
            Err(err) => {
                tracing::error!(?err, "failed to load module");
                self.notice = Some(format!("can't load {}: {err:#}", path.display()));
            }
        }
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped: Option<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .find_map(|file| file.path.clone())
        });
        if let Some(path) = dropped {
            self.load_path(&path);
        }
    }

    fn header_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("header").show(ui, |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let title = self
                    .state
                    .title
                    .lock()
                    .map(|t| t.clone())
                    .unwrap_or_default();
                if title.is_empty() {
                    ui.label(RichText::new("rtrax").color(DIM).monospace().size(18.0));
                    ui.label(RichText::new("· drop a module file here").color(DIM));
                } else {
                    ui.label(RichText::new(title).color(FG).monospace().size(18.0));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let time = format!(
                        "{} / {}",
                        fmt_mmss(self.state.position_secs()),
                        fmt_mmss(self.state.duration_secs())
                    );
                    ui.label(RichText::new(time).color(DIM).monospace());
                });
            });
            ui.horizontal(|ui| {
                let format_label = self
                    .state
                    .format_label
                    .lock()
                    .map(|s| s.clone())
                    .unwrap_or_default();
                let artist = self
                    .state
                    .artist
                    .lock()
                    .map(|s| s.clone())
                    .unwrap_or_default();
                let mut sub = format_label;
                if !artist.is_empty() {
                    if !sub.is_empty() {
                        sub.push_str("  ·  ");
                    }
                    sub.push_str(&artist);
                }
                ui.label(RichText::new(sub).color(DIM).monospace().size(12.0));
            });
            ui.add_space(6.0);
            self.progress_bar(ui);
            ui.add_space(10.0);
        });
    }

    /// Seekable progress bar: click or drag maps to an absolute position, sent
    /// as a relative seek from the current one (the engine's only seek command).
    fn progress_bar(&mut self, ui: &mut egui::Ui) {
        let pos = self.state.position_secs();
        let dur = self.state.duration_secs();
        let frac = if dur > 0.0 {
            (pos / dur).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };

        let desired = egui::vec2(ui.available_width(), 6.0);
        let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::same(3), TRACK);
        if frac > 0.0 {
            let fill = Rect::from_min_max(
                rect.min,
                Pos2::new(rect.left() + rect.width() * frac, rect.bottom()),
            );
            painter.rect_filled(fill, CornerRadius::same(3), GREEN);
        }

        if (response.clicked() || response.dragged()) && dur > 0.0 && rect.width() > 0.0 {
            if let Some(pointer) = response.interact_pointer_pos() {
                let target =
                    ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64 * dur;
                self.audio
                    .send(Command::SeekRelative((target - pos) as f32));
            }
        }
    }

    fn transport_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("transport").show(ui, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let playing = self.state.playing.load(Ordering::Relaxed);
                let toggle = if playing { "pause" } else { "play" };
                if ui.button(RichText::new(toggle).monospace()).clicked() {
                    self.audio.send(if playing {
                        Command::Pause
                    } else {
                        Command::Play
                    });
                }
                if ui.button(RichText::new("stop").monospace()).clicked() {
                    self.audio.send(Command::Stop);
                }
                if ui.button(RichText::new("-5s").monospace()).clicked() {
                    self.audio.send(Command::SeekRelative(-5.0));
                }
                if ui.button(RichText::new("+5s").monospace()).clicked() {
                    self.audio.send(Command::SeekRelative(5.0));
                }

                ui.separator();

                let mut db = self.volume_millibel / 100;
                let slider = egui::Slider::new(&mut db, -40..=12).suffix(" dB");
                if ui.add(slider).changed() {
                    self.volume_millibel = db * 100;
                    self.audio
                        .send(Command::VolumeMillibel(self.volume_millibel));
                }

                if let Some(notice) = self.notice.clone() {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(notice).color(MAGENTA).monospace().size(12.0));
                    });
                }
            });
            ui.add_space(8.0);
        });
    }

    fn central_panel(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default_margins().show(ui, |ui| {
            let rect = ui.available_rect_before_wrap();
            let meter_width = 72.0;
            let gap = 12.0;
            let spectrum_rect = Rect::from_min_max(
                rect.min,
                Pos2::new((rect.max.x - meter_width - gap).max(rect.min.x), rect.max.y),
            );
            let meter_rect =
                Rect::from_min_max(Pos2::new(rect.max.x - meter_width, rect.min.y), rect.max);

            let bands = ((spectrum_rect.width() / 14.0) as usize).clamp(16, 64);
            self.spectrum.resize_bands(bands);
            draw_spectrum(ui.painter(), spectrum_rect, self.spectrum.bands());

            let (left, right) = self.state.master_peak();
            draw_master_meters(ui.painter(), meter_rect, left, right);

            let hovering_file = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
            if hovering_file {
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "drop to play",
                    FontId::monospace(20.0),
                    MAGENTA,
                );
            }
        });
    }
}

impl eframe::App for GuiApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.spectrum.step(&mut self.fft_rx);
        self.audio.drain_drops();
        // Song ended. No queue in the GUI yet, so just clear the flag; the
        // audio thread has already stopped decoding.
        self.state.eof.swap(false, Ordering::Relaxed);

        self.handle_dropped_files(ctx);

        ctx.request_repaint_after(FRAME_TIME);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Panels are outermost-first; the central panel must come last.
        self.header_panel(ui);
        self.transport_panel(ui);
        self.central_panel(ui);
    }
}

fn draw_spectrum(painter: &egui::Painter, rect: Rect, bands: &[f32]) {
    painter.rect_filled(rect, CornerRadius::same(4), PANEL);
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
        let bar = Rect::from_min_max(
            Pos2::new(x0 + 1.0, rect.bottom() - inset - height),
            Pos2::new(x0 + band_width - 1.0, rect.bottom() - inset),
        );
        let t = i as f32 / bands.len() as f32;
        painter.rect_filled(bar, CornerRadius::same(1), lerp_color(CYAN, GREEN, t));
    }
}

/// Two vertical peak bars (post-mix master, per side). The top of the range
/// flips to magenta as a clip warning, matching the TUI's master meter.
fn draw_master_meters(painter: &egui::Painter, rect: Rect, left: f32, right: f32) {
    painter.rect_filled(rect, CornerRadius::same(4), PANEL);
    let inset = 8.0;
    let label_height = 16.0;
    let bar_top = rect.top() + inset;
    let bar_bottom = rect.bottom() - inset - label_height;
    if bar_bottom <= bar_top {
        return;
    }
    let bar_width = (rect.width() - inset * 3.0) / 2.0;

    for (i, (label, value)) in [("L", left), ("R", right)].into_iter().enumerate() {
        let x0 = rect.left() + inset + i as f32 * (bar_width + inset);
        let track = Rect::from_min_max(
            Pos2::new(x0, bar_top),
            Pos2::new(x0 + bar_width, bar_bottom),
        );
        painter.rect_filled(track, CornerRadius::same(2), TRACK);

        let value = value.clamp(0.0, 1.0);
        let height = value * track.height();
        if height >= 1.0 {
            let fill = Rect::from_min_max(
                Pos2::new(track.left(), track.bottom() - height),
                Pos2::new(track.right(), track.bottom()),
            );
            let color = if value > 0.9 { MAGENTA } else { GREEN };
            painter.rect_filled(fill, CornerRadius::same(2), color);
        }

        painter.text(
            Pos2::new(track.center().x, bar_bottom + 2.0),
            Align2::CENTER_TOP,
            label,
            FontId::monospace(12.0),
            DIM,
        );
    }
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let ch = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgb(ch(a.r(), b.r()), ch(a.g(), b.g()), ch(a.b(), b.b()))
}

fn fmt_mmss(secs: f64) -> String {
    let total = secs.max(0.0) as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_mmss_formats_minutes_and_seconds() {
        assert_eq!(fmt_mmss(0.0), "00:00");
        assert_eq!(fmt_mmss(59.9), "00:59");
        assert_eq!(fmt_mmss(61.0), "01:01");
        assert_eq!(fmt_mmss(600.0), "10:00");
        assert_eq!(fmt_mmss(-3.0), "00:00");
    }

    #[test]
    fn lerp_color_endpoints_and_midpoint() {
        let a = Color32::from_rgb(0, 0, 0);
        let b = Color32::from_rgb(200, 100, 50);
        assert_eq!(lerp_color(a, b, 0.0), a);
        assert_eq!(lerp_color(a, b, 1.0), b);
        assert_eq!(lerp_color(a, b, 0.5), Color32::from_rgb(100, 50, 25));
    }
}
