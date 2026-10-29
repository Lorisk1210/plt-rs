//! WebSocket frames: text, binary, continuation and control frames,
//! masked or not, with reserved bits and opcodes, lengths in every
//! encoding up to 2^64 - 1 that may disagree with the payload, close
//! codes valid and not, and text that is not UTF-8 or splits a character
//! across fragments; read in pieces of any size and joined into messages
//! within a size limit.

use super::Trickle;
use super::messages;
use crate::fuzz::{Corruption, Gen, fuzz};
use crate::web::connection::MAX_MESSAGE;
use crate::web::websocket::frame::{MAX_CONTROL_PAYLOAD, apply_mask};
use crate::web::websocket::{Message, MessageReader, WsError};

/// The first byte of a frame: final bit, reserved bits and opcode.
const FIRST_BYTES: [u8; 12] = [
    0x81, // final text
    0x01, // first fragment of a text
    0x80, // final continuation
    0x00, // continuation
    0x82, // binary
    0x88, // close
    0x89, // ping
    0x8A, // pong
    0x09, // fragmented ping
    0x83, // reserved opcode
    0xC1, // reserved bit
    0x8B, // reserved control opcode
];

/// What a frame carries.
#[derive(Clone, Debug)]
pub(super) enum Payload {
    /// A message of the client, as JSON text.
    Message(String),
    /// Text, possibly cut inside a character.
    Text(String, usize),
    /// Bytes that are not UTF-8.
    Invalid(Vec<u8>),
    /// A close frame's status code and reason.
    Close(u16, String),
    /// `.0` zero bytes.
    Zeros(usize),
}

impl Payload {
    fn generate(g: &mut Gen) -> Self {
        match g.below(8) {
            0..=2 => Self::Message(messages::message(g)),
            3 => {
                let text = g.soup(&["é", "€", "😀", "a", " ", "\u{301}", "{\"type\":"], 12);
                let cut = g.below(text.len() + 1);
                Self::Text(text, cut)
            }
            4 => Self::Invalid(vec![
                g.pick(&[0xFF, 0xC0, 0xED, 0x80, 0xF4]);
                1 + g.below(4)
            ]),
            5 => Self::Close(
                g.pick(&[
                    1000, 1001, 1003, 1004, 1005, 1006, 1015, 2999, 3000, 4999, 5000, 0,
                ]),
                g.soup(&["bye", "é", "\u{FFFD}"], 3),
            ),
            _ => Self::Zeros(g.pick(&[0, 1, 125, 126, 127, 65535, 65536, MAX_MESSAGE + 1])),
        }
    }

    fn bytes(&self) -> Vec<u8> {
        match self {
            Self::Message(text) => text.clone().into_bytes(),
            Self::Text(text, cut) => text.as_bytes()[..*cut].to_vec(),
            Self::Invalid(bytes) => bytes.clone(),
            Self::Close(code, reason) => {
                let mut bytes = code.to_be_bytes().to_vec();
                bytes.extend_from_slice(reason.as_bytes());
                bytes
            }
            Self::Zeros(len) => vec![0; *len],
        }
    }
}

/// How a frame states its payload length.
#[derive(Clone, Copy, Debug)]
pub(super) enum Length {
    /// The payload's length, in the shortest encoding.
    Actual,
    /// The payload's length in the 64-bit encoding, as RFC 6455 forbids
    /// for short payloads but readers must accept.
    Long,
    /// Another length: `.0` in the 7-bit field (126 and 127 announce
    /// the extended lengths) and `.1` as the extended length.
    Claimed(u8, u64),
}

/// One frame.
#[derive(Clone, Debug)]
pub(super) struct Frame {
    first: u8,
    masked: bool,
    length: Length,
    payload: Payload,
}

impl Frame {
    /// A random frame.
    pub(super) fn generate(g: &mut Gen) -> Self {
        let length = match g.below(10) {
            0 => Length::Long,
            1 => Length::Claimed(
                g.pick(&[0, 5, 125, 126, 127]),
                g.pick(&[
                    0,
                    1,
                    125,
                    126,
                    65_535,
                    65_536,
                    1 << 31,
                    1 << 62,
                    1 << 63,
                    u64::MAX,
                ]),
            ),
            _ => Length::Actual,
        };
        Self {
            first: if g.chance(0.7) {
                FIRST_BYTES[g.below(3)]
            } else {
                g.pick(&FIRST_BYTES)
            },
            masked: g.chance(0.95),
            length,
            payload: Payload::generate(g),
        }
    }

    /// The frame as a client sends it.
    pub(super) fn bytes(&self, key: [u8; 4]) -> Vec<u8> {
        let mut payload = self.payload.bytes();
        let mask_bit = if self.masked { 0x80 } else { 0 };
        let mut out = vec![self.first];
        let (short, extended) = match self.length {
            Length::Actual => match payload.len() {
                len @ 0..126 => (len as u8, None),
                len @ 126..65_536 => (126, Some(len as u64)),
                len => (127, Some(len as u64)),
            },
            Length::Long => (127, Some(payload.len() as u64)),
            Length::Claimed(short, extended) => (short.min(127), Some(extended)),
        };
        out.push(mask_bit | short);
        match (short, extended) {
            (126, Some(length)) => out.extend_from_slice(&(length as u16).to_be_bytes()),
            (127, Some(length)) => out.extend_from_slice(&length.to_be_bytes()),
            _ => {}
        }
        if self.masked {
            out.extend_from_slice(&key);
            apply_mask(&mut payload, key);
        }
        out.extend_from_slice(&payload);
        out
    }
}

/// What a client sends after the handshake: frames, maybe corrupted.
#[derive(Clone, Debug)]
pub(super) struct Frames {
    sequence: Vec<Frame>,
    key: [u8; 4],
    corruption: Option<Corruption>,
}

impl Frames {
    /// Random frames, mostly well formed.
    pub(super) fn generate(g: &mut Gen) -> Self {
        let sequence: Vec<Frame> = (0..=g.len(8)).map(|_| Frame::generate(g)).collect();
        let key = [0, 1, 2, 3].map(|_| g.below(256) as u8);
        let corruption = g.chance(0.2).then(|| {
            let bytes = Self {
                sequence: sequence.clone(),
                key,
                corruption: None,
            }
            .bytes();
            g.corrupt(&bytes)
        });
        Self {
            sequence,
            key,
            corruption,
        }
    }

    /// The bytes sent.
    pub(super) fn bytes(&self) -> Vec<u8> {
        let bytes: Vec<u8> = self
            .sequence
            .iter()
            .flat_map(|frame| frame.bytes(self.key))
            .collect();
        match &self.corruption {
            Some(corruption) => corruption.apply(&bytes),
            None => bytes,
        }
    }
}

/// Reads the messages in `bytes`, `chunk` bytes arriving at a time, as a
/// connection does: until the input ends, the client closes, or an error
/// that closes the connection with a code.
pub(super) fn read_messages(bytes: &[u8], chunk: usize, max_message: usize) {
    let mut input = Trickle::new(bytes, chunk);
    let mut reader = MessageReader::new(max_message);
    // Every message read takes at least two bytes.
    for _ in 0..=bytes.len() {
        match reader.next(&mut input) {
            Ok(Message::Text(text)) => assert!(text.len() <= max_message),
            Ok(Message::Ping(payload)) => assert!(payload.len() <= MAX_CONTROL_PAYLOAD),
            Ok(Message::Pong) => {}
            Ok(Message::Close(code)) => {
                assert!(code.is_none_or(|code| (1000..5000).contains(&code)));
                return;
            }
            Err(WsError::Io(_)) => return,
            Err(err) => {
                assert!(err.close_code().is_some(), "{err} closes with a code");
                return;
            }
        }
    }
    panic!("messages without end");
}

#[test]
fn websocket_frames() {
    fuzz(
        "web frames",
        3_000,
        |g| {
            let max = g.pick(&[0, 1, 125, 1024, MAX_MESSAGE]);
            (Frames::generate(g), 1 + g.below(4) * g.below(100), max)
        },
        |(frames, chunk, max)| read_messages(&frames.bytes(), *chunk, *max),
    );
}
