//! Song-message overlay. Toggled with `m`; shows the embedded module message
//! at full size in a centered popup. Scroll with arrow keys / pageup / pagedown.

use crate::ui::theme::Theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

pub fn render(f: &mut Frame, area: Rect, theme: &Theme, message: &str, scroll: u16) {
    let popup = centered_rect(70, 70, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_focus))
        .title(Span::styled(
            " song message — ↑/↓ pgup/pgdn scroll · esc/m close ",
            theme.accent_style(),
        ));

    let lines: Vec<Line> = if message.trim().is_empty() {
        vec![Line::from(Span::styled(
            "(this module has no embedded message)",
            Style::default().fg(theme.fg_dim),
        ))]
    } else {
        message
            .lines()
            .map(|l| Line::from(Span::styled(l.to_string(), Style::default().fg(theme.fg))))
            .collect()
    };

    let para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    f.render_widget(para, popup);
}

/// Maximum useful scroll for a message of `line_count` raw lines. Doesn't
/// account for visual wrapping (a long line can occupy multiple rows), so this
/// is a soft cap — the user may bump past the last visible row when wrapping
/// is heavy, but won't be able to scroll arbitrarily far into empty space.
pub fn max_scroll(line_count: usize) -> u16 {
    line_count.saturating_sub(1).min(u16::MAX as usize) as u16
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(v[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_scroll_empty_is_zero() {
        assert_eq!(max_scroll(0), 0);
    }

    #[test]
    fn max_scroll_single_line_is_zero() {
        assert_eq!(max_scroll(1), 0);
    }

    #[test]
    fn max_scroll_n_lines_is_n_minus_one() {
        assert_eq!(max_scroll(10), 9);
        assert_eq!(max_scroll(100), 99);
    }

    #[test]
    fn max_scroll_saturates_at_u16_max() {
        assert_eq!(max_scroll(usize::MAX), u16::MAX);
    }
}
