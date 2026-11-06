//! Randomised robustness tests of what a bus sends: messages corrupted
//! or made up, and values read with any signature. Nothing may panic,
//! hang or allocate without bound.
//!
//! The targets run on pltrs's fuzz harness ([`crate::fuzz`]), so
//! `PLTRS_FUZZ_SCALE`, `PLTRS_FUZZ_SEED` and `PLTRS_FUZZ_REPLAY` apply.

use super::message::{self, FIXED_HEADER, MethodCall, message_length};
use super::value::{Reader, types};
use crate::fuzz::{Corruption, Gen, fuzz};

/// A message to corrupt: a call with a body, as the client writes one.
fn sample() -> Vec<u8> {
    MethodCall {
        destination: "org.freedesktop.portal.Desktop",
        path: "/org/freedesktop/portal/desktop",
        interface: "org.freedesktop.portal.Settings",
        member: "ReadAll",
        strings: Some(&["org.gnome.desktop.wm.preferences", "a"]),
    }
    .encode(3)
}

/// Pieces of signatures.
const SIGNATURE_PIECES: [&str; 16] = [
    "a", "{", "}", "(", ")", "s", "v", "y", "u", "t", "d", "g", "o", "b", "h", "z",
];

/// What is read.
#[derive(Debug)]
enum Input {
    /// The sample, corrupted, read as a message.
    Message(Corruption),
    /// Random bytes read with a signature.
    Values { bytes: Vec<u8>, signature: String },
}

fn generate(g: &mut Gen) -> Input {
    if g.chance(0.5) {
        return Input::Message(g.corrupt(&sample()));
    }
    let bytes = (0..g.len(64)).map(|_| g.below(256) as u8).collect();
    Input::Values {
        bytes,
        signature: g.soup(&SIGNATURE_PIECES, 12),
    }
}

#[test]
fn bus_messages() {
    fuzz("d-bus messages", 800, generate, |input| match input {
        Input::Message(corruption) => {
            let bytes = corruption.apply(&sample());
            if bytes.len() >= FIXED_HEADER {
                let _ = message_length(&bytes[..FIXED_HEADER]);
            }
            let _ = message::parse(&bytes);
        }
        Input::Values { bytes, signature } => {
            let Ok(types) = types(signature.as_bytes()) else {
                return;
            };
            for big_endian in [false, true] {
                let mut reader = Reader::new(bytes, 0, big_endian);
                for value in &types {
                    if reader.value(value).is_err() {
                        break;
                    }
                }
            }
        }
    });
}
