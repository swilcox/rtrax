//! Help overlay.

use crate::ui::theme::Theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    let popup = centered_rect(60, 50, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_focus))
        .title(Span::styled(" help ", theme.accent_style()));

    let line = |k: &'static str, v: &'static str| {
        Line::from(vec![
            Span::styled(
                format!("  {:<12}", k),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(v, Style::default().fg(theme.fg)),
        ])
    };
    let lines = vec![
        line("space", "play / pause"),
        line("s", "stop"),
        line("n / p", "next / previous module in folder"),
        line("← / →", "seek -5s / +5s"),
        line("[ / ]", "volume down / up"),
        line("/", "focus browser"),
        line("tab", "cycle focus between panes"),
        line("t", "cycle theme"),
        line("i", "toggle info panel (samples / metadata)"),
        line("?", "toggle this help"),
        line("q / esc", "quit"),
    ];

    let para = Paragraph::new(lines).block(block);
    f.render_widget(para, popup);
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
