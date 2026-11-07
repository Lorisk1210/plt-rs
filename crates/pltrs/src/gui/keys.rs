//! Keyboard shortcuts of figure windows: key presses named the way
//! matplotlib names them (`"ctrl+s"`, `"left"`, `"L"`) and matched against
//! the `keymap.*` rcParams, and the mouse buttons stepping through the
//! view history.

use pltrs_platform::{Key, KeyEvent, Modifiers};

use crate::navigation::PointerButton;
use crate::rc::RcParams;

/// What a key press asks a figure window to do (matplotlib's
/// `key_press_handler`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum KeyAction {
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
    /// Copy the figure to the clipboard (`keymap.copy`).
    Copy,
    /// Close the window (`keymap.quit`).
    Quit,
    /// Close every figure window (`keymap.quit_all`).
    QuitAll,
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

/// A step through the view history.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum HistoryStep {
    /// The previous view.
    Back,
    /// The next view.
    Forward,
}

/// The step through the view history a press of `button` asks for: back
/// when `keymap.back` names it, forward when `keymap.forward` does, as the
/// side buttons are by default (matplotlib's `button_press_handler`).
pub(crate) fn history_step(button: PointerButton, rc: &RcParams) -> Option<HistoryStep> {
    let name = button.keymap_name();
    let bound = |keys: &[String]| keys.iter().any(|key| key == name);
    if bound(&rc.keymap_back) {
        Some(HistoryStep::Back)
    } else if bound(&rc.keymap_forward) {
        Some(HistoryStep::Forward)
    } else {
        None
    }
}

/// The Super key's name, as matplotlib's Qt backend gives it: `cmd` on
/// macOS, where it is Command (matplotlib's default keymaps bind `cmd+w`
/// and `cmd+c`), and `meta` elsewhere, where Qt calls it Meta.
pub(crate) const SUPER_NAME: &str = if cfg!(target_os = "macos") {
    "cmd"
} else {
    "meta"
};

/// The modifiers in the order matplotlib's Qt backend names them before
/// a key (`ctrl+alt+shift+meta+a`), with their names: Qt's Control,
/// Alt, Shift and Meta modifiers, which on macOS are Command, Option,
/// Shift and Control.
pub(crate) const MODIFIER_ORDER: [(Modifiers, &str); 4] = if cfg!(target_os = "macos") {
    [
        (Modifiers::SUPER, SUPER_NAME),
        (Modifiers::ALT, "alt"),
        (Modifiers::SHIFT, "shift"),
        (Modifiers::CONTROL, "ctrl"),
    ]
} else {
    [
        (Modifiers::CONTROL, "ctrl"),
        (Modifiers::ALT, "alt"),
        (Modifiers::SHIFT, "shift"),
        (Modifiers::SUPER, SUPER_NAME),
    ]
};

/// The character a key press typed in place of its key's own, if it
/// typed one: the end of a compose sequence (´ then e types é), with no
/// shortcut modifier held.
fn composed_character(event: &KeyEvent) -> Option<char> {
    let Key::Character(own) = event.key else {
        return None;
    };
    let modifiers = event.modifiers;
    if modifiers.control() || modifiers.alt() || modifiers.super_key() {
        return None;
    }
    let mut typed = event.text.as_deref()?.chars();
    match (typed.next(), typed.next()) {
        (Some(c), None) if c != own => Some(c),
        _ => None,
    }
}

/// A character key as matplotlib's Qt backend names it: Qt reports the
/// key's capital, which Shift leaves as it is and anything else (Caps
/// Lock included) lowers. Characters without a one-character capital or
/// small form stay as they are.
fn qt_case(c: char, shift: bool) -> char {
    let mut mapped = if shift {
        c.to_uppercase().collect::<Vec<char>>()
    } else {
        c.to_lowercase().collect()
    };
    match (mapped.pop(), mapped.is_empty()) {
        (Some(single), true) => single,
        _ => c,
    }
}

/// The modifier a modifier key sets while held (none for other keys),
/// which matplotlib leaves out of the key's own name (Shift alone is
/// `shift`, not `shift+shift`).
pub(crate) fn own_modifier(key: Key) -> Modifiers {
    match key {
        Key::Shift => Modifiers::SHIFT,
        Key::Control => Modifiers::CONTROL,
        Key::Alt => Modifiers::ALT,
        Key::Super => Modifiers::SUPER,
        _ => Modifiers::NONE,
    }
}

/// matplotlib's name of a key press, as its Qt backend gives it, or
/// `None` for keys it has no name for (F13 and beyond, Num Lock, Menu,
/// Print Screen, `AltGr`, ...). A key ending a compose sequence is named
/// by the character it typed, so that text boxes take accented letters,
/// and dead keys and the Compose key are named as matplotlib's GTK and Tk
/// backends name them (`dead_acute`, `multi_key`): Qt's canvas composes
/// nothing and reports no dead key.
pub(super) fn key_name(event: &KeyEvent) -> Option<String> {
    if let Some(typed) = composed_character(event) {
        return Some(typed.to_string());
    }
    let mut modifiers = event.modifiers.with(own_modifier(event.key), false);
    let base = match &event.key {
        Key::Character(c) => {
            let shift = modifiers.shift();
            // Shift is in the character.
            modifiers = modifiers.with(Modifiers::SHIFT, false);
            qt_case(*c, shift).to_string()
        }
        key @ (Key::Dead(_) | Key::Compose) => key.name().into_owned(),
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
        Key::F(n @ 1..=12) => format!("f{n}"),
        Key::Shift => "shift".into(),
        Key::Control => "control".into(),
        Key::Alt => "alt".into(),
        Key::Super => SUPER_NAME.into(),
        Key::CapsLock => "caps_lock".into(),
        Key::Pause => "pause".into(),
        Key::Clear => "clear".into(),
        _ => return None,
    };
    let mut name = String::new();
    for (flag, prefix) in MODIFIER_ORDER {
        if modifiers.contains(flag) {
            name.push_str(prefix);
            name.push('+');
        }
    }
    name.push_str(&base);
    Some(name)
}

/// A key name of the `keymap.*` rcParams as the help window shows it
/// (matplotlib's `ToolHelpBase.format_shortcut`): a single character as
/// it is, any other name with `Shift+` spelled out before an upper-case
/// key and every word capitalised.
///
/// ```
/// use pltrs::gui::format_shortcut;
/// assert_eq!(format_shortcut("ctrl+A"), "Ctrl+Shift+A");
/// assert_eq!(format_shortcut("pageup"), "Pageup");
/// assert_eq!(format_shortcut("a"), "a");
/// ```
pub fn format_shortcut(key: &str) -> String {
    if key.chars().nth(1).is_none() {
        return key.to_owned();
    }
    let mut spelled = String::with_capacity(key.len() + 6);
    let mut after_plus = false;
    for c in key.chars() {
        if after_plus && c.is_ascii_uppercase() {
            spelled.push_str("Shift+");
        }
        after_plus = c == '+';
        spelled.push(c);
    }
    title_case(&spelled)
}

/// Python's `str.title`: every run of cased letters starts upper case and
/// goes on lower case.
fn title_case(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_word = false;
    for c in text.chars() {
        let cased = c.is_lowercase() || c.is_uppercase();
        if cased && !in_word {
            out.extend(c.to_uppercase());
        } else if cased {
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
        in_word = cased;
    }
    out
}

/// The action bound to key `name` under `rc`.
pub(crate) fn action(name: &str, rc: &RcParams) -> Option<KeyAction> {
    [
        (&rc.keymap_fullscreen, KeyAction::Fullscreen),
        (&rc.keymap_home, KeyAction::Home),
        (&rc.keymap_back, KeyAction::Back),
        (&rc.keymap_forward, KeyAction::Forward),
        (&rc.keymap_pan, KeyAction::Pan),
        (&rc.keymap_zoom, KeyAction::Zoom),
        (&rc.keymap_save, KeyAction::Save),
        (&rc.keymap_copy, KeyAction::Copy),
        (&rc.keymap_quit, KeyAction::Quit),
        (&rc.keymap_quit_all, KeyAction::QuitAll),
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
    use pltrs_platform::ButtonState;

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
    fn side_buttons_step_through_the_history() {
        let mut rc = RcParams::default();
        assert_eq!(
            history_step(PointerButton::Back, &rc),
            Some(HistoryStep::Back)
        );
        assert_eq!(
            history_step(PointerButton::Forward, &rc),
            Some(HistoryStep::Forward)
        );
        assert_eq!(history_step(PointerButton::Left, &rc), None);
        rc.keymap_back.clear();
        rc.keymap_forward.push("MouseButton.MIDDLE".into());
        assert_eq!(history_step(PointerButton::Back, &rc), None);
        assert_eq!(
            history_step(PointerButton::Middle, &rc),
            Some(HistoryStep::Forward)
        );
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
    fn composed_characters_and_dead_keys_are_named() {
        let typed = |key, text: &str, modifiers| KeyEvent {
            text: Some(text.to_owned()),
            ..press(key, modifiers)
        };
        assert_eq!(
            key_name(&press(Key::Dead('´'), Modifiers::NONE)).as_deref(),
            Some("dead_acute")
        );
        assert_eq!(
            key_name(&press(Key::Compose, Modifiers::NONE)).as_deref(),
            Some("multi_key")
        );
        let e_acute = typed(Key::Character('e'), "é", Modifiers::NONE);
        assert_eq!(key_name(&e_acute).as_deref(), Some("é"));
        let capital = typed(Key::Character('E'), "É", Modifiers::SHIFT);
        assert_eq!(key_name(&capital).as_deref(), Some("É"));
        let both = typed(Key::Character('x'), "´x", Modifiers::NONE);
        assert_eq!(key_name(&both).as_deref(), Some("x"), "two characters");
        let option = typed(Key::Character('l'), "@", Modifiers::ALT);
        assert_eq!(key_name(&option).as_deref(), Some("alt+l"));
        let plain = typed(Key::Character('a'), "a", Modifiers::NONE);
        assert_eq!(key_name(&plain).as_deref(), Some("a"));
    }

    #[test]
    fn command_is_cmd_on_macos() {
        let rc = RcParams::matplotlib();
        let close = key_name(&press(Key::Character('w'), Modifiers::SUPER));
        let command = key_name(&press(Key::Super, Modifiers::NONE));
        if cfg!(target_os = "macos") {
            assert_eq!(close.as_deref(), Some("cmd+w"));
            assert_eq!(command.as_deref(), Some("cmd"));
        } else {
            assert_eq!(close.as_deref(), Some("meta+w"));
            assert_eq!(command.as_deref(), Some("meta"));
        }
        assert_eq!(action("cmd+w", &rc), Some(KeyAction::Quit));
        assert_eq!(action("cmd+c", &rc), Some(KeyAction::Copy));
    }

    #[test]
    fn names_follow_matplotlibs_qt_backend() {
        let name = |key, modifiers| key_name(&press(key, modifiers));
        // A modifier key leaves itself out, on its press and its release.
        assert_eq!(name(Key::Shift, Modifiers::SHIFT).as_deref(), Some("shift"));
        let ctrl_shift = Modifiers::CONTROL | Modifiers::SHIFT;
        assert_eq!(name(Key::Shift, ctrl_shift).as_deref(), Some("ctrl+shift"));
        assert_eq!(
            name(Key::Control, Modifiers::CONTROL).as_deref(),
            Some("control")
        );
        // Caps Lock lowers what Shift does not raise.
        assert_eq!(
            name(Key::Character('A'), Modifiers::NONE).as_deref(),
            Some("a")
        );
        assert_eq!(
            name(Key::Character('a'), Modifiers::SHIFT).as_deref(),
            Some("A")
        );
        assert_eq!(
            name(Key::Character('ß'), Modifiers::SHIFT).as_deref(),
            Some("ß")
        );
        assert_eq!(
            name(Key::Character('!'), Modifiers::SHIFT).as_deref(),
            Some("!")
        );
        // Qt's names, and the keys it has none for.
        assert_eq!(
            name(Key::CapsLock, Modifiers::NONE).as_deref(),
            Some("caps_lock")
        );
        assert_eq!(name(Key::Pause, Modifiers::NONE).as_deref(), Some("pause"));
        assert_eq!(name(Key::F(12), Modifiers::NONE).as_deref(), Some("f12"));
        for key in [
            Key::F(13),
            Key::NumLock,
            Key::Menu,
            Key::PrintScreen,
            Key::AltGr,
        ] {
            assert_eq!(name(key, Modifiers::NONE), None, "{key:?}");
        }
        // Modifiers in Qt's order.
        let all = Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT | Modifiers::SUPER;
        let expected = if cfg!(target_os = "macos") {
            "cmd+alt+shift+ctrl+left"
        } else {
            "ctrl+alt+shift+meta+left"
        };
        assert_eq!(name(Key::Left, all).as_deref(), Some(expected));
    }

    #[test]
    fn degenerate_shortcuts_format_without_panicking() {
        assert_eq!(format_shortcut(""), "");
        assert_eq!(format_shortcut("+A"), "+Shift+A");
        assert_eq!(format_shortcut("ctrl+\u{c9}"), "Ctrl+\u{c9}");
        assert_eq!(format_shortcut("ctrl+"), "Ctrl+");
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
