use anyhow::Result;
use rtrax::audio::command::Command;
use rtrax::audio::{self, FFT_RING_CAPACITY};
use rtrax::config::Config;
use rtrax::state::SharedState;
use rtrax::ui::{restore_terminal_for_panic, App};
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    install_logger();
    install_panic_hook();

    let initial_path: Option<PathBuf> = std::env::args().nth(1).map(PathBuf::from);

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

    let config = Config::load();
    let app = App::new(state, audio, fft_rx, config, initial_path)?;
    app.run()
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
