# rtrax

A TUI module player for `.mod` / `.xm` / `.it` / `.s3m` / `.mtm` (and anything
else libopenmpt reads). Per-channel level meters, scrolling pattern view,
master spectrum analyzer, file browser. macOS + Linux.

![rtrax screenshot](docs/screenshot.png)

> The image above is a placeholder mockup — swap in a real terminal capture
> by overwriting `docs/screenshot.png`.

## Install

System library — libopenmpt is a runtime dependency, not vendored.

```sh
# macOS
brew install libopenmpt

# Debian / Ubuntu
sudo apt install libopenmpt-dev

# Arch
sudo pacman -S libopenmpt
```

Then build with cargo:

```sh
cargo build --release
```

If the linker can't find libopenmpt, set `RTRAX_OPENMPT_LIB_DIR` to its
location, or make sure `pkg-config --libs libopenmpt` returns a `-L` path.

## Run

```sh
# Launch the TUI; loads a file immediately if given.
cargo run --release -- some_song.xm

# Headless: play a file to the end, no UI.
cargo run --release --example play -- some_song.xm

# Smoke test: just print metadata.
cargo run --release --example load_print -- some_song.xm
```

Release-mode is meaningful — debug-mode FFT + decode can underrun the audio
buffer on modest hardware.

## Keybindings

| Key       | Action                       |
|-----------|------------------------------|
| `space`   | Play / pause                 |
| `s`       | Stop                         |
| `n` / `p` | Next / previous in folder    |
| `←` / `→` | Seek −5 s / +5 s             |
| `[` / `]` | Volume down / up             |
| `/`       | Focus file browser           |
| `Tab`     | Cycle focus                  |
| `t`       | Cycle theme                  |
| `?`       | Help overlay                 |
| `q`       | Quit                         |

Override in `$XDG_CONFIG_HOME/rtrax/config.toml`.

## Architecture

See `PLAN.md`. In one paragraph: the audio thread owns the openmpt module and
the cpal stream. Inside the cpal callback it decodes interleaved f32 stereo,
copies a downsampled mono slice into an rtrb ring for the FFT, and updates
atomic snapshots of order/row/BPM/VU. The UI thread (~30 fps) reads those
atomics, drains the ring for FFT input, and renders a ratatui frame.
The cpal callback never allocates, locks (other than `try_lock`), or logs.

## Logs

Logs go to `$XDG_CACHE_HOME/rtrax/rtrax.log.YYYY-MM-DD` — file-only, never
stdout, because that would corrupt ratatui's alternate-screen rendering.

## Out of scope

- Editing / sample manipulation. Playback only.
- Network features, streaming protocols, web UI.
- Format conversion (libopenmpt is read-only here).
- Plugin systems, scripting, custom DSP effects.
- Windows support — not blocked, but not a v1 goal.
