//! The keyboard mapping: keysyms per keycode, the roles of the modifier
//! bits, and keysym selection following the core protocol's rules
//! (X11 protocol, section 5 "Keyboards").
//!
//! Each keycode lists keysyms in groups of two (unshifted, shifted):
//! group 1, group 2 (selected by `Mode_switch`), and, as XKB servers lay
//! out the core map, the third and fourth shift levels of group 1
//! (selected by `ISO_Level3_Shift`, i.e. `AltGr`).

use super::connection::Connection;
use super::request;
use super::wire::Reader;
use crate::error::Error;
use crate::keyboard::Modifiers;
use crate::keysym::{self, NO_SYMBOL, names};

/// `Shift` bit of the event state.
const SHIFT: u16 = 1;
/// `Lock` bit of the event state.
const LOCK: u16 = 1 << 1;
/// `Control` bit of the event state.
const CONTROL: u16 = 1 << 2;
/// Index of `Lock` among the eight modifiers.
const LOCK_INDEX: usize = 1;
/// Index of `Mod1`, the first freely assignable modifier.
const MOD1_INDEX: usize = 3;
/// Shift, Lock, Control, Mod1 ..= Mod5.
const MODIFIER_COUNT: usize = 8;
/// Length of a reply header.
const REPLY_HEADER_LEN: usize = 32;
/// Keysym index of the first group-2 keysym.
const GROUP2: usize = 2;
/// Keysym index of the first shift-level-3 keysym.
const LEVEL3: usize = 4;

/// What the `Lock` modifier does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum LockRole {
    /// Nothing (no lock key is mapped to it).
    #[default]
    Ignored,
    /// Caps Lock: uppercases letters only.
    CapsLock,
    /// Shift Lock: acts like a held Shift.
    ShiftLock,
}

/// Which state bits carry which meaning (they vary between keymaps).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct ModifierRoles {
    alt: u16,
    super_key: u16,
    num_lock: u16,
    mode_switch: u16,
    level3: u16,
    lock: LockRole,
}

/// A keyboard mapping fetched from the server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Keymap {
    min_keycode: u8,
    keysyms_per_keycode: usize,
    keysyms: Vec<u32>,
    roles: ModifierRoles,
}

impl Keymap {
    /// Fetches the keyboard and modifier mappings.
    pub(crate) fn query(conn: &mut Connection) -> Result<Self, Error> {
        let (min, max) = (conn.setup().min_keycode, conn.setup().max_keycode);
        let count = max
            .checked_sub(min)
            .and_then(|span| span.checked_add(1))
            .ok_or(Error::Malformed("invalid keycode range"))?;
        let keyboard = conn.query(&request::get_keyboard_mapping(min, count))?;
        let modifiers = conn.query(&request::get_modifier_mapping())?;
        let keyboard = conn.reply(keyboard)?;
        let modifiers = conn.reply(modifiers)?;
        Self::parse(min, &keyboard, &modifiers)
    }

    /// Builds the keymap from `GetKeyboardMapping` and
    /// `GetModifierMapping` replies.
    pub(crate) fn parse(min_keycode: u8, keyboard: &[u8], modifiers: &[u8]) -> Result<Self, Error> {
        let keysyms_per_keycode = usize::from(Reader::at(keyboard, 1).u8()?);
        let words = Reader::at(keyboard, 4).u32()?;
        let mut reader = Reader::at(keyboard, REPLY_HEADER_LEN);
        let keysyms = (0..words)
            .map(|_| reader.u32())
            .collect::<Result<Vec<_>, _>>()?;
        let mut keymap = Self {
            min_keycode,
            keysyms_per_keycode,
            keysyms,
            roles: ModifierRoles::default(),
        };
        keymap.roles = keymap.modifier_roles(modifiers)?;
        Ok(keymap)
    }

    fn modifier_roles(&self, reply: &[u8]) -> Result<ModifierRoles, Error> {
        let per_modifier = usize::from(Reader::at(reply, 1).u8()?);
        let keycodes = Reader::at(reply, REPLY_HEADER_LEN).bytes(per_modifier * MODIFIER_COUNT)?;
        let mut roles = ModifierRoles::default();
        for (index, codes) in keycodes.chunks(per_modifier.max(1)).enumerate() {
            let has = |wanted: &[u32]| {
                codes
                    .iter()
                    .flat_map(|&code| self.keysyms_of(code))
                    .any(|keysym| wanted.contains(keysym))
            };
            if index == LOCK_INDEX {
                roles.lock = if has(&[names::CAPS_LOCK]) {
                    LockRole::CapsLock
                } else if has(&[names::SHIFT_LOCK]) {
                    LockRole::ShiftLock
                } else {
                    LockRole::Ignored
                };
            } else if index >= MOD1_INDEX {
                let bit = 1 << index;
                for (role, keysyms) in [
                    (&mut roles.alt, &names::ALT[..]),
                    (&mut roles.super_key, &names::SUPER[..]),
                    (&mut roles.num_lock, &[names::NUM_LOCK][..]),
                    (&mut roles.mode_switch, &[names::MODE_SWITCH][..]),
                    (&mut roles.level3, &[names::ISO_LEVEL3_SHIFT][..]),
                ] {
                    if has(keysyms) {
                        *role |= bit;
                    }
                }
            }
        }
        Ok(roles)
    }

    /// All keysyms of a keycode (empty if unmapped).
    pub(crate) fn keysyms_of(&self, keycode: u8) -> &[u32] {
        let Some(index) = keycode.checked_sub(self.min_keycode) else {
            return &[];
        };
        let start = usize::from(index) * self.keysyms_per_keycode;
        self.keysyms
            .get(start..start + self.keysyms_per_keycode)
            .unwrap_or(&[])
    }

    /// The modifiers held according to an event state.
    pub(crate) fn modifiers(&self, state: u16) -> Modifiers {
        Modifiers::NONE
            .with(Modifiers::SHIFT, state & SHIFT != 0)
            .with(Modifiers::CONTROL, state & CONTROL != 0)
            .with(Modifiers::ALT, state & self.roles.alt != 0)
            .with(Modifiers::SUPER, state & self.roles.super_key != 0)
    }

    /// The keysym a key produces under the modifier `state`.
    pub(crate) fn keysym(&self, keycode: u8, state: u16) -> u32 {
        let keysyms = self.keysyms_of(keycode);
        let group = |index: usize| {
            let first = keysyms.get(index).copied().unwrap_or(NO_SYMBOL);
            let second = keysyms.get(index + 1).copied().unwrap_or(NO_SYMBOL);
            match (first, second) {
                (NO_SYMBOL, NO_SYMBOL) => None,
                (only, NO_SYMBOL) if keysym::has_case(only) => {
                    Some((keysym::to_lower(only), keysym::to_upper(only)))
                }
                (only, NO_SYMBOL) => Some((only, only)),
                pair => Some(pair),
            }
        };
        let shifted_group = if state & self.roles.level3 != 0 {
            group(LEVEL3)
        } else if state & self.roles.mode_switch != 0 {
            group(GROUP2)
        } else {
            None
        };
        let Some((lower, upper)) = shifted_group.or_else(|| group(0)) else {
            return NO_SYMBOL;
        };
        let shift = state & SHIFT != 0;
        let lock = if state & LOCK == 0 {
            LockRole::Ignored
        } else {
            self.roles.lock
        };
        if state & self.roles.num_lock != 0 && keysym::is_keypad(upper) {
            return if shift || lock == LockRole::ShiftLock {
                lower
            } else {
                upper
            };
        }
        match (shift, lock) {
            (false, LockRole::Ignored) => lower,
            (false, LockRole::CapsLock) => keysym::to_upper(lower),
            (true, LockRole::CapsLock) => keysym::to_upper(upper),
            (true, _) | (false, LockRole::ShiftLock) => upper,
        }
    }

    /// The first keycode whose unshifted keysym is `keysym`.
    #[cfg(test)]
    pub(crate) fn keycode_of(&self, keysym: u32) -> Option<u8> {
        (self.min_keycode..=u8::MAX).find(|&code| self.keysyms_of(code).first() == Some(&keysym))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::x11::fixtures::{KEYBOARD_MAPPING, MODIFIER_MAPPING, keymap};

    const MOD1: u16 = 1 << 3;
    const MOD2: u16 = 1 << 4;
    const MOD4: u16 = 1 << 6;
    const MOD5: u16 = 1 << 7;

    const KEY_1: u8 = 10;
    const KEY_Q: u8 = 24;
    const KEY_UDIAERESIS: u8 = 34;
    const KEY_A: u8 = 38;
    const KEY_SSHARP: u8 = 20;
    const KEY_KP_1: u8 = 87;
    const KEY_ALT_ONLY_SHIFTED: u8 = 204;

    #[test]
    fn parses_fixture_and_modifier_roles() {
        let keymap = keymap();
        assert_eq!(keymap.keysyms_per_keycode, 10);
        assert_eq!(keymap.keysyms.len(), 248 * 10);
        assert_eq!(
            keymap.roles,
            ModifierRoles {
                alt: MOD1,
                super_key: MOD4,
                num_lock: MOD2,
                mode_switch: 0,
                level3: MOD5,
                lock: LockRole::CapsLock,
            }
        );
        assert_eq!(keymap.keycode_of(0x61), Some(KEY_A));
    }

    #[test]
    fn shift_and_caps_lock() {
        let keymap = keymap();
        assert_eq!(keymap.keysym(KEY_A, 0), 0x61);
        assert_eq!(keymap.keysym(KEY_A, SHIFT), 0x41);
        assert_eq!(keymap.keysym(KEY_A, LOCK), 0x41);
        assert_eq!(keymap.keysym(KEY_A, SHIFT | LOCK), 0x41);
        assert_eq!(keymap.keysym(KEY_1, SHIFT), 0x21, "exclam");
        assert_eq!(
            keymap.keysym(KEY_1, LOCK),
            0x31,
            "Caps Lock leaves digits alone"
        );
        assert_eq!(keymap.keysym(KEY_UDIAERESIS, LOCK), 0xdc, "Udiaeresis");
        assert_eq!(
            keymap.keysym(KEY_SSHARP, LOCK),
            0xdf,
            "ssharp has no uppercase keysym"
        );
    }

    #[test]
    fn altgr_selects_the_third_level() {
        let keymap = keymap();
        assert_eq!(keymap.keysym(KEY_Q, MOD5), 0x40, "at");
        assert_eq!(keymap.keysym(KEY_Q, MOD5 | SHIFT), 0x7d9, "Greek_OMEGA");
        assert_eq!(
            keymap.keysym(KEY_Q, MOD1),
            0x71,
            "Alt does not change the keysym"
        );
    }

    #[test]
    fn num_lock_selects_keypad_digits() {
        let keymap = keymap();
        assert_eq!(keymap.keysym(KEY_KP_1, 0), 0xff9c, "KP_End");
        assert_eq!(keymap.keysym(KEY_KP_1, MOD2), 0xffb1, "KP_1");
        assert_eq!(keymap.keysym(KEY_KP_1, MOD2 | SHIFT), 0xff9c);
    }

    #[test]
    fn unmapped_keys_have_no_symbol() {
        let keymap = keymap();
        assert_eq!(keymap.keysym(KEY_ALT_ONLY_SHIFTED, 0), NO_SYMBOL);
        assert_eq!(keymap.keysym(KEY_ALT_ONLY_SHIFTED, SHIFT), names::ALT[0]);
        assert_eq!(keymap.keysym(7, 0), NO_SYMBOL, "below the minimum keycode");
        assert_eq!(keymap.keysym(97, 0), NO_SYMBOL, "keycode without keysyms");
    }

    #[test]
    fn event_state_to_modifiers() {
        let keymap = keymap();
        let all = SHIFT | CONTROL | MOD1 | MOD4;
        assert_eq!(
            keymap.modifiers(all),
            Modifiers::SHIFT | Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER
        );
        assert_eq!(keymap.modifiers(MOD2 | LOCK | MOD5), Modifiers::NONE);
    }

    #[test]
    fn truncated_replies_are_errors() {
        assert!(Keymap::parse(8, &KEYBOARD_MAPPING[..100], MODIFIER_MAPPING).is_err());
        assert!(Keymap::parse(8, KEYBOARD_MAPPING, &MODIFIER_MAPPING[..40]).is_err());
    }
}
