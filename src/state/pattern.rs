//! Snapshot of pattern data around the currently-playing row.
//!
//! The audio thread fills this on row change; the UI reads it ~30fps. Both go
//! through a `Mutex`, but the audio side uses `try_lock` only — see [`PatternSnapshot::try_publish`].

use std::sync::Mutex;

/// How many rows above and below the current row we snapshot.
pub const WINDOW_RADIUS: usize = 16;
pub const WINDOW_ROWS: usize = WINDOW_RADIUS * 2 + 1;

#[derive(Clone, Debug, Default)]
pub struct PatternRow {
    pub row_index: i32,
    /// One pre-formatted cell per channel, e.g. `"C-5 01 .. A20"`.
    pub cells: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct PatternWindow {
    /// The pattern these rows belong to. `-1` means "not initialised yet".
    pub pattern: i32,
    /// Rows centered on the current row. Some may be empty placeholders if the
    /// window falls outside the pattern boundary.
    pub rows: Vec<PatternRow>,
    /// Position within `rows` of the currently-playing row.
    pub current_index: usize,
    /// Number of channels in the snapshot. Useful for UI layout when the live
    /// channel count atomic might briefly mismatch.
    pub channel_count: usize,
}

impl PatternWindow {
    pub fn current_row(&self) -> Option<&PatternRow> {
        self.rows.get(self.current_index)
    }
}

/// Helper for the audio thread: skip the snapshot if the UI is reading.
pub fn try_publish<F: FnOnce(&mut PatternWindow)>(slot: &Mutex<PatternWindow>, fill: F) {
    if let Ok(mut guard) = slot.try_lock() {
        fill(&mut guard);
    }
}
