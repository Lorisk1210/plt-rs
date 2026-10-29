//! Keymaps: the captured keymap with its bytes corrupted or its text
//! edited, and keymaps made up of XKB tokens; whatever parses is then
//! asked for keysyms, modifiers and repeat with any key and state.

use super::word;
use crate::fuzz::{Corruption, Gen, fuzz};
use crate::wayland::fixtures::DE_RU_KEYMAP;
use crate::wayland::xkb::Keymap;

/// Pieces of XKB text.
const PIECES: [&str; 40] = [
    "xkb_keymap {",
    "xkb_keycodes {",
    "xkb_types {",
    "xkb_compat {",
    "xkb_symbols {",
    "};",
    "{",
    "}",
    ";",
    ",",
    "[",
    "]",
    "(",
    ")",
    "=",
    "+",
    ".",
    "<AC01>",
    "<A>",
    "38",
    "0x26",
    "key",
    "type",
    "\"TWO_LEVEL\"",
    "\"KEYPAD\"",
    "symbols[Group2]",
    "map[Shift]",
    "Level2",
    "modifiers",
    "modifier_map Mod1",
    "virtual_modifiers",
    "NumLock=Mod2",
    "interpret",
    "AnyOf(all)",
    "Exactly(Lock)",
    "virtualModifier",
    "useModMapMods=level1",
    "a",
    "Cyrillic_a",
    "U20AC",
];

/// How a keymap is made.
#[derive(Debug)]
enum Made {
    /// The captured keymap, bytes corrupted.
    Corrupted(Corruption),
    /// The captured keymap, text edited.
    Edited(String),
    /// A soup of XKB pieces, in an `xkb_keymap` block or not.
    Soup(String),
}

/// What a parsed keymap is asked.
#[derive(Debug)]
struct Case {
    made: Made,
    keys: Vec<(u32, u32, u32)>,
}

fn generate(g: &mut Gen) -> Case {
    let made = match g.below(4) {
        0 => Made::Corrupted(g.corrupt(DE_RU_KEYMAP.as_bytes())),
        1 => Made::Edited(g.mutate(DE_RU_KEYMAP, &PIECES)),
        _ => {
            let soup = g.soup(&PIECES, 60);
            Made::Soup(if g.chance(0.5) {
                format!("xkb_keymap {{ xkb_symbols {{ {soup} }}; }};")
            } else {
                soup
            })
        }
    };
    let keys = (0..g.len(8))
        .map(|_| {
            let any = word(g);
            (g.pick(&[9, 10, 24, 38, 50, 87, 108, any]), word(g), word(g))
        })
        .collect();
    Case { made, keys }
}

#[test]
fn keymaps() {
    fuzz("xkb keymaps", 400, generate, |case| {
        let text = match &case.made {
            Made::Corrupted(corruption) => {
                String::from_utf8_lossy(&corruption.apply(DE_RU_KEYMAP.as_bytes())).into_owned()
            }
            Made::Edited(text) | Made::Soup(text) => text.clone(),
        };
        let Ok(keymap) = Keymap::parse(&text) else {
            return;
        };
        for &(keycode, state, group) in &case.keys {
            let _ = keymap.keysym(keycode, state, group);
            let _ = keymap.modifiers(state);
            let _ = keymap.repeats(keycode);
        }
    });
}
