//! X keysyms, shared by the X11 and Wayland backends (XKB keymaps name
//! keys by keysym too): translation to [`Key`]s and characters, case
//! mapping, and the keysym names keymaps are written with.

// XKB keymaps name keysyms; only the Wayland backend reads them.
#[cfg(target_os = "linux")]
mod name_table;
mod table;

use crate::event::{ButtonState, KeyEvent};
use crate::keyboard::{Key, Modifiers};
#[cfg(target_os = "linux")]
use name_table::NAMES;
use table::RUNS;

/// The "no symbol" keysym.
pub(crate) const NO_SYMBOL: u32 = 0;

/// Keysyms `0x0100_0000 + c` stand for the Unicode character `c`.
const UNICODE_OFFSET: u32 = 0x0100_0000;

/// Named keysyms used by keymap logic.
pub(crate) mod names {
    /// `Shift_Lock`.
    pub(crate) const SHIFT_LOCK: u32 = 0xffe6;
    /// `Caps_Lock`.
    pub(crate) const CAPS_LOCK: u32 = 0xffe5;
    /// `Num_Lock`.
    pub(crate) const NUM_LOCK: u32 = 0xff7f;
    /// `Mode_switch` (selects the second keysym group).
    pub(crate) const MODE_SWITCH: u32 = 0xff7e;
    /// `ISO_Level3_Shift` (`AltGr`).
    pub(crate) const ISO_LEVEL3_SHIFT: u32 = 0xfe03;
    /// `Alt_L`, `Alt_R`, `Meta_L`, `Meta_R`.
    pub(crate) const ALT: [u32; 4] = [0xffe9, 0xffea, 0xffe7, 0xffe8];
    /// `Super_L`, `Super_R`.
    pub(crate) const SUPER: [u32; 2] = [0xffeb, 0xffec];
}

/// Whether `keysym` is on the numeric keypad (`KP_Space` ..= `KP_Equal`).
pub(crate) const fn is_keypad(keysym: u32) -> bool {
    matches!(keysym, 0xff80..=0xffbd)
}

/// The logical key of a keysym.
pub(crate) fn key(keysym: u32) -> Key {
    match keysym {
        0xff08 => Key::Backspace,
        0xff09 | 0xff89 | 0xfe20 => Key::Tab,
        0xff0b | 0xff9d => Key::Clear,
        0xff0d | 0xff8d => Key::Enter,
        0xff13 => Key::Pause,
        0xff14 => Key::ScrollLock,
        0xff1b => Key::Escape,
        0xff50 | 0xff95 => Key::Home,
        0xff51 | 0xff96 => Key::Left,
        0xff52 | 0xff97 => Key::Up,
        0xff53 | 0xff98 => Key::Right,
        0xff54 | 0xff99 => Key::Down,
        0xff55 | 0xff9a => Key::PageUp,
        0xff56 | 0xff9b => Key::PageDown,
        0xff57 | 0xff9c => Key::End,
        0xff61 => Key::PrintScreen,
        0xff63 | 0xff9e => Key::Insert,
        0xff67 => Key::Menu,
        0xff7f => Key::NumLock,
        0xffff | 0xff9f => Key::Delete,
        // F1 ..= F35.
        0xffbe..=0xffe0 => Key::F((keysym - 0xffbd) as u8),
        0xffe1 | 0xffe2 => Key::Shift,
        0xffe3 | 0xffe4 => Key::Control,
        0xffe5 => Key::CapsLock,
        0xffe7..=0xffea => Key::Alt,
        0xffeb | 0xffec => Key::Super,
        names::ISO_LEVEL3_SHIFT => Key::AltGr,
        _ => match to_char(keysym) {
            Some(c) if !c.is_control() => Key::Character(c),
            _ => Key::Unidentified(keysym),
        },
    }
}

/// The event of a key producing `keysym` with `modifiers` held: presses
/// of printable keys carry their text, unless Control, Alt or Super make
/// them a shortcut.
pub(crate) fn key_event(
    keysym: u32,
    state: ButtonState,
    modifiers: Modifiers,
    repeat: bool,
    scancode: u32,
) -> KeyEvent {
    let shortcut = modifiers.control() || modifiers.alt() || modifiers.super_key();
    let text = (state.is_pressed() && !shortcut)
        .then(|| to_char(keysym))
        .flatten()
        .filter(|c| !c.is_control())
        .map(String::from);
    KeyEvent {
        key: key(keysym),
        state,
        repeat,
        text,
        modifiers,
        scancode,
    }
}

/// The character a keysym stands for, if any.
pub(crate) fn to_char(keysym: u32) -> Option<char> {
    match keysym {
        0x20..=0x7e | 0xa0..=0xff => char::from_u32(keysym),
        0x0100_0001..=0x0110_ffff => char::from_u32(keysym - UNICODE_OFFSET),
        0xff80 => Some(' '),
        0xffaa => Some('*'),
        0xffab => Some('+'),
        0xffac => Some(','),
        0xffad => Some('-'),
        0xffae => Some('.'),
        0xffaf => Some('/'),
        0xffb0..=0xffb9 => char::from_u32(keysym - 0xffb0 + u32::from(b'0')),
        0xffbd => Some('='),
        0x0100..=0xffff => legacy_char(keysym as u16),
        _ => None,
    }
}

/// The keysym `name` stands for, as keymaps write it: a keysym name
/// (`a`, `Return`, `Cyrillic_a`), `U` and a hexadecimal code point
/// (`U20AC`), a hexadecimal number (`0x1008ff11`), or `NoSymbol`.
#[cfg(target_os = "linux")]
pub(crate) fn from_name(name: &str) -> Option<u32> {
    if name == "NoSymbol" {
        return Some(NO_SYMBOL);
    }
    if let Ok(index) = NAMES.binary_search_by(|(known, _)| known.as_bytes().cmp(name.as_bytes())) {
        return Some(NAMES[index].1);
    }
    if let Some(digits) = name.strip_prefix("0x") {
        return u32::from_str_radix(digits, 16).ok();
    }
    let code = name
        .strip_prefix('U')
        .filter(|digits| (4..=6).contains(&digits.len()))
        .and_then(|digits| u32::from_str_radix(digits, 16).ok())?;
    char::from_u32(code).map(from_char)
}

/// The keysym standing for character `c`.
pub(crate) fn from_char(c: char) -> u32 {
    match u32::from(c) {
        code @ (0x20..=0x7e | 0xa0..=0xff) => code,
        code => UNICODE_OFFSET + code,
    }
}

/// Whether `keysym` is a letter with distinct upper- and lowercase forms.
pub(crate) fn has_case(keysym: u32) -> bool {
    to_upper(keysym) != to_lower(keysym)
}

/// The uppercase form of a cased letter keysym; other keysyms unchanged.
pub(crate) fn to_upper(keysym: u32) -> u32 {
    convert_case(keysym, char::to_uppercase)
}

/// The lowercase form of a cased letter keysym; other keysyms unchanged.
pub(crate) fn to_lower(keysym: u32) -> u32 {
    convert_case(keysym, char::to_lowercase)
}

fn convert_case<I: Iterator<Item = char>>(keysym: u32, convert: impl Fn(char) -> I) -> u32 {
    match to_char(keysym).and_then(|c| single(convert(c)).filter(|&converted| converted != c)) {
        Some(converted) => from_char(converted),
        None => keysym,
    }
}

/// The only item of an iterator, if it has exactly one.
fn single(mut chars: impl Iterator<Item = char>) -> Option<char> {
    let first = chars.next()?;
    chars.next().is_none().then_some(first)
}

fn legacy_char(keysym: u16) -> Option<char> {
    let index = RUNS
        .partition_point(|&(first, _, _)| first <= keysym)
        .checked_sub(1)?;
    let (first, start, len) = RUNS[index];
    let offset = keysym - first;
    (offset < len).then(|| char::from_u32(u32::from(start) + u32::from(offset)))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_keys() {
        assert_eq!(key(0xff0d), Key::Enter);
        assert_eq!(key(0xff8d), Key::Enter, "KP_Enter");
        assert_eq!(key(0xfe20), Key::Tab, "ISO_Left_Tab");
        assert_eq!(key(0xff1b), Key::Escape);
        assert_eq!(key(0xff55), Key::PageUp);
        assert_eq!(key(0xff9b), Key::PageDown, "KP_Next");
        assert_eq!(key(0xffbe), Key::F(1));
        assert_eq!(key(0xffe0), Key::F(35));
        assert_eq!(key(0xffe9), Key::Alt);
        assert_eq!(key(0xfe03), Key::AltGr);
        assert_eq!(
            key(0x1008_ff11),
            Key::Unidentified(0x1008_ff11),
            "XF86AudioLowerVolume"
        );
        assert_eq!(key(NO_SYMBOL), Key::Unidentified(0));
    }

    #[test]
    fn character_keys() {
        assert_eq!(key(0x61), Key::Character('a'));
        assert_eq!(key(0x41), Key::Character('A'));
        assert_eq!(key(0x20), Key::Character(' '));
        assert_eq!(key(0xdf), Key::Character('ß'));
        assert_eq!(key(0xffb7), Key::Character('7'), "KP_7");
        assert_eq!(key(0xffaa), Key::Character('*'), "KP_Multiply");
        assert_eq!(key(0x20ac), Key::Character('€'), "EuroSign");
        assert_eq!(key(0x6c1), Key::Character('а'), "Cyrillic_a");
        assert_eq!(key(0x7e1), Key::Character('α'), "Greek_alpha");
        assert_eq!(key(0x0100_1e9e), Key::Character('ẞ'), "U1E9E");
    }

    #[test]
    fn legacy_table_lookup() {
        assert_eq!(to_char(0x1a1), Some('Ą'));
        assert_eq!(to_char(0x1a4), None, "gap in Latin-2");
        assert_eq!(to_char(0xcfa), Some('ת'), "hebrew_taw");
        assert_eq!(to_char(0x100), None);
        assert_eq!(to_char(0x0100_0000), None, "U+0000 is not a key");
        assert!(
            RUNS.windows(2).all(|w| w[0].0 + w[0].2 <= w[1].0),
            "sorted, disjoint runs"
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn names_are_looked_up() {
        assert!(
            NAMES
                .windows(2)
                .all(|w| w[0].0.as_bytes() < w[1].0.as_bytes()),
            "sorted, unique names"
        );
        assert_eq!(from_name("a"), Some(0x61));
        assert_eq!(from_name("Return"), Some(0xff0d));
        assert_eq!(from_name("ISO_Level3_Shift"), Some(names::ISO_LEVEL3_SHIFT));
        assert_eq!(from_name("Cyrillic_a"), Some(0x6c1));
        assert_eq!(from_name("XF86AudioMute"), Some(0x1008_ff12));
        assert_eq!(from_name("NoSymbol"), Some(NO_SYMBOL));
        assert_eq!(from_name("U20AC"), Some(0x0100_20ac));
        assert_eq!(
            from_name("U00E9"),
            Some(0xe9),
            "Latin-1 has its own keysyms"
        );
        assert_eq!(from_name("0x1008ff11"), Some(0x1008_ff11));
        assert_eq!(from_name("U110000"), None, "beyond Unicode");
        assert_eq!(from_name("UD800"), None, "a surrogate");
        assert_eq!(from_name("U12"), None, "too few digits");
        assert_eq!(from_name("0xzz"), None);
        assert_eq!(from_name("no_such_key"), None);
        assert_eq!(from_name(""), None);
    }

    #[test]
    fn case_mapping() {
        assert!(has_case(0x61));
        assert!(!has_case(0x31));
        assert!(!has_case(0xdf), "ß has no single-letter uppercase");
        assert_eq!(to_upper(0x61), 0x41);
        assert_eq!(to_lower(0x41), 0x61);
        assert_eq!(to_upper(0xe4), 0xc4, "adiaeresis");
        assert_eq!(
            to_char(to_upper(0x6c1)),
            Some('А'),
            "Cyrillic via Unicode keysyms"
        );
        assert_eq!(to_upper(0xdf), 0xdf, "ß uppercases to two letters");
        assert_eq!(to_upper(0xff0d), 0xff0d);
        assert_eq!(from_char('a'), 0x61);
        assert_eq!(from_char('ą'), 0x0100_0105);
    }
}
