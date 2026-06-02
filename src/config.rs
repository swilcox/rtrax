//! Configuration: keybinds, theme selection, default browse path.
//!
//! Loaded from `$XDG_CONFIG_HOME/rtrax/config.toml` or
//! `~/.config/rtrax/config.toml` if present, otherwise falls back to compiled
//! defaults.

use anyhow::Result;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: ThemeChoice,
    pub default_browse_path: Option<PathBuf>,
    pub progress_bar_style: ProgressBarStyle,
    /// When true, the pattern view auto-picks its lane count + compact mode from
    /// the channel count each time a new module loads. Manual `w`/`c` overrides
    /// last until the next load. Defaults to on.
    pub auto_layout: bool,
    pub keymap: KeyMap,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::default(),
            default_browse_path: None,
            progress_bar_style: ProgressBarStyle::default(),
            auto_layout: true,
            keymap: KeyMap::default(),
        }
    }
}

/// Visual style for the song progress bar in the header.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ProgressBarStyle {
    /// `[━━━━▲────]` — single marker over an empty track.
    Triangle,
    /// `████▌    ` — solid fill with smooth eighth-block trailing edge.
    #[default]
    Blocks,
    /// `━━━━╸────` — line that swaps from heavy to light at the play head.
    Line,
    /// `▰▰▰▰▱▱▱▱` — discrete pip segments.
    Segments,
}

impl ProgressBarStyle {
    pub fn name(self) -> &'static str {
        match self {
            Self::Triangle => "triangle",
            Self::Blocks => "blocks",
            Self::Line => "line",
            Self::Segments => "segments",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        let normalized = s.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "triangle" | "tri" => Some(Self::Triangle),
            "blocks" | "block" => Some(Self::Blocks),
            "line" => Some(Self::Line),
            "segments" | "segment" | "segmented" => Some(Self::Segments),
            _ => None,
        }
    }

    /// Every variant in cycle order — used by the `b` keybinding.
    pub const ALL: &'static [ProgressBarStyle] =
        &[Self::Triangle, Self::Blocks, Self::Line, Self::Segments];
}

impl Serialize for ProgressBarStyle {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for ProgressBarStyle {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Ok(Self::from_name(&raw).unwrap_or_default())
    }
}

impl std::str::FromStr for ProgressBarStyle {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_name(s).ok_or_else(|| format!("unknown progress bar style: {s}"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltInTheme {
    Default,
    HighContrast,
    Sixteen,
    NeonBlue,
    NeonGreen,
    NeonOrange,
    C64,
    Mono,
}

impl BuiltInTheme {
    pub fn config_name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::HighContrast => "high-contrast",
            Self::Sixteen => "sixteen",
            Self::NeonBlue => "neon-blue",
            Self::NeonGreen => "neon-green",
            Self::NeonOrange => "neon-orange",
            Self::C64 => "c64",
            Self::Mono => "mono",
        }
    }

    /// Every built-in, in cycle order.
    pub const ALL: &'static [BuiltInTheme] = &[
        Self::Default,
        Self::HighContrast,
        Self::Sixteen,
        Self::NeonBlue,
        Self::NeonGreen,
        Self::NeonOrange,
        Self::C64,
        Self::Mono,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeChoice {
    BuiltIn(BuiltInTheme),
    Custom(String),
}

impl Default for ThemeChoice {
    fn default() -> Self {
        Self::BuiltIn(BuiltInTheme::Default)
    }
}

impl ThemeChoice {
    pub fn name(&self) -> &str {
        match self {
            Self::BuiltIn(theme) => theme.config_name(),
            Self::Custom(name) => name,
        }
    }

    pub fn from_name(s: &str) -> Self {
        let normalized = s.trim().to_ascii_lowercase().replace(['_', ' '], "-");
        match normalized.as_str() {
            "default" => Self::BuiltIn(BuiltInTheme::Default),
            "highcontrast" | "high-contrast" => Self::BuiltIn(BuiltInTheme::HighContrast),
            "sixteen" | "16" => Self::BuiltIn(BuiltInTheme::Sixteen),
            "neon-blue" => Self::BuiltIn(BuiltInTheme::NeonBlue),
            "neon-green" => Self::BuiltIn(BuiltInTheme::NeonGreen),
            "neon-orange" => Self::BuiltIn(BuiltInTheme::NeonOrange),
            "c64" | "commodore-64" | "commodore64" => Self::BuiltIn(BuiltInTheme::C64),
            "mono" | "monochrome" => Self::BuiltIn(BuiltInTheme::Mono),
            _ => Self::Custom(s.to_string()),
        }
    }
}

impl std::str::FromStr for ThemeChoice {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_name(s))
    }
}

impl Serialize for ThemeChoice {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for ThemeChoice {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Ok(Self::from_name(&raw))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyMap {
    pub quit: Vec<String>,
    pub play_pause: Vec<String>,
    pub stop: Vec<String>,
    pub next: Vec<String>,
    pub prev: Vec<String>,
    pub seek_forward: Vec<String>,
    pub seek_back: Vec<String>,
    pub volume_up: Vec<String>,
    pub volume_down: Vec<String>,
    pub reset_gain: Vec<String>,
    pub focus_browser: Vec<String>,
    pub cycle_focus: Vec<String>,
    pub cycle_theme: Vec<String>,
    pub toggle_info: Vec<String>,
    pub cycle_pattern_stack: Vec<String>,
    pub toggle_pattern_compact: Vec<String>,
    pub help: Vec<String>,
    pub toggle_song_message: Vec<String>,
    pub add_to_playlist: Vec<String>,
    pub cycle_progress_bar_style: Vec<String>,
}

impl Default for KeyMap {
    fn default() -> Self {
        // Each binding is a list so users can add aliases.
        Self {
            quit: vec!["q".into(), "ctrl+c".into()],
            play_pause: vec!["space".into()],
            stop: vec!["s".into()],
            next: vec!["n".into()],
            prev: vec!["p".into()],
            seek_forward: vec!["right".into()],
            seek_back: vec!["left".into()],
            volume_up: vec!["]".into()],
            volume_down: vec!["[".into()],
            reset_gain: vec!["\\".into()],
            focus_browser: vec!["/".into()],
            cycle_focus: vec!["tab".into()],
            cycle_theme: vec!["t".into()],
            toggle_info: vec!["i".into()],
            cycle_pattern_stack: vec!["w".into()],
            toggle_pattern_compact: vec!["c".into()],
            help: vec!["?".into()],
            toggle_song_message: vec!["m".into()],
            add_to_playlist: vec!["a".into()],
            cycle_progress_bar_style: vec!["b".into()],
        }
    }
}

impl Config {
    pub fn config_dir() -> Option<PathBuf> {
        std::env::var_os("XDG_CONFIG_HOME")
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .or_else(|| dirs::home_dir().map(|home| home.join(".config")))
            .or_else(dirs::config_dir)
            .map(|base| base.join("rtrax"))
    }

    pub fn theme_dir() -> Option<PathBuf> {
        Self::config_dir().map(|dir| dir.join("themes"))
    }

    pub fn load() -> Self {
        match Self::try_load() {
            Ok(Some(cfg)) => cfg,
            Ok(None) => Self::default(),
            Err(err) => {
                tracing::warn!(?err, "failed to load config, using defaults");
                Self::default()
            }
        }
    }

    fn try_load() -> Result<Option<Self>> {
        let Some(base) = Self::config_dir() else {
            return Ok(None);
        };
        let path = base.join("config.toml");
        if !path.exists() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(&path)?;
        let cfg: Config = toml::from_str(&text)?;
        Ok(Some(cfg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ThemeChoice::from_name ───────────────────────────────────────────────

    #[test]
    fn from_name_recognises_all_builtin_names() {
        let cases = [
            ("default", BuiltInTheme::Default),
            ("high-contrast", BuiltInTheme::HighContrast),
            ("sixteen", BuiltInTheme::Sixteen),
            ("neon-blue", BuiltInTheme::NeonBlue),
            ("neon-green", BuiltInTheme::NeonGreen),
            ("neon-orange", BuiltInTheme::NeonOrange),
            ("c64", BuiltInTheme::C64),
            ("mono", BuiltInTheme::Mono),
        ];
        for (name, expected) in cases {
            assert_eq!(
                ThemeChoice::from_name(name),
                ThemeChoice::BuiltIn(expected),
                "failed for {name}"
            );
        }
    }

    #[test]
    fn from_name_recognises_aliases() {
        assert_eq!(
            ThemeChoice::from_name("16"),
            ThemeChoice::BuiltIn(BuiltInTheme::Sixteen)
        );
        assert_eq!(
            ThemeChoice::from_name("commodore-64"),
            ThemeChoice::BuiltIn(BuiltInTheme::C64)
        );
        assert_eq!(
            ThemeChoice::from_name("commodore64"),
            ThemeChoice::BuiltIn(BuiltInTheme::C64)
        );
        assert_eq!(
            ThemeChoice::from_name("monochrome"),
            ThemeChoice::BuiltIn(BuiltInTheme::Mono)
        );
        assert_eq!(
            ThemeChoice::from_name("highcontrast"),
            ThemeChoice::BuiltIn(BuiltInTheme::HighContrast)
        );
    }

    #[test]
    fn from_name_is_case_insensitive() {
        assert_eq!(
            ThemeChoice::from_name("DEFAULT"),
            ThemeChoice::BuiltIn(BuiltInTheme::Default)
        );
        assert_eq!(
            ThemeChoice::from_name("Mono"),
            ThemeChoice::BuiltIn(BuiltInTheme::Mono)
        );
        assert_eq!(
            ThemeChoice::from_name("NEON-BLUE"),
            ThemeChoice::BuiltIn(BuiltInTheme::NeonBlue)
        );
    }

    #[test]
    fn from_name_normalises_underscores_and_spaces() {
        assert_eq!(
            ThemeChoice::from_name("neon_blue"),
            ThemeChoice::BuiltIn(BuiltInTheme::NeonBlue)
        );
        assert_eq!(
            ThemeChoice::from_name("neon blue"),
            ThemeChoice::BuiltIn(BuiltInTheme::NeonBlue)
        );
        assert_eq!(
            ThemeChoice::from_name("high_contrast"),
            ThemeChoice::BuiltIn(BuiltInTheme::HighContrast)
        );
    }

    #[test]
    fn from_name_unknown_becomes_custom() {
        let choice = ThemeChoice::from_name("my-custom-theme");
        assert!(matches!(choice, ThemeChoice::Custom(_)));
        assert_eq!(choice.name(), "my-custom-theme");
    }

    #[test]
    fn theme_choice_name_roundtrips_for_all_builtins() {
        for &builtin in BuiltInTheme::ALL {
            let choice = ThemeChoice::BuiltIn(builtin);
            let roundtripped = ThemeChoice::from_name(choice.name());
            assert_eq!(roundtripped, choice, "roundtrip failed for {:?}", builtin);
        }
    }

    // ── serde ───────────────────────────────────────────────────────────────

    #[test]
    fn theme_choice_serializes_to_name_string() {
        #[derive(serde::Serialize)]
        struct W {
            theme: ThemeChoice,
        }
        let s = toml::to_string(&W {
            theme: ThemeChoice::BuiltIn(BuiltInTheme::NeonBlue),
        })
        .unwrap();
        assert!(s.contains("neon-blue"), "got: {s}");
    }

    #[test]
    fn theme_choice_deserializes_from_name_string() {
        #[derive(serde::Deserialize)]
        struct W {
            theme: ThemeChoice,
        }
        let w: W = toml::from_str("theme = \"c64\"").unwrap();
        assert_eq!(w.theme, ThemeChoice::BuiltIn(BuiltInTheme::C64));
    }

    #[test]
    fn config_default_has_default_theme() {
        let cfg = Config::default();
        assert_eq!(cfg.theme, ThemeChoice::BuiltIn(BuiltInTheme::Default));
    }

    // ── KeyMap ──────────────────────────────────────────────────────────────

    #[test]
    fn keymap_default_contains_expected_bindings() {
        let km = KeyMap::default();
        assert!(km.quit.contains(&"q".to_string()));
        assert!(km.quit.contains(&"ctrl+c".to_string()));
        assert!(km.play_pause.contains(&"space".to_string()));
        assert!(km.add_to_playlist.contains(&"a".to_string()));
        assert!(km.help.contains(&"?".to_string()));
        assert!(km.toggle_song_message.contains(&"m".to_string()));
        assert!(km.cycle_progress_bar_style.contains(&"b".to_string()));
    }

    // ── ProgressBarStyle ────────────────────────────────────────────────────

    #[test]
    fn progress_bar_style_default_is_blocks() {
        assert_eq!(ProgressBarStyle::default(), ProgressBarStyle::Blocks);
    }

    #[test]
    fn progress_bar_style_from_name_recognises_canonical_names() {
        for &style in ProgressBarStyle::ALL {
            assert_eq!(
                ProgressBarStyle::from_name(style.name()),
                Some(style),
                "roundtrip failed for {:?}",
                style
            );
        }
    }

    #[test]
    fn progress_bar_style_from_name_accepts_aliases() {
        assert_eq!(
            ProgressBarStyle::from_name("BLOCK"),
            Some(ProgressBarStyle::Blocks)
        );
        assert_eq!(
            ProgressBarStyle::from_name("tri"),
            Some(ProgressBarStyle::Triangle)
        );
        assert_eq!(
            ProgressBarStyle::from_name("segmented"),
            Some(ProgressBarStyle::Segments)
        );
    }

    #[test]
    fn progress_bar_style_deserializes_from_string_and_falls_back_on_unknown() {
        #[derive(serde::Deserialize)]
        struct W {
            style: ProgressBarStyle,
        }
        let w: W = toml::from_str("style = \"line\"").unwrap();
        assert_eq!(w.style, ProgressBarStyle::Line);

        let w: W = toml::from_str("style = \"made-up\"").unwrap();
        assert_eq!(w.style, ProgressBarStyle::Blocks); // default fallback
    }
}
