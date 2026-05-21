//! Preformatted pattern data and helpers for slicing a window around the
//! currently-playing row.

/// How many rows above and below the current row we snapshot.
pub const WINDOW_RADIUS: usize = 16;
pub const WINDOW_ROWS: usize = WINDOW_RADIUS * 2 + 1;

#[derive(Clone, Debug, Default)]
pub struct PatternRow {
    pub row_index: i32,
    /// One pre-formatted cell per channel, e.g. `"C-5 01 .. A20"`.
    pub cells: Vec<String>,
    /// Raw instrument number per channel from libopenmpt (0 = none). Captured
    /// alongside the formatted string so the UI doesn't have to parse it back
    /// out of the cell text (which is hex-formatted for most module formats).
    pub instruments: Vec<u8>,
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

#[derive(Clone, Debug, Default)]
pub struct PatternData {
    pub rows: Vec<PatternRow>,
    pub channel_count: usize,
}

#[derive(Clone, Debug, Default)]
pub struct PatternCache {
    pub patterns: Vec<PatternData>,
}

impl PatternCache {
    pub fn window(&self, pattern: i32, row: i32) -> PatternWindow {
        let Some(pattern_data) = usize::try_from(pattern)
            .ok()
            .and_then(|idx| self.patterns.get(idx))
        else {
            return PatternWindow::default();
        };

        let start = row - WINDOW_RADIUS as i32;
        let mut rows = Vec::with_capacity(WINDOW_ROWS);
        for offset in 0..WINDOW_ROWS as i32 {
            let row_index = start + offset;
            if row_index < 0 {
                rows.push(empty_row(row_index, pattern_data.channel_count));
                continue;
            }
            match pattern_data.rows.get(row_index as usize) {
                Some(row) => rows.push(row.clone()),
                None => rows.push(empty_row(row_index, pattern_data.channel_count)),
            }
        }

        PatternWindow {
            pattern,
            rows,
            current_index: WINDOW_RADIUS,
            channel_count: pattern_data.channel_count,
        }
    }
}

fn empty_row(row_index: i32, channel_count: usize) -> PatternRow {
    PatternRow {
        row_index,
        cells: vec![String::new(); channel_count],
        instruments: vec![0u8; channel_count],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(row_index: i32, cell: &str, instrument: u8) -> PatternRow {
        PatternRow {
            row_index,
            cells: vec![cell.to_string()],
            instruments: vec![instrument],
        }
    }

    #[test]
    fn current_row_returns_centered_row() {
        let cache = PatternCache {
            patterns: vec![PatternData {
                rows: (0..64).map(|idx| row(idx, "C-5 01 .. ...", 1)).collect(),
                channel_count: 1,
            }],
        };

        let window = cache.window(0, 24);

        assert_eq!(window.rows.len(), WINDOW_ROWS);
        assert_eq!(window.current_row().unwrap().row_index, 24);
        assert_eq!(window.rows.first().unwrap().row_index, 8);
        assert_eq!(window.rows.last().unwrap().row_index, 40);
    }

    #[test]
    fn window_pads_out_of_bounds_rows() {
        let cache = PatternCache {
            patterns: vec![PatternData {
                rows: vec![row(0, "C-5 01 .. ...", 1), row(1, "D-5 02 .. ...", 2)],
                channel_count: 1,
            }],
        };

        let window = cache.window(0, 0);

        assert_eq!(window.current_row().unwrap().row_index, 0);
        assert!(window.rows[..WINDOW_RADIUS]
            .iter()
            .all(|row| row.cells == vec![String::new()]));
        assert!(window.rows[WINDOW_RADIUS + 2..]
            .iter()
            .all(|row| row.cells == vec![String::new()]));
    }

    #[test]
    fn invalid_pattern_returns_empty_window() {
        let cache = PatternCache::default();

        let window = cache.window(99, 0);

        assert!(window.rows.is_empty());
        assert_eq!(window.channel_count, 0);
    }
}
