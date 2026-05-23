# rtrax

[![CI](https://github.com/swilcox/rtrax/actions/workflows/ci.yml/badge.svg)](https://github.com/swilcox/rtrax/actions/workflows/ci.yml)
[![Coverage](https://codecov.io/gh/swilcox/rtrax/branch/main/graph/badge.svg)](https://codecov.io/gh/swilcox/rtrax)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Made with Rust](https://img.shields.io/badge/made_with-rust-CE412B?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Edition 2021](https://img.shields.io/badge/edition-2021-blue.svg)](https://doc.rust-lang.org/edition-guide/rust-2021/index.html)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux-lightgrey.svg)](#install)
[![Type: TUI](https://img.shields.io/badge/type-TUI-9cf.svg)](https://github.com/ratatui-org/ratatui)
[![Audio: libopenmpt](https://img.shields.io/badge/audio-libopenmpt-purple.svg)](https://lib.openmpt.org/libopenmpt/)
[![Latest release](https://img.shields.io/github/v/release/swilcox/rtrax?sort=semver&display_name=tag)](https://github.com/swilcox/rtrax/releases/latest)
[![Last commit](https://img.shields.io/github/last-commit/swilcox/rtrax)](https://github.com/swilcox/rtrax/commits/main)
[![Code size](https://img.shields.io/github/languages/code-size/swilcox/rtrax)](https://github.com/swilcox/rtrax)
[![Top language](https://img.shields.io/github/languages/top/swilcox/rtrax)](https://github.com/swilcox/rtrax)

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
| `i`       | Toggle info panel (samples + metadata) |
| `w`       | Cycle pattern stack (1 / 2 / 4 lanes)  |
| `c`       | Toggle compact cells (drops volume + effect) |
| `?`       | Help overlay                 |
| `q`       | Quit                         |

Override in `$XDG_CONFIG_HOME/rtrax/config.toml`.

## Themes

Select a built-in or custom theme in `$XDG_CONFIG_HOME/rtrax/config.toml`:

```toml
theme = "default"
```

Built-ins are `default`, `high-contrast`, and `sixteen`. Custom themes live in
`$XDG_CONFIG_HOME/rtrax/themes/<name>.toml` and are selected by file stem:

```toml
theme = "amber"
```

```toml
# $XDG_CONFIG_HOME/rtrax/themes/amber.toml
extends = "default"

accent = "#ffb454"
note = "#ffe6a3"
instrument = "light-cyan"
volume = "yellow"
effect = "#ff7a90"
current_row_bg = "#302414"
```

Theme files may override any subset of these color keys: `bg`, `fg`, `fg_dim`,
`border`, `border_focus`, `accent`, `note`, `instrument`, `volume`, `effect`,
`meter_low`, `meter_mid`, `meter_high`, and `current_row_bg`. Values can be
`#rrggbb`, `reset`, or ratatui ANSI color names such as `cyan`, `dark-gray`,
and `light-magenta`. Pressing `t` cycles built-ins plus any `.toml` files found
in the themes directory. See [docs/themes.md](docs/themes.md) for the full
theme reference.

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
