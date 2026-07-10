//! Song progress bar — a small horizontal indicator that slots into the
//! header between the order/pattern column and the time column. Four styles
//! are supported; pick one in `config.toml` or cycle at runtime with `b`.
//!
//! All styles take a target `width` in cells and a `fraction` in `[0.0, 1.0]`.
//! They return a ratatui `Line` whose total displayed width is exactly `width`
//! (modulo zero-width inputs, which produce an empty line).

use crate::config::ProgressBarStyle;
use crate::ui::theme::Theme;
use ratatui::style::Style;
use ratatui::text::{Line, Span};

/// Render the bar in the configured style. `fraction` is clamped to `[0,1]`.
pub fn render(
    width: usize,
    fraction: f32,
    style: ProgressBarStyle,
    theme: &Theme,
) -> Line<'static> {
    let frac = if fraction.is_finite() {
        fraction.clamp(0.0, 1.0)
    } else {
        0.0
    };
    match style {
        ProgressBarStyle::Triangle => triangle(width, frac, theme),
        ProgressBarStyle::Blocks => blocks(width, frac, theme),
        ProgressBarStyle::Line => line(width, frac, theme),
        ProgressBarStyle::Segments => segments(width, frac, theme),
    }
}

fn triangle(width: usize, frac: f32, theme: &Theme) -> Line<'static> {
    // [━━━━▲────]  brackets count as part of the width
    if width < 3 {
        return Line::from("");
    }
    let inner = width - 2;
    let pos = ((frac * inner as f32) as usize).min(inner - 1);
    let filled: String = "━".repeat(pos);
    let empty: String = "─".repeat(inner - pos - 1);

    Line::from(vec![
        Span::styled("[", Style::default().fg(theme.fg_dim)),
        Span::styled(filled, Style::default().fg(theme.fg)),
        Span::styled("▲", theme.accent_style()),
        Span::styled(empty, Style::default().fg(theme.fg_dim)),
        Span::styled("]", Style::default().fg(theme.fg_dim)),
    ])
}

fn blocks(width: usize, frac: f32, theme: &Theme) -> Line<'static> {
    // ████▌      smooth fill via the eighth-block characters
    if width == 0 {
        return Line::from("");
    }
    let total_eighths = ((frac * (width as f32) * 8.0).round() as usize).min(width * 8);
    let full = total_eighths / 8;
    let partial = total_eighths % 8;

    let mut filled = "█".repeat(full);
    let mut consumed = full;
    if partial > 0 && full < width {
        filled.push(eighth_partial(partial));
        consumed += 1;
    }
    let empty: String = " ".repeat(width - consumed);

    Line::from(vec![
        Span::styled(filled, Style::default().fg(theme.accent)),
        Span::raw(empty),
    ])
}

fn line(width: usize, frac: f32, theme: &Theme) -> Line<'static> {
    // ━━━━╸────   heavy elapsed, "heavy-left-only" head, light remaining
    if width == 0 {
        return Line::from("");
    }
    let head = ((frac * width as f32) as usize).min(width - 1);
    let before: String = "━".repeat(head);
    let after: String = "─".repeat(width - head - 1);

    Line::from(vec![
        Span::styled(before, Style::default().fg(theme.fg)),
        Span::styled("╸", theme.accent_style()),
        Span::styled(after, Style::default().fg(theme.fg_dim)),
    ])
}

fn segments(width: usize, frac: f32, theme: &Theme) -> Line<'static> {
    // ▰▰▰▰▱▱▱▱
    if width == 0 {
        return Line::from("");
    }
    let filled_count = ((frac * width as f32).round() as usize).min(width);
    let filled: String = "▰".repeat(filled_count);
    let empty: String = "▱".repeat(width - filled_count);

    Line::from(vec![
        Span::styled(filled, Style::default().fg(theme.accent)),
        Span::styled(empty, Style::default().fg(theme.fg_dim)),
    ])
}

fn eighth_partial(eighths: usize) -> char {
    match eighths {
        1 => '▏',
        2 => '▎',
        3 => '▍',
        4 => '▌',
        5 => '▋',
        6 => '▊',
        7 => '▉',
        _ => '█',
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BuiltInTheme;

    fn theme() -> Theme {
        Theme::built_in(BuiltInTheme::Default)
    }

    fn text(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    fn cell_width(line: &Line) -> usize {
        // Every character used by these styles is single-cell wide, so a
        // simple char count is the displayed width. Avoids pulling in a
        // unicode-width dep just for tests.
        text(line).chars().count()
    }

    // ── triangle ────────────────────────────────────────────────────────────

    #[test]
    fn triangle_under_three_wide_is_empty() {
        assert_eq!(text(&triangle(0, 0.5, &theme())), "");
        assert_eq!(text(&triangle(2, 0.5, &theme())), "");
    }

    #[test]
    fn triangle_at_start_places_marker_at_left_inside_brackets() {
        // width 10, inner 8, pos 0
        assert_eq!(text(&triangle(10, 0.0, &theme())), "[▲───────]");
    }

    #[test]
    fn triangle_at_end_places_marker_at_right_inside_brackets() {
        // pos clamped to inner - 1 = 7
        assert_eq!(text(&triangle(10, 1.0, &theme())), "[━━━━━━━▲]");
    }

    #[test]
    fn triangle_midpoint_falls_in_middle() {
        // width 10, inner 8, frac 0.5 → pos = 4 → 4 filled before marker
        assert_eq!(text(&triangle(10, 0.5, &theme())), "[━━━━▲───]");
    }

    #[test]
    fn triangle_total_width_matches_requested() {
        for w in 3..=24 {
            for f in [0.0_f32, 0.25, 0.5, 0.75, 1.0] {
                assert_eq!(cell_width(&triangle(w, f, &theme())), w, "w={w} f={f}");
            }
        }
    }

    // ── blocks ──────────────────────────────────────────────────────────────

    #[test]
    fn blocks_empty_width_is_empty() {
        assert_eq!(text(&blocks(0, 0.5, &theme())), "");
    }

    #[test]
    fn blocks_at_zero_is_all_space() {
        assert_eq!(text(&blocks(8, 0.0, &theme())), "        ");
    }

    #[test]
    fn blocks_at_one_is_all_full() {
        assert_eq!(text(&blocks(8, 1.0, &theme())), "████████");
    }

    #[test]
    fn blocks_half_is_half_full() {
        assert_eq!(text(&blocks(8, 0.5, &theme())), "████    ");
    }

    #[test]
    fn blocks_uses_eighth_partial_for_subcell() {
        // 8 cells × 8 eighths = 64 total. frac 1/16 = 4 eighths = ▌ at cell 0.
        assert_eq!(text(&blocks(8, 1.0 / 16.0, &theme())), "▌       ");
    }

    #[test]
    fn blocks_total_width_matches_requested() {
        for w in 1..=24 {
            for f in [0.0_f32, 0.1, 0.5, 0.9, 1.0] {
                assert_eq!(cell_width(&blocks(w, f, &theme())), w, "w={w} f={f}");
            }
        }
    }

    // ── line ────────────────────────────────────────────────────────────────

    #[test]
    fn line_empty_width_is_empty() {
        assert_eq!(text(&line(0, 0.5, &theme())), "");
    }

    #[test]
    fn line_at_start_head_at_left() {
        assert_eq!(text(&line(10, 0.0, &theme())), "╸─────────");
    }

    #[test]
    fn line_at_end_head_at_right() {
        // head clamped to width - 1
        assert_eq!(text(&line(10, 1.0, &theme())), "━━━━━━━━━╸");
    }

    #[test]
    fn line_total_width_matches_requested() {
        for w in 1..=24 {
            for f in [0.0_f32, 0.25, 0.5, 0.75, 1.0] {
                assert_eq!(cell_width(&line(w, f, &theme())), w, "w={w} f={f}");
            }
        }
    }

    // ── segments ────────────────────────────────────────────────────────────

    #[test]
    fn segments_empty_width_is_empty() {
        assert_eq!(text(&segments(0, 0.5, &theme())), "");
    }

    #[test]
    fn segments_at_zero_is_all_empty_pips() {
        assert_eq!(text(&segments(8, 0.0, &theme())), "▱▱▱▱▱▱▱▱");
    }

    #[test]
    fn segments_at_one_is_all_filled_pips() {
        assert_eq!(text(&segments(8, 1.0, &theme())), "▰▰▰▰▰▰▰▰");
    }

    #[test]
    fn segments_half_is_half_filled() {
        assert_eq!(text(&segments(8, 0.5, &theme())), "▰▰▰▰▱▱▱▱");
    }

    #[test]
    fn segments_total_width_matches_requested() {
        for w in 1..=24 {
            for f in [0.0_f32, 0.25, 0.5, 0.75, 1.0] {
                assert_eq!(cell_width(&segments(w, f, &theme())), w, "w={w} f={f}");
            }
        }
    }

    // ── nan / inf safety ────────────────────────────────────────────────────

    #[test]
    fn render_treats_nan_as_zero() {
        let l = render(8, f32::NAN, ProgressBarStyle::Segments, &theme());
        assert_eq!(text(&l), "▱▱▱▱▱▱▱▱");
    }

    #[test]
    fn render_clamps_over_one() {
        let l = render(8, 5.0, ProgressBarStyle::Segments, &theme());
        assert_eq!(text(&l), "▰▰▰▰▰▰▰▰");
    }
}
