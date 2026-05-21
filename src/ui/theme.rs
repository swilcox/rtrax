//! Theme palette. Low-saturation greens/cyans with magenta accents.
//!
//! 16-color terminals get a degraded palette via `Theme::sixteen()`. The
//! current terminal's color capability is detected at startup.

use crate::config::{BuiltInTheme, Config, ThemeChoice};
use anyhow::{bail, Context, Result};
use ratatui::style::{Color, Modifier, Style};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

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
    pub fn for_choice(choice: &ThemeChoice) -> Result<Self> {
        match choice {
            ThemeChoice::BuiltIn(theme) => Ok(Self::built_in(*theme)),
            ThemeChoice::Custom(name) => Self::custom(name),
        }
    }

    pub fn built_in(theme: BuiltInTheme) -> Self {
        match theme {
            BuiltInTheme::Default => Self::default_truecolor(),
            BuiltInTheme::HighContrast => Self::high_contrast(),
            BuiltInTheme::Sixteen => Self::sixteen(),
        }
    }

    pub fn available_choices() -> Vec<ThemeChoice> {
        let mut choices = vec![
            ThemeChoice::BuiltIn(BuiltInTheme::Default),
            ThemeChoice::BuiltIn(BuiltInTheme::HighContrast),
            ThemeChoice::BuiltIn(BuiltInTheme::Sixteen),
        ];

        let Some(theme_dir) = Config::theme_dir() else {
            return choices;
        };

        let Ok(entries) = fs::read_dir(theme_dir) else {
            return choices;
        };

        let mut custom = entries
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let path = entry.path();
                let is_toml = path.extension().is_some_and(|ext| ext == "toml");
                if !is_toml {
                    return None;
                }
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(|stem| ThemeChoice::Custom(stem.to_owned()))
            })
            .collect::<Vec<_>>();
        custom.sort_by(|a, b| a.name().cmp(b.name()));
        choices.extend(custom);
        choices
    }

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

    fn custom(name: &str) -> Result<Self> {
        Self::custom_with_stack(name, &mut Vec::new())
    }

    fn custom_with_stack(name: &str, stack: &mut Vec<String>) -> Result<Self> {
        if stack.iter().any(|seen| seen == name) {
            bail!("theme inheritance cycle involving {name:?}");
        }
        stack.push(name.to_owned());

        let path = custom_theme_path(name)?;
        let text = fs::read_to_string(&path)
            .with_context(|| format!("reading theme file {}", path.display()))?;
        let file: ThemeFile = toml::from_str(&text)
            .with_context(|| format!("parsing theme file {}", path.display()))?;

        let mut theme = match file.extends.as_ref() {
            Some(ThemeChoice::BuiltIn(built_in)) => Self::built_in(*built_in),
            Some(ThemeChoice::Custom(parent)) => Self::custom_with_stack(parent, stack)?,
            None => Self::default_truecolor(),
        };
        file.apply(&mut theme)?;
        stack.pop();
        Ok(theme)
    }
}

fn custom_theme_path(name: &str) -> Result<PathBuf> {
    let theme_dir = Config::theme_dir().context("locating rtrax theme directory")?;
    let file_name = if name.ends_with(".toml") {
        name.to_owned()
    } else {
        format!("{name}.toml")
    };
    Ok(theme_dir.join(file_name))
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ThemeFile {
    extends: Option<ThemeChoice>,
    bg: Option<String>,
    fg: Option<String>,
    fg_dim: Option<String>,
    border: Option<String>,
    border_focus: Option<String>,
    accent: Option<String>,
    note: Option<String>,
    instrument: Option<String>,
    volume: Option<String>,
    effect: Option<String>,
    meter_low: Option<String>,
    meter_mid: Option<String>,
    meter_high: Option<String>,
    current_row_bg: Option<String>,
}

impl ThemeFile {
    fn apply(self, theme: &mut Theme) -> Result<()> {
        apply_color(&mut theme.bg, self.bg, "bg")?;
        apply_color(&mut theme.fg, self.fg, "fg")?;
        apply_color(&mut theme.fg_dim, self.fg_dim, "fg_dim")?;
        apply_color(&mut theme.border, self.border, "border")?;
        apply_color(&mut theme.border_focus, self.border_focus, "border_focus")?;
        apply_color(&mut theme.accent, self.accent, "accent")?;
        apply_color(&mut theme.note, self.note, "note")?;
        apply_color(&mut theme.instrument, self.instrument, "instrument")?;
        apply_color(&mut theme.volume, self.volume, "volume")?;
        apply_color(&mut theme.effect, self.effect, "effect")?;
        apply_color(&mut theme.meter_low, self.meter_low, "meter_low")?;
        apply_color(&mut theme.meter_mid, self.meter_mid, "meter_mid")?;
        apply_color(&mut theme.meter_high, self.meter_high, "meter_high")?;
        apply_color(
            &mut theme.current_row_bg,
            self.current_row_bg,
            "current_row_bg",
        )?;
        Ok(())
    }
}

fn apply_color(slot: &mut Color, value: Option<String>, field: &str) -> Result<()> {
    if let Some(value) = value {
        *slot = parse_color(&value).with_context(|| format!("invalid color for {field}"))?;
    }
    Ok(())
}

fn parse_color(value: &str) -> Result<Color> {
    let value = value.trim();
    let normalized = value.to_ascii_lowercase().replace(['_', ' '], "-");
    let color = match normalized.as_str() {
        "reset" => Color::Reset,
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "dark-gray" | "dark-grey" => Color::DarkGray,
        "light-red" => Color::LightRed,
        "light-green" => Color::LightGreen,
        "light-yellow" => Color::LightYellow,
        "light-blue" => Color::LightBlue,
        "light-magenta" => Color::LightMagenta,
        "light-cyan" => Color::LightCyan,
        "white" => Color::White,
        hex if hex.starts_with('#') && hex.len() == 7 => {
            let r = u8::from_str_radix(&hex[1..3], 16)?;
            let g = u8::from_str_radix(&hex[3..5], 16)?;
            let b = u8::from_str_radix(&hex[5..7], 16)?;
            Color::Rgb(r, g, b)
        }
        _ => bail!("expected #rrggbb, reset, or a ratatui ANSI color name"),
    };
    Ok(color)
}
