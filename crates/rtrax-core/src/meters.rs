//! UI-side level-meter smoothing: attack/decay envelope with peak-hold.
//!
//! The audio thread publishes raw instantaneous levels (per-channel VU and
//! post-mix master peaks); frontends step these envelopes once per rendered
//! frame to get eye-friendly bars. Instant attack, linear decay, and a peak
//! marker that holds then falls slowly.

use crate::state::SharedState;
use std::sync::atomic::Ordering;
use std::time::Instant;

pub const PEAK_HOLD_SECS: f32 = 1.5;
pub const PEAK_FALL_PER_FRAME: f32 = 0.02;
/// Per-channel VU decay (~30dB/sec at 30fps).
pub const CHANNEL_DECAY_PER_FRAME: f32 = 0.10;
/// Master output decay — slower than per-channel, more like a VU needle.
pub const MASTER_DECAY_PER_FRAME: f32 = 0.06;

#[derive(Clone, Copy)]
pub struct Envelope {
    pub smoothed: f32,
    pub peak: f32,
    peak_set_at: Option<Instant>,
    decay_per_frame: f32,
}

impl Envelope {
    pub fn new(decay_per_frame: f32) -> Self {
        Self {
            smoothed: 0.0,
            peak: 0.0,
            peak_set_at: None,
            decay_per_frame,
        }
    }

    pub fn step(&mut self, v: f32, now: Instant) {
        let v = v.clamp(0.0, 1.0);
        // Rises instantly to the new sample, then decays linearly.
        let s = if v >= self.smoothed {
            v
        } else {
            (self.smoothed - self.decay_per_frame).max(v)
        };
        self.smoothed = s;
        if s >= self.peak {
            self.peak = s;
            self.peak_set_at = Some(now);
        } else if let Some(t) = self.peak_set_at {
            if now.duration_since(t).as_secs_f32() > PEAK_HOLD_SECS {
                self.peak = (self.peak - PEAK_FALL_PER_FRAME).max(s);
            }
        }
    }
}

/// Envelopes for every channel's L/R VU pair. Resizes itself to the loaded
/// module's channel count on each step.
pub struct ChannelMeters {
    left: Vec<Envelope>,
    right: Vec<Envelope>,
}

impl Default for ChannelMeters {
    fn default() -> Self {
        Self::new()
    }
}

impl ChannelMeters {
    pub fn new() -> Self {
        Self {
            left: Vec::new(),
            right: Vec::new(),
        }
    }

    pub fn step(&mut self, state: &SharedState) {
        let n = state.num_channels.load(Ordering::Relaxed).max(0) as usize;
        if self.left.len() != n {
            self.left.resize(n, Envelope::new(CHANNEL_DECAY_PER_FRAME));
            self.right.resize(n, Envelope::new(CHANNEL_DECAY_PER_FRAME));
        }
        let now = Instant::now();
        for ch in 0..n {
            let (l, r) = state.vu(ch);
            self.left[ch].step(l, now);
            self.right[ch].step(r, now);
        }
    }

    pub fn len(&self) -> usize {
        self.left.len()
    }

    pub fn is_empty(&self) -> bool {
        self.left.is_empty()
    }

    /// The (left, right) envelopes for `channel`, zeros when out of range.
    pub fn channel(&self, channel: usize) -> (Envelope, Envelope) {
        let default = Envelope::new(CHANNEL_DECAY_PER_FRAME);
        (
            self.left.get(channel).copied().unwrap_or(default),
            self.right.get(channel).copied().unwrap_or(default),
        )
    }
}

/// Envelopes for the post-mix master L/R output peaks.
pub struct MasterMeter {
    pub left: Envelope,
    pub right: Envelope,
}

impl Default for MasterMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl MasterMeter {
    pub fn new() -> Self {
        Self {
            left: Envelope::new(MASTER_DECAY_PER_FRAME),
            right: Envelope::new(MASTER_DECAY_PER_FRAME),
        }
    }

    pub fn step(&mut self, state: &SharedState) {
        let (l, r) = state.master_peak();
        let now = Instant::now();
        self.left.step(l, now);
        self.right.step(r, now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_attacks_instantly_and_decays_linearly() {
        let mut env = Envelope::new(0.1);
        let now = Instant::now();
        env.step(0.8, now);
        assert_eq!(env.smoothed, 0.8);
        env.step(0.0, now);
        assert!((env.smoothed - 0.7).abs() < 1e-6);
        env.step(0.0, now);
        assert!((env.smoothed - 0.6).abs() < 1e-6);
    }

    #[test]
    fn envelope_peak_holds() {
        let mut env = Envelope::new(0.1);
        let now = Instant::now();
        env.step(0.9, now);
        for _ in 0..5 {
            env.step(0.0, now);
        }
        // Within the hold window the peak must not fall.
        assert_eq!(env.peak, 0.9);
        assert!(env.smoothed < 0.9);
    }

    #[test]
    fn channel_meters_resize_to_channel_count() {
        let state = SharedState::new();
        state.num_channels.store(4, Ordering::Relaxed);
        state.set_vu(0, 0.5, 0.25);

        let mut meters = ChannelMeters::new();
        meters.step(&state);

        assert_eq!(meters.len(), 4);
        let (l, r) = meters.channel(0);
        assert_eq!(l.smoothed, 0.5);
        assert_eq!(r.smoothed, 0.25);
        // Out of range reads as silent.
        assert_eq!(meters.channel(99).0.smoothed, 0.0);
    }
}
