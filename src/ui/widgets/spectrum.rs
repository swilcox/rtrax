//! Master spectrum analyzer bar row.

use crate::ui::fft::Spectrum;
use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

const BAR_BLOCKS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

pub fn render(f: &mut Frame, area: Rect, spectrum: &Spectrum, theme: &Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" spectrum ", theme.dim_style()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let bands = spectrum.bands();
    if bands.is_empty() {
        return;
    }

    let width = inner.width as usize;
    let height = inner.height as usize;

    // Map each terminal column to a band, sampling/upsampling as needed.
    let mut cols: Vec<f32> = Vec::with_capacity(width);
    for c in 0..width {
        let frac = c as f32 / width.max(1) as f32;
        let idx = (frac * bands.len() as f32).floor() as usize;
        cols.push(bands.get(idx).copied().unwrap_or(0.0));
    }

    // Render rows top-down. For each row from top (= highest value) to bottom,
    // a column is filled if its value crosses the row threshold.
    let mut lines: Vec<Line> = Vec::with_capacity(height);
    for row in 0..height {
        let row_from_bottom = height - row;
        let mut spans: Vec<Span> = Vec::with_capacity(width);
        for &v in &cols {
            let level = (v.clamp(0.0, 1.0) * height as f32).max(0.0);
            let level_int = level.floor() as usize;
            let glyph = if level_int >= row_from_bottom {
                *BAR_BLOCKS.last().unwrap()
            } else if level_int + 1 == row_from_bottom {
                let frac = level - level_int as f32;
                let step = ((frac * BAR_BLOCKS.len() as f32) as usize).min(BAR_BLOCKS.len() - 1);
                BAR_BLOCKS[step]
            } else {
                ' '
            };
            let color = if v < 0.55 {
                theme.meter_low
            } else if v < 0.85 {
                theme.meter_mid
            } else {
                theme.meter_high
            };
            spans.push(Span::styled(glyph.to_string(), Style::default().fg(color)));
        }
        lines.push(Line::from(spans));
    }

    f.render_widget(Paragraph::new(lines), inner);
}
