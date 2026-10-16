//! Protocol messages recorded from a real server (`XWayland` under GNOME),
//! shared by the unit tests.

use super::keymap::Keymap;

/// The connection setup reply.
pub(crate) const SETUP: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/fixtures/x11/setup_xwayland.bin"
));

/// A `GetKeyboardMapping` reply for keycodes 8 ..= 255, 10 keysyms each:
/// a German layout with US as the second group.
pub(crate) const KEYBOARD_MAPPING: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/fixtures/x11/keyboard_mapping.bin"
));

/// The matching `GetModifierMapping` reply: Mod1 Alt, Mod2 Num Lock,
/// Mod3 Level 5, Mod4 Super, Mod5 `AltGr`, Lock Caps Lock.
pub(crate) const MODIFIER_MAPPING: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/fixtures/x11/modifier_mapping.bin"
));

/// The keymap built from the recorded mappings.
pub(crate) fn keymap() -> Keymap {
    Keymap::parse(8, KEYBOARD_MAPPING, MODIFIER_MAPPING).expect("the fixtures are valid")
}
