//! Per-channel level meters. Reads VU atomics; applies attack/decay envelope
//! in the UI thread. Peak-hold marker with a slow fall.

use crate::state::SharedState;
use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use std::sync::atomic::Ordering;
use std::time::Instant;

const BAR_BLOCKS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
const DECAY_PER_FRAME: f32 = 0.10; // ~30dB/sec at 30fps
const ATTACK: f32 = 1.0;
const PEAK_HOLD_SECS: f32 = 1.5;

#[derive(Default)]
pub struct MeterState {
    smoothed: Vec<f32>,
    peak: Vec<f32>,
    peak_set_at: Vec<Instant>,
}

impl MeterState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn step(&mut self, state: &SharedState) {
        let n = state.num_channels.load(Ordering::Relaxed).max(0) as usize;
        if self.smoothed.len() != n {
            self.smoothed.resize(n, 0.0);
            self.peak.resize(n, 0.0);
            self.peak_set_at.resize(n, Instant::now());
        }
        let now = Instant::now();
        for ch in 0..n {
            let (l, r) = state.vu(ch);
            let v = (l + r).clamp(0.0, 1.0);
            let s = if v >= self.smoothed[ch] {
                self.smoothed[ch] + (v - self.smoothed[ch]) * ATTACK
            } else {
                (self.smoothed[ch] - DECAY_PER_FRAME).max(v)
            };
            self.smoothed[ch] = s;
            if s >= self.peak[ch] {
                self.peak[ch] = s;
                self.peak_set_at[ch] = now;
            } else if now.duration_since(self.peak_set_at[ch]).as_secs_f32() > PEAK_HOLD_SECS {
                self.peak[ch] = (self.peak[ch] - 0.02).max(s);
            }
        }
    }
}

pub fn render(
    f: &mut Frame,
    area: Rect,
    state: &SharedState,
    meter_state: &MeterState,
    theme: &Theme,
    focused: bool,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if focused {
            theme.border_focus
        } else {
            theme.border
        }))
        .title(Span::styled(" meters ", theme.dim_style()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let n = state.num_channels.load(Ordering::Relaxed).max(0) as usize;
    if n == 0 {
        return;
    }

    // Lay channels into N columns to fit the height. Each entry is one row,
    // formatted "NN ▁▂▃▄▅▆▇█". Total chars per entry depend on bar width.
    let bar_w: usize = 8;
    let entry_w: usize = 3 /* label */ + 1 /* space */ + bar_w + 1 /* peak */;
    let cols = ((inner.width as usize) / (entry_w + 2)).max(1);
    let per_col = n.div_ceil(cols);
    let visible_per_col = (inner.height as usize).max(1);
    let per_col_actual = per_col.min(visible_per_col);

    let mut lines: Vec<Line> = Vec::with_capacity(per_col_actual);
    for row in 0..per_col_actual {
        let mut spans: Vec<Span> = Vec::with_capacity(cols * 8);
        for col in 0..cols {
            let ch = col * per_col_actual + row;
            if ch >= n {
                spans.push(Span::raw(" ".repeat(entry_w + 2)));
                continue;
            }
            let level = meter_state.smoothed.get(ch).copied().unwrap_or(0.0);
            let peak = meter_state.peak.get(ch).copied().unwrap_or(0.0);
            spans.push(Span::styled(format!("{:>2} ", ch + 1), theme.dim_style()));
            spans.extend(bar_spans(level, peak, bar_w, theme));
            if col + 1 < cols {
                spans.push(Span::raw("  "));
            }
        }
        lines.push(Line::from(spans));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn bar_spans(level: f32, peak: f32, width: usize, theme: &Theme) -> Vec<Span<'static>> {
    let total_steps = width * BAR_BLOCKS.len();
    let filled_steps = ((level.clamp(0.0, 1.0)) * total_steps as f32).round() as usize;
    let peak_pos = ((peak.clamp(0.0, 1.0)) * width as f32).round() as usize;
    let mut out: Vec<Span<'static>> = Vec::with_capacity(width + 1);
    for cell in 0..width {
        let cell_lo = cell * BAR_BLOCKS.len();
        let cell_hi = cell_lo + BAR_BLOCKS.len();
        let glyph = if filled_steps >= cell_hi {
            *BAR_BLOCKS.last().unwrap()
        } else if filled_steps > cell_lo {
            BAR_BLOCKS[filled_steps - cell_lo - 1]
        } else {
            ' '
        };
        let frac = cell as f32 / (width.max(1) as f32 - 1.0).max(1.0);
        let color = if frac < 0.6 {
            theme.meter_low
        } else if frac < 0.85 {
            theme.meter_mid
        } else {
            theme.meter_high
        };
        let mut style = Style::default().fg(color);
        if cell + 1 == peak_pos && peak_pos > 0 {
            style = style.add_modifier(Modifier::BOLD);
        }
        out.push(Span::styled(glyph.to_string(), style));
    }
    out
}
