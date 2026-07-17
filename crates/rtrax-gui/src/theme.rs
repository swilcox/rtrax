//! Themes. The default is the modern-minimal tracker palette (low-saturation
//! greens/cyans on near-black, magenta accents); the other built-ins are GUI
//! ports of the TUI's palettes. Terminal-only themes (`sixteen`, `mono`) are
//! not ported — they exist for terminal color limits the GUI doesn't have.
//!
//! Custom themes are read from the same `~/.config/rtrax/themes/*.toml` files
//! the TUI uses, same schema and `extends` inheritance, so one file skins
//! both frontends. TUI fields map onto the GUI palette: `fg_dim` → `dim`,
//! `border` → `track`, `meter_low/mid/high` → `fill`/`peak`/`meter_hot`; the
//! panel surface is derived from `bg`. `border_focus` has no GUI equivalent
//! and is ignored.

use anyhow::{bail, Context, Result};
use eframe::egui::Color32;
use serde::Deserialize;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Theme {
    pub name: String,
    // Surfaces.
    pub bg: Color32,
    pub panel: Color32,
    pub track: Color32,
    pub current_row_bg: Color32,
    // Text.
    pub fg: Color32,
    pub dim: Color32,
    /// Markers, notices, drop hints (the TUI's `accent`).
    pub accent: Color32,
    // Meters / progress: the three bar zones (low/mid/high), TUI-style.
    pub fill: Color32,
    pub peak: Color32,
    pub meter_hot: Color32,
    // Pattern tokens.
    pub note: Color32,
    pub instrument: Color32,
    pub volume: Color32,
    pub effect: Color32,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// The current green look — everything the GUI shipped with.
fn default_theme() -> Theme {
    Theme {
        name: "default".into(),
        bg: rgb(0x0e, 0x12, 0x14),
        panel: rgb(0x12, 0x17, 0x1a),
        track: rgb(0x1e, 0x28, 0x2b),
        current_row_bg: rgb(0x1a, 0x26, 0x24),
        fg: rgb(0xcf, 0xe6, 0xd8),
        dim: rgb(0x5a, 0x6e, 0x66),
        accent: rgb(0xc6, 0x78, 0xa8),
        fill: rgb(0x66, 0xd9, 0xa5),
        peak: rgb(0x5a, 0xc8, 0xc8),
        meter_hot: rgb(0xc6, 0x78, 0xa8),
        note: rgb(0xcf, 0xe6, 0xd8),
        instrument: rgb(0x5a, 0xc8, 0xc8),
        volume: rgb(0x66, 0xd9, 0xa5),
        effect: rgb(0xc6, 0x78, 0xa8),
    }
}

fn neon_blue() -> Theme {
    Theme {
        name: "neon-blue".into(),
        bg: rgb(0x04, 0x0c, 0x12),
        panel: rgb(0x07, 0x13, 0x1c),
        track: rgb(0x0e, 0x2a, 0x3c),
        current_row_bg: rgb(0x06, 0x28, 0x3b),
        fg: rgb(0xd8, 0xf7, 0xff),
        dim: rgb(0x5a, 0x8f, 0xaa),
        accent: rgb(0x33, 0xf6, 0xff),
        fill: rgb(0x22, 0xd8, 0xff),
        peak: rgb(0xe2, 0xfb, 0xff),
        meter_hot: rgb(0xe2, 0xfb, 0xff),
        note: rgb(0x8f, 0xef, 0xff),
        instrument: rgb(0x4c, 0xb8, 0xff),
        volume: rgb(0x6c, 0xe7, 0xff),
        effect: rgb(0xb6, 0xf4, 0xff),
    }
}

fn neon_green() -> Theme {
    Theme {
        name: "neon-green".into(),
        bg: rgb(0x04, 0x12, 0x07),
        panel: rgb(0x07, 0x1c, 0x0c),
        track: rgb(0x0e, 0x3c, 0x1a),
        current_row_bg: rgb(0x06, 0x28, 0x10),
        fg: rgb(0xd8, 0xff, 0xdf),
        dim: rgb(0x5a, 0xaa, 0x6a),
        accent: rgb(0x33, 0xff, 0x88),
        fill: rgb(0x22, 0xff, 0x77),
        peak: rgb(0xe2, 0xff, 0xf0),
        meter_hot: rgb(0xe2, 0xff, 0xf0),
        note: rgb(0x8f, 0xff, 0xa0),
        instrument: rgb(0x4c, 0xff, 0xaa),
        volume: rgb(0x6c, 0xff, 0xc0),
        effect: rgb(0xb6, 0xff, 0xe0),
    }
}

fn neon_orange() -> Theme {
    Theme {
        name: "neon-orange".into(),
        bg: rgb(0x12, 0x0a, 0x04),
        panel: rgb(0x1c, 0x10, 0x07),
        track: rgb(0x3c, 0x22, 0x0e),
        current_row_bg: rgb(0x3b, 0x1f, 0x06),
        fg: rgb(0xff, 0xf0, 0xd8),
        dim: rgb(0xaa, 0x88, 0x5a),
        accent: rgb(0xff, 0xaa, 0x33),
        fill: rgb(0xff, 0xa0, 0x22),
        peak: rgb(0xff, 0xf2, 0xe2),
        meter_hot: rgb(0xff, 0x6a, 0x16),
        note: rgb(0xff, 0xd5, 0x8f),
        instrument: rgb(0xff, 0x8c, 0x4c),
        volume: rgb(0xff, 0xc6, 0x6c),
        effect: rgb(0xff, 0xd4, 0xb6),
    }
}

fn c64() -> Theme {
    Theme {
        name: "c64".into(),
        bg: rgb(0x35, 0x28, 0x79),
        panel: rgb(0x3b, 0x2e, 0x85),
        track: rgb(0x4a, 0x3d, 0x99),
        current_row_bg: rgb(0x4a, 0x3d, 0x99),
        fg: rgb(0x9c, 0x8f, 0xe0),
        dim: rgb(0x5e, 0x50, 0xaf),
        accent: rgb(0xff, 0xff, 0xff),
        fill: rgb(0x70, 0xa4, 0xb2),
        peak: rgb(0xff, 0xff, 0xff),
        meter_hot: rgb(0x9a, 0x67, 0x59),
        note: rgb(0x9c, 0x8f, 0xe0),
        instrument: rgb(0x70, 0xa4, 0xb2),
        volume: rgb(0xff, 0xff, 0xff),
        effect: rgb(0x9a, 0x67, 0x59),
    }
}

fn high_contrast() -> Theme {
    Theme {
        name: "high-contrast".into(),
        bg: rgb(0x00, 0x00, 0x00),
        panel: rgb(0x0c, 0x0c, 0x0c),
        track: rgb(0x2a, 0x2a, 0x2a),
        current_row_bg: rgb(0x33, 0x33, 0x33),
        fg: rgb(0xff, 0xff, 0xff),
        dim: rgb(0xa0, 0xa0, 0xa0),
        accent: rgb(0xff, 0x4f, 0xd0),
        fill: rgb(0x00, 0xff, 0x88),
        peak: rgb(0xff, 0xee, 0x00),
        meter_hot: rgb(0xff, 0x5d, 0x5d),
        note: rgb(0x9d, 0xe6, 0xc5),
        instrument: rgb(0x8d, 0xc2, 0xff),
        volume: rgb(0xff, 0xc4, 0x7a),
        effect: rgb(0xff, 0x8a, 0xa9),
    }
}

/// Cycle order for the theme button / `t` key.
pub fn built_ins() -> Vec<Theme> {
    vec![
        default_theme(),
        neon_blue(),
        neon_green(),
        neon_orange(),
        c64(),
        high_contrast(),
    ]
}

/// All themes: built-ins plus custom TOML themes from the shared theme dir.
pub fn available() -> Vec<Theme> {
    let mut themes = built_ins();
    if let Some(dir) = rtrax_core::paths::theme_dir() {
        themes.extend(customs_in_dir(&dir));
    }
    themes
}

/// Loadable custom themes in `dir`, sorted by name. Files whose stem shadows
/// a built-in are skipped; files that fail to parse are logged and skipped.
pub fn customs_in_dir(dir: &Path) -> Vec<Theme> {
    let built_in_names: Vec<String> = built_ins().into_iter().map(|t| t.name).collect();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut customs: Vec<Theme> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "toml") {
                return None;
            }
            let stem = path.file_stem()?.to_str()?.to_owned();
            if built_in_names.contains(&stem) {
                return None;
            }
            match custom_from_dir(dir, &stem) {
                Ok(theme) => Some(theme),
                Err(err) => {
                    tracing::warn!(?err, theme = %stem, "skipping unloadable custom theme");
                    None
                }
            }
        })
        .collect();
    customs.sort_by(|a, b| a.name.cmp(&b.name));
    customs
}

/// Load a custom theme by name from `dir`, resolving `extends` chains.
pub fn custom_from_dir(dir: &Path, name: &str) -> Result<Theme> {
    custom_with_stack(dir, name, &mut Vec::new())
}

fn custom_with_stack(dir: &Path, name: &str, stack: &mut Vec<String>) -> Result<Theme> {
    if stack.iter().any(|seen| seen == name) {
        bail!("theme inheritance cycle involving {name:?}");
    }
    stack.push(name.to_owned());

    let path = dir.join(format!("{name}.toml"));
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("reading theme file {}", path.display()))?;
    let file: ThemeFile =
        toml::from_str(&text).with_context(|| format!("parsing theme file {}", path.display()))?;

    let mut theme = match file.extends.as_deref() {
        Some(parent) => match built_in_by_name(parent) {
            Some(base) => base,
            None if dir.join(format!("{parent}.toml")).exists() => {
                custom_with_stack(dir, parent, stack)?
            }
            None => {
                // Terminal-only bases (sixteen/mono) and unknown names fall
                // back to the default palette rather than failing the theme.
                tracing::warn!(
                    theme = name,
                    extends = parent,
                    "unknown base theme, using default"
                );
                default_theme()
            }
        },
        None => default_theme(),
    };
    file.apply(&mut theme)?;
    theme.name = name.to_owned();
    stack.pop();
    Ok(theme)
}

/// Find a built-in by (normalized) name, accepting the TUI's aliases.
pub fn built_in_by_name(name: &str) -> Option<Theme> {
    let normalized = normalize_name(name);
    built_ins().into_iter().find(|t| t.name == normalized)
}

pub fn normalize_name(name: &str) -> String {
    name.trim().to_ascii_lowercase().replace(['_', ' '], "-")
}

/// The TUI theme-file schema. Unknown fields are tolerated (unlike the TUI)
/// so future additions don't brick older GUI builds.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ThemeFile {
    extends: Option<String>,
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
    fn apply(&self, theme: &mut Theme) -> Result<()> {
        let set = |slot: &mut Color32, value: &Option<String>, field: &str| -> Result<bool> {
            if let Some(raw) = value {
                if let Some(color) =
                    parse_color(raw).with_context(|| format!("invalid color for {field}"))?
                {
                    *slot = color;
                    return Ok(true);
                }
            }
            Ok(false)
        };

        // Text first, then bg, so derived surfaces blend against the final fg.
        set(&mut theme.fg, &self.fg, "fg")?;
        set(&mut theme.dim, &self.fg_dim, "fg_dim")?;
        if set(&mut theme.bg, &self.bg, "bg")? {
            // The TUI schema has no panel/track surfaces — derive them from
            // the new background, overridable by `border`/`current_row_bg`.
            theme.panel = lerp_color(theme.bg, theme.fg, 0.05);
            theme.track = lerp_color(theme.bg, theme.fg, 0.16);
            theme.current_row_bg = lerp_color(theme.bg, theme.fg, 0.10);
        }
        set(&mut theme.track, &self.border, "border")?;
        set(
            &mut theme.current_row_bg,
            &self.current_row_bg,
            "current_row_bg",
        )?;
        set(&mut theme.accent, &self.accent, "accent")?;
        set(&mut theme.note, &self.note, "note")?;
        set(&mut theme.instrument, &self.instrument, "instrument")?;
        set(&mut theme.volume, &self.volume, "volume")?;
        set(&mut theme.effect, &self.effect, "effect")?;
        set(&mut theme.fill, &self.meter_low, "meter_low")?;
        set(&mut theme.peak, &self.meter_mid, "meter_mid")?;
        set(&mut theme.meter_hot, &self.meter_high, "meter_high")?;
        // border_focus has no GUI equivalent; accepted and ignored.
        let _ = &self.border_focus;
        Ok(())
    }
}

/// Parse the TUI color syntax: `#rrggbb`, ANSI color names, or `reset`.
/// `reset` means "keep the base theme's value" and returns `Ok(None)` — in a
/// terminal it defers to the terminal background, which has no GUI analogue.
fn parse_color(value: &str) -> Result<Option<Color32>> {
    let normalized = normalize_name(value);
    let color = match normalized.as_str() {
        "reset" => return Ok(None),
        "black" => rgb(0x00, 0x00, 0x00),
        "red" => rgb(0xcd, 0x00, 0x00),
        "green" => rgb(0x00, 0xcd, 0x00),
        "yellow" => rgb(0xcd, 0xcd, 0x00),
        "blue" => rgb(0x00, 0x00, 0xee),
        "magenta" => rgb(0xcd, 0x00, 0xcd),
        "cyan" => rgb(0x00, 0xcd, 0xcd),
        "gray" | "grey" => rgb(0xe5, 0xe5, 0xe5),
        "dark-gray" | "dark-grey" => rgb(0x7f, 0x7f, 0x7f),
        "light-red" => rgb(0xff, 0x00, 0x00),
        "light-green" => rgb(0x00, 0xff, 0x00),
        "light-yellow" => rgb(0xff, 0xff, 0x00),
        "light-blue" => rgb(0x5c, 0x5c, 0xff),
        "light-magenta" => rgb(0xff, 0x00, 0xff),
        "light-cyan" => rgb(0x00, 0xff, 0xff),
        "white" => rgb(0xff, 0xff, 0xff),
        hex if hex.starts_with('#') && hex.len() == 7 => {
            let r = u8::from_str_radix(&hex[1..3], 16)?;
            let g = u8::from_str_radix(&hex[3..5], 16)?;
            let b = u8::from_str_radix(&hex[5..7], 16)?;
            rgb(r, g, b)
        }
        _ => bail!("expected #rrggbb, reset, or an ANSI color name, got {value:?}"),
    };
    Ok(Some(color))
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let ch = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgb(ch(a.r(), b.r()), ch(a.g(), b.g()), ch(a.b(), b.b()))
}

pub fn fmt_mmss(secs: f64) -> String {
    let total = secs.max(0.0) as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn fmt_mmss_formats_minutes_and_seconds() {
        assert_eq!(fmt_mmss(0.0), "00:00");
        assert_eq!(fmt_mmss(59.9), "00:59");
        assert_eq!(fmt_mmss(61.0), "01:01");
        assert_eq!(fmt_mmss(600.0), "10:00");
        assert_eq!(fmt_mmss(-3.0), "00:00");
    }

    #[test]
    fn lerp_color_endpoints_and_midpoint() {
        let a = Color32::from_rgb(0, 0, 0);
        let b = Color32::from_rgb(200, 100, 50);
        assert_eq!(lerp_color(a, b, 0.0), a);
        assert_eq!(lerp_color(a, b, 1.0), b);
        assert_eq!(lerp_color(a, b, 0.5), Color32::from_rgb(100, 50, 25));
    }

    #[test]
    fn built_in_theme_names_are_unique() {
        let mut names: Vec<String> = built_ins().into_iter().map(|t| t.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), built_ins().len());
    }

    #[test]
    fn parse_color_handles_hex_names_and_reset() {
        assert_eq!(
            parse_color("#ff8800").unwrap(),
            Some(Color32::from_rgb(0xff, 0x88, 0x00))
        );
        assert_eq!(
            parse_color("light_blue").unwrap(),
            Some(Color32::from_rgb(0x5c, 0x5c, 0xff))
        );
        assert_eq!(parse_color("reset").unwrap(), None);
        assert!(parse_color("not-a-color").is_err());
    }

    #[test]
    fn custom_theme_maps_tui_fields_and_derives_surfaces() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("lava.toml"),
            r##"
extends = "neon-orange"
bg = "#200000"
fg = "#ffe0d0"
meter_low = "#ff3300"
border = "#552211"
accent = "light-magenta"
"##,
        )
        .unwrap();

        let theme = custom_from_dir(dir.path(), "lava").unwrap();
        assert_eq!(theme.name, "lava");
        assert_eq!(theme.bg, Color32::from_rgb(0x20, 0x00, 0x00));
        assert_eq!(theme.fill, Color32::from_rgb(0xff, 0x33, 0x00));
        // border maps onto track, overriding the bg-derived value.
        assert_eq!(theme.track, Color32::from_rgb(0x55, 0x22, 0x11));
        assert_eq!(theme.accent, Color32::from_rgb(0xff, 0x00, 0xff));
        // Unset fields inherit from the neon-orange base.
        assert_eq!(theme.note, Color32::from_rgb(0xff, 0xd5, 0x8f));
        // panel derives from the new bg, blended toward fg.
        assert_ne!(theme.panel, neon_orange().panel);
    }

    #[test]
    fn custom_theme_extends_another_custom() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("base.toml"), "bg = \"#101010\"\n").unwrap();
        fs::write(
            dir.path().join("child.toml"),
            "extends = \"base\"\nfg = \"white\"\n",
        )
        .unwrap();

        let theme = custom_from_dir(dir.path(), "child").unwrap();
        assert_eq!(theme.bg, Color32::from_rgb(0x10, 0x10, 0x10));
        assert_eq!(theme.fg, Color32::from_rgb(0xff, 0xff, 0xff));
    }

    #[test]
    fn extends_cycle_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.toml"), "extends = \"b\"\n").unwrap();
        fs::write(dir.path().join("b.toml"), "extends = \"a\"\n").unwrap();

        let err = custom_from_dir(dir.path(), "a").unwrap_err();
        assert!(err.to_string().contains("cycle"), "got: {err}");
    }

    #[test]
    fn customs_in_dir_skips_broken_and_shadowing_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("good.toml"), "bg = \"#111111\"\n").unwrap();
        fs::write(dir.path().join("broken.toml"), "bg = \"nope\"\n").unwrap();
        fs::write(dir.path().join("default.toml"), "bg = \"#222222\"\n").unwrap();
        fs::write(dir.path().join("notes.txt"), "not a theme").unwrap();

        let customs = customs_in_dir(dir.path());
        let names: Vec<&str> = customs.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["good"]);
    }
}
