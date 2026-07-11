//! Master L/R output meter. Renders the smoothed post-mix peaks from
//! `rtrax_core::meters::MasterMeter` as two horizontal bars.

use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use rtrax_core::meters::{Envelope, MasterMeter};

const BAR_BLOCKS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

pub fn render(f: &mut Frame, area: Rect, meter: &MasterMeter, gain_millibel: i32, theme: &Theme) {
    // Gain readout lives in the block title so it's always visible next to the
    // output bars without stealing a row from them.
    let db = gain_millibel / 100;
    let gain_label = if db == 0 {
        "0 dB".to_string()
    } else {
        format!("{db:+} dB")
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" master ", theme.dim_style()))
        .title_top(
            Line::from(Span::styled(
                format!(" gain {gain_label} "),
                theme.dim_style(),
            ))
            .right_aligned(),
        );
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width < 4 || inner.height == 0 {
        return;
    }

    // "L " label + bar; one space margin on the right.
    let label_w = 2usize;
    let right_margin = 1usize;
    let bar_w = (inner.width as usize).saturating_sub(label_w + right_margin);
    if bar_w == 0 {
        return;
    }

    // Center the two bars vertically inside the block.
    let total_rows = inner.height as usize;
    let top_pad = total_rows.saturating_sub(2) / 2;

    let mut lines: Vec<Line> = Vec::with_capacity(total_rows);
    for _ in 0..top_pad {
        lines.push(Line::from(""));
    }
    lines.push(bar_line("L", meter.left, bar_w, theme));
    if total_rows >= 2 {
        lines.push(bar_line("R", meter.right, bar_w, theme));
    }
    while lines.len() < total_rows {
        lines.push(Line::from(""));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn bar_line(label: &str, env: Envelope, width: usize, theme: &Theme) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::with_capacity(width + 2);
    spans.push(Span::styled(
        format!("{label} "),
        Style::default().fg(theme.fg_dim),
    ));
    spans.extend(bar_spans(env.smoothed, env.peak, width, theme));
    Line::from(spans)
}

fn bar_spans(level: f32, peak: f32, width: usize, theme: &Theme) -> Vec<Span<'static>> {
    let steps_per_cell = BAR_BLOCKS.len();
    let total_steps = width * steps_per_cell;
    let filled_steps = ((level.clamp(0.0, 1.0)) * total_steps as f32).round() as usize;
    let peak_pos = ((peak.clamp(0.0, 1.0)) * width as f32).round() as usize;
    let mut out: Vec<Span<'static>> = Vec::with_capacity(width);
    for cell in 0..width {
        let cell_lo = cell * steps_per_cell;
        let cell_hi = cell_lo + steps_per_cell;
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
