//! rtrax-core — frontend-agnostic playback engine for rtrax.
//!
//! Owns the audio thread (openmpt decoding through a cpal stream), the
//! lock-free state shared with whatever frontend is polling it, playlist
//! handling, and the FFT analysis for spectrum displays. Contains no UI:
//! frontends (TUI, GUI) read [`state::SharedState`], drain the FFT ring, and
//! drive playback through [`audio::command::Command`].

pub mod audio;
pub mod fft;
pub mod files;
pub mod launch;
pub mod meters;
pub mod paths;
pub mod playlist;
pub mod rng;
pub mod state;
