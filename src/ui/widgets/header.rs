//! Header bar: title, format, BPM, pattern position, mm:ss / mm:ss.

use crate::state::SharedState;
use crate::ui::theme::Theme;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use std::sync::atomic::Ordering;

pub fn render(f: &mut Frame, area: Rect, state: &SharedState, theme: &Theme) {
    let title = state.title.lock().map(|s| s.clone()).unwrap_or_default();
    let title = if title.trim().is_empty() {
        state
            .current_path
            .lock()
            .ok()
            .and_then(|p| {
                p.as_ref()
                    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            })
            .unwrap_or_else(|| "no file loaded".into())
    } else {
        title.trim().to_string()
    };

    let format = state
        .format_label
        .lock()
        .map(|s| s.clone())
        .unwrap_or_default();
    let channels = state.num_channels.load(Ordering::Relaxed);
    let tempo = state.current_tempo.load(Ordering::Relaxed);
    let speed = state.current_speed.load(Ordering::Relaxed);
    let pattern = state.current_pattern.load(Ordering::Relaxed);
    let orders = state.num_orders.load(Ordering::Relaxed);
    let order = state.current_order.load(Ordering::Relaxed);
    let pos = state.position_secs();
    let dur = state.duration_secs();
    let playing = state.playing.load(Ordering::Relaxed);
    let eof = state.eof.load(Ordering::Relaxed);

    let status_marker = if eof {
        Span::styled("⏹ end", Style::default().fg(theme.fg_dim))
    } else if playing {
        Span::styled("▶ play", theme.accent_style())
    } else {
        Span::styled("⏸ pause", Style::default().fg(theme.fg_dim))
    };

    let left = Line::from(vec![
        status_marker,
        Span::raw("  "),
        Span::styled(
            title,
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  ·  "),
        Span::styled(format, theme.dim_style()),
    ]);

    let mid = Line::from(vec![
        Span::styled(format!("{channels} ch"), theme.fg_style()),
        Span::raw("  ·  "),
        Span::styled(format!("{tempo} BPM"), theme.fg_style()),
        Span::raw("  ·  "),
        Span::styled(format!("spd {speed}"), theme.dim_style()),
        Span::raw("  ·  "),
        Span::styled(
            format!("ord {}/{}  pat {}", order, orders.max(1), pattern),
            theme.fg_style(),
        ),
    ]);

    let right = Line::from(vec![Span::styled(
        format!("{} / {}", fmt_mmss(pos), fmt_mmss(dur)),
        theme.fg_style(),
    )]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" rtrax ", theme.accent_style()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Split inner into three roughly equal parts; render each line.
    let cols = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Percentage(45),
            ratatui::layout::Constraint::Percentage(35),
            ratatui::layout::Constraint::Percentage(20),
        ])
        .split(inner);

    f.render_widget(Paragraph::new(left).alignment(Alignment::Left), cols[0]);
    f.render_widget(Paragraph::new(mid).alignment(Alignment::Center), cols[1]);
    f.render_widget(Paragraph::new(right).alignment(Alignment::Right), cols[2]);
}

fn fmt_mmss(secs: f64) -> String {
    if !secs.is_finite() || secs < 0.0 {
        return "--:--".into();
    }
    let t = secs as u32;
    format!("{:02}:{:02}", t / 60, t % 60)
}
