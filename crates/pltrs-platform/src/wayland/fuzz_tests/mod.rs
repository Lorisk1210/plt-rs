//! Randomised robustness tests of what a compositor sends: a broken or
//! malicious compositor can send anything. The message parsers get
//! random and cut-off bytes ([`messages`]); keymaps are mutated and
//! made up ([`keymap`]); then the scripted compositor drives a whole
//! backend and slips random events, descriptors and garbage in
//! ([`compositor`]). Nothing may panic, hang or allocate without bound.
//!
//! The targets run on pltrs's fuzz harness ([`crate::fuzz`]), so
//! `PLTRS_FUZZ_SCALE`, `PLTRS_FUZZ_SEED` and `PLTRS_FUZZ_REPLAY` apply.

mod compositor;
mod keymap;
mod messages;

use crate::fuzz::Gen;
use crate::wayland::wire::Request;

/// A 32-bit value biased towards the ones that break arithmetic and
/// lookups: zero, all ones, single bits, small numbers.
fn word(g: &mut Gen) -> u32 {
    match g.below(6) {
        0 => g.pick(&[
            0,
            1,
            u32::MAX,
            u32::MAX - 1,
            0x8000_0000,
            0x7FFF_FFFF,
            0xff00_0000,
        ]),
        1 => 1 << g.below(32),
        2 => g.below(300) as u32,
        _ => (g.below(1 << 16) as u32) << 16 | g.below(1 << 16) as u32,
    }
}

/// Strings compositors send: interface and MIME names, tokens.
const STRINGS: [&str; 10] = [
    "wl_compositor",
    "wl_seat",
    "wl_output",
    "xdg_wm_base",
    "text/plain;charset=utf-8",
    "UTF8_STRING",
    "image/png",
    "token",
    "",
    "\u{fffd}é\u{1F600}",
];

/// An argument of a made-up message.
#[derive(Debug, Clone)]
enum Arg {
    Word(u32),
    Text(String),
    Bytes(Vec<u8>),
}

/// The arguments of a made-up message: words, strings and arrays in any
/// order.
fn args(g: &mut Gen) -> Vec<Arg> {
    (0..g.len(6))
        .map(|_| match g.below(6) {
            0 => Arg::Text(if g.chance(0.5) {
                g.pick(&STRINGS).to_owned()
            } else {
                g.soup(&STRINGS, 4)
            }),
            1 => Arg::Bytes((0..g.len(24)).map(|_| g.below(256) as u8).collect()),
            _ => Arg::Word(word(g)),
        })
        .collect()
}

/// The message `opcode` to `object` with `args`.
fn message(object: u32, opcode: u16, args: &[Arg]) -> Request {
    args.iter()
        .fold(Request::new(object, opcode), |request, arg| match arg {
            Arg::Word(word) => request.uint(*word),
            Arg::Text(text) => request.string(text),
            Arg::Bytes(bytes) => request.array(bytes),
        })
}
