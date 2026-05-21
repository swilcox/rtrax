//! Info side panel — toggled on with `i` to replace the per-channel meters.
//!
//! Top half: a "now playing" list per channel, with the most-recently-seen
//! instrument number + name. Names come from libopenmpt's instrument list
//! first, then fall back to the sample list (MOD/S3M files often have only
//! samples; XM/IT have both).
//!
//! Bottom half: module-wide metadata — format, channel/sample/instrument
//! counts, duration, tracker, artist, and the song message if any.

use crate::state::SharedState;
use crate::ui::theme::Theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use std::sync::atomic::Ordering;

pub fn render(f: &mut Frame, area: Rect, state: &SharedState, theme: &Theme) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" info ", theme.dim_style()));
    let inner = outer.inner(area);
    f.render_widget(outer, area);

    if inner.width < 6 || inner.height < 3 {
        return;
    }

    // Resolve lookup tables once up front.
    let sample_names = state
        .sample_names
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();
    let instrument_names = state
        .instrument_names
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();

    // 60% to the live channel list, 40% to module metadata.
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(inner);

    render_channels(f, split[0], state, &instrument_names, &sample_names, theme);
    render_meta(f, split[1], state, &instrument_names, &sample_names, theme);
}

fn render_channels(
    f: &mut Frame,
    area: Rect,
    state: &SharedState,
    instrument_names: &[String],
    sample_names: &[String],
    theme: &Theme,
) {
    let header = Span::styled(" channels ", theme.dim_style());
    let block = Block::default().borders(Borders::TOP).title(header);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let n = state.num_channels.load(Ordering::Relaxed).max(0) as usize;
    if n == 0 || inner.height == 0 {
        return;
    }

    let width = inner.width as usize;
    let height = inner.height as usize;
    let rows_visible = height.min(n);

    let mut lines: Vec<Line> = Vec::with_capacity(rows_visible);
    for ch in 0..rows_visible {
        let inst = state.last_instrument(ch);
        // 0 means "no instrument event on this channel yet" — show as idle.
        let mut spans: Vec<Span> = Vec::with_capacity(5);
        spans.push(Span::styled(format!("{:>2} ", ch + 1), theme.dim_style()));

        if inst <= 0 {
            spans.push(Span::styled("·  ", theme.dim_style()));
            spans.push(Span::styled("(idle)", Style::default().fg(theme.fg_dim)));
        } else {
            spans.push(Span::styled(
                "▶ ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                format!("{inst:02X} "),
                Style::default().fg(theme.instrument),
            ));
            let name = resolve_name(inst, instrument_names, sample_names);
            // "NN " (3) + "▶ " (2) + "NN " (3) = 8 cols already used.
            let avail = width.saturating_sub(8);
            spans.push(Span::styled(
                truncate(&name, avail),
                Style::default().fg(theme.fg),
            ));
        }
        lines.push(Line::from(spans));
    }

    if n > rows_visible {
        // Last row turns into a "+N more" hint instead of getting cut off.
        let last = lines.len().saturating_sub(1);
        if let Some(l) = lines.get_mut(last) {
            *l = Line::from(Span::styled(
                format!("  … +{} more channels", n - rows_visible + 1),
                Style::default().fg(theme.fg_dim),
            ));
        }
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn render_meta(
    f: &mut Frame,
    area: Rect,
    state: &SharedState,
    instrument_names: &[String],
    sample_names: &[String],
    theme: &Theme,
) {
    let block = Block::default()
        .borders(Borders::TOP)
        .title(Span::styled(" module ", theme.dim_style()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height == 0 {
        return;
    }

    let format_label = state
        .format_label
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();
    let artist = state.artist.lock().map(|g| g.clone()).unwrap_or_default();
    let tracker = state.tracker.lock().map(|g| g.clone()).unwrap_or_default();
    let message = state
        .song_message
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();

    let n_ch = state.num_channels.load(Ordering::Relaxed);
    let n_samp = sample_names.len();
    let n_inst = instrument_names.len();
    let dur = state.duration_secs();

    let kv = |k: &'static str, v: String| {
        Line::from(vec![
            Span::styled(format!("{k:>10}: "), theme.dim_style()),
            Span::styled(v, Style::default().fg(theme.fg)),
        ])
    };

    let mut lines: Vec<Line> = Vec::new();
    if !format_label.is_empty() {
        lines.push(kv("format", format_label));
    }
    lines.push(kv("channels", n_ch.to_string()));
    lines.push(kv(
        "samples",
        if n_inst > 0 {
            format!("{n_samp} ({n_inst} inst)")
        } else {
            n_samp.to_string()
        },
    ));
    lines.push(kv("duration", format_duration(dur)));
    if !artist.is_empty() {
        lines.push(kv("artist", artist));
    }
    if !tracker.is_empty() {
        lines.push(kv("tracker", tracker));
    }

    let mut used = lines.len();
    let available = inner.height as usize;

    if !message.is_empty() && used + 2 <= available {
        lines.push(Line::from(Span::styled("  message:", theme.dim_style())));
        used += 1;
        let remaining = available - used;
        // Take only the first `remaining` lines worth; ratatui will wrap.
        let trimmed: String = message
            .lines()
            .take(remaining.max(1))
            .collect::<Vec<_>>()
            .join("\n");
        lines.push(Line::from(Span::styled(
            trimmed,
            Style::default().fg(theme.fg_dim),
        )));
    }

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn resolve_name(instrument_1based: i32, instruments: &[String], samples: &[String]) -> String {
    let idx = (instrument_1based - 1).max(0) as usize;
    if let Some(name) = instruments.get(idx) {
        if !name.trim().is_empty() {
            return name.clone();
        }
    }
    if let Some(name) = samples.get(idx) {
        if !name.trim().is_empty() {
            return name.clone();
        }
    }
    "—".to_string()
}

fn truncate(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let count = s.chars().count();
    if count <= max {
        s.to_string()
    } else if max <= 1 {
        "…".to_string()
    } else {
        let take = max - 1;
        let mut out: String = s.chars().take(take).collect();
        out.push('…');
        out
    }
}

fn format_duration(secs: f64) -> String {
    if !secs.is_finite() || secs < 0.0 {
        return "—".to_string();
    }
    let total = secs.round() as u64;
    let m = total / 60;
    let s = total % 60;
    format!("{m}:{s:02}")
}
