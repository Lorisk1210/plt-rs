//! Keymap tests against a keymap as compositors send it (German with
//! Russian as the second layout, from xkbcommon 1.7), and small
//! handwritten ones for the corners.

use super::*;
use crate::wayland::fixtures::{DE_RU_KEYMAP, keymap};

const MOD2: u32 = 1 << 4;
const MOD5: u32 = 1 << 7;

const KEY_1: u32 = 10;
const KEY_3: u32 = 12;
const KEY_SSHARP: u32 = 20;
const KEY_Q: u32 = 24;
const KEY_Z: u32 = 29;
const KEY_A: u32 = 38;
const KEY_ODIAERESIS: u32 = 47;
const KEY_SHIFT_L: u32 = 50;
const KEY_CAPS: u32 = 66;
const KEY_KP1: u32 = 87;
const KEY_ESCAPE: u32 = 9;

#[test]
fn letters_follow_shift_and_caps_lock() {
    let keymap = keymap();
    assert_eq!(keymap.keysym(KEY_A, 0, 0), 0x61);
    assert_eq!(keymap.keysym(KEY_A, SHIFT, 0), 0x41);
    assert_eq!(keymap.keysym(KEY_A, LOCK, 0), 0x41);
    assert_eq!(keymap.keysym(KEY_Z, 0, 0), 0x7a, "QWERTZ: y and z swap");
    assert_eq!(keymap.keysym(KEY_ODIAERESIS, 0, 0), 0xf6);
    assert_eq!(keymap.keysym(KEY_ODIAERESIS, LOCK, 0), 0xd6);
    assert_eq!(keymap.keysym(KEY_1, SHIFT, 0), 0x21, "exclam");
    assert_eq!(
        keymap.keysym(KEY_1, LOCK, 0),
        0x31,
        "Caps Lock leaves digits alone"
    );
    assert_eq!(keymap.keysym(KEY_3, SHIFT, 0), 0xa7, "section");
}

#[test]
fn altgr_selects_the_third_and_fourth_levels() {
    let keymap = keymap();
    assert_eq!(keymap.keysym(KEY_Q, MOD5, 0), 0x40, "at");
    assert_eq!(keymap.keysym(KEY_Q, MOD5 | SHIFT, 0), 0x7d9, "Greek_OMEGA");
    assert_eq!(keymap.keysym(KEY_Q, MOD1, 0), 0x71, "Alt changes nothing");
    assert_eq!(
        keymap.keysym(KEY_Q, MOD5 | LOCK, 0),
        0x40,
        "Caps Lock keeps @"
    );
    assert_eq!(
        keymap.keysym(KEY_SSHARP, MOD5 | SHIFT, 0),
        0xbf,
        "questiondown, of an explicit five-level type"
    );
}

#[test]
fn the_second_layout_is_russian() {
    let keymap = keymap();
    assert_eq!(keymap.keysym(KEY_Q, 0, 1), 0x6ca, "Cyrillic_shorti");
    assert_eq!(keymap.keysym(KEY_Q, SHIFT, 1), 0x6ea, "Cyrillic_SHORTI");
    assert_eq!(keymap.keysym(KEY_A, LOCK, 1), 0x6e6, "Cyrillic_EF");
    assert_eq!(keymap.keysym(KEY_3, SHIFT, 1), 0x6b0, "numerosign");
    assert_eq!(
        keymap.keysym(KEY_ESCAPE, 0, 1),
        0xff1b,
        "one-group keys wrap"
    );
    assert_eq!(keymap.keysym(KEY_A, 0, 7), 0x6c6, "layouts wrap around");
}

#[test]
fn num_lock_selects_keypad_digits() {
    let keymap = keymap();
    assert_eq!(keymap.keysym(KEY_KP1, 0, 0), 0xff9c, "KP_End");
    assert_eq!(keymap.keysym(KEY_KP1, MOD2, 0), 0xffb1, "KP_1");
    assert_eq!(keymap.keysym(KEY_KP1, MOD2 | SHIFT, 0), 0xff9c);
}

#[test]
fn modifiers_and_repeat() {
    let keymap = keymap();
    assert_eq!(
        keymap.modifiers(SHIFT | CONTROL | MOD1 | MOD4),
        Modifiers::SHIFT | Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER
    );
    assert_eq!(keymap.modifiers(LOCK | MOD2 | MOD5), Modifiers::NONE);
    assert!(keymap.repeats(KEY_A));
    assert!(keymap.repeats(KEY_KP1));
    assert!(!keymap.repeats(KEY_SHIFT_L));
    assert!(!keymap.repeats(KEY_CAPS));
    assert!(!keymap.repeats(9999));
    assert_eq!(keymap.keysym(9999, 0, 0), NO_SYMBOL);
}

#[test]
fn virtual_modifiers_map_explicitly_or_through_keys() {
    let text = r#"xkb_keymap {
        xkb_keycodes { <A> = 10; <B> = 11; <C> = 12; <L3> = 13; };
        xkb_types {
            virtual_modifiers LevelThree,Alt=Mod2,Unbound;
            type "THREE" { modifiers= Shift+LevelThree+Unbound;
                map[Shift]= 2; map[LevelThree]= 3; map[Unbound]= 2; };
        };
        xkb_compat {
            interpret ISO_Level3_Shift+AnyOf(all) { virtualModifier= LevelThree; };
        };
        xkb_symbols {
            key <A> { type= "THREE", [ a, b, c ] };
            key <B> { [ x ] };
            key <L3> { [ ISO_Level3_Shift ] };
            key <C> { virtualMods= LevelThree, [ y, Y ] };
            modifier_map Mod5 { <L3> };
        };
    };"#;
    let keymap = Keymap::parse(text).unwrap();
    assert_eq!(
        keymap.keysym(10, MOD5, 0),
        0x63,
        "LevelThree → Mod5 through <L3>"
    );
    assert_eq!(keymap.keysym(10, SHIFT, 0), 0x62);
    assert_eq!(
        keymap.keysym(10, 0, 0),
        0x61,
        "the unbound entry never matches"
    );
    assert_eq!(keymap.modifiers(MOD2), Modifiers::ALT, "Alt=Mod2");
    assert_eq!(keymap.modifiers(MOD1), Modifiers::NONE);
    assert_eq!(
        keymap.keysym(12, SHIFT, 0),
        0x59,
        "an automatic ALPHABETIC type"
    );
    assert_eq!(keymap.keysym(11, SHIFT, 0), 0x78);
}

#[test]
fn virtual_modifier_bits_from_the_compositor_count() {
    let keymap = keymap();
    // `LevelThree` is the third virtual modifier declared: bit 10.
    assert_eq!(keymap.keysym(KEY_Q, 1 << 10, 0), 0x40);
}

#[test]
fn missing_types_fall_back_to_the_standard_ones() {
    let text = "xkb_keymap { xkb_keycodes { <A> = 10; }; \
                xkb_symbols { key <A> { [ a, A ] }; key <B> { type= \"NOPE\", [ b, B ] }; }; };";
    let keymap = Keymap::parse(text).unwrap();
    assert_eq!(keymap.keysym(10, SHIFT, 0), 0x41);
    assert_eq!(keymap.keysym(10, LOCK, 0), 0x41);
    assert!(keymap.repeats(10));
}

#[test]
fn broken_keymaps_are_errors() {
    assert!(Keymap::parse("").is_err());
    assert!(Keymap::parse("xkb_keymap { xkb_symbols { key <A> { [ a ] }; };").is_err());
    assert!(Keymap::parse(&DE_RU_KEYMAP[..DE_RU_KEYMAP.len() / 2]).is_err());
}
