//! macOS keyboard input as pltrs keys: hardware key codes (`kVK_*`),
//! the characters `NSEvent` gives, and modifier flags, including the
//! `flagsChanged:` events that report modifier keys.

use crate::event::{ButtonState, KeyEvent};
use crate::keyboard::{Key, Modifiers};

/// `NSEventModifierFlagCapsLock`.
const CAPS_LOCK: u64 = 1 << 16;
/// `NSEventModifierFlagShift`.
const SHIFT: u64 = 1 << 17;
/// `NSEventModifierFlagControl`.
const CONTROL: u64 = 1 << 18;
/// `NSEventModifierFlagOption`.
const OPTION: u64 = 1 << 19;
/// `NSEventModifierFlagCommand`.
const COMMAND: u64 = 1 << 20;

/// The device-dependent bits of the modifier flags that tell left and
/// right modifier keys apart (`NX_DEVICE*KEYMASK` of `IOKit`'s `IOLLEvent.h`).
const DEVICE_KEYS: u64 = 0x2000 | 0x7F;

/// The modifier keys: key code, the key, its flag, and its
/// device-dependent bit and that of its twin on the other side.
const MODIFIER_KEYS: [(u16, Key, u64, u64, u64); 8] = [
    (0x38, Key::Shift, SHIFT, 0x0002, 0x0004),
    (0x3C, Key::Shift, SHIFT, 0x0004, 0x0002),
    (0x3B, Key::Control, CONTROL, 0x0001, 0x2000),
    (0x3E, Key::Control, CONTROL, 0x2000, 0x0001),
    (0x3A, Key::Alt, OPTION, 0x0020, 0x0040),
    (0x3D, Key::Alt, OPTION, 0x0040, 0x0020),
    (0x37, Key::Super, COMMAND, 0x0008, 0x0010),
    (0x36, Key::Super, COMMAND, 0x0010, 0x0008),
];

/// The key code of Caps Lock (`kVK_CapsLock`).
const CAPS_LOCK_KEY: u16 = 0x39;

/// What a keyboard event reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyKind {
    /// A key press or release (`keyDown:`, `keyUp:`).
    Press(ButtonState),
    /// A modifier key went down or up (`flagsChanged:`).
    FlagsChanged,
}

/// What the keyboard event of `NSEventType` `kind` reports.
pub(crate) const fn key_kind(kind: u64) -> Option<KeyKind> {
    match kind {
        10 => Some(KeyKind::Press(ButtonState::Pressed)),
        11 => Some(KeyKind::Press(ButtonState::Released)),
        12 => Some(KeyKind::FlagsChanged),
        _ => None,
    }
}

/// The modifiers in `NSEvent` modifier flags: Option is Alt and Command
/// is Super, as matplotlib's macOS backend has them (it names them `alt`
/// and `cmd`).
pub(crate) fn modifiers(flags: u64) -> Modifiers {
    [
        (SHIFT, Modifiers::SHIFT),
        (CONTROL, Modifiers::CONTROL),
        (OPTION, Modifiers::ALT),
        (COMMAND, Modifiers::SUPER),
    ]
    .into_iter()
    .fold(Modifiers::NONE, |held, (flag, modifier)| {
        held.with(modifier, flags & flag != 0)
    })
}

/// The key a hardware key code names, if it produces no character.
fn named_key(code: u16) -> Option<Key> {
    let key = match code {
        0x24 | 0x4C => Key::Enter,
        0x30 => Key::Tab,
        0x33 => Key::Backspace,
        0x35 => Key::Escape,
        0x36 | 0x37 => Key::Super,
        0x38 | 0x3C => Key::Shift,
        0x39 => Key::CapsLock,
        0x3A | 0x3D => Key::Alt,
        0x3B | 0x3E => Key::Control,
        0x47 => Key::Clear,
        0x6E => Key::Menu,
        0x72 => Key::Insert,
        0x73 => Key::Home,
        0x74 => Key::PageUp,
        0x75 => Key::Delete,
        0x77 => Key::End,
        0x79 => Key::PageDown,
        0x7B => Key::Left,
        0x7C => Key::Right,
        0x7D => Key::Down,
        0x7E => Key::Up,
        code => return function_key(code).map(Key::F),
    };
    Some(key)
}

/// The number of the function key with this key code.
const fn function_key(code: u16) -> Option<u8> {
    let number = match code {
        0x7A => 1,
        0x78 => 2,
        0x63 => 3,
        0x76 => 4,
        0x60 => 5,
        0x61 => 6,
        0x62 => 7,
        0x64 => 8,
        0x65 => 9,
        0x6D => 10,
        0x67 => 11,
        0x6F => 12,
        0x69 => 13,
        0x6B => 14,
        0x71 => 15,
        0x6A => 16,
        0x40 => 17,
        0x4F => 18,
        0x50 => 19,
        0x5A => 20,
        _ => return None,
    };
    Some(number)
}

/// Whether `c` is a character a key types: not a control character, nor
/// one of the private-use characters `AppKit` gives function keys
/// (`NSUpArrowFunctionKey`, ...).
fn is_typed(c: char) -> bool {
    !c.is_control() && !('\u{F700}'..='\u{F8FF}').contains(&c)
}

/// A key press or release (`keyDown:`, `keyUp:`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeyPress<'a> {
    /// The hardware key code (`keyCode`).
    pub(crate) code: u16,
    /// Pressed or released.
    pub(crate) state: ButtonState,
    /// Whether this is an auto-repeat (`isARepeat`).
    pub(crate) repeat: bool,
    /// The modifier flags (`modifierFlags`).
    pub(crate) flags: u64,
    /// The characters with no modifier but Shift applied
    /// (`charactersIgnoringModifiers`).
    pub(crate) unmodified: &'a str,
    /// The characters typed (`characters`).
    pub(crate) typed: &'a str,
}

/// The event of a key press or release.
pub(crate) fn key_event(press: KeyPress<'_>) -> KeyEvent {
    let modifiers = modifiers(press.flags);
    let mut unmodified = press.unmodified.chars();
    let character = match (unmodified.next(), unmodified.next()) {
        (Some(c), None) if is_typed(c) => Some(c),
        _ => None,
    };
    let key = named_key(press.code)
        .or_else(|| character.map(Key::Character))
        .unwrap_or(Key::Unidentified(u32::from(press.code)));
    // Control and Command make shortcuts, which type nothing. Option
    // types the layout's other characters (`@` on German layouts, `å` on
    // American ones), as AltGr does on other systems.
    let shortcut = modifiers.control() || modifiers.super_key();
    let text = (press.state.is_pressed() && !shortcut && !press.typed.is_empty())
        .then_some(press.typed)
        .filter(|typed| typed.chars().all(is_typed))
        .map(str::to_owned);
    KeyEvent {
        key,
        state: press.state,
        repeat: press.repeat,
        text,
        modifiers,
        scancode: u32::from(press.code),
    }
}

/// The key event of a `flagsChanged:` event: a modifier key (or Caps
/// Lock) went down or up. `None` for keys pltrs does not report (`fn`).
///
/// The flags are those after the change; the event's modifiers are those
/// before it, as pltrs reports them.
pub(crate) fn flags_changed(code: u16, flags: u64) -> Option<KeyEvent> {
    let after = modifiers(flags);
    let event = |key, state, modifiers| KeyEvent {
        key,
        state,
        repeat: false,
        text: None,
        modifiers,
        scancode: u32::from(code),
    };
    if code == CAPS_LOCK_KEY {
        let state = if flags & CAPS_LOCK == 0 {
            ButtonState::Released
        } else {
            ButtonState::Pressed
        };
        return Some(event(Key::CapsLock, state, after));
    }
    let &(_, key, flag, own, twin) = MODIFIER_KEYS.iter().find(|entry| entry.0 == code)?;
    // Some keyboards (and remote desktops) send no device-dependent bits:
    // then the key is down when its flag is set.
    let down = if flags & DEVICE_KEYS == 0 {
        flags & flag != 0
    } else {
        flags & own != 0
    };
    let modifier = modifiers(flag);
    if down {
        let twin_held = flags & twin != 0;
        Some(event(
            key,
            ButtonState::Pressed,
            after.with(modifier, twin_held),
        ))
    } else {
        Some(event(key, ButtonState::Released, after | modifier))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press<'a>(code: u16, flags: u64, unmodified: &'a str, typed: &'a str) -> KeyEvent {
        key_event(KeyPress {
            code,
            state: ButtonState::Pressed,
            repeat: false,
            flags,
            unmodified,
            typed,
        })
    }

    #[test]
    fn event_types_tell_presses_from_modifier_changes() {
        assert_eq!(key_kind(10), Some(KeyKind::Press(ButtonState::Pressed)));
        assert_eq!(key_kind(11), Some(KeyKind::Press(ButtonState::Released)));
        assert_eq!(key_kind(12), Some(KeyKind::FlagsChanged));
        assert_eq!(key_kind(1), None, "a mouse event");
    }

    #[test]
    fn flags_map_to_modifiers() {
        assert_eq!(modifiers(0), Modifiers::NONE);
        assert_eq!(
            modifiers(SHIFT | COMMAND | CAPS_LOCK | 0x0102),
            Modifiers::SHIFT | Modifiers::SUPER
        );
        assert_eq!(
            modifiers(CONTROL | OPTION),
            Modifiers::CONTROL | Modifiers::ALT
        );
    }

    #[test]
    fn key_codes_name_keys() {
        let cases = [
            (0x24, Key::Enter),
            (0x4C, Key::Enter),
            (0x33, Key::Backspace),
            (0x75, Key::Delete),
            (0x72, Key::Insert),
            (0x74, Key::PageUp),
            (0x7E, Key::Up),
            (0x7A, Key::F(1)),
            (0x6F, Key::F(12)),
            (0x5A, Key::F(20)),
            (0x36, Key::Super),
        ];
        for (code, key) in cases {
            assert_eq!(press(code, 0, "\u{F700}", "").key, key, "{code:#x}");
        }
        assert_eq!(named_key(0x00), None, "the A key types a character");
    }

    #[test]
    fn characters_carry_shift_and_text() {
        let event = press(0x00, SHIFT, "A", "A");
        assert_eq!(event.key, Key::Character('A'));
        assert_eq!(event.text.as_deref(), Some("A"));
        assert_eq!(event.modifiers, Modifiers::SHIFT);
        assert_eq!(event.scancode, 0);
        let keypad = press(0x52, 0, "0", "0");
        assert_eq!(keypad.key, Key::Character('0'));
    }

    #[test]
    fn shortcuts_function_keys_and_releases_carry_no_text() {
        let command_c = press(0x08, COMMAND, "c", "c");
        assert_eq!((command_c.key, command_c.text), (Key::Character('c'), None));
        let option_e = press(0x0E, OPTION, "e", "");
        assert_eq!((option_e.key, option_e.text), (Key::Character('e'), None));
        let arrow = press(0x7B, 0, "\u{F702}", "\u{F702}");
        assert_eq!((arrow.key, arrow.text), (Key::Left, None));
        let enter = press(0x24, 0, "\r", "\r");
        assert_eq!(enter.text, None);
        let release = key_event(KeyPress {
            code: 0x00,
            state: ButtonState::Released,
            repeat: false,
            flags: 0,
            unmodified: "a",
            typed: "a",
        });
        assert_eq!((release.key, release.text), (Key::Character('a'), None));
    }

    #[test]
    fn option_types_the_layouts_other_characters() {
        let at = press(0x25, OPTION, "l", "@");
        assert_eq!(
            (at.key, at.text.as_deref(), at.modifiers),
            (Key::Character('l'), Some("@"), Modifiers::ALT)
        );
        let pipe = press(0x1A, OPTION | SHIFT, "/", "\\");
        assert_eq!(pipe.text.as_deref(), Some("\\"));
        let command_option = press(0x25, OPTION | COMMAND, "l", "@");
        assert_eq!(command_option.text, None, "a shortcut");
    }

    #[test]
    fn unknown_keys_keep_their_code() {
        let event = press(0x48, 0, "", "");
        assert_eq!(event.key, Key::Unidentified(0x48));
        let composed = press(0x00, 0, "ab", "ab");
        assert_eq!(composed.key, Key::Unidentified(0));
        assert_eq!(composed.text.as_deref(), Some("ab"));
    }

    #[test]
    fn modifier_keys_report_the_modifiers_before_them() {
        let pressed = flags_changed(0x38, SHIFT | 0x0002).unwrap();
        assert_eq!(pressed.key, Key::Shift);
        assert_eq!(pressed.state, ButtonState::Pressed);
        assert_eq!(pressed.modifiers, Modifiers::NONE);
        let released = flags_changed(0x38, 0).unwrap();
        assert_eq!(released.state, ButtonState::Released);
        assert_eq!(released.modifiers, Modifiers::SHIFT);
        // The right Shift, while the left one is held.
        let right = flags_changed(0x3C, SHIFT | 0x0006).unwrap();
        assert_eq!(
            (right.state, right.modifiers),
            (ButtonState::Pressed, Modifiers::SHIFT)
        );
        // The right Shift released, the left one still held.
        let right_up = flags_changed(0x3C, SHIFT | 0x0002).unwrap();
        assert_eq!(right_up.state, ButtonState::Released);
        let command = flags_changed(0x37, COMMAND | CONTROL | 0x0009).unwrap();
        assert_eq!(
            (command.key, command.modifiers),
            (Key::Super, Modifiers::CONTROL)
        );
    }

    #[test]
    fn modifier_keys_without_device_bits() {
        let pressed = flags_changed(0x3A, OPTION).unwrap();
        assert_eq!(
            (pressed.key, pressed.state),
            (Key::Alt, ButtonState::Pressed)
        );
        let released = flags_changed(0x3A, 0).unwrap();
        assert_eq!(released.state, ButtonState::Released);
    }

    #[test]
    fn caps_lock_and_unreported_keys() {
        let on = flags_changed(CAPS_LOCK_KEY, CAPS_LOCK).unwrap();
        assert_eq!((on.key, on.state), (Key::CapsLock, ButtonState::Pressed));
        let off = flags_changed(CAPS_LOCK_KEY, 0).unwrap();
        assert_eq!(off.state, ButtonState::Released);
        assert_eq!(flags_changed(0x3F, 0x0080_0000), None, "the fn key");
    }
}
