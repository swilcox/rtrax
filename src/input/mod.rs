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
    FocusBrowser,
    CycleFocus,
    CycleTheme,
    ToggleInfo,
    Help,
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
        (&keymap.focus_browser, Action::FocusBrowser),
        (&keymap.cycle_focus, Action::CycleFocus),
        (&keymap.cycle_theme, Action::CycleTheme),
        (&keymap.toggle_info, Action::ToggleInfo),
        (&keymap.help, Action::Help),
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
