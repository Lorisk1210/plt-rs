//! Randomised robustness tests of what an X server sends: a malicious or
//! broken server, or another client handing over a selection, can send
//! anything. The parsers of the setup reply and the pixel formats it
//! describes ([`setup`]), of replies, events and the keyboard mapping
//! ([`messages`]), and of clipboard data, resource databases,
//! Xauthority files and display names ([`clipboard`]) get random and
//! mutated bytes, starting from the captured fixtures; then a fake
//! server ([`server`]) answers a whole backend with fuzzed replies and
//! events. Nothing may panic, hang or allocate without bound.
//!
//! The targets run on plt's fuzz harness ([`crate::fuzz`]), so
//! `PLT_FUZZ_SCALE`, `PLT_FUZZ_SEED` and `PLT_FUZZ_REPLAY` apply.

mod clipboard;
mod messages;
mod server;
mod setup;

use crate::fuzz::Gen;

/// Length of reply headers.
const HEADER_LEN: usize = 32;

/// A reply: type 1, `detail`, `sequence`, the length of `data` in words,
/// then `fields` (in the rest of the header) and `data`, padded to whole
/// words.
fn reply(detail: u8, sequence: u16, fields: &[u32], data: &[u8]) -> Vec<u8> {
    let mut out = vec![1, detail];
    out.extend(sequence.to_le_bytes());
    let words = u32::try_from(data.len().div_ceil(4)).unwrap_or(u32::MAX);
    out.extend(words.to_le_bytes());
    for field in fields {
        out.extend(field.to_le_bytes());
    }
    out.resize(HEADER_LEN, 0);
    out.extend_from_slice(data);
    out.resize(HEADER_LEN + data.len().next_multiple_of(4), 0);
    out
}

/// A 32-bit value biased towards the ones that break arithmetic and
/// lookups: zero, all ones, single bits, small numbers.
fn word(g: &mut Gen) -> u32 {
    match g.below(6) {
        0 => g.pick(&[0, 1, u32::MAX, u32::MAX - 1, 0x8000_0000, 0x7FFF_FFFF]),
        1 => 1 << g.below(32),
        2 => g.below(300) as u32,
        _ => (g.below(1 << 16) as u32) << 16 | g.below(1 << 16) as u32,
    }
}
