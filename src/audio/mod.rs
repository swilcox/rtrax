//! Audio thread. Owns the `openmpt::Module` and the cpal stream.
//!
//! The audio thread:
//! - Decodes interleaved f32 stereo into the cpal output slice.
//! - Pushes a downsampled mono copy into the rtrb producer for the FFT.
//! - Reads per-channel VU from libopenmpt and stores into shared atomics.
//! - Updates order/row/BPM/speed atomics.
//! - Updates sticky per-channel instrument state for the info panel.
//!
//! It receives commands (Load/Play/Pause/Stop/Seek/Volume) over a
//! `crossbeam_channel::Receiver` polled non-blockingly at the top of each
//! buffer.

pub mod command;

use anyhow::{anyhow, Context, Result};
use command::{Command, LoadedModule, SendModule};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{Receiver, Sender};
use openmpt::module::iteration::ModuleCommandIndex;
use openmpt::module::metadata::MetadataKey;
use openmpt::module::{Logger, Module};
use rtrb::Producer;
use std::fs::File;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::state::pattern::{PatternCache, PatternData, PatternRow};
use crate::state::{SharedState, MAX_CHANNELS};

/// Number of mono samples pushed into the FFT ring per second (after
/// downsampling from the audio output rate).
pub const FFT_RING_RATE_HZ: u32 = 12_000;
/// Total capacity of the FFT ring (mono f32 samples).
pub const FFT_RING_CAPACITY: usize = 8192;

/// Handle to the running audio stream.
pub struct AudioHandle {
    _stream: cpal::Stream,
    pub cmd_tx: Sender<Command>,
    pub drop_rx: Receiver<SendModule>,
    pub state: Arc<SharedState>,
}

impl AudioHandle {
    pub fn send(&self, cmd: Command) {
        let _ = self.cmd_tx.send(cmd);
    }

    /// Drain any old modules the audio thread handed back, so we drop them
    /// here. `Module::drop` may allocate; we don't want that inside the cpal
    /// callback.
    pub fn drain_drops(&self) {
        while self.drop_rx.try_recv().is_ok() {}
    }
}

/// Build the cpal output stream and start it. The stream is held alive by the
/// returned `AudioHandle`.
pub fn start(state: Arc<SharedState>, fft_tx: Producer<f32>) -> Result<AudioHandle> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .context("no default audio output device")?;

    let config = device
        .default_output_config()
        .context("querying default output config")?;
    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as usize;

    if channels < 2 {
        return Err(anyhow!(
            "default output has {channels} channels; rtrax needs stereo"
        ));
    }

    state.sample_rate.store(sample_rate, Ordering::Relaxed);

    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<Command>();
    let (drop_tx, drop_rx) = crossbeam_channel::unbounded::<SendModule>();

    let mut callback = CallbackState::new(
        state.clone(),
        cmd_rx,
        drop_tx,
        fft_tx,
        sample_rate,
        channels,
    );

    let err_fn = |err| tracing::error!(?err, "cpal stream error");

    let stream_config = cpal::StreamConfig {
        channels: config.channels(),
        sample_rate: config.sample_rate(),
        buffer_size: cpal::BufferSize::Default,
    };

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_output_stream(
            &stream_config,
            move |data: &mut [f32], _info| callback.fill(data),
            err_fn,
            None,
        ),
        other => {
            return Err(anyhow!(
                "default output uses sample format {other:?}; only F32 supported"
            ));
        }
    }
    .context("building cpal output stream")?;

    stream.play().context("starting cpal stream")?;

    Ok(AudioHandle {
        _stream: stream,
        cmd_tx,
        drop_rx,
        state,
    })
}

/// All state the cpal callback owns and mutates.
struct CallbackState {
    state: Arc<SharedState>,
    cmd_rx: Receiver<Command>,
    drop_tx: Sender<SendModule>,
    fft_tx: Producer<f32>,
    sample_rate: u32,
    output_channels: usize,
    module: Option<SendModule>,
    /// Reusable buffer; capacity is sized up to the largest cpal slice seen.
    decode_buf: Vec<f32>,
    /// Phase accumulator for the audio->FFT downsampler.
    downsample_accum: u32,
    last_snapshot_row: i32,
    last_snapshot_pattern: i32,
}

impl CallbackState {
    fn new(
        state: Arc<SharedState>,
        cmd_rx: Receiver<Command>,
        drop_tx: Sender<SendModule>,
        fft_tx: Producer<f32>,
        sample_rate: u32,
        output_channels: usize,
    ) -> Self {
        Self {
            state,
            cmd_rx,
            drop_tx,
            fft_tx,
            sample_rate,
            output_channels,
            module: None,
            decode_buf: Vec::with_capacity(8192),
            downsample_accum: 0,
            last_snapshot_row: -1,
            last_snapshot_pattern: -1,
        }
    }

    /// Called by cpal once per output buffer. Real-time critical.
    fn fill(&mut self, output: &mut [f32]) {
        while let Ok(cmd) = self.cmd_rx.try_recv() {
            self.apply_command(cmd);
        }

        for s in output.iter_mut() {
            *s = 0.0;
        }

        // Move the module out of `self.module` for the duration of this buffer
        // so the borrow checker lets us also reborrow `self` for the helpers.
        // This is a memmove of one pointer — cheap.
        let Some(mut send_module) = self.module.take() else {
            return;
        };
        if !self.state.playing.load(Ordering::Relaxed) {
            self.module = Some(send_module);
            return;
        }

        let frames = output.len() / self.output_channels;
        let needed = frames * 2;
        // libopenmpt's safe wrapper uses `Vec::capacity() >> 1` as the render
        // count — NOT `len()`. If our buffer is bigger than needed we'd
        // over-render and the song would race ahead of playback. Keep
        // capacity exactly equal to `needed`; in steady state (cpal buffer
        // size is stable) this allocates once at startup.
        if self.decode_buf.capacity() != needed {
            self.decode_buf = Vec::with_capacity(needed);
        }
        let rendered = send_module
            .module_mut()
            .read_interleaved_float_stereo(self.sample_rate as i32, &mut self.decode_buf);

        if rendered == 0 {
            self.state.eof.store(true, Ordering::Relaxed);
            self.state.playing.store(false, Ordering::Relaxed);
            self.module = Some(send_module);
            return;
        }

        // SAFETY: libopenmpt wrote `rendered * 2` floats into the buffer; we
        // sized capacity to at least `frames * 2` >= `rendered * 2`.
        let stereo: &[f32] =
            unsafe { std::slice::from_raw_parts(self.decode_buf.as_ptr(), rendered * 2) };

        let copy_frames = rendered.min(frames);
        for i in 0..copy_frames {
            output[i * self.output_channels] = stereo[i * 2];
            output[i * self.output_channels + 1] = stereo[i * 2 + 1];
        }

        self.publish_master_peak(stereo, copy_frames);
        self.push_to_fft(stereo, copy_frames);
        self.publish_state(send_module.module_mut());
        self.publish_last_instruments(send_module.module_mut());

        self.module = Some(send_module);
    }

    fn apply_command(&mut self, cmd: Command) {
        match cmd {
            Command::Load(loaded) => {
                let old = self.module.replace(loaded);
                self.state.playing.store(true, Ordering::Relaxed);
                self.state.stopped.store(false, Ordering::Relaxed);
                self.state.eof.store(false, Ordering::Relaxed);
                if let Some(old_mod) = old {
                    let _ = self.drop_tx.send(old_mod);
                }
                self.state.clear_last_instruments();
                self.last_snapshot_row = -1;
                self.last_snapshot_pattern = -1;
            }
            Command::Play => {
                if self.module.is_some() {
                    self.state.playing.store(true, Ordering::Relaxed);
                    self.state.stopped.store(false, Ordering::Relaxed);
                }
            }
            Command::Pause => {
                self.state.playing.store(false, Ordering::Relaxed);
            }
            Command::Stop => {
                self.state.playing.store(false, Ordering::Relaxed);
                self.state.stopped.store(true, Ordering::Relaxed);
                if let Some(send_module) = self.module.as_mut() {
                    send_module.module_mut().set_position_seconds(0.0);
                }
                for slot in self.state.vu_bits.iter() {
                    slot.store(0u32, Ordering::Relaxed);
                }
                self.state.set_master_peak(0.0, 0.0);
                self.state.clear_last_instruments();
            }
            Command::SeekRelative(delta_secs) => {
                if let Some(send_module) = self.module.as_mut() {
                    let module = send_module.module_mut();
                    let now = module.get_position_seconds();
                    let target = (now + delta_secs as f64).max(0.0);
                    module.set_position_seconds(target);
                }
            }
            Command::VolumeMillibel(mb) => {
                if let Some(send_module) = self.module.as_mut() {
                    send_module.module_mut().set_render_mastergain_millibel(mb);
                }
                self.state.master_gain_millibel.store(mb, Ordering::Relaxed);
            }
        }
    }

    fn publish_master_peak(&self, stereo: &[f32], frames: usize) {
        // Single linear pass over the buffer for the post-mix peak envelope per
        // side. No alloc, no branches per sample beyond a max.
        let mut peak_l = 0.0f32;
        let mut peak_r = 0.0f32;
        for i in 0..frames {
            let l = stereo[i * 2].abs();
            let r = stereo[i * 2 + 1].abs();
            if l > peak_l {
                peak_l = l;
            }
            if r > peak_r {
                peak_r = r;
            }
        }
        self.state.set_master_peak(peak_l.min(1.0), peak_r.min(1.0));
    }

    fn push_to_fft(&mut self, stereo: &[f32], frames: usize) {
        for i in 0..frames {
            if advance_downsample(&mut self.downsample_accum, self.sample_rate) {
                let l = stereo[i * 2];
                let r = stereo[i * 2 + 1];
                let mono = (l + r) * 0.5;
                if self.fft_tx.push(mono).is_err() {
                    return;
                }
            }
        }
    }

    fn publish_state(&mut self, module: &mut Module) {
        let order = module.get_current_order();
        let pattern = module.get_current_pattern();
        let row = module.get_current_row();
        let speed = module.get_current_speed();
        let tempo = module.get_current_tempo();
        let pos = module.get_position_seconds();
        let dur = module.get_duration_seconds();

        let prev_row = self.state.current_row.load(Ordering::Relaxed);
        let prev_pat = self.state.current_pattern.load(Ordering::Relaxed);
        if row != prev_row || pattern != prev_pat {
            self.state.row_generation.fetch_add(1, Ordering::Relaxed);
        }

        self.state.current_order.store(order, Ordering::Relaxed);
        self.state.current_pattern.store(pattern, Ordering::Relaxed);
        self.state.current_row.store(row, Ordering::Relaxed);
        self.state.current_speed.store(speed, Ordering::Relaxed);
        self.state.current_tempo.store(tempo, Ordering::Relaxed);
        self.state.set_position_secs(pos);
        self.state.set_duration_secs(dur);

        let nch = module.get_num_channels().max(0) as usize;
        self.state.num_channels.store(nch as i32, Ordering::Relaxed);
        for ch in 0..nch.min(MAX_CHANNELS) {
            let l = module.get_current_channel_vu_left(ch as i32);
            let r = module.get_current_channel_vu_right(ch as i32);
            self.state.set_vu(ch, l, r);
        }
        for ch in nch..MAX_CHANNELS {
            self.state.set_vu(ch, 0.0, 0.0);
        }

        let orders = module.get_num_orders();
        self.state.num_orders.store(orders, Ordering::Relaxed);
    }

    fn publish_last_instruments(&mut self, module: &mut Module) {
        let pattern = module.get_current_pattern();
        let row = module.get_current_row();
        if pattern == self.last_snapshot_pattern && row == self.last_snapshot_row {
            return;
        }
        let pattern_changed = pattern != self.last_snapshot_pattern;
        self.last_snapshot_pattern = pattern;
        self.last_snapshot_row = row;

        let num_channels = module.get_num_channels().max(0) as usize;
        let Some(mut pat) = module.get_pattern_by_number(pattern) else {
            return;
        };
        let num_rows = pat.get_num_rows();
        self.state
            .current_rows_in_pattern
            .store(num_rows, Ordering::Relaxed);

        // First time we've seen this pattern (initial play, order change, seek
        // into a different pattern): walk every row from 0..=current to
        // reconstruct each channel's last-seen instrument. Cheap to do once
        // per pattern boundary, and catches sustained notes whose trigger row
        // is older than our 16-row snapshot window.
        if pattern_changed && num_rows > 0 {
            let upto = row.min(num_rows - 1);
            for r in 0..=upto {
                if let Some(mut row_h) = pat.get_row_by_number(r) {
                    for ch in 0..num_channels as i32 {
                        if let Some(mut cell) = row_h.get_cell_by_channel(ch) {
                            let inst = cell.get_data_by_command(ModuleCommandIndex::Instrument);
                            if inst > 0 {
                                self.state.set_last_instrument(ch as usize, inst as i32);
                            }
                        }
                    }
                }
            }
        }

        if let Some(mut row_h) = pat.get_row_by_number(row) {
            for ch in 0..num_channels as i32 {
                if let Some(mut cell) = row_h.get_cell_by_channel(ch) {
                    let inst = cell.get_data_by_command(ModuleCommandIndex::Instrument);
                    if inst > 0 {
                        self.state.set_last_instrument(ch as usize, inst as i32);
                    }
                }
            }
        }
    }
}

fn advance_downsample(accum: &mut u32, sample_rate: u32) -> bool {
    *accum = (*accum).saturating_add(FFT_RING_RATE_HZ);
    if *accum >= sample_rate {
        *accum -= sample_rate;
        true
    } else {
        false
    }
}

/// Load a module from disk on the calling thread (UI), then ship it to the
/// audio thread via [`Command::Load`].
pub fn load_module(path: &Path) -> Result<LoadedModule> {
    let mut file =
        File::open(path).with_context(|| format!("opening module file {}", path.display()))?;
    let mut module = Module::create(&mut file, Logger::None, &[])
        .map_err(|_| anyhow!("libopenmpt could not parse {}", path.display()))?;

    let title = module
        .get_metadata(MetadataKey::ModuleTitle)
        .unwrap_or_default();
    let type_long = module
        .get_metadata(MetadataKey::TypeName)
        .unwrap_or_default();
    let song_message = module
        .get_metadata(MetadataKey::SongMessage)
        .unwrap_or_default();
    let artist = module
        .get_metadata(MetadataKey::ModuleArtist)
        .unwrap_or_default();
    let tracker = module
        .get_metadata(MetadataKey::ModuleTracker)
        .unwrap_or_default();

    let num_samples = module.get_num_samples().max(0);
    let mut sample_names = Vec::with_capacity(num_samples as usize);
    for i in 0..num_samples {
        sample_names.push(module.get_sample_name(i));
    }
    let num_instruments = module.get_num_instruments().max(0);
    let mut instrument_names = Vec::with_capacity(num_instruments as usize);
    for i in 0..num_instruments {
        instrument_names.push(module.get_instrument_name(i));
    }
    let pattern_cache = Arc::new(build_pattern_cache(&mut module));

    Ok(LoadedModule {
        module: SendModule::new(module),
        path: Some(path.to_path_buf()),
        title,
        format_label: type_long,
        sample_names,
        instrument_names,
        song_message,
        artist,
        tracker,
        pattern_cache,
    })
}

pub fn publish_loaded_metadata(state: &SharedState, loaded: &LoadedModule) {
    if let Ok(mut t) = state.title.lock() {
        *t = loaded.title.clone();
    }
    if let Ok(mut f) = state.format_label.lock() {
        *f = loaded.format_label.clone();
    }
    if let Ok(mut p) = state.current_path.lock() {
        *p = loaded.path.clone();
    }
    if let Ok(mut v) = state.sample_names.lock() {
        *v = loaded.sample_names.clone();
    }
    if let Ok(mut v) = state.instrument_names.lock() {
        *v = loaded.instrument_names.clone();
    }
    if let Ok(mut s) = state.song_message.lock() {
        *s = loaded.song_message.clone();
    }
    if let Ok(mut s) = state.artist.lock() {
        *s = loaded.artist.clone();
    }
    if let Ok(mut s) = state.tracker.lock() {
        *s = loaded.tracker.clone();
    }
    state.set_pattern_cache(loaded.pattern_cache.clone());
    state.clear_last_instruments();
}

fn build_pattern_cache(module: &mut Module) -> PatternCache {
    let num_patterns = module.get_num_patterns().max(0);
    let num_channels = module.get_num_channels().max(0) as usize;
    let mut patterns = Vec::with_capacity(num_patterns as usize);

    for pattern in 0..num_patterns {
        let Some(mut pat) = module.get_pattern_by_number(pattern) else {
            patterns.push(PatternData::default());
            continue;
        };
        let num_rows = pat.get_num_rows().max(0);
        let mut rows = Vec::with_capacity(num_rows as usize);
        for row_index in 0..num_rows {
            let mut cells = Vec::with_capacity(num_channels);
            let mut instruments = Vec::with_capacity(num_channels);
            if let Some(mut row_h) = pat.get_row_by_number(row_index) {
                for ch in 0..num_channels as i32 {
                    if let Some(mut cell) = row_h.get_cell_by_channel(ch) {
                        cells.push(cell.get_formatted(0, false));
                        instruments.push(cell.get_data_by_command(ModuleCommandIndex::Instrument));
                    } else {
                        cells.push(String::new());
                        instruments.push(0);
                    }
                }
            } else {
                cells.resize(num_channels, String::new());
                instruments.resize(num_channels, 0);
            }
            rows.push(PatternRow {
                row_index,
                cells,
                instruments,
            });
        }
        patterns.push(PatternData {
            rows,
            channel_count: num_channels,
        });
    }

    PatternCache { patterns }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn downsampled_frames(sample_rate: u32, input_frames: u32) -> u32 {
        let mut accum = 0;
        (0..input_frames)
            .filter(|_| advance_downsample(&mut accum, sample_rate))
            .count() as u32
    }

    #[test]
    fn downsampler_targets_fft_ring_rate_for_common_sample_rates() {
        assert_eq!(downsampled_frames(44_100, 44_100), FFT_RING_RATE_HZ);
        assert_eq!(downsampled_frames(48_000, 48_000), FFT_RING_RATE_HZ);
    }
}
