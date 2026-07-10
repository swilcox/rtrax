# rtrax

A TUI-based MOD/XM/IT/S3M/MTM module player in Rust. Modern-minimal tracker aesthetic
with per-channel level meters, scrolling pattern view, master spectrum analyzer,
and a file browser / playlist. Targets macOS and Linux.

See `PLAN.md` for the full architecture, phasing, and known gotchas.

## Stack

- **Decoding**: `openmpt` crate (bindings to libopenmpt). System lib required:
  `brew install libopenmpt` on macOS, `apt install libopenmpt-dev` on Debian/Ubuntu.
- **Audio output**: `cpal` — direct callback-driven stream, cross-platform.
- **TUI**: `ratatui` + `crossterm` backend.
- **FFT**: `rustfft` for the master spectrum analyzer.
- **Audio↔UI handoff**: `rtrb` (lock-free SPSC ring buffer).
- **Input/events**: `crossterm` event loop, dispatched via `crossbeam-channel`.

## Architecture in one paragraph

The audio thread owns the `openmpt::Module` and the cpal stream. Inside the cpal
callback it decodes interleaved f32 stereo, copies a downsampled slice into an
`rtrb` producer for the UI's FFT, and updates `AtomicU32` snapshots of order/row/BPM.
The UI thread (~30 fps) reads those atomics, drains the rtrb consumer for FFT input,
calls libopenmpt VU getters for per-channel meters, and renders a ratatui frame.
**Rule:** the cpal callback never allocates, never locks, never logs.

## Conventions

- Edition 2021, `cargo fmt` clean, `cargo clippy -- -D warnings` clean.
- Cargo workspace with two crates:
  - `crates/rtrax-core` — frontend-agnostic engine: `audio/` (cpal + decoder),
    `state/` (shared atomics + ring buffer types), `fft.rs` (spectrum
    analysis), `playlist.rs`, `launch.rs` (startup resolution), `rng.rs`.
    No UI dependencies; any frontend consumes it by polling `SharedState`,
    draining the FFT ring, and sending `audio::command::Command`.
  - `crates/rtrax` — the TUI binary: `ui/` (ratatui widgets + render loop),
    `input/` (key dispatch), `config.rs` (TUI config: keymap, themes),
    `main.rs` (wiring).
  New frontends (e.g. a GUI) get their own crate beside `crates/rtrax` and
  depend only on `rtrax-core`. Core code must never import a UI toolkit.
- No `unsafe` outside of FFI wrappers (the `openmpt` crate already wraps these).
- Errors: `anyhow::Result` at app boundaries, `thiserror` for library-shaped
  modules if/when we extract them. Don't sprinkle `.unwrap()` outside `main`.

## Build & run

```
cargo run --release           # release-mode is meaningful — FFT + decode are hot
cargo run --release -- <file.xm>
cargo fmt && cargo clippy
```

The release build matters: debug-mode FFT + decode can underrun the audio buffer
on smaller machines. Always test playback with `--release`.

## Out of scope (do not add)

- Editing / sample manipulation. Playback only.
- Network features, streaming protocols, web UI.
- Format converters (libopenmpt already reads everything; we don't write).
- Plugin systems, scripting, custom DSP effects.

## When extending the UI

The aesthetic target is *modern minimal* — tracker-inspired, not a faithful
Fasttracker II replica. Low-saturation palette (greens/cyans with magenta
accents). Degrade gracefully on 16-color terminals; detect via `crossterm`'s
capability query and pick a palette accordingly.
