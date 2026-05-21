//! TUI shell. Owns the terminal, runs the event loop at ~30fps, dispatches
//! input actions, and renders the composed widget tree each frame.

pub mod fft;
pub mod theme;
pub mod widgets;

use crate::audio::command::Command;
use crate::audio::AudioHandle;
use crate::config::{BuiltInTheme, Config, ThemeChoice};
use crate::input::{match_key, Action};
use crate::state::SharedState;
use crate::ui::fft::Spectrum;
use crate::ui::theme::Theme;
use crate::ui::widgets::browser::Browser;
use crate::ui::widgets::master::MasterMeterState;
use crate::ui::widgets::meters::MeterState;
use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Terminal;
use rtrb::Consumer;
use std::io::{stdout, Stdout};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// RAII guard that puts the terminal into raw + alternate-screen mode on
/// construction, and restores it on Drop — even on panic. Pair with the panic
/// hook installed in main.
pub struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    pub fn install() -> Result<Self> {
        enable_raw_mode().context("enabling raw mode")?;
        let mut out = stdout();
        execute!(out, EnterAlternateScreen).context("entering alternate screen")?;
        let backend = CrosstermBackend::new(out);
        let mut terminal = Terminal::new(backend).context("creating ratatui terminal")?;
        terminal.hide_cursor().ok();
        terminal.clear().ok();
        Ok(Self { terminal })
    }

    pub fn terminal(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        &mut self.terminal
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
    }
}

/// Restore the terminal without going through Drop. Called from the panic
/// hook so the panic message lands on a clean shell.
pub fn restore_terminal_for_panic() {
    let _ = disable_raw_mode();
    let _ = execute!(stdout(), LeaveAlternateScreen);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Focus {
    Pattern,
    Browser,
}

pub struct App {
    state: Arc<SharedState>,
    audio: AudioHandle,
    fft_rx: Consumer<f32>,
    spectrum: Spectrum,
    meter_state: MeterState,
    master_state: MasterMeterState,
    browser: Browser,
    theme: Theme,
    theme_choice: ThemeChoice,
    theme_choices: Vec<ThemeChoice>,
    config: Config,
    focus: Focus,
    show_help: bool,
    show_info: bool,
    should_quit: bool,
    volume_millibel: i32,
    /// Most recent path we asked the audio thread to play. Drives n/p in the
    /// folder.
    current_path: Option<PathBuf>,
}

impl App {
    pub fn new(
        state: Arc<SharedState>,
        audio: AudioHandle,
        fft_rx: Consumer<f32>,
        config: Config,
        initial_path: Option<PathBuf>,
    ) -> Result<Self> {
        let spectrum = Spectrum::new(crate::audio::FFT_RING_RATE_HZ as f32, 48);

        let browse_root = initial_path
            .as_ref()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .or_else(|| config.default_browse_path.clone())
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));

        let mut theme_choices = Theme::available_choices();
        if !theme_choices.contains(&config.theme) {
            theme_choices.push(config.theme.clone());
        }
        let theme = resolve_theme(&config.theme);

        Ok(Self {
            state,
            audio,
            fft_rx,
            spectrum,
            meter_state: MeterState::new(),
            master_state: MasterMeterState::new(),
            browser: Browser::new(browse_root),
            theme,
            theme_choice: config.theme.clone(),
            theme_choices,
            config,
            focus: Focus::Pattern,
            show_help: false,
            show_info: false,
            should_quit: false,
            volume_millibel: 0,
            current_path: initial_path,
        })
    }

    pub fn run(mut self) -> Result<()> {
        let mut guard = TerminalGuard::install()?;
        let frame_time = Duration::from_millis(33); // ~30fps
        let mut last_draw = Instant::now() - frame_time;

        while !self.should_quit {
            self.spectrum.step(&mut self.fft_rx);
            self.meter_state.step(&self.state);
            self.master_state.step(&self.state);
            self.audio.drain_drops();

            // Auto-advance if the song ended.
            if self.state.eof.swap(false, Ordering::Relaxed) {
                if let Some(path) = self.current_path.clone() {
                    if let Some(next) = self.browser.next_module(Some(&path)) {
                        self.load_path(next);
                    }
                }
            }

            // Poll input (non-blocking with a short timeout so render still ticks).
            let timeout = frame_time
                .checked_sub(last_draw.elapsed())
                .unwrap_or_else(|| Duration::from_millis(0));
            if event::poll(timeout).unwrap_or(false) {
                match event::read() {
                    Ok(Event::Key(k)) if k.kind != KeyEventKind::Release => {
                        self.handle_key(k);
                    }
                    Ok(Event::Resize(_, _)) => {
                        // ratatui handles redraw; we'll resize bands below.
                    }
                    _ => {}
                }
            }

            if last_draw.elapsed() >= frame_time {
                self.draw(guard.terminal())?;
                last_draw = Instant::now();
            }
        }

        // Stop audio cleanly. The Stream is held by the AudioHandle; dropping
        // AudioHandle stops the cpal stream.
        self.audio.send(Command::Pause);
        Ok(())
    }

    fn handle_key(&mut self, k: crossterm::event::KeyEvent) {
        let Some(action) = match_key(&self.config.keymap, &k) else {
            return;
        };
        match action {
            Action::Quit => self.should_quit = true,
            Action::Esc => {
                if self.show_help {
                    self.show_help = false;
                } else if self.focus == Focus::Browser {
                    self.focus = Focus::Pattern;
                }
            }
            Action::Help => self.show_help = !self.show_help,
            Action::ToggleInfo => self.show_info = !self.show_info,
            Action::PlayPause => {
                let playing = self.state.playing.load(Ordering::Relaxed);
                self.audio.send(if playing {
                    Command::Pause
                } else {
                    Command::Play
                });
            }
            Action::Stop => self.audio.send(Command::Stop),
            Action::Next => {
                let cur = self.current_path.clone();
                if let Some(next) = self.browser.next_module(cur.as_deref()) {
                    self.load_path(next);
                }
            }
            Action::Prev => {
                let cur = self.current_path.clone();
                if let Some(prev) = self.browser.prev_module(cur.as_deref()) {
                    self.load_path(prev);
                }
            }
            Action::SeekForward => self.audio.send(Command::SeekRelative(5.0)),
            Action::SeekBack => self.audio.send(Command::SeekRelative(-5.0)),
            Action::VolumeUp => {
                self.volume_millibel = (self.volume_millibel + 200).min(1200);
                self.audio
                    .send(Command::VolumeMillibel(self.volume_millibel));
            }
            Action::VolumeDown => {
                self.volume_millibel = (self.volume_millibel - 200).max(-4000);
                self.audio
                    .send(Command::VolumeMillibel(self.volume_millibel));
            }
            Action::FocusBrowser => self.focus = Focus::Browser,
            Action::CycleFocus => {
                self.focus = match self.focus {
                    Focus::Pattern => Focus::Browser,
                    Focus::Browser => Focus::Pattern,
                };
            }
            Action::CycleTheme => self.cycle_theme(),
            Action::Up => {
                if self.focus == Focus::Browser {
                    self.browser.select_delta(-1);
                }
            }
            Action::Down => {
                if self.focus == Focus::Browser {
                    self.browser.select_delta(1);
                }
            }
            Action::PageUp => {
                if self.focus == Focus::Browser {
                    self.browser.select_delta(-10);
                }
            }
            Action::PageDown => {
                if self.focus == Focus::Browser {
                    self.browser.select_delta(10);
                }
            }
            Action::Enter => {
                if self.focus == Focus::Browser {
                    if let Some(path) = self.browser.activate() {
                        self.load_path(path);
                        self.focus = Focus::Pattern;
                    }
                }
            }
        }
    }

    fn load_path(&mut self, path: PathBuf) {
        match crate::audio::load_module(&path) {
            Ok(loaded) => {
                self.current_path = Some(path);
                crate::audio::publish_loaded_metadata(&self.state, &loaded);
                self.audio.send(Command::Load(loaded.module));
            }
            Err(err) => {
                tracing::error!(?err, "failed to load module");
            }
        }
    }

    fn cycle_theme(&mut self) {
        if self.theme_choices.is_empty() {
            return;
        }
        let current = self
            .theme_choices
            .iter()
            .position(|choice| choice == &self.theme_choice)
            .unwrap_or(0);
        let next = (current + 1) % self.theme_choices.len();
        self.theme_choice = self.theme_choices[next].clone();
        self.theme = resolve_theme(&self.theme_choice);
    }

    fn draw(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        terminal.draw(|f| {
            let area = f.area();
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // header
                    Constraint::Min(8),    // main split
                    Constraint::Length(8), // spectrum
                    Constraint::Length(1), // status hint
                ])
                .split(area);

            widgets::header::render(f, rows[0], &self.state, &self.theme);

            let main = if self.focus == Focus::Browser {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(30),
                        Constraint::Percentage(50),
                        Constraint::Percentage(20),
                    ])
                    .split(rows[1])
            } else {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(0),
                        Constraint::Percentage(70),
                        Constraint::Percentage(30),
                    ])
                    .split(rows[1])
            };

            if self.focus == Focus::Browser {
                widgets::browser::render(f, main[0], &mut self.browser, &self.theme, true);
            }
            widgets::pattern::render(
                f,
                main[1],
                &self.state,
                &self.theme,
                self.focus == Focus::Pattern,
            );
            if self.show_info {
                widgets::info::render(f, main[2], &self.state, &self.theme);
            } else {
                widgets::meters::render(
                    f,
                    main[2],
                    &self.state,
                    &self.meter_state,
                    &self.theme,
                    false,
                );
            }

            // Spectrum on the left, master L/R meter on the right.
            let bottom = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(16), Constraint::Length(30)])
                .split(rows[2]);
            // Resize bands to roughly match the spectrum *sub-area* width now.
            let band_count = (bottom[0].width as usize).clamp(8, 96);
            self.spectrum.resize_bands(band_count);
            widgets::spectrum::render(f, bottom[0], &self.spectrum, &self.theme);
            widgets::master::render(f, bottom[1], &self.master_state, &self.theme);

            // Status hint
            let hint = "[space] play  [n] next  [/] browse  [?] help  [q] quit";
            use ratatui::style::Style;
            use ratatui::text::{Line, Span};
            use ratatui::widgets::Paragraph;
            let p = Paragraph::new(Line::from(Span::styled(
                hint,
                Style::default().fg(self.theme.fg_dim),
            )));
            f.render_widget(p, rows[3]);

            if self.show_help {
                widgets::help::render(f, area, &self.theme);
            }
        })?;
        Ok(())
    }
}

fn resolve_theme(choice: &ThemeChoice) -> Theme {
    match Theme::for_choice(choice) {
        Ok(theme) => theme,
        Err(err) => {
            tracing::warn!(
                ?err,
                theme = choice.name(),
                "failed to load theme, using default"
            );
            Theme::built_in(BuiltInTheme::Default)
        }
    }
}
