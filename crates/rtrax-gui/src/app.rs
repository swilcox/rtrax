//! The GUI application: panel layout, playback control, queue handling, and
//! keyboard shortcuts. Same frontend contract as the TUI — poll `SharedState`
//! per frame, drain the FFT ring, send `Command`s.

use crate::media::Media;
use crate::theme::{self, Theme};
use crate::widgets;
use crate::widgets::icons::{icon_button, toggle_icon_button, Icon};
use eframe::egui::{self, CornerRadius, Key, Pos2, Rect, RichText, Sense};
use rtrax_core::audio::command::Command;
use rtrax_core::audio::{self, AudioHandle, FFT_RING_RATE_HZ};
use rtrax_core::fft::Spectrum;
use rtrax_core::files;
use rtrax_core::meters::{ChannelMeters, MasterMeter};
use rtrax_core::playlist::Playlist;
use rtrax_core::state::SharedState;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

/// UI tick. The audio thread runs independently; this only paces redraws.
const FRAME_TIME: Duration = Duration::from_millis(33);
const VIZ_HEIGHT: f32 = 96.0;
/// Above this many channels the pattern view starts in compact cells.
const COMPACT_CHANNEL_THRESHOLD: i32 = 16;

pub struct GuiApp {
    state: Arc<SharedState>,
    audio: AudioHandle,
    fft_rx: rtrb::Consumer<f32>,
    spectrum: Spectrum,
    channel_meters: ChannelMeters,
    master: MasterMeter,
    queue: Option<Playlist>,
    current_path: Option<PathBuf>,
    shuffle: bool,
    volume_millibel: i32,
    /// Load-error text shown in the transport bar until the next load.
    notice: Option<String>,
    /// Last title pushed to the OS window, to avoid resending every frame.
    window_title: String,
    /// System media controls (Now Playing). `None` when unavailable.
    media: Option<Media>,
    /// Built-in + custom themes, in cycle order.
    themes: Vec<Theme>,
    theme: Theme,
    theme_idx: usize,
    show_queue: bool,
    /// Right pane: instrument/sample names instead of channel meters.
    show_info: bool,
    /// Floating song-info window (metadata + song message).
    show_message: bool,
    /// Compact pattern cells (note + instrument only). Auto-set from the
    /// channel count on load; manual toggles stick until the next module.
    compact: bool,
    /// Channel count the compact auto-pick was last applied for.
    last_layout_channels: i32,
}

impl GuiApp {
    pub fn new(
        state: Arc<SharedState>,
        audio: AudioHandle,
        fft_rx: rtrb::Consumer<f32>,
        queue: Option<Playlist>,
        shuffle: bool,
        themes: Vec<Theme>,
        theme_idx: usize,
    ) -> Self {
        let themes = if themes.is_empty() {
            theme::built_ins()
        } else {
            themes
        };
        let theme_idx = theme_idx.min(themes.len() - 1);
        Self {
            state,
            audio,
            fft_rx,
            spectrum: Spectrum::new(FFT_RING_RATE_HZ as f32, 48),
            channel_meters: ChannelMeters::new(),
            master: MasterMeter::new(),
            queue,
            current_path: None,
            shuffle,
            volume_millibel: 0,
            notice: None,
            window_title: String::new(),
            media: None,
            theme: themes[theme_idx].clone(),
            themes,
            theme_idx,
            show_queue: false,
            show_info: false,
            show_message: false,
            compact: false,
            last_layout_channels: -1,
        }
    }

    /// One-time setup from the eframe creator, once the egui context exists:
    /// apply the theme and attach OS media controls (whose event callback
    /// uses the context to wake the UI).
    pub fn init(&mut self, ctx: &egui::Context) {
        apply_theme(ctx, &self.theme);
        self.media = Media::new(ctx.clone());
    }

    fn cycle_theme(&mut self, ctx: &egui::Context) {
        self.theme_idx = (self.theme_idx + 1) % self.themes.len();
        self.theme = self.themes[self.theme_idx].clone();
        apply_theme(ctx, &self.theme);
    }

    fn handle_media_events(&mut self, ctx: &egui::Context) {
        use souvlaki::{MediaControlEvent as E, SeekDirection};
        let events = self.media.as_ref().map(Media::events).unwrap_or_default();
        for event in events {
            match event {
                E::Play => self.audio.send(Command::Play),
                E::Pause => self.audio.send(Command::Pause),
                E::Toggle => self.toggle_play(),
                E::Next => self.play_next(),
                E::Previous => self.play_prev(),
                E::Stop => self.audio.send(Command::Stop),
                E::Seek(direction) => {
                    let secs = match direction {
                        SeekDirection::Forward => 5.0,
                        SeekDirection::Backward => -5.0,
                    };
                    self.audio.send(Command::SeekRelative(secs));
                }
                E::SeekBy(direction, amount) => {
                    let secs = amount.as_secs_f32();
                    let secs = match direction {
                        SeekDirection::Forward => secs,
                        SeekDirection::Backward => -secs,
                    };
                    self.audio.send(Command::SeekRelative(secs));
                }
                E::SetPosition(position) => {
                    let delta = position.0.as_secs_f64() - self.state.position_secs();
                    self.audio.send(Command::SeekRelative(delta as f32));
                }
                E::OpenUri(uri) => {
                    let path = uri.strip_prefix("file://").unwrap_or(&uri);
                    self.open_paths(&[PathBuf::from(path)]);
                }
                E::SetVolume(_) => {} // MPRIS-only; the in-app slider owns gain
                E::Raise => ctx.send_viewport_cmd(egui::ViewportCommand::Focus),
                E::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            }
        }
        if let Some(media) = self.media.as_mut() {
            media.sync(&self.state);
        }
    }

    pub fn load_path(&mut self, path: &Path) {
        match audio::load_module(path) {
            Ok(loaded) => {
                audio::publish_loaded_metadata(&self.state, &loaded);
                self.audio.send(Command::Load(loaded.module));
                self.current_path = Some(path.to_path_buf());
                self.notice = None;
            }
            Err(err) => {
                tracing::error!(?err, "failed to load module");
                self.notice = Some(format!("can't load {}: {err:#}", path.display()));
            }
        }
    }

    fn play_next(&mut self) {
        let next = match (&self.queue, &self.current_path) {
            (Some(queue), Some(current)) => queue.next_after(current),
            _ => None,
        };
        if let Some(next) = next {
            self.load_path(&next);
        }
    }

    fn play_prev(&mut self) {
        let prev = match (&self.queue, &self.current_path) {
            (Some(queue), Some(current)) => queue.prev_before(current),
            _ => None,
        };
        if let Some(prev) = prev {
            self.load_path(&prev);
        }
    }

    fn toggle_shuffle(&mut self) {
        self.shuffle = !self.shuffle;
        let anchor = self.current_path.clone();
        if let Some(queue) = self.queue.as_mut() {
            queue.set_shuffle(self.shuffle, anchor.as_deref());
        }
    }

    fn toggle_play(&mut self) {
        let playing = self.state.playing.load(Ordering::Relaxed);
        self.audio.send(if playing {
            Command::Pause
        } else {
            Command::Play
        });
    }

    /// Replace the queue from dropped/opened paths and start playing.
    fn open_paths(&mut self, paths: &[PathBuf]) {
        let (mut queue, initial) = build_queue(paths);
        if queue.is_empty() {
            self.notice = Some("no module files found in the dropped paths".into());
            return;
        }
        queue.set_shuffle(self.shuffle, initial.as_deref());
        let initial = initial.or_else(|| queue.start().cloned());
        self.queue = Some(queue);
        if let Some(path) = initial {
            self.load_path(&path);
        }
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|file| file.path.clone())
                .collect()
        });
        if !dropped.is_empty() {
            self.open_paths(&dropped);
        }
    }

    /// Auto-pick compact cells when a newly loaded module's channel count
    /// crosses the threshold. Manual toggles stick until the next load.
    fn maybe_auto_layout(&mut self) {
        let n = self.state.num_channels.load(Ordering::Relaxed);
        if n > 0 && n != self.last_layout_channels {
            self.last_layout_channels = n;
            self.compact = n > COMPACT_CHANNEL_THRESHOLD;
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        // Don't steal keys from a focused widget (e.g. a slider being driven
        // by arrow keys) — and leave Tab to egui's focus traversal then.
        if ctx.memory(|m| m.focused().is_some()) {
            return;
        }
        // Tab must be consumed, not just read, or egui focuses a widget too.
        let tab = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Tab));
        if tab {
            self.show_queue = !self.show_queue;
        }
        struct Keys {
            play: bool,
            next: bool,
            prev: bool,
            stop: bool,
            fwd: bool,
            back: bool,
            shuffle: bool,
            theme: bool,
            compact: bool,
            message: bool,
            info: bool,
        }
        let keys = ctx.input(|i| Keys {
            play: i.key_pressed(Key::Space),
            next: i.key_pressed(Key::N),
            prev: i.key_pressed(Key::P),
            stop: i.key_pressed(Key::S),
            fwd: i.key_pressed(Key::ArrowRight),
            back: i.key_pressed(Key::ArrowLeft),
            shuffle: i.key_pressed(Key::Z),
            theme: i.key_pressed(Key::T),
            compact: i.key_pressed(Key::C),
            message: i.key_pressed(Key::M),
            info: i.key_pressed(Key::I),
        });
        if keys.play {
            self.toggle_play();
        }
        if keys.next {
            self.play_next();
        }
        if keys.prev {
            self.play_prev();
        }
        if keys.stop {
            self.audio.send(Command::Stop);
        }
        if keys.fwd {
            self.audio.send(Command::SeekRelative(5.0));
        }
        if keys.back {
            self.audio.send(Command::SeekRelative(-5.0));
        }
        if keys.shuffle {
            self.toggle_shuffle();
        }
        if keys.theme {
            self.cycle_theme(ctx);
        }
        if keys.compact {
            self.compact = !self.compact;
        }
        if keys.message {
            self.show_message = !self.show_message;
        }
        if keys.info {
            self.show_info = !self.show_info;
        }
    }

    fn sync_window_title(&mut self, ctx: &egui::Context) {
        let song = self
            .state
            .title
            .lock()
            .map(|t| t.clone())
            .unwrap_or_default();
        let title = if song.is_empty() {
            "rtrax".to_string()
        } else {
            format!("{song} — rtrax")
        };
        if title != self.window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
    }

    // ── panels ──────────────────────────────────────────────────────────────

    fn header_panel(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme.clone();
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
                    ui.label(
                        RichText::new("rtrax")
                            .color(theme.dim)
                            .monospace()
                            .size(18.0),
                    );
                    ui.label(RichText::new("· drop module files or folders here").color(theme.dim));
                } else {
                    ui.label(RichText::new(title).color(theme.fg).monospace().size(18.0));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let time = format!(
                        "{} / {}",
                        theme::fmt_mmss(self.state.position_secs()),
                        theme::fmt_mmss(self.state.duration_secs())
                    );
                    ui.label(RichText::new(time).color(theme.dim).monospace());
                });
            });
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(self.subtitle_left())
                        .color(theme.dim)
                        .monospace()
                        .size(12.0),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(self.subtitle_right())
                            .color(theme.dim)
                            .monospace()
                            .size(12.0),
                    );
                });
            });
            ui.add_space(6.0);
            self.progress_bar(ui);
            ui.add_space(10.0);
        });
    }

    fn subtitle_left(&self) -> String {
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
        sub
    }

    fn subtitle_right(&self) -> String {
        let order = self.state.current_order.load(Ordering::Relaxed);
        let orders = self.state.num_orders.load(Ordering::Relaxed);
        if orders <= 0 {
            return String::new();
        }
        let row = self.state.current_row.load(Ordering::Relaxed);
        let rows = self.state.current_rows_in_pattern.load(Ordering::Relaxed);
        let tempo = self.state.current_tempo.load(Ordering::Relaxed);
        let speed = self.state.current_speed.load(Ordering::Relaxed);
        format!("ord {order:02}/{orders:02}  row {row:02}/{rows:02}  bpm {tempo}  spd {speed}")
    }

    /// Seekable progress bar: click or drag maps to an absolute position, sent
    /// as a relative seek from the current one (the engine's only seek command).
    fn progress_bar(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme.clone();
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
        painter.rect_filled(rect, CornerRadius::same(3), theme.track);
        if frac > 0.0 {
            let fill = Rect::from_min_max(
                rect.min,
                Pos2::new(rect.left() + rect.width() * frac, rect.bottom()),
            );
            painter.rect_filled(fill, CornerRadius::same(3), theme.fill);
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
        let theme = self.theme.clone();
        egui::Panel::bottom("transport").show(ui, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if icon_button(ui, Icon::Prev, "previous track (p)", &theme).clicked() {
                    self.play_prev();
                }
                if icon_button(ui, Icon::Rewind, "back 5s (←)", &theme).clicked() {
                    self.audio.send(Command::SeekRelative(-5.0));
                }
                let playing = self.state.playing.load(Ordering::Relaxed);
                let (toggle_icon, toggle_tip) = if playing {
                    (Icon::Pause, "pause (space)")
                } else {
                    (Icon::Play, "play (space)")
                };
                if icon_button(ui, toggle_icon, toggle_tip, &theme).clicked() {
                    self.toggle_play();
                }
                if icon_button(ui, Icon::Stop, "stop (s)", &theme).clicked() {
                    self.audio.send(Command::Stop);
                }
                if icon_button(ui, Icon::FastForward, "forward 5s (→)", &theme).clicked() {
                    self.audio.send(Command::SeekRelative(5.0));
                }
                if icon_button(ui, Icon::Next, "next track (n)", &theme).clicked() {
                    self.play_next();
                }

                ui.separator();

                let mut db = self.volume_millibel / 100;
                let slider = egui::Slider::new(&mut db, -40..=12).suffix(" dB");
                if ui.add(slider).changed() {
                    self.volume_millibel = db * 100;
                    self.audio
                        .send(Command::VolumeMillibel(self.volume_millibel));
                }

                // View toggles + theme cycler live on the right; a load-error
                // notice squeezes in to their left when present. Keyboard
                // shortcuts are documented in the tooltips.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let tip = format!("theme: {} — click to cycle (t)", theme.name);
                    if icon_button(ui, Icon::Palette, &tip, &theme).clicked() {
                        let ctx = ui.ctx().clone();
                        self.cycle_theme(&ctx);
                    }
                    if toggle_icon_button(
                        ui,
                        Icon::Compact,
                        self.compact,
                        "compact pattern cells (c)",
                        &theme,
                    )
                    .clicked()
                    {
                        self.compact = !self.compact;
                    }
                    if toggle_icon_button(
                        ui,
                        Icon::SongInfo,
                        self.show_message,
                        "song message + metadata (m)",
                        &theme,
                    )
                    .clicked()
                    {
                        self.show_message = !self.show_message;
                    }
                    if toggle_icon_button(
                        ui,
                        Icon::Instruments,
                        self.show_info,
                        "live instrument per channel (i)",
                        &theme,
                    )
                    .clicked()
                    {
                        self.show_info = !self.show_info;
                    }
                    if toggle_icon_button(
                        ui,
                        Icon::Queue,
                        self.show_queue,
                        "show/hide the queue (tab)",
                        &theme,
                    )
                    .clicked()
                    {
                        self.show_queue = !self.show_queue;
                    }
                    if let Some(notice) = &self.notice {
                        ui.label(
                            RichText::new(notice)
                                .color(theme.accent)
                                .monospace()
                                .size(12.0),
                        );
                    }
                });
            });
            ui.add_space(8.0);
        });
    }

    fn viz_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("viz").show(ui, |ui| {
            ui.add_space(6.0);
            let (rect, _) = ui
                .allocate_exact_size(egui::vec2(ui.available_width(), VIZ_HEIGHT), Sense::hover());
            self.spectrum
                .resize_bands(widgets::viz::band_count_for(rect));
            widgets::viz::show(
                ui.painter(),
                rect,
                self.spectrum.bands(),
                &self.master,
                &self.theme,
            );
            ui.add_space(6.0);
        });
    }

    fn queue_panel(&mut self, ui: &mut egui::Ui) {
        let show_queue = self.show_queue && self.queue.as_ref().is_some_and(|q| !q.is_empty());
        if !show_queue {
            return;
        }
        let theme = self.theme.clone();
        let mut action = None;
        egui::Panel::left("queue")
            .default_size(230.0)
            .show(ui, |ui| {
                if let Some(queue) = &self.queue {
                    action = widgets::queue::show(
                        ui,
                        queue,
                        self.current_path.as_deref(),
                        self.shuffle,
                        &theme,
                    );
                }
            });
        match action {
            Some(widgets::queue::Action::Play(path)) => self.load_path(&path),
            Some(widgets::queue::Action::ToggleShuffle) => self.toggle_shuffle(),
            None => {}
        }
    }

    fn side_panel(&mut self, ui: &mut egui::Ui) {
        if self.channel_meters.is_empty() {
            return;
        }
        let theme = self.theme.clone();
        egui::Panel::right("side")
            .default_size(170.0)
            .show(ui, |ui| {
                if self.show_info {
                    widgets::info::show(ui, &self.state, &theme);
                } else {
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(format!("channels · {}", self.channel_meters.len()))
                            .color(theme.dim)
                            .monospace(),
                    );
                    ui.separator();
                    widgets::meters::show(ui, &self.channel_meters, &theme);
                }
            });
    }

    /// Floating song-info window: metadata block + the module's free-form
    /// song message / liner notes.
    fn message_window(&mut self, ctx: &egui::Context) {
        if !self.show_message {
            return;
        }
        let theme = self.theme.clone();
        let mut open = self.show_message;
        egui::Window::new("song info")
            .open(&mut open)
            .collapsible(false)
            .default_size([480.0, 380.0])
            .show(ctx, |ui| {
                let grab =
                    |m: &std::sync::Mutex<String>| m.lock().map(|s| s.clone()).unwrap_or_default();
                let rows = [
                    ("title", grab(&self.state.title)),
                    ("format", grab(&self.state.format_label)),
                    ("artist", grab(&self.state.artist)),
                    ("tracker", grab(&self.state.tracker)),
                ];
                for (label, value) in rows {
                    if value.trim().is_empty() {
                        continue;
                    }
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{label:>8}"))
                                .color(theme.dim)
                                .monospace()
                                .size(12.0),
                        );
                        ui.label(RichText::new(value).color(theme.fg).monospace().size(12.0));
                    });
                }
                ui.separator();
                let message = grab(&self.state.song_message);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if message.trim().is_empty() {
                            ui.label(
                                RichText::new("no song message")
                                    .color(theme.dim)
                                    .monospace()
                                    .size(12.0),
                            );
                        } else {
                            ui.label(
                                RichText::new(message)
                                    .color(theme.fg.gamma_multiply(0.9))
                                    .monospace()
                                    .size(12.0),
                            );
                        }
                    });
            });
        self.show_message = open;
    }
}

impl eframe::App for GuiApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.spectrum.step(&mut self.fft_rx);
        self.channel_meters.step(&self.state);
        self.master.step(&self.state);
        self.audio.drain_drops();
        self.maybe_auto_layout();

        // Song ended: advance through the queue, if there is more to play.
        if self.state.eof.swap(false, Ordering::Relaxed) {
            self.play_next();
        }

        self.handle_dropped_files(ctx);
        self.handle_shortcuts(ctx);
        self.handle_media_events(ctx);
        self.sync_window_title(ctx);

        ctx.request_repaint_after(FRAME_TIME);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Panels are outermost-first; the central panel must come last.
        self.header_panel(ui);
        self.transport_panel(ui);
        self.viz_panel(ui);
        self.queue_panel(ui);
        self.side_panel(ui);
        let theme = self.theme.clone();
        let compact = self.compact;
        egui::CentralPanel::default_margins().show(ui, |ui| {
            widgets::pattern::show(ui, &self.state, &theme, compact);
            let hovering_file = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
            if hovering_file {
                let rect = ui.max_rect();
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "drop to play",
                    egui::FontId::monospace(20.0),
                    theme.accent,
                );
            }
        });
        self.message_window(&ui.ctx().clone());
    }
}

pub fn apply_theme(ctx: &egui::Context, theme: &Theme) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = theme.bg;
    visuals.window_fill = theme.panel;
    visuals.override_text_color = Some(theme.fg);
    visuals.selection.bg_fill = theme.fill.gamma_multiply(0.55);
    visuals.slider_trailing_fill = true;
    ctx.set_visuals(visuals);
}

/// Build a queue from files and/or directories:
/// - a single file expands to all modules in its folder (so auto-advance walks
///   the folder, like the TUI's browse mode), starting at that file;
/// - directories expand to the modules directly inside them;
/// - multiple explicit files stay exactly as given.
///
/// Returns the queue plus the track to start on (`None` when the caller
/// should fall back to the queue's play-order head).
pub fn build_queue(paths: &[PathBuf]) -> (Playlist, Option<PathBuf>) {
    let paths: Vec<PathBuf> = paths.iter().map(|p| absolutize(p)).collect();
    match paths.as_slice() {
        [single] if single.is_file() => {
            let folder = single
                .parent()
                .map(files::modules_in_dir)
                .unwrap_or_default();
            if folder.iter().any(|p| p == single) {
                (Playlist::from_files(folder), Some(single.clone()))
            } else {
                // Extension not recognized or scan failed — queue just the file
                // and let libopenmpt decide whether it can play it.
                (
                    Playlist::from_files(vec![single.clone()]),
                    Some(single.clone()),
                )
            }
        }
        many => {
            let mut out = Vec::new();
            for path in many {
                if path.is_dir() {
                    out.extend(files::modules_in_dir(path));
                } else {
                    out.push(path.clone());
                }
            }
            let initial = out.first().cloned();
            (Playlist::from_files(out), initial)
        }
    }
}

fn absolutize(p: &Path) -> PathBuf {
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|d| d.join(p))
            .unwrap_or_else(|_| p.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn single_file_expands_to_its_folder() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["a.xm", "b.mod", "c.it"] {
            fs::write(dir.path().join(name), b"").unwrap();
        }
        fs::write(dir.path().join("cover.png"), b"").unwrap();

        let target = dir.path().join("b.mod");
        let (queue, initial) = build_queue(std::slice::from_ref(&target));

        assert_eq!(queue.len(), 3);
        assert_eq!(initial.as_deref(), Some(target.as_path()));
        // Sorted folder scan: a.xm, b.mod, c.it
        assert!(queue.get(0).unwrap().ends_with("a.xm"));
    }

    #[test]
    fn directory_expands_to_its_modules() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("x.s3m"), b"").unwrap();
        fs::write(dir.path().join("y.xm"), b"").unwrap();

        let (queue, initial) = build_queue(&[dir.path().to_path_buf()]);

        assert_eq!(queue.len(), 2);
        assert!(initial.unwrap().ends_with("x.s3m"));
    }

    #[test]
    fn multiple_files_stay_as_given() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.xm");
        let b = dir.path().join("b.xm");
        let c = dir.path().join("c.xm");
        for p in [&a, &b, &c] {
            fs::write(p, b"").unwrap();
        }

        // Only two of the three are passed — the third must not sneak in.
        let (queue, initial) = build_queue(&[c.clone(), a.clone()]);

        assert_eq!(queue.len(), 2);
        assert_eq!(initial.as_deref(), Some(c.as_path()));
        assert_eq!(queue.get(0), Some(&c));
    }

    #[test]
    fn unrecognized_single_file_queues_alone() {
        let dir = tempfile::tempdir().unwrap();
        let odd = dir.path().join("song.weird");
        fs::write(&odd, b"").unwrap();
        fs::write(dir.path().join("other.xm"), b"").unwrap();

        let (queue, initial) = build_queue(std::slice::from_ref(&odd));

        assert_eq!(queue.len(), 1);
        assert_eq!(initial.as_deref(), Some(odd.as_path()));
    }

    #[test]
    fn empty_input_is_empty_queue() {
        let (queue, initial) = build_queue(&[]);
        assert!(queue.is_empty());
        assert!(initial.is_none());
    }
}
