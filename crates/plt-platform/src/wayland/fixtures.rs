//! A keymap as a compositor sends it, shared by the unit tests: German
//! with Russian as the second layout, written by xkbcommon 1.7
//! (`xkbcli compile-keymap --layout de,ru --options grp:alt_shift_toggle`).

use super::xkb::Keymap;

/// The keymap's text.
pub(crate) const DE_RU_KEYMAP: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/fixtures/wayland/de_ru.xkb"
));

/// The keymap, parsed.
pub(crate) fn keymap() -> Keymap {
    Keymap::parse(DE_RU_KEYMAP).expect("the fixture is valid")
}
