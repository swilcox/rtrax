//! Configuration: keybinds, theme selection, default browse path.
//!
//! Loaded from `$XDG_CONFIG_HOME/rtrax/config.toml` or
//! `~/.config/rtrax/config.toml` if present, otherwise falls back to compiled
//! defaults.

use anyhow::Result;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: ThemeChoice,
    pub default_browse_path: Option<PathBuf>,
    pub keymap: KeyMap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltInTheme {
    Default,
    HighContrast,
    Sixteen,
}

impl BuiltInTheme {
    pub fn config_name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::HighContrast => "high-contrast",
            Self::Sixteen => "sixteen",
        }
    }
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
        let normalized = raw.trim().to_ascii_lowercase().replace(['_', ' '], "-");
        Ok(match normalized.as_str() {
            "default" => Self::BuiltIn(BuiltInTheme::Default),
            "highcontrast" | "high-contrast" => Self::BuiltIn(BuiltInTheme::HighContrast),
            "sixteen" | "16" => Self::BuiltIn(BuiltInTheme::Sixteen),
            _ => Self::Custom(raw),
        })
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
    pub focus_browser: Vec<String>,
    pub cycle_focus: Vec<String>,
    pub cycle_theme: Vec<String>,
    pub toggle_info: Vec<String>,
    pub cycle_pattern_stack: Vec<String>,
    pub toggle_pattern_compact: Vec<String>,
    pub help: Vec<String>,
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
            focus_browser: vec!["/".into()],
            cycle_focus: vec!["tab".into()],
            cycle_theme: vec!["t".into()],
            toggle_info: vec!["i".into()],
            cycle_pattern_stack: vec!["w".into()],
            toggle_pattern_compact: vec!["c".into()],
            help: vec!["?".into()],
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
