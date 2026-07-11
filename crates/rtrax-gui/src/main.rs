//! rtrax-gui — native GUI frontend (egui/eframe) for the rtrax engine.
//!
//! Thin entry point: parse the CLI, resolve the starting queue, start the
//! audio engine, and hand everything to [`app::GuiApp`].

mod app;
mod media;
mod theme;
mod widgets;

use crate::app::GuiApp;
use anyhow::Result;
use clap::Parser;
use eframe::egui;
use rtrax_core::audio::{self, FFT_RING_CAPACITY};
use rtrax_core::playlist::Playlist;
use rtrax_core::state::SharedState;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "rtrax-gui",
    version,
    about = "GUI MOD/XM/IT/S3M/MTM module player"
)]
struct Cli {
    /// Module file(s) or directories. A single file queues its whole folder
    /// (auto-advance walks it); directories queue the modules inside them.
    /// Files can also be dropped onto the window.
    files: Vec<PathBuf>,

    /// Playlist file (.m3u) to play as the queue. Ignored if files are given.
    #[arg(long, short = 'l', value_name = "FILE")]
    playlist: Option<PathBuf>,

    /// Shuffle play order on startup. Toggle at runtime with `z`.
    #[arg(long, short = 'z')]
    shuffle: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let (queue, initial) = resolve_queue(&cli)?;

    let state = Arc::new(SharedState::new());
    let (fft_tx, fft_rx) = rtrb::RingBuffer::<f32>::new(FFT_RING_CAPACITY);
    let audio = audio::start(state.clone(), fft_tx)?;

    let mut gui = GuiApp::new(state, audio, fft_rx, queue, cli.shuffle);
    if let Some(path) = initial.as_deref() {
        gui.load_path(path);
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("rtrax")
            .with_inner_size([1080.0, 640.0])
            .with_min_inner_size([640.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "rtrax",
        options,
        Box::new(move |cc| {
            app::apply_theme(&cc.egui_ctx);
            gui.init_media(cc.egui_ctx.clone());
            Ok(Box::new(gui))
        }),
    )
    .map_err(|err| anyhow::anyhow!("eframe: {err}"))
}

/// Decide the starting queue and track from the CLI. Files (or directories)
/// win over `--playlist`; shuffle is applied anchored on the explicit initial
/// track, so that track still plays first.
fn resolve_queue(cli: &Cli) -> Result<(Option<Playlist>, Option<PathBuf>)> {
    if !cli.files.is_empty() {
        if cli.playlist.is_some() {
            tracing::warn!("--playlist is ignored when files are given");
        }
        let (mut queue, mut initial) = app::build_queue(&cli.files);
        queue.set_shuffle(cli.shuffle, initial.as_deref());
        if initial.is_none() {
            initial = queue.start().cloned();
        }
        return Ok((Some(queue), initial));
    }
    if let Some(path) = &cli.playlist {
        let mut queue = Playlist::load(path.clone())?;
        queue.set_shuffle(cli.shuffle, None);
        let initial = queue.start().cloned();
        return Ok((Some(queue), initial));
    }
    Ok((None, None))
}
