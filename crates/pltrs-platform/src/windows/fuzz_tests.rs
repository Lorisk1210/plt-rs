//! Randomised robustness tests of the Windows backend's decoding, which
//! holds no Win32 calls and so runs on every system: runs of window
//! messages (the ones the window procedure handles, with any parameters,
//! and any others) are decoded, and their key messages, the left Control
//! Windows sends ahead of `AltGr` among them, go through the `AltGr`
//! tracking into key events, with any modifiers held, characters and
//! typed text; the UTF-16 the system hands over (unpaired surrogates,
//! NULs, any count `ToUnicodeEx` reports), icons of any size, DPIs and
//! wait deadlines are converted. Nothing may panic.
//!
//! The targets run on pltrs's fuzz harness ([`crate::fuzz`]), so
//! `PLTRS_FUZZ_SCALE`, `PLTRS_FUZZ_SEED` and `PLTRS_FUZZ_REPLAY` apply.

use std::time::{Duration, Instant};

use super::convert::{
    from_wide, icon_pixels, mask_len, scale_factor, single_character, timeout_ms, to_wide,
    typed_text,
};
use super::keys::{AltGrTracker, HeldModifiers, KeyRole, Typed, key_event, vk};
use super::message::{Message, is_key, wm};
use crate::backend::Wait;
use crate::fuzz::{Gen, fuzz};

/// The messages the window procedure handles, and a few it leaves alone.
const MESSAGES: [u32; 33] = [
    wm::SIZE,
    wm::SETFOCUS,
    wm::KILLFOCUS,
    wm::PAINT,
    wm::CLOSE,
    wm::ERASEBKGND,
    wm::SETCURSOR,
    wm::GETMINMAXINFO,
    wm::KEYDOWN,
    wm::KEYUP,
    wm::CHAR,
    wm::DEADCHAR,
    wm::SYSKEYDOWN,
    wm::SYSKEYUP,
    wm::SYSCHAR,
    wm::MOUSEMOVE,
    wm::LBUTTONDOWN,
    wm::LBUTTONUP,
    wm::RBUTTONDOWN,
    wm::RBUTTONUP,
    wm::MBUTTONDOWN,
    wm::MBUTTONUP,
    wm::MOUSEWHEEL,
    wm::XBUTTONDOWN,
    wm::XBUTTONUP,
    wm::MOUSEHWHEEL,
    wm::MOUSELEAVE,
    wm::DPICHANGED,
    wm::WAKE,
    0x0000,
    0x0001,
    0x0400,
    u32::MAX,
];

/// The virtual keys of the modifiers (both sides and either), which the
/// `AltGr` tracking and the held modifiers look at.
const MODIFIER_KEYS: [u16; 13] = [
    vk::SHIFT,
    vk::LSHIFT,
    vk::RSHIFT,
    vk::CONTROL,
    vk::LCONTROL,
    vk::RCONTROL,
    vk::MENU,
    vk::LMENU,
    vk::RMENU,
    vk::LWIN,
    vk::RWIN,
    vk::F10,
    vk::F24,
];

/// A raw message: `(message, wParam, lParam, time)`.
type Raw = (u32, usize, isize, u32);

/// A machine word biased towards the values that break decoding: zero,
/// all ones, single bits, packed words with their signs set.
fn word(g: &mut Gen) -> usize {
    match g.below(5) {
        0 => g.pick(&[0, 1, usize::MAX, 0xFFFF, 0x8000_8000, 0x7FFF_7FFF]),
        1 => 1 << g.below(usize::BITS as usize),
        2 => g.below(0x1_0000) << 16 | g.below(0x1_0000),
        _ => g.below(usize::MAX),
    }
}

/// A run of messages; key presses and releases come in pairs at the same
/// time half the time, as Windows sends the left Control of `AltGr`.
fn messages(g: &mut Gen) -> Vec<Raw> {
    let mut time = g.below(u32::MAX as usize) as u32;
    let mut out = Vec::new();
    for _ in 0..=g.len(40) {
        time = time.wrapping_add(g.pick(&[0, 0, 1, 16, u32::MAX]));
        let message = if g.chance(0.9) {
            g.pick(&MESSAGES)
        } else {
            g.below(0x1_0000) as u32
        };
        let wparam = if is_key(message) && g.chance(0.7) {
            usize::from(g.pick(&MODIFIER_KEYS))
        } else {
            word(g)
        };
        let lparam = word(g).cast_signed();
        if is_key(message) && g.chance(0.5) {
            // The left Control Windows sends ahead of `AltGr`.
            out.push((message, usize::from(vk::LCONTROL), lparam, time));
        }
        out.push((message, wparam, lparam, time));
    }
    out
}

/// What the window procedure holds while it reads the messages: the
/// keyboard's held keys, the character the layout gives, the text typed
/// and the accent of a dead key's `WM_DEADCHAR`.
#[derive(Debug)]
struct Keyboard {
    held: u32,
    character: Option<char>,
    typed: Vec<u16>,
    dead: Option<u16>,
}

/// A random keyboard state.
fn keyboard(g: &mut Gen) -> Keyboard {
    Keyboard {
        held: g.below(1 << 16) as u32,
        character: g.chance(0.7).then(|| g.character()),
        typed: units(g),
        dead: g.chance(0.2).then(|| g.below(1 << 16) as u16),
    }
}

/// UTF-16 units: text, unpaired surrogates, NULs.
fn units(g: &mut Gen) -> Vec<u16> {
    (0..g.len(12))
        .map(|_| match g.below(4) {
            0 => g.pick(&[0, 0xD800, 0xDBFF, 0xDC00, 0xDFFF, 0xFFFF, 0x7F, 0x0D]),
            1 => g.below(0x1_0000) as u16,
            _ => (b' ' + g.below(95) as u8).into(),
        })
        .collect()
}

/// Decodes the run, as the window procedure reads it: each message, and
/// each key message with the one after it (`PeekMessage`'s look ahead).
fn decode(run: &[Raw], keyboard: &Keyboard) {
    let mut tracker = AltGrTracker::default();
    let held = HeldModifiers::from_keys(|key| keyboard.held & (1 << (key % 16)) != 0);
    let keys: Vec<_> = run
        .iter()
        .filter_map(|&(message, wparam, lparam, time)| {
            match Message::decode(message, wparam, lparam, time) {
                Message::Key { key, .. } => Some(key),
                Message::Resized(Some(size)) => {
                    assert!(
                        u16::try_from(size.width).is_ok() && u16::try_from(size.height).is_ok()
                    );
                    None
                }
                _ => None,
            }
        })
        .collect();
    for (index, &key) in keys.iter().enumerate() {
        let role = tracker.classify(key, keys.get(index + 1).copied());
        if role == KeyRole::FakeControl {
            continue;
        }
        let event = key_event(
            key,
            role,
            tracker.held(),
            held,
            keyboard.character,
            Typed::from_messages(&keyboard.typed, keyboard.dead),
        );
        assert_eq!(event.state, key.state);
    }
}

#[test]
fn window_messages() {
    fuzz(
        "windows messages",
        3_000,
        |g| (messages(g), keyboard(g)),
        |(run, keyboard)| decode(run, keyboard),
    );
}

/// What the conversions get.
#[derive(Debug)]
struct Conversions {
    units: Vec<u16>,
    count: i32,
    text: String,
    rgba_len: usize,
    icon: (u32, u32),
    dpi: u32,
    deadline: i64,
}

/// Random input for every conversion.
fn conversions(g: &mut Gen) -> Conversions {
    Conversions {
        units: units(g),
        count: g.pick(&[-1, 0, 1, 2, 3, 12, 100, i32::MIN, i32::MAX]),
        text: g.soup(&["a", "\0", "é", "😀", "\u{FFFD}", "\r\n"], 8),
        rgba_len: g.len(64),
        icon: (
            g.pick(&[0, 1, 15, 16, 17, 256, u32::from(u16::MAX), u32::MAX]),
            g.pick(&[0, 1, 16, 256, u32::from(u16::MAX), u32::MAX]),
        ),
        dpi: g.pick(&[0, 1, 72, 96, 144, 192, 480, u32::MAX]),
        deadline: g.pick(&[i64::MIN, -1_000, -1, 0, 1, 999_999, 1_000_000, i64::MAX]),
    }
}

/// Converts the input: the results keep what Win32 expects of them.
fn convert(input: &Conversions) {
    let text = from_wide(&input.units);
    assert!(!text.contains('\0'));
    if let Some(typed) = typed_text(&input.units) {
        assert_ne!(typed, "");
    }
    let _ = single_character(&input.units, input.count);
    let wide = to_wide(&input.text);
    assert_eq!(wide.iter().filter(|&&unit| unit == 0).count(), 1);
    assert_eq!(
        icon_pixels(&vec![7; input.rgba_len]).len(),
        input.rgba_len / 4 * 4
    );
    let (width, height) = input.icon;
    let _ = mask_len(width.min(1 << 16), height.min(1 << 16));
    assert!(scale_factor(input.dpi) > 0.0);
    let now = Instant::now();
    let offset = Duration::from_nanos(input.deadline.unsigned_abs());
    let deadline = if input.deadline < 0 {
        now.checked_sub(offset)
    } else {
        now.checked_add(offset)
    };
    if let Some(deadline) = deadline {
        let _ = timeout_ms(Wait::Until(deadline), now);
    }
}

#[test]
fn conversions_of_system_data() {
    fuzz("windows conversions", 3_000, conversions, convert);
}
