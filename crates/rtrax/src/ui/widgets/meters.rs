//! Per-channel level meters. Reads VU atomics; applies attack/decay envelope
//! in the UI thread. Peak-hold marker with a slow fall.

use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use rtrax_core::state::SharedState;
use std::sync::atomic::Ordering;
use std::time::Instant;

const BAR_BLOCKS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
const DECAY_PER_FRAME: f32 = 0.10; // ~30dB/sec at 30fps
const PEAK_HOLD_SECS: f32 = 1.5;
const BAR_W: usize = 8;
// "NN L ████████" — 2 digit label + space + L/R + space + bar
const ENTRY_W: usize = 2 + 1 + 1 + 1 + BAR_W;
const COL_GAP: usize = 2;

#[derive(Default, Clone, Copy)]
struct Envelope {
    smoothed: f32,
    peak: f32,
    peak_set_at: Option<Instant>,
}

impl Envelope {
    fn step(&mut self, v: f32, now: Instant) {
        let v = v.clamp(0.0, 1.0);
        // ATTACK=1.0 means rises instantly to the new sample, then decays linearly.
        let s = if v >= self.smoothed {
            v
        } else {
            (self.smoothed - DECAY_PER_FRAME).max(v)
        };
        self.smoothed = s;
        if s >= self.peak {
            self.peak = s;
            self.peak_set_at = Some(now);
        } else if let Some(t) = self.peak_set_at {
            if now.duration_since(t).as_secs_f32() > PEAK_HOLD_SECS {
                self.peak = (self.peak - 0.02).max(s);
            }
        }
    }
}

#[derive(Default)]
pub struct MeterState {
    left: Vec<Envelope>,
    right: Vec<Envelope>,
}

impl MeterState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn step(&mut self, state: &SharedState) {
        let n = state.num_channels.load(Ordering::Relaxed).max(0) as usize;
        if self.left.len() != n {
            self.left.resize(n, Envelope::default());
            self.right.resize(n, Envelope::default());
        }
        let now = Instant::now();
        for ch in 0..n {
            let (l, r) = state.vu(ch);
            self.left[ch].step(l, now);
            self.right[ch].step(r, now);
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

    // Two rows per channel (L stacked over R). Lay channels into columns so
    // a full L+R pair never gets split across the column boundary.
    let cols = ((inner.width as usize) / (ENTRY_W + COL_GAP)).max(1);
    let channels_per_col = n.div_ceil(cols);
    // Round visible row count down to even so we never show a lone L without R.
    let visible_rows = ((inner.height as usize) / 2) * 2;
    let actual_rows = (channels_per_col * 2).min(visible_rows);
    let visible_channels_per_col = actual_rows / 2;
    if visible_channels_per_col == 0 {
        return;
    }

    let mut lines: Vec<Line> = Vec::with_capacity(actual_rows);
    for row in 0..actual_rows {
        let local_ch = row / 2;
        let is_left = row % 2 == 0;
        let mut spans: Vec<Span> = Vec::with_capacity(cols * 6);
        for col in 0..cols {
            let ch = col * visible_channels_per_col + local_ch;
            if ch >= n {
                spans.push(Span::raw(" ".repeat(ENTRY_W + COL_GAP)));
                continue;
            }
            let env = if is_left {
                meter_state.left.get(ch).copied()
            } else {
                meter_state.right.get(ch).copied()
            }
            .unwrap_or_default();
            let label = if is_left {
                format!("{:>2} L ", ch + 1)
            } else {
                "   R ".to_string()
            };
            spans.push(Span::styled(label, theme.dim_style()));
            spans.extend(bar_spans(env.smoothed, env.peak, BAR_W, theme));
            if col + 1 < cols {
                spans.push(Span::raw(" ".repeat(COL_GAP)));
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
