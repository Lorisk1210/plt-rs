//! Replies, events and the keyboard mapping: replies with random fields,
//! lying lengths and cut short; random events of every code, sent ones
//! included, translated into input through the captured keymap; the
//! captured keyboard and modifier mappings mutated; and keysyms of every
//! range.

use super::{reply, word};
use crate::event::ButtonState;
use crate::fuzz::{Corruption, Gen, fuzz};
use crate::keysym;
use crate::x11::connection::RawEvent;
use crate::x11::fixtures::{self, KEYBOARD_MAPPING, MODIFIER_MAPPING};
use crate::x11::input::Input;
use crate::x11::keymap::Keymap;
use crate::x11::reply as parse;
use crate::x11::server_event::ServerEvent;

/// A reply as a server may send it.
#[derive(Debug)]
struct Reply {
    detail: u8,
    fields: [u32; 3],
    data_len: usize,
    /// A length word that disagrees with the data, if any.
    claimed: Option<u32>,
    /// Where the reply is cut off, if anywhere.
    cut: Option<usize>,
}

impl Reply {
    fn generate(g: &mut Gen) -> Self {
        Self {
            detail: g.pick(&[0, 1, 8, 16, 24, 32, 7, 255]),
            fields: [word(g), word(g), word(g)],
            data_len: g.pick(&[0, 1, 3, 4, 5, 64, 1000]),
            claimed: g.chance(0.2).then(|| word(g)),
            cut: g.chance(0.2).then(|| g.below(40)),
        }
    }

    fn bytes(&self) -> Vec<u8> {
        let data: Vec<u8> = (0..self.data_len).map(|i| i as u8).collect();
        let mut bytes = reply(self.detail, 0, &self.fields, &data);
        if let Some(claimed) = self.claimed {
            bytes[4..8].copy_from_slice(&claimed.to_le_bytes());
        }
        if let Some(cut) = self.cut {
            bytes.truncate(cut);
        }
        bytes
    }
}

#[test]
fn replies() {
    fuzz("x11 replies", 3_000, Reply::generate, |reply| {
        let bytes = reply.bytes();
        let _ = parse::intern_atom(&bytes);
        let _ = parse::query_extension(&bytes);
        let _ = parse::big_requests_enable(&bytes);
        if let Ok(data) = parse::image_data(&bytes) {
            assert_eq!(data.len() + 32, bytes.len());
        }
        if let Ok(property) = parse::property(&bytes) {
            let units = usize::try_from(reply.fields[2]).expect("32-bit units fit");
            assert_eq!(
                property.value.len(),
                units * usize::from(property.format / 8)
            );
        }
    });
}

/// The event codes the backend reads, sent ones (high bit set), and
/// others.
const EVENT_CODES: [u8; 22] = [
    2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 22, 28, 29, 30, 31, 33, 34, 35, 0x82, 0xA1, 0x7F,
];

/// A random event: a code the backend reads (or another), random fields
/// and pointer coordinates at the extremes.
fn event(g: &mut Gen) -> RawEvent {
    let mut raw = [0; 32];
    for byte in &mut raw {
        *byte = g.below(256) as u8;
    }
    raw[0] = if g.chance(0.9) {
        g.pick(&EVENT_CODES)
    } else {
        raw[0]
    };
    if g.chance(0.5) {
        // Common details: buttons, keycodes, focus details, formats.
        raw[1] = g.pick(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 32, 38, 255]);
    }
    raw
}

/// Translates `event` into input as the backend does.
fn translate(input: &mut Input, event: &ServerEvent) {
    match *event {
        ServerEvent::Key {
            state,
            keycode,
            pointer,
        } => {
            let key = input.key(state, keycode, pointer.mask, false);
            if state == ButtonState::Released {
                assert!(key.text.is_none(), "releases type nothing");
            }
            if let Some(text) = &key.text {
                assert!(!text.chars().any(char::is_control));
            }
        }
        ServerEvent::Button {
            state,
            button,
            pointer,
        } => {
            let _ = input.button(state, button, &pointer);
        }
        ServerEvent::Motion(pointer) | ServerEvent::Crossing { pointer, .. } => {
            let _ = input.motion(&pointer);
        }
        _ => {}
    }
}

#[test]
fn server_events() {
    fuzz(
        "x11 events",
        2_000,
        |g| (0..=g.len(16)).map(|_| event(g)).collect::<Vec<_>>(),
        |events| {
            let mut input = Input::new(fixtures::keymap());
            for raw in events {
                translate(&mut input, &ServerEvent::parse(raw));
            }
        },
    );
}

/// Mutated keyboard and modifier mappings, and the first keycode.
#[derive(Debug)]
struct Mappings {
    min_keycode: u8,
    keyboard: Option<Corruption>,
    modifiers: Option<Corruption>,
}

#[test]
fn keyboard_mappings() {
    fuzz(
        "x11 keymaps",
        2_000,
        |g| {
            let keyboard = g.chance(0.7).then(|| g.corrupt(KEYBOARD_MAPPING));
            let modifiers =
                (keyboard.is_none() || g.chance(0.3)).then(|| g.corrupt(MODIFIER_MAPPING));
            let any = g.below(256) as u8;
            let mappings = Mappings {
                min_keycode: g.pick(&[8, 8, 0, 1, 254, 255, any]),
                keyboard,
                modifiers,
            };
            let keys: Vec<(u8, u16)> = (0..20)
                .map(|_| (g.below(256) as u8, g.below(1 << 16) as u16))
                .collect();
            (mappings, keys)
        },
        |(mappings, keys)| {
            let apply = |corruption: &Option<Corruption>, bytes: &[u8]| {
                corruption
                    .as_ref()
                    .map_or_else(|| bytes.to_vec(), |c| c.apply(bytes))
            };
            let keyboard = apply(&mappings.keyboard, KEYBOARD_MAPPING);
            let modifiers = apply(&mappings.modifiers, MODIFIER_MAPPING);
            let Ok(keymap) = Keymap::parse(mappings.min_keycode, &keyboard, &modifiers) else {
                return;
            };
            let mut input = Input::new(keymap.clone());
            for &(keycode, state) in keys {
                let _ = keymap.keysym(keycode, state);
                let _ = keymap.modifiers(state);
                let _ = input.key(ButtonState::Pressed, keycode, state, false);
            }
            input.set_keymap(keymap);
        },
    );
}

/// A keysym from every range: Latin-1, the legacy tables, function and
/// keypad keys, Unicode keysyms and beyond.
fn any_keysym(g: &mut Gen) -> u32 {
    match g.below(6) {
        0 => g.below(0x100) as u32,
        1 => 0x100 + g.below(0x2000) as u32,
        2 => 0xFE00 + g.below(0x200) as u32,
        3 => 0x0100_0000 + g.below(0x11_0001) as u32,
        _ => word(g),
    }
}

#[test]
fn keysyms() {
    fuzz("x11 keysyms", 5_000, any_keysym, |&keysym| {
        let _ = keysym::key(keysym);
        let upper = keysym::to_upper(keysym);
        let lower = keysym::to_lower(keysym);
        assert_eq!(keysym::has_case(keysym), upper != lower);
        if let Some(c) = keysym::to_char(keysym) {
            assert_eq!(keysym::to_char(keysym::from_char(c)), Some(c));
        }
    });
}
