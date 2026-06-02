//! Keymap matching. Reads crossterm `KeyEvent`s, returns an `Action`.

use crate::config::KeyMap;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    PlayPause,
    Stop,
    Next,
    Prev,
    SeekForward,
    SeekBack,
    VolumeUp,
    VolumeDown,
    ResetGain,
    FocusBrowser,
    CycleFocus,
    CycleTheme,
    CycleProgressBarStyle,
    ToggleInfo,
    CyclePatternStack,
    TogglePatternCompact,
    Help,
    ToggleSongMessage,
    AddToPlaylist,
    Up,
    Down,
    PageUp,
    PageDown,
    Enter,
    Esc,
}

pub fn match_key(keymap: &KeyMap, ev: &KeyEvent) -> Option<Action> {
    // Browser/navigation keys we always wire — independent of config.
    match ev.code {
        KeyCode::Up => return Some(Action::Up),
        KeyCode::Down => return Some(Action::Down),
        KeyCode::PageUp => return Some(Action::PageUp),
        KeyCode::PageDown => return Some(Action::PageDown),
        KeyCode::Enter => return Some(Action::Enter),
        KeyCode::Esc => return Some(Action::Esc),
        _ => {}
    }

    let pairs: &[(&[String], Action)] = &[
        (&keymap.quit, Action::Quit),
        (&keymap.play_pause, Action::PlayPause),
        (&keymap.stop, Action::Stop),
        (&keymap.next, Action::Next),
        (&keymap.prev, Action::Prev),
        (&keymap.seek_forward, Action::SeekForward),
        (&keymap.seek_back, Action::SeekBack),
        (&keymap.volume_up, Action::VolumeUp),
        (&keymap.volume_down, Action::VolumeDown),
        (&keymap.reset_gain, Action::ResetGain),
        (&keymap.focus_browser, Action::FocusBrowser),
        (&keymap.cycle_focus, Action::CycleFocus),
        (&keymap.cycle_theme, Action::CycleTheme),
        (
            &keymap.cycle_progress_bar_style,
            Action::CycleProgressBarStyle,
        ),
        (&keymap.toggle_info, Action::ToggleInfo),
        (&keymap.cycle_pattern_stack, Action::CyclePatternStack),
        (&keymap.toggle_pattern_compact, Action::TogglePatternCompact),
        (&keymap.help, Action::Help),
        (&keymap.toggle_song_message, Action::ToggleSongMessage),
        (&keymap.add_to_playlist, Action::AddToPlaylist),
    ];

    for (binds, action) in pairs {
        for b in *binds {
            if matches_binding(b, ev) {
                return Some(*action);
            }
        }
    }
    None
}

fn matches_binding(binding: &str, ev: &KeyEvent) -> bool {
    // "ctrl+c", "shift+tab", "space", "right", "q", etc.
    let s = binding.trim().to_lowercase();
    if s == "shift+tab" {
        return ev.code == KeyCode::BackTab
            || (ev.code == KeyCode::Tab && ev.modifiers == KeyModifiers::SHIFT);
    }
    let mut mods = KeyModifiers::empty();
    let mut key_part = s.as_str();

    let parts: Vec<&str> = s.split('+').collect();
    if parts.len() > 1 {
        for p in &parts[..parts.len() - 1] {
            match *p {
                "ctrl" => mods |= KeyModifiers::CONTROL,
                "shift" => mods |= KeyModifiers::SHIFT,
                "alt" => mods |= KeyModifiers::ALT,
                _ => return false,
            }
        }
        key_part = parts.last().copied().unwrap_or("");
    }

    // Modifier match. Ignore the SHIFT distinction when the user binds a plain
    // ASCII character — terminals are inconsistent about reporting it.
    let ev_mods = ev.modifiers
        & !(KeyModifiers::SHIFT | KeyModifiers::SUPER | KeyModifiers::HYPER | KeyModifiers::META);
    let want_mods = mods & !KeyModifiers::SHIFT;
    if ev_mods != want_mods {
        return false;
    }

    match (ev.code, key_part) {
        (KeyCode::Char(c), s) if s.chars().count() == 1 => {
            c.to_ascii_lowercase() == s.chars().next().unwrap()
        }
        (KeyCode::Char(' '), "space") => true,
        (KeyCode::Tab, "tab") => true,
        (KeyCode::BackTab, "shift+tab") => true,
        (KeyCode::Left, "left") => true,
        (KeyCode::Right, "right") => true,
        (KeyCode::Up, "up") => true,
        (KeyCode::Down, "down") => true,
        (KeyCode::Enter, "enter") => true,
        (KeyCode::Esc, "esc") => true,
        (KeyCode::Home, "home") => true,
        (KeyCode::End, "end") => true,
        (KeyCode::PageUp, "pageup") => true,
        (KeyCode::PageDown, "pagedown") => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn matches_default_bindings() {
        let keymap = KeyMap::default();

        assert_eq!(
            match_key(&keymap, &key(KeyCode::Char('q'), KeyModifiers::empty())),
            Some(Action::Quit)
        );
        assert_eq!(
            match_key(&keymap, &key(KeyCode::Char(' '), KeyModifiers::empty())),
            Some(Action::PlayPause)
        );
        assert_eq!(
            match_key(&keymap, &key(KeyCode::Right, KeyModifiers::empty())),
            Some(Action::SeekForward)
        );
    }

    #[test]
    fn plain_character_bindings_ignore_shift() {
        let keymap = KeyMap::default();

        assert_eq!(
            match_key(&keymap, &key(KeyCode::Char('Q'), KeyModifiers::SHIFT)),
            Some(Action::Quit)
        );
    }

    #[test]
    fn shift_tab_does_not_match_plain_tab() {
        let keymap = KeyMap {
            cycle_focus: vec!["shift+tab".to_string()],
            ..KeyMap::default()
        };

        assert_eq!(
            match_key(&keymap, &key(KeyCode::Tab, KeyModifiers::empty())),
            None
        );
        assert_eq!(
            match_key(&keymap, &key(KeyCode::BackTab, KeyModifiers::SHIFT)),
            Some(Action::CycleFocus)
        );
    }

    #[test]
    fn add_to_playlist_binding_is_wired() {
        let keymap = KeyMap::default();
        assert_eq!(
            match_key(&keymap, &key(KeyCode::Char('a'), KeyModifiers::empty())),
            Some(Action::AddToPlaylist)
        );
    }

    #[test]
    fn cycle_progress_bar_style_binding_is_wired() {
        let keymap = KeyMap::default();
        assert_eq!(
            match_key(&keymap, &key(KeyCode::Char('b'), KeyModifiers::empty())),
            Some(Action::CycleProgressBarStyle)
        );
    }

    #[test]
    fn reset_gain_binding_is_wired() {
        let keymap = KeyMap::default();
        assert_eq!(
            match_key(&keymap, &key(KeyCode::Char('\\'), KeyModifiers::empty())),
            Some(Action::ResetGain)
        );
    }

    #[test]
    fn toggle_song_message_binding_is_wired() {
        let keymap = KeyMap::default();
        assert_eq!(
            match_key(&keymap, &key(KeyCode::Char('m'), KeyModifiers::empty())),
            Some(Action::ToggleSongMessage)
        );
    }

    #[test]
    fn navigation_keys_are_always_available() {
        let keymap = KeyMap {
            seek_forward: Vec::new(),
            ..KeyMap::default()
        };

        assert_eq!(
            match_key(&keymap, &key(KeyCode::Up, KeyModifiers::empty())),
            Some(Action::Up)
        );
        assert_eq!(
            match_key(&keymap, &key(KeyCode::Right, KeyModifiers::empty())),
            None
        );
    }
}
