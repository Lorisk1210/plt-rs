//! Windows keyboard input as pltrs keys: virtual-key codes, left and right
//! modifier keys, and `AltGr`, which Windows reports as a fake left
//! Control press followed by a right Alt press.

use crate::event::{ButtonState, KeyEvent};
use crate::keyboard::{Key, Modifiers};

/// The virtual-key codes this module names (`VK_*`).
pub(crate) mod vk {
    pub(crate) const BACK: u16 = 0x08;
    pub(crate) const TAB: u16 = 0x09;
    pub(crate) const CLEAR: u16 = 0x0C;
    pub(crate) const RETURN: u16 = 0x0D;
    pub(crate) const SHIFT: u16 = 0x10;
    pub(crate) const CONTROL: u16 = 0x11;
    pub(crate) const MENU: u16 = 0x12;
    pub(crate) const PAUSE: u16 = 0x13;
    pub(crate) const CAPITAL: u16 = 0x14;
    pub(crate) const ESCAPE: u16 = 0x1B;
    pub(crate) const PRIOR: u16 = 0x21;
    pub(crate) const NEXT: u16 = 0x22;
    pub(crate) const END: u16 = 0x23;
    pub(crate) const HOME: u16 = 0x24;
    pub(crate) const LEFT: u16 = 0x25;
    pub(crate) const UP: u16 = 0x26;
    pub(crate) const RIGHT: u16 = 0x27;
    pub(crate) const DOWN: u16 = 0x28;
    pub(crate) const SNAPSHOT: u16 = 0x2C;
    pub(crate) const INSERT: u16 = 0x2D;
    pub(crate) const DELETE: u16 = 0x2E;
    pub(crate) const LWIN: u16 = 0x5B;
    pub(crate) const RWIN: u16 = 0x5C;
    pub(crate) const APPS: u16 = 0x5D;
    pub(crate) const F1: u16 = 0x70;
    pub(crate) const F24: u16 = 0x87;
    pub(crate) const F10: u16 = 0x79;
    pub(crate) const NUMLOCK: u16 = 0x90;
    pub(crate) const SCROLL: u16 = 0x91;
    pub(crate) const LSHIFT: u16 = 0xA0;
    pub(crate) const RSHIFT: u16 = 0xA1;
    pub(crate) const LCONTROL: u16 = 0xA2;
    pub(crate) const RCONTROL: u16 = 0xA3;
    pub(crate) const LMENU: u16 = 0xA4;
    pub(crate) const RMENU: u16 = 0xA5;
}

/// The scan code of the right Shift key, which Windows reports as
/// `VK_SHIFT` like the left one.
const RIGHT_SHIFT_SCANCODE: u16 = 0x36;

/// A key message as Windows reports it (`WM_KEYDOWN`, `WM_SYSKEYUP`, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeyMessage {
    /// The virtual-key code.
    pub(crate) vk: u16,
    /// The scan code.
    pub(crate) scancode: u16,
    /// Whether the key is an extended one (the right Control and Alt
    /// keys, the arrows of the navigation block, keypad Enter, ...).
    pub(crate) extended: bool,
    /// Pressed or released.
    pub(crate) state: ButtonState,
    /// Whether the key was down already (auto-repeat).
    pub(crate) repeat: bool,
    /// The message time, in milliseconds.
    pub(crate) time: u32,
}

impl KeyMessage {
    /// The scan code pltrs reports: extended keys get `0xE0` in the high
    /// byte, as in scan code set 1.
    pub(crate) fn scancode(self) -> u32 {
        u32::from(self.scancode) | if self.extended { 0xE000 } else { 0 }
    }

    /// The modifier key this is, and its side.
    fn modifier(self) -> Option<(Modifiers, Side)> {
        let side = |right: bool| if right { Side::Right } else { Side::Left };
        match self.vk {
            vk::SHIFT => Some((
                Modifiers::SHIFT,
                side(self.scancode == RIGHT_SHIFT_SCANCODE),
            )),
            vk::LSHIFT => Some((Modifiers::SHIFT, Side::Left)),
            vk::RSHIFT => Some((Modifiers::SHIFT, Side::Right)),
            vk::CONTROL => Some((Modifiers::CONTROL, side(self.extended))),
            vk::LCONTROL => Some((Modifiers::CONTROL, Side::Left)),
            vk::RCONTROL => Some((Modifiers::CONTROL, Side::Right)),
            vk::MENU => Some((Modifiers::ALT, side(self.extended))),
            vk::LMENU => Some((Modifiers::ALT, Side::Left)),
            vk::RMENU => Some((Modifiers::ALT, Side::Right)),
            vk::LWIN => Some((Modifiers::SUPER, Side::Left)),
            vk::RWIN => Some((Modifiers::SUPER, Side::Right)),
            _ => None,
        }
    }

    /// Whether this is the left Control key.
    fn is_left_control(self) -> bool {
        self.modifier() == Some((Modifiers::CONTROL, Side::Left))
    }

    /// Whether this is the right Alt key.
    fn is_right_alt(self) -> bool {
        self.modifier() == Some((Modifiers::ALT, Side::Right))
    }
}

/// A side of the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

/// The modifier keys held, left and right (from `GetKeyState`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct HeldModifiers {
    /// The left Shift, Control, Alt and Windows keys held.
    pub(crate) left: Modifiers,
    /// The right Shift, Control, Alt and Windows keys held.
    pub(crate) right: Modifiers,
}

impl HeldModifiers {
    /// The modifier keys held, given whether the key of each virtual-key
    /// code is down.
    pub(crate) fn from_keys(down: impl Fn(u16) -> bool) -> Self {
        let side = |shift, control, alt, windows| {
            [
                (shift, Modifiers::SHIFT),
                (control, Modifiers::CONTROL),
                (alt, Modifiers::ALT),
                (windows, Modifiers::SUPER),
            ]
            .into_iter()
            .fold(Modifiers::NONE, |held, (vk, flag)| {
                held.with(flag, down(vk))
            })
        };
        Self {
            left: side(vk::LSHIFT, vk::LCONTROL, vk::LMENU, vk::LWIN),
            right: side(vk::RSHIFT, vk::RCONTROL, vk::RMENU, vk::RWIN),
        }
    }

    /// The modifiers held. While `AltGr` is, its fake left Control and its
    /// right Alt do not count: `AltGr` types characters, it is no shortcut.
    pub(crate) fn modifiers(self, alt_gr: bool) -> Modifiers {
        if alt_gr {
            self.left.with(Modifiers::CONTROL, false) | self.right.with(Modifiers::ALT, false)
        } else {
            self.left | self.right
        }
    }

    /// The modifier keys held before `key` (pltrs reports the modifiers
    /// of a key event as they were before it).
    #[must_use]
    fn before(self, key: KeyMessage) -> Self {
        let Some((flag, side)) = key.modifier() else {
            return self;
        };
        let was_down = !key.state.is_pressed() || key.repeat;
        match side {
            Side::Left => Self {
                left: self.left.with(flag, was_down),
                ..self
            },
            Side::Right => Self {
                right: self.right.with(flag, was_down),
                ..self
            },
        }
    }
}

/// What a key message is, once `AltGr` is told apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyRole {
    /// The fake left Control press or release that comes with `AltGr`:
    /// not reported.
    FakeControl,
    /// The `AltGr` key itself.
    AltGr,
    /// Any other key.
    Plain,
}

/// Tells `AltGr` from the left Control and right Alt keys.
///
/// Layouts with `AltGr` make Windows send a left Control message right
/// before the right Alt one, with the same time, when it is pressed, and
/// both messages, in either order, when it is released.
#[derive(Debug, Default)]
pub(crate) struct AltGrTracker {
    /// A fake Control press was seen, the right Alt press is due.
    pending: bool,
    /// `AltGr` is held.
    held: bool,
    /// The time `AltGr` was released, if its fake Control release is due.
    released_at: Option<u32>,
}

impl AltGrTracker {
    /// Whether `AltGr` is held.
    pub(crate) fn held(&self) -> bool {
        self.held
    }

    /// Classifies `key`, given the key message queued after it, if any.
    pub(crate) fn classify(&mut self, key: KeyMessage, next: Option<KeyMessage>) -> KeyRole {
        if key.is_left_control() {
            let pairs_with_next = next.is_some_and(|next| {
                next.is_right_alt() && next.time == key.time && next.state == key.state
            });
            if pairs_with_next {
                self.pending = key.state.is_pressed();
                return KeyRole::FakeControl;
            }
            if !key.state.is_pressed() && self.released_at.take() == Some(key.time) {
                return KeyRole::FakeControl;
            }
            return KeyRole::Plain;
        }
        if !key.is_right_alt() {
            return KeyRole::Plain;
        }
        if key.state.is_pressed() {
            if std::mem::take(&mut self.pending) {
                self.held = true;
            }
            if self.held {
                return KeyRole::AltGr;
            }
        } else if std::mem::take(&mut self.held) {
            self.released_at = Some(key.time);
            return KeyRole::AltGr;
        }
        KeyRole::Plain
    }
}

/// The key a virtual key names, if it produces no character.
pub(crate) fn named_key(vk: u16) -> Option<Key> {
    let key = match vk {
        vk::BACK => Key::Backspace,
        vk::TAB => Key::Tab,
        vk::CLEAR => Key::Clear,
        vk::RETURN => Key::Enter,
        vk::SHIFT | vk::LSHIFT | vk::RSHIFT => Key::Shift,
        vk::CONTROL | vk::LCONTROL | vk::RCONTROL => Key::Control,
        vk::MENU | vk::LMENU | vk::RMENU => Key::Alt,
        vk::PAUSE => Key::Pause,
        vk::CAPITAL => Key::CapsLock,
        vk::ESCAPE => Key::Escape,
        vk::PRIOR => Key::PageUp,
        vk::NEXT => Key::PageDown,
        vk::END => Key::End,
        vk::HOME => Key::Home,
        vk::LEFT => Key::Left,
        vk::UP => Key::Up,
        vk::RIGHT => Key::Right,
        vk::DOWN => Key::Down,
        vk::SNAPSHOT => Key::PrintScreen,
        vk::INSERT => Key::Insert,
        vk::DELETE => Key::Delete,
        vk::LWIN | vk::RWIN => Key::Super,
        vk::APPS => Key::Menu,
        vk::F1..=vk::F24 => Key::F((vk - vk::F1 + 1) as u8),
        vk::NUMLOCK => Key::NumLock,
        vk::SCROLL => Key::ScrollLock,
        _ => return None,
    };
    Some(key)
}

/// The key event of `key`.
///
/// * `role`: what [`AltGrTracker::classify`] made of it (not
///   [`KeyRole::FakeControl`], which is not reported);
/// * `alt_gr_held`: whether `AltGr` is held ([`AltGrTracker::held`]);
/// * `held`: the modifier keys held now;
/// * `character`: the character the layout gives the key with Shift and
///   Caps Lock (and `AltGr`) applied, but not Control or Alt;
/// * `text`: the text Windows typed for it (its `WM_CHAR` messages).
pub(crate) fn key_event(
    key: KeyMessage,
    role: KeyRole,
    alt_gr_held: bool,
    held: HeldModifiers,
    character: Option<char>,
    text: Option<String>,
) -> KeyEvent {
    let alt_gr = role == KeyRole::AltGr;
    let modifiers = held.before(key).modifiers(alt_gr || alt_gr_held);
    let logical = if alt_gr {
        Key::AltGr
    } else {
        named_key(key.vk)
            .or_else(|| character.map(Key::Character))
            .unwrap_or(Key::Unidentified(u32::from(key.vk)))
    };
    let shortcut = modifiers.control() || modifiers.alt() || modifiers.super_key();
    KeyEvent {
        key: logical,
        state: key.state,
        repeat: key.repeat,
        text: text.filter(|_| key.state.is_pressed() && !shortcut),
        modifiers,
        scancode: key.scancode(),
    }
}

/// Releases Control and Alt in a keyboard state (`GetKeyboardState`'s 256
/// bytes), so that `ToUnicodeEx` gives a key's character with Shift and
/// Caps Lock applied but not the shortcut modifiers (Control+Shift+A is
/// `A`, not a control character).
pub(crate) fn release_control_and_alt(state: &mut [u8; 256]) {
    for key in [
        vk::CONTROL,
        vk::LCONTROL,
        vk::RCONTROL,
        vk::MENU,
        vk::LMENU,
        vk::RMENU,
    ] {
        state[usize::from(key)] = 0;
    }
}

/// Whether a system key message (Alt held, or F10) must not reach
/// `DefWindowProcW`: Alt and F10 alone would enter the menu mode of a
/// window without menus, swallowing the next key. Others, Alt+F4 and
/// Alt+Space among them, go on to the system's handling.
pub(crate) fn hides_system_key(vk: u16) -> bool {
    matches!(vk, vk::MENU | vk::LMENU | vk::RMENU | vk::F10)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(vk: u16, state: ButtonState, time: u32) -> KeyMessage {
        KeyMessage {
            vk,
            scancode: 0x1E,
            extended: false,
            state,
            repeat: false,
            time,
        }
    }

    fn press(vk: u16) -> KeyMessage {
        message(vk, ButtonState::Pressed, 0)
    }

    fn left_control(state: ButtonState, time: u32) -> KeyMessage {
        KeyMessage {
            scancode: 0x1D,
            ..message(vk::CONTROL, state, time)
        }
    }

    fn right_alt(state: ButtonState, time: u32) -> KeyMessage {
        KeyMessage {
            scancode: 0x38,
            extended: true,
            ..message(vk::MENU, state, time)
        }
    }

    #[test]
    fn virtual_keys_name_keys() {
        assert_eq!(named_key(vk::RETURN), Some(Key::Enter));
        assert_eq!(named_key(vk::PRIOR), Some(Key::PageUp));
        assert_eq!(named_key(vk::F1), Some(Key::F(1)));
        assert_eq!(named_key(vk::F24), Some(Key::F(24)));
        assert_eq!(named_key(vk::RWIN), Some(Key::Super));
        assert_eq!(named_key(vk::APPS), Some(Key::Menu));
        assert_eq!(named_key(u16::from(b'A')), None, "a character key");
        assert_eq!(named_key(0x20), None, "space types a character");
    }

    #[test]
    fn characters_come_from_the_layout_and_text_from_wm_char() {
        let shift = HeldModifiers {
            left: Modifiers::SHIFT,
            right: Modifiers::NONE,
        };
        let event = key_event(
            press(u16::from(b'A')),
            KeyRole::Plain,
            false,
            shift,
            Some('A'),
            Some("A".into()),
        );
        assert_eq!(event.key, Key::Character('A'));
        assert_eq!(event.text.as_deref(), Some("A"));
        assert_eq!(event.modifiers, Modifiers::SHIFT);
        assert_eq!(event.scancode, 0x1E);
        let unknown = key_event(press(0xFF), KeyRole::Plain, false, shift, None, None);
        assert_eq!(unknown.key, Key::Unidentified(0xFF));
    }

    #[test]
    fn shortcuts_and_releases_carry_no_text() {
        let control = HeldModifiers {
            left: Modifiers::CONTROL,
            right: Modifiers::NONE,
        };
        let ctrl_s = key_event(
            press(u16::from(b'S')),
            KeyRole::Plain,
            false,
            control,
            Some('s'),
            Some("\u{13}".into()),
        );
        assert_eq!((ctrl_s.key, ctrl_s.text), (Key::Character('s'), None));
        let release = key_event(
            message(u16::from(b'S'), ButtonState::Released, 0),
            KeyRole::Plain,
            false,
            HeldModifiers::default(),
            Some('s'),
            Some("s".into()),
        );
        assert_eq!(release.text, None);
    }

    #[test]
    fn modifier_keys_report_the_modifiers_before_them() {
        let shift_down = HeldModifiers {
            left: Modifiers::SHIFT,
            right: Modifiers::NONE,
        };
        let pressed = key_event(
            press(vk::SHIFT),
            KeyRole::Plain,
            false,
            shift_down,
            None,
            None,
        );
        assert_eq!(
            (pressed.key, pressed.modifiers),
            (Key::Shift, Modifiers::NONE)
        );
        let released = key_event(
            message(vk::SHIFT, ButtonState::Released, 0),
            KeyRole::Plain,
            false,
            HeldModifiers::default(),
            None,
            None,
        );
        assert_eq!(released.modifiers, Modifiers::SHIFT);
        // The right Shift, while the left one is held, adds nothing new.
        let right = KeyMessage {
            scancode: RIGHT_SHIFT_SCANCODE,
            ..press(vk::SHIFT)
        };
        let both = HeldModifiers {
            left: Modifiers::SHIFT,
            right: Modifiers::SHIFT,
        };
        let event = key_event(right, KeyRole::Plain, false, both, None, None);
        assert_eq!(event.modifiers, Modifiers::SHIFT);
        // Auto-repeat of a held modifier: it was down already.
        let repeat = KeyMessage {
            repeat: true,
            ..press(vk::LWIN)
        };
        let windows = HeldModifiers {
            left: Modifiers::SUPER,
            right: Modifiers::NONE,
        };
        let event = key_event(repeat, KeyRole::Plain, false, windows, None, None);
        assert_eq!((event.key, event.modifiers), (Key::Super, Modifiers::SUPER));
    }

    #[test]
    fn alt_gr_is_told_from_control_and_alt() {
        let mut tracker = AltGrTracker::default();
        let pressed = ButtonState::Pressed;
        let released = ButtonState::Released;
        let fake = left_control(pressed, 10);
        let alt = right_alt(pressed, 10);
        assert_eq!(tracker.classify(fake, Some(alt)), KeyRole::FakeControl);
        assert_eq!(tracker.classify(alt, None), KeyRole::AltGr);
        assert!(tracker.held());
        // Released right Alt first, then the fake Control.
        assert_eq!(
            tracker.classify(right_alt(released, 20), Some(left_control(released, 20))),
            KeyRole::AltGr
        );
        assert_eq!(
            tracker.classify(left_control(released, 20), None),
            KeyRole::FakeControl
        );
        assert!(!tracker.held());
        // A real Control press, and a plain right Alt (no AltGr layout).
        assert_eq!(
            tracker.classify(left_control(pressed, 30), Some(press(u16::from(b'C')))),
            KeyRole::Plain
        );
        assert_eq!(
            tracker.classify(right_alt(pressed, 40), None),
            KeyRole::Plain
        );
        assert_eq!(
            tracker.classify(right_alt(released, 50), None),
            KeyRole::Plain
        );
        assert_eq!(
            tracker.classify(left_control(released, 50), None),
            KeyRole::Plain
        );
    }

    #[test]
    fn alt_gr_released_with_the_fake_control_first() {
        let mut tracker = AltGrTracker::default();
        let pressed = ButtonState::Pressed;
        let released = ButtonState::Released;
        tracker.classify(left_control(pressed, 1), Some(right_alt(pressed, 1)));
        tracker.classify(right_alt(pressed, 1), None);
        assert_eq!(
            tracker.classify(left_control(released, 2), Some(right_alt(released, 2))),
            KeyRole::FakeControl
        );
        assert_eq!(
            tracker.classify(right_alt(released, 2), None),
            KeyRole::AltGr
        );
        assert!(!tracker.held());
    }

    #[test]
    fn alt_gr_types_characters() {
        // AltGr+E on a German layout: Windows holds left Control and
        // right Alt.
        let held = HeldModifiers {
            left: Modifiers::CONTROL,
            right: Modifiers::ALT,
        };
        assert_eq!(held.modifiers(false), Modifiers::CONTROL | Modifiers::ALT);
        assert_eq!(held.modifiers(true), Modifiers::NONE);
        let typed = |alt_gr_held| {
            key_event(
                press(u16::from(b'E')),
                KeyRole::Plain,
                alt_gr_held,
                held,
                Some('€'),
                Some("€".into()),
            )
        };
        let euro = typed(true);
        assert_eq!(euro.key, Key::Character('€'));
        assert_eq!(euro.text.as_deref(), Some("€"));
        assert_eq!(euro.modifiers, Modifiers::NONE);
        assert_eq!(
            typed(false).text,
            None,
            "without AltGr, Control+Alt is a shortcut"
        );
        let alt_gr = key_event(
            right_alt(ButtonState::Pressed, 0),
            KeyRole::AltGr,
            false,
            held,
            None,
            None,
        );
        assert_eq!(alt_gr.key, Key::AltGr);
        assert_eq!(alt_gr.modifiers, Modifiers::NONE);
        assert_eq!(alt_gr.scancode, 0xE038);
    }

    #[test]
    fn held_keys_are_read_by_side() {
        let held = HeldModifiers::from_keys(|key| matches!(key, vk::LSHIFT | vk::RMENU | vk::RWIN));
        assert_eq!(held.left, Modifiers::SHIFT);
        assert_eq!(held.right, Modifiers::ALT | Modifiers::SUPER);
        assert_eq!(
            HeldModifiers::from_keys(|_| false),
            HeldModifiers::default()
        );
    }

    #[test]
    fn layouts_are_asked_without_control_and_alt() {
        let mut state = [0x80; 256];
        release_control_and_alt(&mut state);
        assert_eq!(state[usize::from(vk::LCONTROL)], 0);
        assert_eq!(state[usize::from(vk::MENU)], 0);
        assert_eq!(state[usize::from(vk::SHIFT)], 0x80, "Shift stays");
        assert_eq!(state[usize::from(vk::CAPITAL)], 0x80, "Caps Lock stays");
    }

    #[test]
    fn alt_and_f10_stay_out_of_menu_mode() {
        assert!(hides_system_key(vk::MENU));
        assert!(hides_system_key(vk::F10));
        assert!(!hides_system_key(0x73), "Alt+F4 closes the window");
        assert!(!hides_system_key(0x20), "Alt+Space opens the system menu");
    }
}
