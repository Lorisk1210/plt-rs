//! Keyboard shortcuts of figure windows: key presses named the way
//! matplotlib names them (`"ctrl+s"`, `"left"`, `"L"`) and matched against
//! the `keymap.*` rcParams.

use plt_platform::{Key, KeyEvent, Modifiers};

use crate::rc::RcParams;

/// What a key press asks a figure window to do (matplotlib's
/// `key_press_handler`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum KeyAction {
    /// Toggle full screen (`keymap.fullscreen`).
    Fullscreen,
    /// Restore the original view (`keymap.home`).
    Home,
    /// Previous view (`keymap.back`).
    Back,
    /// Next view (`keymap.forward`).
    Forward,
    /// Toggle the pan tool (`keymap.pan`).
    Pan,
    /// Toggle the zoom tool (`keymap.zoom`).
    Zoom,
    /// Save the figure (`keymap.save`).
    Save,
    /// Close the window (`keymap.quit`).
    Quit,
    /// Cycle the major grids of the axes under the pointer
    /// (`keymap.grid`).
    Grid,
    /// Cycle the minor grids of the axes under the pointer
    /// (`keymap.grid_minor`).
    GridMinor,
    /// Toggle a logarithmic y axis (`keymap.yscale`).
    LogY,
    /// Toggle a logarithmic x axis (`keymap.xscale`).
    LogX,
    /// Show the keyboard shortcuts (`keymap.help`).
    Help,
}

/// matplotlib's name of a key press, or `None` for keys it ignores.
pub(super) fn key_name(event: &KeyEvent) -> Option<String> {
    let base = match &event.key {
        Key::Character(c) => c.to_string(),
        Key::Enter => "enter".into(),
        Key::Escape => "escape".into(),
        Key::Backspace => "backspace".into(),
        Key::Tab => "tab".into(),
        Key::Delete => "delete".into(),
        Key::Insert => "insert".into(),
        Key::Home => "home".into(),
        Key::End => "end".into(),
        Key::PageUp => "pageup".into(),
        Key::PageDown => "pagedown".into(),
        Key::Left => "left".into(),
        Key::Right => "right".into(),
        Key::Up => "up".into(),
        Key::Down => "down".into(),
        Key::F(n) => format!("f{n}"),
        Key::Shift => "shift".into(),
        Key::Control => "control".into(),
        Key::Alt => "alt".into(),
        _ => return None,
    };
    let modifiers = event.modifiers;
    let mut name = String::new();
    for (flag, prefix) in [
        (Modifiers::CONTROL, "ctrl+"),
        (Modifiers::ALT, "alt+"),
        (Modifiers::SUPER, "super+"),
    ] {
        if modifiers.contains(flag) {
            name.push_str(prefix);
        }
    }
    // Shift is already applied to characters; other keys spell it out.
    if modifiers.contains(Modifiers::SHIFT) && !matches!(event.key, Key::Character(_)) {
        name.push_str("shift+");
    }
    name.push_str(&base);
    Some(name)
}

/// The action bound to key `name` under `rc`.
pub(super) fn action(name: &str, rc: &RcParams) -> Option<KeyAction> {
    [
        (&rc.keymap_fullscreen, KeyAction::Fullscreen),
        (&rc.keymap_home, KeyAction::Home),
        (&rc.keymap_back, KeyAction::Back),
        (&rc.keymap_forward, KeyAction::Forward),
        (&rc.keymap_pan, KeyAction::Pan),
        (&rc.keymap_zoom, KeyAction::Zoom),
        (&rc.keymap_save, KeyAction::Save),
        (&rc.keymap_quit, KeyAction::Quit),
        (&rc.keymap_grid, KeyAction::Grid),
        (&rc.keymap_grid_minor, KeyAction::GridMinor),
        (&rc.keymap_yscale, KeyAction::LogY),
        (&rc.keymap_xscale, KeyAction::LogX),
        (&rc.keymap_help, KeyAction::Help),
    ]
    .into_iter()
    .find(|(keys, _)| keys.iter().any(|key| key == name))
    .map(|(_, action)| action)
}

#[cfg(test)]
mod tests {
    use plt_platform::ButtonState;

    use super::*;

    fn press(key: Key, modifiers: Modifiers) -> KeyEvent {
        KeyEvent {
            key,
            state: ButtonState::Pressed,
            repeat: false,
            text: None,
            modifiers,
            scancode: 0,
        }
    }

    #[test]
    fn names_follow_matplotlib() {
        assert_eq!(
            key_name(&press(Key::Character('s'), Modifiers::CONTROL)).as_deref(),
            Some("ctrl+s")
        );
        assert_eq!(
            key_name(&press(Key::Character('L'), Modifiers::SHIFT)).as_deref(),
            Some("L")
        );
        assert_eq!(
            key_name(&press(Key::Left, Modifiers::NONE)).as_deref(),
            Some("left")
        );
    }

    #[test]
    fn default_keymap() {
        let rc = RcParams::matplotlib();
        assert_eq!(action("h", &rc), Some(KeyAction::Home));
        assert_eq!(action("backspace", &rc), Some(KeyAction::Back));
        assert_eq!(action("ctrl+s", &rc), Some(KeyAction::Save));
        assert_eq!(action("L", &rc), Some(KeyAction::LogX));
        assert_eq!(action("ctrl+w", &rc), Some(KeyAction::Quit));
        assert_eq!(action("f", &rc), Some(KeyAction::Fullscreen));
        assert_eq!(action("z", &rc), None);
    }
}
