//! Commands sent from the UI thread to the audio thread.

use crate::state::pattern::PatternCache;
use openmpt::module::Module;
use std::path::PathBuf;
use std::sync::Arc;

/// `openmpt::Module` wraps a raw C pointer and isn't Send. libopenmpt itself
/// is fine to use from any thread as long as only one thread touches a given
/// module at a time — which is the exact discipline we follow (audio thread
/// exclusively owns the module). So we hand-implement Send and document the
/// invariant.
pub struct SendModule(Module);

impl SendModule {
    pub fn new(module: Module) -> Self {
        Self(module)
    }

    pub fn module_mut(&mut self) -> &mut Module {
        &mut self.0
    }
}

// SAFETY: see comment above. The audio thread holds exclusive access for the
// module's whole runtime lifetime.
unsafe impl Send for SendModule {}

/// A module loaded on the UI thread, ready to be shipped to the audio thread.
pub struct LoadedModule {
    pub module: SendModule,
    pub path: Option<PathBuf>,
    pub title: String,
    pub format_label: String,
    /// Sample names in libopenmpt index order. Pattern instrument "01" maps to
    /// `sample_names[0]`. May be empty for formats with no samples.
    pub sample_names: Vec<String>,
    pub instrument_names: Vec<String>,
    pub song_message: String,
    pub artist: String,
    pub tracker: String,
    pub pattern_cache: Arc<PatternCache>,
}

pub enum Command {
    /// Replace the currently-playing module. Metadata is published by the
    /// caller before this reaches the audio callback.
    Load(SendModule),
    Play,
    Pause,
    Stop,
    /// Positive = forward, negative = backward. Seconds.
    SeekRelative(f32),
    /// Master gain in millibels (1 dB = 100 mB; 0 = unity).
    VolumeMillibel(i32),
}
