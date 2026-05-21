//! Configuration: keybinds, theme selection, default browse path.
//!
//! Loaded from `$XDG_CONFIG_HOME/rtrax/config.toml` if present, otherwise
//! falls back to compiled defaults.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: ThemeChoice,
    pub default_browse_path: Option<PathBuf>,
    pub keymap: KeyMap,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::Default,
            default_browse_path: None,
            keymap: KeyMap::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeChoice {
    Default,
    HighContrast,
    Sixteen,
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
            help: vec!["?".into()],
        }
    }
}

impl Config {
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
        let Some(base) = dirs::config_dir() else {
            return Ok(None);
        };
        let path = base.join("rtrax").join("config.toml");
        if !path.exists() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(&path)?;
        let cfg: Config = toml::from_str(&text)?;
        Ok(Some(cfg))
    }
}
