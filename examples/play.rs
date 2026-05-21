//! Phase 1 headless playback: open a file, play it to the end, exit.
//!
//! Usage: `cargo run --release --example play -- path/to/song.xm`

use anyhow::{Context, Result};
use rtrax::audio::command::Command;
use rtrax::audio::{self, FFT_RING_CAPACITY};
use rtrax::state::SharedState;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .context("usage: play <module-file>")?;

    let state = Arc::new(SharedState::new());
    let (fft_tx, mut fft_rx) = rtrb::RingBuffer::<f32>::new(FFT_RING_CAPACITY);
    let handle = audio::start(state.clone(), fft_tx)?;

    let loaded = audio::load_module(std::path::Path::new(&path))?;
    let title = loaded.title.clone();
    audio::publish_loaded_metadata(&state, &loaded);
    handle.send(Command::Load(loaded.module));

    println!("playing: {title}");

    // Block until the audio thread signals EOF, draining the FFT ring meanwhile
    // so the producer side keeps making forward progress.
    loop {
        std::thread::sleep(Duration::from_millis(200));
        while fft_rx.pop().is_ok() {}
        handle.drain_drops();
        if state.eof.load(Ordering::Relaxed) {
            break;
        }
        let pos = state.position_secs();
        let dur = state.duration_secs();
        print!("\r{:>5.1}s / {:>5.1}s  ", pos, dur);
        use std::io::Write;
        std::io::stdout().flush().ok();
    }

    println!("\ndone");
    Ok(())
}
