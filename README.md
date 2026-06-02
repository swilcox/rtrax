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

## CLI options

```
rtrax [OPTIONS] [FILES]...
```

| Option | Description |
|--------|-------------|
| `[FILES]...` | One or more module files. Two or more become an inline playlist; `n`/`p` navigate within it. |
| `-l`, `--playlist <FILE>` | Load an M3U playlist; `n`/`p` navigate within it and `a` saves to it. |
| `--theme <THEME>` | Override the theme from config (e.g. `neon-blue`, `c64`, `mono`). |
| `--no-config` | Skip the config file and use built-in defaults. |
| `-h`, `--help` | Print help. |
| `-V`, `--version` | Print version. |

## Playlists

> Full details: [docs/playlists.md](docs/playlists.md)

rtrax uses the standard M3U format — plain text, one file path per line,
lines starting with `#` are comments or metadata and are ignored.

**Loading a playlist:**

```sh
# Load from a .m3u file; n/p navigate within it.
rtrax --playlist my-favourites.m3u

# Pass multiple files directly — they become an inline playlist for the session.
rtrax *.xm
```

**Adding songs while playing:**

Press `a` to append the currently-playing file to the active playlist. If no
playlist is loaded, the song is saved to the default playlist at:

- **Linux:** `~/.local/share/rtrax/playlist.m3u`
- **macOS:** `~/Library/Application Support/rtrax/playlist.m3u`

The file is created automatically (with an `#EXTM3U` header) if it doesn't
exist yet. Pressing `a` multiple times is safe — each press appends one entry.

**Navigation priority:**

`n` and `p` check the active playlist first. If no playlist is loaded (e.g.
you opened a single file), they fall back to the files in the same folder.

## Keybindings

| Key       | Action                       |
|-----------|------------------------------|
| `space`   | Play / pause                 |
| `s`       | Stop                         |
| `n` / `p` | Next / previous (playlist, then folder) |
| `a`       | Add current song to playlist |
| `←` / `→` | Seek −5 s / +5 s             |
| `[` / `]` | Volume down / up             |
| `/`       | Focus file browser           |
| `Tab`     | Cycle focus                  |
| `t`       | Cycle theme                  |
| `b`       | Cycle progress bar style     |
| `i`       | Toggle info panel (samples + metadata) |
| `m`       | Show full song message (scrollable) |
| `w`       | Cycle pattern stack (1 / 2 / 4 lanes); overrides auto-layout |
| `c`       | Toggle compact cells (drops volume + effect) |
| `?`       | Help overlay                 |
| `q`       | Quit                         |

Override any binding in `$XDG_CONFIG_HOME/rtrax/config.toml`.

## Themes

Select a built-in or custom theme in `$XDG_CONFIG_HOME/rtrax/config.toml`:

```toml
theme = "default"
```

Built-ins are `default`, `high-contrast`, `sixteen`, `neon-blue`, `neon-green`,
`neon-orange`, `c64`, and `mono`. Custom themes live in
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

## Progress bar

The header shows a progress bar between the order/pattern stats and the time
display. Four styles are available:

| Style       | Looks like        | Notes |
|-------------|-------------------|-------|
| `triangle`  | `[━━━━▲────]`     | Single marker over an empty track |
| `blocks`    | `████▌    `       | Smooth fill via eighth-block chars (default) |
| `line`      | `━━━━╸────`       | Heavy elapsed, light remaining, notch at head |
| `segments`  | `▰▰▰▰▱▱▱▱`        | Discrete pip segments |

Pick one in config, or press `b` to cycle them at runtime:

```toml
progress_bar_style = "blocks"
```

## Pattern layout

By default the pattern view sizes itself to the module each time a new song
loads: a 4-channel MOD shows a single full-width lane, while denser modules fan
out into 2 or 4 stacked lanes and switch to compact cells so every channel stays
visible. Pressing `w` or `c` overrides the choice until the next song loads.

Turn the behavior off (and keep whatever `w`/`c` you set) in
`$XDG_CONFIG_HOME/rtrax/config.toml`:

```toml
auto_layout = true   # default; set to false to size lanes manually
```

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
