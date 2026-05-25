//! Spectrum analyzer FFT pipeline.
//!
//! - Pulls samples from the rtrb consumer, maintains a rolling window.
//! - Applies a Hann window, runs a `rustfft` forward FFT, computes magnitudes.
//! - Log-bins into N bands and smooths with attack/decay envelope.

use rtrb::Consumer;
use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

pub const FFT_SIZE: usize = 2048;
pub const DEFAULT_BANDS: usize = 32;
const ATTACK: f32 = 0.9;
const DECAY: f32 = 0.18;

pub struct Spectrum {
    fft: Arc<dyn Fft<f32>>,
    sample_rate: f32,
    window: Vec<f32>,
    hann: Vec<f32>,
    scratch: Vec<Complex32>,
    bands: Vec<f32>,
}

impl Spectrum {
    pub fn new(sample_rate: f32, bands: usize) -> Self {
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let hann = (0..FFT_SIZE)
            .map(|i| {
                let phase = 2.0 * std::f32::consts::PI * i as f32 / (FFT_SIZE as f32 - 1.0);
                0.5 - 0.5 * phase.cos()
            })
            .collect();
        Self {
            fft,
            sample_rate,
            window: vec![0.0; FFT_SIZE],
            hann,
            scratch: vec![Complex32::default(); FFT_SIZE],
            bands: vec![0.0; bands.max(1)],
        }
    }

    pub fn resize_bands(&mut self, bands: usize) {
        let bands = bands.max(1);
        if self.bands.len() != bands {
            self.bands.resize(bands, 0.0);
        }
    }

    pub fn bands(&self) -> &[f32] {
        &self.bands
    }

    /// Pull from the consumer, advance the rolling window, run the FFT, and
    /// update the smoothed band magnitudes.
    pub fn step(&mut self, rx: &mut Consumer<f32>) {
        // Drain everything the audio thread has produced, sliding the window.
        let mut consumed = 0usize;
        while let Ok(sample) = rx.pop() {
            self.window.rotate_left(1);
            *self.window.last_mut().unwrap() = sample;
            consumed += 1;
            if consumed >= FFT_SIZE {
                break;
            }
        }

        for (s, w) in self.scratch.iter_mut().zip(self.window.iter()) {
            *s = Complex32::new(*w, 0.0);
        }
        for (s, h) in self.scratch.iter_mut().zip(self.hann.iter()) {
            s.re *= *h;
            s.im = 0.0;
        }
        self.fft.process(&mut self.scratch);

        // Log-spaced bands between 30Hz and Nyquist.
        let nyquist = self.sample_rate * 0.5;
        let lo_hz = 30.0_f32.min(nyquist - 1.0);
        let hi_hz = nyquist.max(lo_hz + 1.0);
        let log_lo = lo_hz.ln();
        let log_hi = hi_hz.ln();
        let bands_len = self.bands.len();
        let bin_hz = self.sample_rate / FFT_SIZE as f32;

        for (i, slot) in self.bands.iter_mut().enumerate() {
            let f0 = (log_lo + (log_hi - log_lo) * i as f32 / bands_len as f32).exp();
            let f1 = (log_lo + (log_hi - log_lo) * (i as f32 + 1.0) / bands_len as f32).exp();
            let b0 = (f0 / bin_hz).floor() as usize;
            let b1 = ((f1 / bin_hz).ceil() as usize).max(b0 + 1);
            let b1 = b1.min(FFT_SIZE / 2);
            let b0 = b0.min(b1);
            let mut peak = 0.0f32;
            for b in b0..b1 {
                let c = self.scratch[b];
                let mag = (c.re * c.re + c.im * c.im).sqrt();
                if mag > peak {
                    peak = mag;
                }
            }
            // Compress to roughly [0, 1] via log scaling. The tiny floor only
            // exists to keep log10 finite — it sits well below the bottom of
            // the dB window so true silence maps to v = 0.
            let norm = (peak / FFT_SIZE as f32).max(1e-6);
            let db = 20.0 * norm.log10(); // dBFS-ish (negative)
            let v = ((db + 60.0) / 60.0).clamp(0.0, 1.0);

            *slot = if v > *slot {
                *slot + (v - *slot) * ATTACK
            } else {
                *slot + (v - *slot) * DECAY
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_spectrum_has_zero_bands() {
        let s = Spectrum::new(44100.0, 16);
        assert_eq!(s.bands().len(), 16);
        assert!(s.bands().iter().all(|&v| v == 0.0));
    }

    #[test]
    fn resize_bands_grows_and_shrinks() {
        let mut s = Spectrum::new(44100.0, 8);
        assert_eq!(s.bands().len(), 8);

        s.resize_bands(32);
        assert_eq!(s.bands().len(), 32);

        s.resize_bands(4);
        assert_eq!(s.bands().len(), 4);
    }

    #[test]
    fn resize_bands_noop_when_same_size() {
        let mut s = Spectrum::new(44100.0, 16);
        s.resize_bands(16);
        assert_eq!(s.bands().len(), 16);
    }

    #[test]
    fn resize_bands_clamps_zero_to_one() {
        let mut s = Spectrum::new(44100.0, 4);
        s.resize_bands(0);
        assert_eq!(s.bands().len(), 1, "0 bands should clamp to 1");
    }

    #[test]
    fn step_with_empty_consumer_does_not_panic() {
        let (_tx, mut rx) = rtrb::RingBuffer::<f32>::new(64);
        let mut s = Spectrum::new(44100.0, 8);
        s.step(&mut rx);
        // bands decay toward zero; they should stay in [0, 1]
        assert!(s.bands().iter().all(|&v| v >= 0.0 && v <= 1.0));
    }

    #[test]
    fn step_with_silence_keeps_bands_near_zero() {
        let (mut tx, mut rx) = rtrb::RingBuffer::<f32>::new(FFT_SIZE * 2);
        for _ in 0..FFT_SIZE {
            let _ = tx.push(0.0);
        }
        let mut s = Spectrum::new(44100.0, 8);
        // Run a few steps so the decay settles.
        for _ in 0..5 {
            s.step(&mut rx);
        }
        for &v in s.bands() {
            assert!(v < 0.01, "expected near-zero band, got {v}");
        }
    }

    #[test]
    fn default_bands_count_is_used_when_constructed() {
        let s = Spectrum::new(48000.0, DEFAULT_BANDS);
        assert_eq!(s.bands().len(), DEFAULT_BANDS);
    }
}
