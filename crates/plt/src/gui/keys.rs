//! Keyboard shortcuts of figure windows: key presses named the way
//! matplotlib names them (`"ctrl+s"`, `"left"`, `"L"`) and matched against
//! the `keymap.*` rcParams, and the mouse buttons stepping through the
//! view history.

use plt_platform::{Key, KeyEvent, Modifiers};

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

/// A key name of the `keymap.*` rcParams as the help window shows it
/// (matplotlib's `ToolHelpBase.format_shortcut`): a single character as
/// it is, any other name with `Shift+` spelled out before an upper-case
/// key and every word capitalised.
///
/// ```
/// use plt::gui::format_shortcut;
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
