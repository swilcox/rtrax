//! Theme palette. Low-saturation greens/cyans with magenta accents.
//!
//! 16-color terminals get a degraded palette via `Theme::sixteen()`. The
//! current terminal's color capability is detected at startup.

use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub fg_dim: Color,
    pub border: Color,
    pub border_focus: Color,
    pub accent: Color,
    pub note: Color,
    pub instrument: Color,
    pub volume: Color,
    pub effect: Color,
    pub meter_low: Color,
    pub meter_mid: Color,
    pub meter_high: Color,
    pub current_row_bg: Color,
}

impl Theme {
    pub fn default_truecolor() -> Self {
        Self {
            bg: Color::Reset,
            fg: Color::Rgb(0xc8, 0xd0, 0xc4),
            fg_dim: Color::Rgb(0x60, 0x6a, 0x66),
            border: Color::Rgb(0x36, 0x44, 0x44),
            border_focus: Color::Rgb(0x7a, 0xc8, 0xb0),
            accent: Color::Rgb(0xff, 0x6f, 0xc0),
            note: Color::Rgb(0x9d, 0xe6, 0xc5),
            instrument: Color::Rgb(0x8d, 0xc2, 0xff),
            volume: Color::Rgb(0xff, 0xc4, 0x7a),
            effect: Color::Rgb(0xff, 0x8a, 0xa9),
            meter_low: Color::Rgb(0x5d, 0xa8, 0x88),
            meter_mid: Color::Rgb(0xff, 0xc4, 0x7a),
            meter_high: Color::Rgb(0xff, 0x5d, 0x5d),
            current_row_bg: Color::Rgb(0x1d, 0x2a, 0x28),
        }
    }

    pub fn high_contrast() -> Self {
        let mut t = Self::default_truecolor();
        t.fg = Color::White;
        t.fg_dim = Color::Gray;
        t.border = Color::White;
        t.border_focus = Color::Yellow;
        t.accent = Color::Magenta;
        t.current_row_bg = Color::DarkGray;
        t
    }

    pub fn sixteen() -> Self {
        Self {
            bg: Color::Reset,
            fg: Color::Gray,
            fg_dim: Color::DarkGray,
            border: Color::DarkGray,
            border_focus: Color::Cyan,
            accent: Color::Magenta,
            note: Color::Green,
            instrument: Color::Cyan,
            volume: Color::Yellow,
            effect: Color::LightMagenta,
            meter_low: Color::Green,
            meter_mid: Color::Yellow,
            meter_high: Color::Red,
            current_row_bg: Color::DarkGray,
        }
    }

    pub fn fg_style(&self) -> Style {
        Style::default().fg(self.fg)
    }

    pub fn dim_style(&self) -> Style {
        Style::default().fg(self.fg_dim)
    }

    pub fn accent_style(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }
}
