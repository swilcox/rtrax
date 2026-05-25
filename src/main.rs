use anyhow::Result;
use clap::Parser;
use rtrax::audio::command::Command;
use rtrax::audio::{self, FFT_RING_CAPACITY};
use rtrax::config::{Config, ThemeChoice};
use rtrax::playlist::Playlist;
use rtrax::state::SharedState;
use rtrax::ui::{restore_terminal_for_panic, App};
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "rtrax", version, about = "TUI MOD/XM/IT/S3M/MTM module player")]
struct Cli {
    /// Module file(s) to play. Multiple files become an inline playlist.
    files: Vec<PathBuf>,

    /// Playlist file (.m3u) to load; n/p navigate within it and `a` saves here.
    #[arg(long, short = 'l', value_name = "FILE")]
    playlist: Option<PathBuf>,

    /// Override the theme set in config (e.g. neon-blue, c64, mono).
    #[arg(long, value_name = "THEME")]
    theme: Option<ThemeChoice>,

    /// Skip the config file and use built-in defaults.
    #[arg(long)]
    no_config: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    install_logger();
    install_panic_hook();

    let mut config = if cli.no_config {
        Config::default()
    } else {
        Config::load()
    };
    if let Some(theme) = cli.theme {
        config.theme = theme;
    }

    let (initial_path, playlist) = resolve_sources(cli.files, cli.playlist)?;

    let state = Arc::new(SharedState::new());
    let (fft_tx, fft_rx) = rtrb::RingBuffer::<f32>::new(FFT_RING_CAPACITY);
    let audio = audio::start(state.clone(), fft_tx)?;

    if let Some(path) = initial_path.as_deref() {
        match audio::load_module(path) {
            Ok(loaded) => {
                audio::publish_loaded_metadata(&state, &loaded);
                audio.send(Command::Load(loaded.module));
            }
            Err(err) => tracing::warn!(?err, "failed to load initial module"),
        }
    }

    let app = App::new(state, audio, fft_rx, config, initial_path, playlist)?;
    app.run()
}

/// Decide what to play first and what playlist (if any) governs n/p navigation.
///
/// - `--playlist <file>`: load from disk; play the first entry.
/// - Two or more positional files: build an in-memory playlist; play the first.
/// - One positional file: play it directly; no playlist (n/p uses the browser).
/// - No arguments: no initial file, no playlist (open with the browser).
fn resolve_sources(
    files: Vec<PathBuf>,
    playlist_path: Option<PathBuf>,
) -> Result<(Option<PathBuf>, Option<Playlist>)> {
    if let Some(pl_path) = playlist_path {
        let playlist = Playlist::load(pl_path)?;
        let initial = playlist.first().cloned();
        return Ok((initial, Some(playlist)));
    }
    match files.len() {
        0 => Ok((None, None)),
        1 => Ok((Some(files.into_iter().next().unwrap()), None)),
        _ => {
            let playlist = Playlist::from_files(files);
            let initial = playlist.first().cloned();
            Ok((initial, Some(playlist)))
        }
    }
}

/// File-only logger. We MUST NOT write to stdout/stderr while ratatui owns the
/// terminal, since that corrupts the alternate-screen rendering.
fn install_logger() {
    let log_dir = dirs::cache_dir()
        .map(|p| p.join("rtrax"))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    if std::fs::create_dir_all(&log_dir).is_ok() {
        let file_appender = tracing_appender::rolling::daily(&log_dir, "rtrax.log");
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
            )
            .with_writer(file_appender)
            .with_ansi(false)
            .try_init();
    }
}

/// Restore the terminal *before* the default panic handler prints, so the
/// panic message lands on a clean shell.
fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal_for_panic();
        prev(info);
    }));
}
