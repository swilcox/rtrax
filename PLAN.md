# rtrax — Implementation Plan

A TUI module player in Rust. Plays MOD/XM/IT/S3M/MTM (anything libopenmpt reads),
with per-channel meters, a master spectrum analyzer, a scrolling pattern view,
and a file browser / playlist. Modern-minimal aesthetic. macOS + Linux.

---

## 1. Stack & rationale

| Concern              | Crate              | Why this one |
|----------------------|--------------------|--------------|
| Module decoding      | `openmpt`          | Existing safe-ish bindings to libopenmpt; spares us hand-writing FFI. |
| Audio output         | `cpal`             | Gives us the raw callback. `rodio` is too source-oriented for a streaming decoder. |
| TUI                  | `ratatui`          | The mature successor to tui-rs; active, good widget set, double-buffered. |
| Terminal backend     | `crossterm`        | Cross-platform (macOS + Linux), event API, color capability detection. |
| FFT                  | `rustfft`          | Standard. Plan once, reuse the plan. |
| Lock-free queue      | `rtrb`             | SPSC, zero-allocation, perfect for audio→UI sample handoff. |
| Channels (control)   | `crossbeam-channel`| For input events and command dispatch off the hot path. |
| Directory walking    | `walkdir`          | Tiny, fits the file browser without a full picker dep. |
| Error type           | `anyhow`           | App-level errors; switch to `thiserror` if/when we extract reusable modules. |
| Logging              | `tracing`          | File-only sink — *never* write to stdout/stderr while ratatui owns the terminal. |

### System library

libopenmpt is a runtime dependency. The crate links against it via `pkg-config`.

- macOS: `brew install libopenmpt`
- Debian/Ubuntu: `apt install libopenmpt-dev`
- Arch: `pacman -S libopenmpt`

We do not vendor it. The README will document the install step.

---

## 2. Architecture

### Threads

```
┌──────────────────┐    rtrb (f32 samples)    ┌────────────────────┐
│  Audio thread    │ ───────────────────────► │  UI thread (30fps) │
│  (cpal callback) │                          │                    │
│  owns Module     │ ◄──── crossbeam ──────── │  reads atomics +   │
│                  │   (Load/Play/Pause/      │  ring, runs FFT,   │
│                  │    Seek/Stop)            │  ratatui render    │
└──────────────────┘                          └────────────────────┘
       ▲                                              ▲
       │ AtomicU32 (order, row, bpm, speed)           │
       │ AtomicBool (playing)                         │
       │ Per-channel VU read via libopenmpt API ──────┘
                                                      ▲
                                              ┌────────────────────┐
                                              │  Input thread      │
                                              │  crossterm events  │
                                              │  → crossbeam tx    │
                                              └────────────────────┘
```

### Why the audio thread owns the Module

`openmpt::Module` is `!Sync`. Two options:

1. Wrap it in a `Mutex`, share between threads. The UI would need to lock it
   to read pattern data — and the audio callback would block on UI contention.
   Unacceptable for real-time audio.
2. **The audio thread exclusively owns the Module.** It writes derived state
   (current order/row/BPM, per-channel VU snapshots) to atomics. The UI reads
   only atomics + the sample ring. The UI never touches libopenmpt directly.

We go with (2). For pattern view content (notes/effects ahead of and behind
the current row), the audio thread snapshots a small window of rows into a
`Mutex<PatternWindow>` once per buffer (cheap, low contention because the UI
only reads it ~30 times/sec).

### The cpal callback — discipline

The callback is real-time. Inside it:

- ✅ Decode into the output slice via `read_interleaved_float_stereo`.
- ✅ Push a downsampled copy into the `rtrb` producer.
- ✅ Read VU values from libopenmpt, store in `AtomicU32` (bit-cast f32).
- ✅ Update `AtomicU32` for order/row/BPM/speed.
- ❌ No `println!`, no `tracing::info!`, no `Vec::push`, no `Mutex::lock` on
  anything the UI also locks aggressively. The pattern-window mutex is
  `try_lock` only — if contended, skip the snapshot this buffer.

Commands (load/pause/seek) arrive via a `crossbeam_channel::Receiver` polled
non-blockingly at the top of the callback. Loading a new module happens in the
callback by swapping `Option<Module>` — the old module is sent back through a
"drop channel" and freed on the UI thread, since `Module::drop` may allocate.

---

## 3. Phasing

Each phase ends with something demoable. Don't move on until the previous one
plays/renders/feels right.

### Phase 0 — Skeleton & FFI smoke test
- `cargo new`, add `openmpt`, `cpal`, `ratatui`, `crossterm`, `rustfft`, `rtrb`.
- Write `examples/load_print.rs`: load a module path from argv, print
  `Module::get_title`, channel count, duration. Verify links + runs on both OSes.
- **Done when**: `cargo run --example load_print -- some.xm` prints metadata.

### Phase 1 — Audio MVP (headless)
- `audio::Player` owns the Module and a cpal stream.
- `examples/play.rs`: load a file, play to end, exit. No UI, no controls.
- **Done when**: a known-good XM plays cleanly through both macOS CoreAudio
  and a Linux machine (ALSA or PulseAudio/PipeWire via cpal). No glitches at
  default buffer size.

### Phase 2 — TUI shell
- `ui::App` with the ratatui event loop, alternate screen, raw mode, cursor hide.
- Static layout: header (song info placeholder), main split (pattern | meters),
  spectrum footer placeholder, status line.
- Quit on `q` or Ctrl-C. No audio yet.
- **Done when**: shell renders cleanly, resizes cleanly, exits cleanly (terminal
  restored on panic — use a guard struct with `Drop`).

### Phase 3 — Transport + file load
- Wire audio thread + UI thread together. Atomics + the command channel.
- Keys: `space` play/pause, `s` stop, command-line arg loads initial file.
- Header reflects current title / channel count / position (mm:ss / mm:ss).
- **Done when**: launching with a file plays it, `space` toggles, header updates
  live, no terminal corruption on Ctrl-C.

### Phase 4 — File browser & playlist
- Left-or-modal directory browser via `walkdir` (filter: `.mod .xm .it .s3m .mtm .mptm`).
- Enter loads the selected file. `n`/`p` next/prev in the current directory listing.
- Optional: persist a recent-files list to `$XDG_STATE_HOME/rtrax/recent`.
- **Done when**: can browse a folder, pick files, advance through them.

### Phase 5 — Per-channel level meters
- Audio thread polls `module_get_channel_vu_{left,right}` for each channel after
  every read, writes f32 (bit-cast u32) into a `[AtomicU32; MAX_CHANNELS*2]`.
- UI renders a column of meters using `▁▂▃▄▅▆▇█` blocks. Apply attack/decay
  envelope in the UI (e.g. attack instant, decay linear ~30 dB/sec, peak hold
  with 1.5 s fall).
- **Done when**: meters feel responsive, no jitter, scale fits the channel count
  (group into rows if a module has many channels).

### Phase 6 — Pattern view
- Audio thread maintains a `PatternWindow` (the N rows surrounding current row)
  via `Module::get_pattern_row_channel_command`. Refreshed when row changes.
- UI renders the window with the current row centered and highlighted.
- Syntax color: notes one hue, instruments another, volume/effect a third.
  Empty cells dim. Truncate gracefully if the channel count exceeds visible width
  (horizontal scroll or column folding).
- **Done when**: pattern view stays in sync with audio, no visible tearing on
  fast rows.

### Phase 7 — Master spectrum analyzer
- Audio thread pushes mixed-down mono samples into `rtrb` (`L+R)/2`, after
  decimation if needed to keep FFT size manageable.
- UI thread drains, maintains a rolling window (e.g. 2048 samples), applies a
  Hann window, runs `rustfft`, computes magnitudes, log-bins into ~32 bands
  (20 Hz – 20 kHz log-spaced), smooths with a fast-attack/slow-decay envelope.
- Render as a horizontal bar row using block characters.
- **Done when**: spectrum reacts musically — bass thumps the low bars, hi-hats
  light up the top bars — without visible per-frame flicker.

### Phase 8 — Polish
- Theme: low-saturation green/cyan palette with magenta accents. Detect
  16-color terminals via crossterm and pick a fallback palette.
- Config file at `$XDG_CONFIG_HOME/rtrax/config.toml`: keybinds, color
  overrides, default browse path.
- Graceful terminal restore on panic (`std::panic::set_hook` + the Drop guard).
- README with build instructions, screenshots, supported formats.

---

## 4. Visual target

```
┌─ rtrax ─────────────────────────────────────── 2:31 / 4:08 ─┐
│ space_debris.xm  ·  64 ch  ·  140 BPM  ·  pattern 12/24    │
├──────────────────────────────────────┬──────────────────────┤
│ 23 │ C-5 01 .. A20 │ ... │ ... │ ... │ 01 ▁▃▅▇  05 ▁▂▄▆     │
│ 24 │ --- .. .. ... │ G-4 │ ... │ ... │ 02 ▁▂▃▅  06 ▁▁▂▃     │
│ 25 │ ▶ E-5 .. .. … │ ... │ ... │ A-3 │ 03 ▁▃▆▇  07 ▁▂▄▅     │
│ 26 │ --- .. .. ... │ ... │ ... │ ... │ 04 ▁▁▂▃  08 ▁▁▁▂     │
├──────────────────────────────────────┴──────────────────────┤
│ ▇▆▅▄▃▃▂▂▁▁▁▁▂▂▃▃▄▄▅▅▄▄▃▃▂▂▁▁▁▁▁▁                            │
└─ [space] play  [n] next  [/] browse  [q] quit ──────────────┘
```

Pattern view left, meter column right, spectrum across the footer. The current
row is centered and highlighted with `▶`. Empty pattern cells render dim.

---

## 5. Default keybindings (configurable)

| Key       | Action                       |
|-----------|------------------------------|
| `space`   | Play / pause                 |
| `s`       | Stop                         |
| `n` / `p` | Next / previous in playlist  |
| `←` / `→` | Seek -5s / +5s               |
| `[` / `]` | Volume down / up             |
| `/`       | Focus file browser           |
| `Tab`     | Cycle focus between panes    |
| `t`       | Cycle theme                  |
| `?`       | Help overlay                 |
| `q`       | Quit                         |

---

## 6. Known gotchas (drawn from prior tracker-player work)

- **libopenmpt VU lag.** The VU values update per-mixed-buffer, not per-sample.
  If meters look sluggish, *decrease the cpal buffer size* before tuning the
  decay envelope. Buffer size, not envelope, is usually the cause.
- **cpal device switch on macOS.** Unplugging headphones doesn't auto-migrate
  the stream — cpal returns a stream error. Trap it, rebuild the stream against
  the new default device, resume from the same playback position.
- **Terminal resize.** Ratatui handles redraw, but we must recompute spectrum
  bin counts and meter column layout on resize. Listen for `Event::Resize`.
- **Panic = corrupt terminal.** Install a panic hook that calls
  `crossterm::terminal::disable_raw_mode` + `LeaveAlternateScreen` *before* the
  default hook prints the panic message. Otherwise the user's shell is wrecked.
- **`!Sync` Module.** See §2 — audio thread owns it exclusively.
- **FFT cost at 60 fps.** A 2048-point f32 FFT is cheap, but if the UI thread
  starves (e.g. on a tiny VM), skip recomputing this frame and reuse the last
  magnitudes. Don't block the render loop on the FFT.
- **libopenmpt and very high channel counts.** Some MPTM/IT modules have 64+
  channels. Plan the meter column layout to wrap into multiple sub-columns
  rather than overflowing horizontally.
- **Release vs debug.** Always test playback with `--release`. Debug-mode FFT +
  decode can underrun the audio buffer on modest hardware.

---

## 7. Out of scope for v1

- Editing, sample browsing, instrument editing — playback only.
- Network streaming, web UI, remote control.
- Format conversion (libopenmpt is read-only for us).
- Plugins, scripting, custom DSP effects.
- Windows support. Not blocked — cpal + crossterm both work on Windows — but
  not a v1 goal. Revisit after macOS + Linux are solid.

---

## 8. First task after directory rename

1. `cargo init` in the renamed directory.
2. Add the Phase 0 dependencies to `Cargo.toml`.
3. Write `examples/load_print.rs` and verify libopenmpt links on this machine.
4. Commit. That's Phase 0 done — Phase 1 begins next session.
