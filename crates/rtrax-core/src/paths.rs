//! Shared on-disk locations. Both frontends read config and themes from the
//! same place, so a custom theme file works in the TUI and the GUI alike.

use std::path::PathBuf;

/// `$XDG_CONFIG_HOME/rtrax`, falling back to `~/.config/rtrax`, then the
/// platform config dir.
pub fn config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".config")))
        .or_else(dirs::config_dir)
        .map(|base| base.join("rtrax"))
}

pub fn theme_dir() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("themes"))
}
