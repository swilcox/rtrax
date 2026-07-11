//! Lock-free state shared between the audio callback and the UI thread.
//!
//! Discipline: the audio callback only writes atomics; the UI only reads them.
//! Float values are bit-cast through `AtomicU32`. The pattern-window snapshot is
//! the only place we use a `Mutex`, and the callback uses `try_lock` only — if
//! contended, it skips the snapshot rather than blocking.

pub mod pattern;

use pattern::PatternCache;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Cap on how many per-channel meter slots we publish to the UI. libopenmpt
/// modules can have many channels (some MPTM/IT > 64); we lay them out in
/// multiple sub-columns when rendering rather than refusing the file.
pub const MAX_CHANNELS: usize = 128;

pub struct SharedState {
    pub playing: AtomicBool,
    pub stopped: AtomicBool,
    /// Set true by audio thread when the song reaches the end.
    pub eof: AtomicBool,

    pub sample_rate: AtomicU32,
    pub master_gain_millibel: AtomicI32,

    pub num_channels: AtomicI32,
    pub num_orders: AtomicI32,
    pub current_order: AtomicI32,
    pub current_pattern: AtomicI32,
    pub current_row: AtomicI32,
    pub current_rows_in_pattern: AtomicI32,
    pub current_speed: AtomicI32,
    pub current_tempo: AtomicI32,

    /// f64 bit-cast to u64 across two u32 halves (`lo`, `hi`).
    pub position_secs_bits: AtomicU64Pair,
    pub duration_secs_bits: AtomicU64Pair,

    /// Per-channel VU. Indices 2*i and 2*i+1 are left/right for channel i.
    /// Each value is `f32::to_bits()`.
    pub vu_bits: [AtomicU32; MAX_CHANNELS * 2],

    /// Post-mix master output peak for the most recent audio buffer, per side.
    /// f32 bits in [0, 1]. Computed directly from the decoded interleaved
    /// buffer in the cpal callback, so this reflects what's actually being
    /// sent to the device — including master gain and any clipping headroom.
    pub master_peak_l_bits: AtomicU32,
    pub master_peak_r_bits: AtomicU32,

    /// Most-recent non-empty instrument number seen per channel in the pattern
    /// stream (1-based to match libopenmpt's pattern formatting; 0 = unseen).
    /// Sticky: rows without an instrument event keep the previous value, which
    /// is what trackers actually do when a note continues with no inst change.
    pub last_instrument: [AtomicI32; MAX_CHANNELS],

    /// Sample and instrument names captured at load time. Indexed in order
    /// returned by libopenmpt — i.e. pattern instrument "01" maps to slot 0.
    pub sample_names: Mutex<Vec<String>>,
    pub instrument_names: Mutex<Vec<String>>,
    /// Free-form song message / liner notes from the module file.
    pub song_message: Mutex<String>,
    /// Artist + tracker metadata strings.
    pub artist: Mutex<String>,
    pub tracker: Mutex<String>,

    /// One-line song title from libopenmpt metadata, plus tracker/format.
    pub title: Mutex<String>,
    pub format_label: Mutex<String>,

    /// Currently-loaded file path. UI uses this for the header + browser.
    pub current_path: Mutex<Option<std::path::PathBuf>>,

    /// Preformatted pattern data captured at load time. The audio callback only
    /// publishes current pattern/row; the UI slices this cache for display.
    pub pattern_cache: Mutex<Arc<PatternCache>>,

    /// Generation counter: incremented every time the audio thread updates
    /// `current_row`. UI uses this to decide whether to redraw the pattern view.
    pub row_generation: AtomicUsize,
}

impl Default for SharedState {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            playing: AtomicBool::new(false),
            stopped: AtomicBool::new(true),
            eof: AtomicBool::new(false),
            sample_rate: AtomicU32::new(48_000),
            master_gain_millibel: AtomicI32::new(0),
            num_channels: AtomicI32::new(0),
            num_orders: AtomicI32::new(0),
            current_order: AtomicI32::new(0),
            current_pattern: AtomicI32::new(0),
            current_row: AtomicI32::new(0),
            current_rows_in_pattern: AtomicI32::new(0),
            current_speed: AtomicI32::new(0),
            current_tempo: AtomicI32::new(0),
            position_secs_bits: AtomicU64Pair::new(0),
            duration_secs_bits: AtomicU64Pair::new(0),
            vu_bits: std::array::from_fn(|_| AtomicU32::new(0)),
            master_peak_l_bits: AtomicU32::new(0),
            master_peak_r_bits: AtomicU32::new(0),
            last_instrument: std::array::from_fn(|_| AtomicI32::new(0)),
            sample_names: Mutex::new(Vec::new()),
            instrument_names: Mutex::new(Vec::new()),
            song_message: Mutex::new(String::new()),
            artist: Mutex::new(String::new()),
            tracker: Mutex::new(String::new()),
            title: Mutex::new(String::new()),
            format_label: Mutex::new(String::new()),
            current_path: Mutex::new(None),
            pattern_cache: Mutex::new(Arc::new(PatternCache::default())),
            row_generation: AtomicUsize::new(0),
        }
    }

    pub fn set_vu(&self, channel: usize, left: f32, right: f32) {
        if channel >= MAX_CHANNELS {
            return;
        }
        self.vu_bits[channel * 2].store(left.to_bits(), Ordering::Relaxed);
        self.vu_bits[channel * 2 + 1].store(right.to_bits(), Ordering::Relaxed);
    }

    pub fn vu(&self, channel: usize) -> (f32, f32) {
        if channel >= MAX_CHANNELS {
            return (0.0, 0.0);
        }
        let l = f32::from_bits(self.vu_bits[channel * 2].load(Ordering::Relaxed));
        let r = f32::from_bits(self.vu_bits[channel * 2 + 1].load(Ordering::Relaxed));
        (l, r)
    }

    pub fn position_secs(&self) -> f64 {
        f64::from_bits(self.position_secs_bits.load())
    }

    pub fn duration_secs(&self) -> f64 {
        f64::from_bits(self.duration_secs_bits.load())
    }

    pub fn set_position_secs(&self, v: f64) {
        self.position_secs_bits.store(v.to_bits());
    }

    pub fn set_duration_secs(&self, v: f64) {
        self.duration_secs_bits.store(v.to_bits());
    }

    pub fn set_master_peak(&self, left: f32, right: f32) {
        self.master_peak_l_bits
            .store(left.to_bits(), Ordering::Relaxed);
        self.master_peak_r_bits
            .store(right.to_bits(), Ordering::Relaxed);
    }

    pub fn master_peak(&self) -> (f32, f32) {
        let l = f32::from_bits(self.master_peak_l_bits.load(Ordering::Relaxed));
        let r = f32::from_bits(self.master_peak_r_bits.load(Ordering::Relaxed));
        (l, r)
    }

    pub fn set_last_instrument(&self, channel: usize, instrument: i32) {
        if channel < MAX_CHANNELS {
            self.last_instrument[channel].store(instrument, Ordering::Relaxed);
        }
    }

    pub fn last_instrument(&self, channel: usize) -> i32 {
        if channel < MAX_CHANNELS {
            self.last_instrument[channel].load(Ordering::Relaxed)
        } else {
            0
        }
    }

    pub fn clear_last_instruments(&self) {
        for slot in self.last_instrument.iter() {
            slot.store(0, Ordering::Relaxed);
        }
    }

    pub fn set_pattern_cache(&self, cache: Arc<PatternCache>) {
        if let Ok(mut slot) = self.pattern_cache.lock() {
            *slot = cache;
        }
    }
}

/// Two-u32 atomic store for u64. Each half stored Relaxed; readers tolerate the
/// brief inconsistency window because the values (position/duration in seconds)
/// are advisory display state.
pub struct AtomicU64Pair {
    lo: AtomicU32,
    hi: AtomicU32,
}

impl AtomicU64Pair {
    pub const fn new(v: u64) -> Self {
        Self {
            lo: AtomicU32::new(v as u32),
            hi: AtomicU32::new((v >> 32) as u32),
        }
    }

    pub fn load(&self) -> u64 {
        let lo = self.lo.load(Ordering::Relaxed) as u64;
        let hi = self.hi.load(Ordering::Relaxed) as u64;
        (hi << 32) | lo
    }

    pub fn store(&self, v: u64) {
        self.lo.store(v as u32, Ordering::Relaxed);
        self.hi.store((v >> 32) as u32, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::pattern::PatternCache;

    #[test]
    fn vu_and_master_peak_round_trip() {
        let state = SharedState::new();

        state.set_vu(0, 0.25, 0.75);
        state.set_master_peak(0.5, 1.0);

        assert_eq!(state.vu(0), (0.25, 0.75));
        assert_eq!(state.vu(MAX_CHANNELS), (0.0, 0.0));
        assert_eq!(state.master_peak(), (0.5, 1.0));
    }

    #[test]
    fn seconds_round_trip() {
        let state = SharedState::new();

        state.set_position_secs(12.5);
        state.set_duration_secs(123.0);

        assert_eq!(state.position_secs(), 12.5);
        assert_eq!(state.duration_secs(), 123.0);
    }

    #[test]
    fn last_instruments_can_be_cleared() {
        let state = SharedState::new();

        state.set_last_instrument(0, 3);
        state.set_last_instrument(MAX_CHANNELS, 9);
        assert_eq!(state.last_instrument(0), 3);
        assert_eq!(state.last_instrument(MAX_CHANNELS), 0);

        state.clear_last_instruments();
        assert_eq!(state.last_instrument(0), 0);
    }

    #[test]
    fn atomic_u64_pair_roundtrip_zero_and_max() {
        let pair = AtomicU64Pair::new(0);
        assert_eq!(pair.load(), 0);

        pair.store(u64::MAX);
        assert_eq!(pair.load(), u64::MAX);

        // Value that exercises both halves non-trivially.
        let v: u64 = 0xDEAD_BEEF_CAFE_1234;
        pair.store(v);
        assert_eq!(pair.load(), v);
    }

    #[test]
    fn vu_last_valid_channel_works() {
        let state = SharedState::new();
        state.set_vu(MAX_CHANNELS - 1, 0.1, 0.9);
        assert_eq!(state.vu(MAX_CHANNELS - 1), (0.1, 0.9));
    }

    #[test]
    fn set_last_instrument_out_of_bounds_is_silent() {
        let state = SharedState::new();
        state.set_last_instrument(MAX_CHANNELS, 42);
        assert_eq!(state.last_instrument(MAX_CHANNELS), 0);
    }

    #[test]
    fn pattern_cache_is_replaceable() {
        let state = SharedState::new();
        let cache = Arc::new(PatternCache::default());

        state.set_pattern_cache(cache.clone());

        let stored = state.pattern_cache.lock().unwrap().clone();
        assert!(Arc::ptr_eq(&stored, &cache));
    }
}
