//! System media controls ("Now Playing") via souvlaki: macOS
//! `MPRemoteCommandCenter`/`MPNowPlayingInfoCenter`, MPRIS on Linux, SMTC on
//! Windows. Media-key presses and Control Center actions arrive through a
//! channel drained once per frame; song metadata and playback state are
//! pushed back out so the OS scrubber stays truthful.

use eframe::egui;
use rtrax_core::state::SharedState;
use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

pub struct Media {
    controls: MediaControls,
    rx: Receiver<MediaControlEvent>,
    last_title: String,
    /// 0 stopped · 1 paused · 2 playing. `None` forces the next push.
    last_status: Option<u8>,
    last_push: Instant,
}

impl Media {
    /// `None` (with a warning logged) when the platform backend can't attach —
    /// the app works fine without it.
    pub fn new(ctx: egui::Context) -> Option<Self> {
        let config = PlatformConfig {
            display_name: "rtrax",
            dbus_name: "rtrax",
            hwnd: None,
        };
        let mut controls = match MediaControls::new(config) {
            Ok(controls) => controls,
            Err(err) => {
                tracing::warn!(?err, "system media controls unavailable");
                return None;
            }
        };
        let (tx, rx) = channel();
        let attach = controls.attach(move |event| {
            let _ = tx.send(event);
            // Wake the UI so the command is applied promptly even when idle.
            ctx.request_repaint();
        });
        if let Err(err) = attach {
            tracing::warn!(?err, "failed to attach media control handler");
            return None;
        }
        Some(Self {
            controls,
            rx,
            last_title: String::new(),
            last_status: None,
            last_push: Instant::now(),
        })
    }

    /// Pending events from the OS (media keys, Control Center, MPRIS…).
    pub fn events(&self) -> Vec<MediaControlEvent> {
        self.rx.try_iter().collect()
    }

    /// Push metadata on song change, and playback state on change plus a 1s
    /// heartbeat while playing so the OS position scrubber tracks seeks.
    pub fn sync(&mut self, state: &SharedState) {
        let title = state.title.lock().map(|t| t.clone()).unwrap_or_default();
        if !title.is_empty() && title != self.last_title {
            let artist = state.artist.lock().map(|a| a.clone()).unwrap_or_default();
            let metadata = MediaMetadata {
                title: Some(&title),
                artist: (!artist.is_empty()).then_some(artist.as_str()),
                album: None,
                cover_url: None,
                duration: Some(Duration::from_secs_f64(state.duration_secs().max(0.0))),
            };
            if let Err(err) = self.controls.set_metadata(metadata) {
                tracing::debug!(?err, "set_metadata failed");
            }
            self.last_title = title;
            self.last_status = None; // force a playback push for the new song
        }

        let playing = state.playing.load(Ordering::Relaxed);
        let stopped = state.stopped.load(Ordering::Relaxed);
        let status = if stopped {
            0
        } else if playing {
            2
        } else {
            1
        };
        let heartbeat = status == 2 && self.last_push.elapsed() > Duration::from_secs(1);
        if Some(status) != self.last_status || heartbeat {
            let progress = Some(MediaPosition(Duration::from_secs_f64(
                state.position_secs().max(0.0),
            )));
            let playback = match status {
                0 => MediaPlayback::Stopped,
                1 => MediaPlayback::Paused { progress },
                _ => MediaPlayback::Playing { progress },
            };
            if let Err(err) = self.controls.set_playback(playback) {
                tracing::debug!(?err, "set_playback failed");
            }
            self.last_status = Some(status);
            self.last_push = Instant::now();
        }
    }
}
