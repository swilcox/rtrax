//! What a frontend needs to know at startup: which collection drives playback
//! and where to start. Resolved from CLI arguments (or a GUI's open dialog)
//! before the frontend's main loop is constructed.

use crate::playlist::Playlist;
use std::path::PathBuf;

/// Which collection drives playback, decided once at launch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayMode {
    /// "Play the playlist": next/prev and auto-advance walk the loaded
    /// playlist, and the frontend shows it as a queue.
    Queue,
    /// "Build / browse": next/prev and auto-advance walk the browsed
    /// directory, and additions append to a save target.
    Browse,
}

/// Everything resolved before constructing the frontend's app loop.
pub struct Launch {
    /// Track to start playing, if any.
    pub initial_path: Option<PathBuf>,
    /// Playback/navigation mode.
    pub mode: PlayMode,
    /// The playlist that drives navigation in [`PlayMode::Queue`]. `None` in
    /// browse mode.
    pub queue: Option<Playlist>,
    /// Where playlist additions append. `None` falls back to the default
    /// playlist file.
    pub save_target: Option<PathBuf>,
    /// Explicit browser root (e.g. a directory passed on the command line).
    pub browse_root: Option<PathBuf>,
    /// Start with shuffled play order.
    pub shuffle: bool,
}
