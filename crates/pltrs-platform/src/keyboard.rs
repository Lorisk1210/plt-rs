//! Platform-neutral keys and modifier state.

use std::borrow::Cow;
use std::fmt;
use std::ops::{BitOr, BitOrAssign};

/// A logical key, after the keyboard layout and Shift/Caps Lock were
/// applied.
///
/// [`Key::name`] yields the key names matplotlib uses in its
/// `key_press_event`s (`"enter"`, `"pageup"`, `"f1"`, `"a"`, `"A"`, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Key {
    /// A key producing a printable character (letters, digits,
    /// punctuation, space, keypad digits, ...), with Shift/Caps Lock
    /// applied: Shift+a yields `Character('A')`.
    Character(char),
    /// Return / keypad Enter.
    Enter,
    /// Escape.
    Escape,
    /// Backspace.
    Backspace,
    /// Tab (also Shift+Tab).
    Tab,
    /// Delete (forward delete).
    Delete,
    /// Insert.
    Insert,
    /// Home.
    Home,
    /// End.
    End,
    /// Page Up.
    PageUp,
    /// Page Down.
    PageDown,
    /// Left arrow.
    Left,
    /// Right arrow.
    Right,
    /// Up arrow.
    Up,
    /// Down arrow.
    Down,
    /// Function key `F1`..=`F35` (the payload is the number).
    F(u8),
    /// Either Shift key.
    Shift,
    /// Either Control key.
    Control,
    /// Either Alt (or Meta) key.
    Alt,
    /// Either Super ("Windows", "Command") key.
    Super,
    /// `AltGr` / ISO Level 3 Shift.
    AltGr,
    /// Caps Lock.
    CapsLock,
    /// Num Lock.
    NumLock,
    /// Scroll Lock.
    ScrollLock,
    /// Pause / Break.
    Pause,
    /// Print Screen.
    PrintScreen,
    /// The context-menu key.
    Menu,
    /// Clear (keypad 5 without Num Lock on some layouts).
    Clear,
    /// A dead key: it types nothing itself, but puts its accent on the
    /// character typed next (´ then e types é). The payload is the
    /// accent as the platform gives it: a spacing character (´, ^, ~, ¨,
    /// the grave accent, ...), or a combining one where Unicode has no
    /// spacing form.
    Dead(char),
    /// The Compose key (`Multi_key`), which starts a compose sequence
    /// (Compose, o, c types ©).
    Compose,
    /// A key this crate has no name for; the payload is the platform's raw
    /// key symbol: a keysym on X11 and Wayland, the hardware key code on
    /// macOS, the virtual-key code on Windows.
    Unidentified(u32),
}

/// The dead keys of accents: their X keysym, the accent they put on
/// characters and their name (the keysym's, without `dead_`), which
/// matplotlib's GTK and Tk backends report (`dead_acute`).
pub(crate) const DEAD_KEYS: [(u32, char, &str); 38] = [
    (0xfe50, '`', "grave"),
    (0xfe51, '´', "acute"),
    (0xfe52, '^', "circumflex"),
    (0xfe53, '~', "tilde"),
    (0xfe54, '¯', "macron"),
    (0xfe55, '˘', "breve"),
    (0xfe56, '˙', "abovedot"),
    (0xfe57, '¨', "diaeresis"),
    (0xfe58, '˚', "abovering"),
    (0xfe59, '˝', "doubleacute"),
    (0xfe5a, 'ˇ', "caron"),
    (0xfe5b, '¸', "cedilla"),
    (0xfe5c, '˛', "ogonek"),
    (0xfe5d, 'ͺ', "iota"),
    (0xfe5e, '゛', "voiced_sound"),
    (0xfe5f, '゜', "semivoiced_sound"),
    (0xfe60, '\u{323}', "belowdot"),
    (0xfe61, '\u{309}', "hook"),
    (0xfe62, '\u{31b}', "horn"),
    (0xfe63, '\u{335}', "stroke"),
    (0xfe64, '\u{313}', "abovecomma"),
    (0xfe65, '\u{314}', "abovereversedcomma"),
    (0xfe66, '\u{30f}', "doublegrave"),
    (0xfe67, '\u{325}', "belowring"),
    (0xfe68, 'ˍ', "belowmacron"),
    (0xfe69, '\u{32d}', "belowcircumflex"),
    (0xfe6a, '\u{330}', "belowtilde"),
    (0xfe6b, '\u{32e}', "belowbreve"),
    (0xfe6c, '\u{324}', "belowdiaeresis"),
    (0xfe6d, '\u{311}', "invertedbreve"),
    (0xfe6e, '\u{326}', "belowcomma"),
    (0xfe6f, '¤', "currency"),
    (0xfe8c, 'µ', "greek"),
    (0xfe8d, 'ء', "hamza"),
    (0xfe90, '\u{332}', "lowline"),
    (0xfe91, '\u{30d}', "aboveverticalline"),
    (0xfe92, '\u{329}', "belowverticalline"),
    (0xfe93, '\u{338}', "longsolidusoverlay"),
];

/// Other characters the systems give dead keys: Windows' US
/// International layout gives the apostrophe and the quotation mark,
/// macOS the modifier letters circumflex and small tilde.
const DEAD_KEY_ALIASES: [(char, &str); 4] = [
    ('\'', "acute"),
    ('"', "diaeresis"),
    ('ˆ', "circumflex"),
    ('˜', "tilde"),
];

/// The name of the dead key putting `accent` on characters:
/// `dead_acute`, `dead_diaeresis`, ...; `dead_` and the accent itself
/// for accents without one.
fn dead_key_name(accent: char) -> String {
    let known = DEAD_KEYS
        .iter()
        .map(|&(_, c, name)| (c, name))
        .chain(DEAD_KEY_ALIASES)
        .find(|&(c, _)| c == accent);
    match known {
        Some((_, name)) => format!("dead_{name}"),
        None => format!("dead_{accent}"),
    }
}

impl Key {
    /// The matplotlib key name, e.g. `"enter"`, `"pagedown"`, `"f5"`,
    /// `"control"`, `"a"`, `"A"` or `"dead_acute"`.
    ///
    /// Modifier prefixes (`"ctrl+"`, `"alt+"`, ...) are not included; they
    /// are derived from [`Modifiers`] by the caller.
    pub fn name(self) -> Cow<'static, str> {
        let fixed = match self {
            Self::Character(c) => return Cow::Owned(c.to_string()),
            Self::F(n) => return Cow::Owned(format!("f{n}")),
            Self::Dead(accent) => return Cow::Owned(dead_key_name(accent)),
            Self::Unidentified(raw) => return Cow::Owned(format!("unknown-{raw:#x}")),
            Self::Compose => "multi_key",
            Self::Enter => "enter",
            Self::Escape => "escape",
            Self::Backspace => "backspace",
            Self::Tab => "tab",
            Self::Delete => "delete",
            Self::Insert => "insert",
            Self::Home => "home",
            Self::End => "end",
            Self::PageUp => "pageup",
            Self::PageDown => "pagedown",
            Self::Left => "left",
            Self::Right => "right",
            Self::Up => "up",
            Self::Down => "down",
            Self::Shift => "shift",
            Self::Control => "control",
            Self::Alt => "alt",
            Self::Super => "super",
            Self::AltGr => "iso_level3_shift",
            Self::CapsLock => "caps_lock",
            Self::NumLock => "num_lock",
            Self::ScrollLock => "scroll_lock",
            Self::Pause => "pause",
            Self::PrintScreen => "print",
            Self::Menu => "menu",
            Self::Clear => "clear",
        };
        Cow::Borrowed(fixed)
    }

    /// Whether this is a modifier key (Shift, Control, Alt, Super, `AltGr`).
    pub const fn is_modifier(self) -> bool {
        matches!(
            self,
            Self::Shift | Self::Control | Self::Alt | Self::Super | Self::AltGr
        )
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name())
    }
}

/// The set of held modifier keys at the time of an input event.
///
/// Combine flags with `|`:
///
/// ```
/// use pltrs_platform::Modifiers;
/// let mods = Modifiers::CONTROL | Modifiers::SHIFT;
/// assert!(mods.control() && mods.shift() && !mods.alt());
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers(u8);

impl Modifiers {
    /// No modifier held.
    pub const NONE: Self = Self(0);
    /// A Shift key.
    pub const SHIFT: Self = Self(1);
    /// A Control key.
    pub const CONTROL: Self = Self(1 << 1);
    /// An Alt / Meta key.
    pub const ALT: Self = Self(1 << 2);
    /// A Super / Windows / Command key.
    pub const SUPER: Self = Self(1 << 3);

    /// Whether every flag of `other` is set in `self`.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Whether no modifier is held.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Whether Shift is held.
    pub const fn shift(self) -> bool {
        self.contains(Self::SHIFT)
    }

    /// Whether Control is held.
    pub const fn control(self) -> bool {
        self.contains(Self::CONTROL)
    }

    /// Whether Alt is held.
    pub const fn alt(self) -> bool {
        self.contains(Self::ALT)
    }

    /// Whether Super is held.
    pub const fn super_key(self) -> bool {
        self.contains(Self::SUPER)
    }

    /// Returns `self` with the flags of `other` set or cleared.
    #[must_use]
    pub const fn with(self, other: Self, enabled: bool) -> Self {
        if enabled {
            Self(self.0 | other.0)
        } else {
            Self(self.0 & !other.0)
        }
    }
}

impl BitOr for Modifiers {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Modifiers {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl fmt::Debug for Modifiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const NAMES: [(Modifiers, &str); 4] = [
            (Modifiers::CONTROL, "CONTROL"),
            (Modifiers::ALT, "ALT"),
            (Modifiers::SUPER, "SUPER"),
            (Modifiers::SHIFT, "SHIFT"),
        ];
        let mut set = f.debug_set();
        for (flag, name) in NAMES {
            if self.contains(flag) {
                set.entry(&format_args!("{name}"));
            }
        }
        set.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matplotlib_key_names() {
        let cases = [
            (Key::Character('a'), "a"),
            (Key::Character('A'), "A"),
            (Key::Character(' '), " "),
            (Key::Enter, "enter"),
            (Key::Escape, "escape"),
            (Key::Backspace, "backspace"),
            (Key::Tab, "tab"),
            (Key::Delete, "delete"),
            (Key::Home, "home"),
            (Key::End, "end"),
            (Key::PageUp, "pageup"),
            (Key::PageDown, "pagedown"),
            (Key::Left, "left"),
            (Key::Right, "right"),
            (Key::Up, "up"),
            (Key::Down, "down"),
            (Key::F(1), "f1"),
            (Key::F(12), "f12"),
            (Key::Shift, "shift"),
            (Key::Control, "control"),
            (Key::Alt, "alt"),
            (Key::Super, "super"),
            (Key::CapsLock, "caps_lock"),
        ];
        for (key, name) in cases {
            assert_eq!(key.name(), name, "{key:?}");
            assert_eq!(key.to_string(), name);
        }
        assert_eq!(Key::Unidentified(0x1234).name(), "unknown-0x1234");
    }

    #[test]
    fn dead_keys_are_named_as_x_names_them() {
        assert_eq!(Key::Dead('´').name(), "dead_acute");
        assert_eq!(Key::Dead('`').name(), "dead_grave");
        assert_eq!(Key::Dead('\u{323}').name(), "dead_belowdot");
        assert_eq!(
            Key::Dead('\'').name(),
            "dead_acute",
            "Windows' US International"
        );
        assert_eq!(Key::Dead('ˆ').name(), "dead_circumflex", "macOS");
        assert_eq!(Key::Dead('°').name(), "dead_°", "no X name");
        assert_eq!(Key::Compose.name(), "multi_key");
        let mut accents: Vec<char> = DEAD_KEYS.iter().map(|&(_, accent, _)| accent).collect();
        accents.sort_unstable();
        accents.dedup();
        assert_eq!(accents.len(), DEAD_KEYS.len(), "one accent per dead key");
        assert!(
            DEAD_KEYS.windows(2).all(|pair| pair[0].0 < pair[1].0),
            "sorted by keysym"
        );
    }

    #[test]
    fn modifier_keys_are_flagged() {
        assert!(Key::Shift.is_modifier());
        assert!(Key::AltGr.is_modifier());
        assert!(!Key::CapsLock.is_modifier());
        assert!(!Key::Character('x').is_modifier());
    }

    #[test]
    fn modifier_set_operations() {
        let mut mods = Modifiers::NONE;
        assert!(mods.is_empty());
        mods |= Modifiers::ALT;
        assert!(mods.alt() && !mods.control());
        let mods = mods | Modifiers::SUPER;
        assert!(mods.super_key());
        assert!(mods.contains(Modifiers::ALT | Modifiers::SUPER));
        assert!(!mods.contains(Modifiers::ALT | Modifiers::SHIFT));
        assert_eq!(mods.with(Modifiers::ALT, false), Modifiers::SUPER);
        assert_eq!(format!("{mods:?}"), "{ALT, SUPER}");
    }
}
