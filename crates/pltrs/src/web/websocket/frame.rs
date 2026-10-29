//! WebSocket frames (RFC 6455 section 5): reading the masked frames of a
//! client, with size limits checked before anything is allocated, and
//! encoding the unmasked frames of the server.

use std::fmt;
use std::io::{self, Read};

/// The largest payload of a control frame (RFC 6455 section 5.5).
pub(crate) const MAX_CONTROL_PAYLOAD: usize = 125;

/// What a frame carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Opcode {
    /// The next fragment of a message.
    Continuation,
    /// A UTF-8 text message (or its first fragment).
    Text,
    /// A binary message (or its first fragment).
    Binary,
    /// The closing handshake.
    Close,
    /// A ping, answered with a pong.
    Ping,
    /// A pong.
    Pong,
}

impl Opcode {
    /// The opcode of the low four bits, unless reserved.
    fn from_bits(bits: u8) -> Option<Self> {
        Some(match bits {
            0x0 => Self::Continuation,
            0x1 => Self::Text,
            0x2 => Self::Binary,
            0x8 => Self::Close,
            0x9 => Self::Ping,
            0xA => Self::Pong,
            _ => return None,
        })
    }

    /// The four opcode bits.
    fn bits(self) -> u8 {
        match self {
            Self::Continuation => 0x0,
            Self::Text => 0x1,
            Self::Binary => 0x2,
            Self::Close => 0x8,
            Self::Ping => 0x9,
            Self::Pong => 0xA,
        }
    }

    /// Whether this is a control opcode (close, ping or pong).
    pub(crate) fn is_control(self) -> bool {
        matches!(self, Self::Close | Self::Ping | Self::Pong)
    }
}

/// A frame received from a client, unmasked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Frame {
    /// Whether this is the final fragment of its message.
    pub(crate) fin: bool,
    /// What the frame carries.
    pub(crate) opcode: Opcode,
    /// The unmasked payload.
    pub(crate) payload: Vec<u8>,
}

/// Why a WebSocket connection failed.
#[derive(Debug)]
pub(crate) enum WsError {
    /// Reading or writing failed, or the peer closed the connection.
    Io(io::Error),
    /// The peer broke the framing rules (close code 1002).
    Protocol(&'static str),
    /// A message exceeds the size limit (close code 1009).
    TooBig,
    /// A text message or close reason is not UTF-8 (close code 1007).
    InvalidUtf8,
    /// A binary message, which the protocol never uses (close code 1003).
    Unsupported,
    /// A text message is not a valid client message (close code 1008).
    BadMessage(String),
}

impl fmt::Display for WsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "connection error: {err}"),
            Self::Protocol(why) => write!(f, "WebSocket protocol error: {why}"),
            Self::TooBig => f.write_str("WebSocket message too big"),
            Self::InvalidUtf8 => f.write_str("WebSocket text is not UTF-8"),
            Self::Unsupported => f.write_str("binary WebSocket messages are not supported"),
            Self::BadMessage(why) => write!(f, "invalid message: {why}"),
        }
    }
}

impl std::error::Error for WsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for WsError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl WsError {
    /// The close code to send the peer, unless the connection is gone.
    pub(crate) fn close_code(&self) -> Option<u16> {
        match self {
            Self::Io(_) => None,
            Self::Protocol(_) => Some(1002),
            Self::Unsupported => Some(1003),
            Self::InvalidUtf8 => Some(1007),
            Self::BadMessage(_) => Some(1008),
            Self::TooBig => Some(1009),
        }
    }
}

/// XORs `data` with the four-byte masking `key` (RFC 6455 section 5.3).
pub(crate) fn apply_mask(data: &mut [u8], key: [u8; 4]) {
    for (byte, mask) in data.iter_mut().zip(key.iter().cycle()) {
        *byte ^= mask;
    }
}

/// Reads one frame of a client: masked, without reserved bits or opcodes,
/// with control frames whole and short, and a data payload of at most
/// `max_payload` bytes.
pub(crate) fn read_frame(reader: &mut impl Read, max_payload: usize) -> Result<Frame, WsError> {
    let mut head = [0; 2];
    reader.read_exact(&mut head)?;
    let fin = head[0] & 0x80 != 0;
    if head[0] & 0x70 != 0 {
        return Err(WsError::Protocol("reserved bits set"));
    }
    let opcode = Opcode::from_bits(head[0] & 0x0F).ok_or(WsError::Protocol("reserved opcode"))?;
    if head[1] & 0x80 == 0 {
        return Err(WsError::Protocol("client frames must be masked"));
    }
    let length = match head[1] & 0x7F {
        126 => {
            let mut bytes = [0; 2];
            reader.read_exact(&mut bytes)?;
            u64::from(u16::from_be_bytes(bytes))
        }
        127 => {
            let mut bytes = [0; 8];
            reader.read_exact(&mut bytes)?;
            let length = u64::from_be_bytes(bytes);
            if length >> 63 != 0 {
                return Err(WsError::Protocol("payload length out of range"));
            }
            length
        }
        short => u64::from(short),
    };
    if opcode.is_control() {
        if !fin {
            return Err(WsError::Protocol("fragmented control frame"));
        }
        if length > MAX_CONTROL_PAYLOAD as u64 {
            return Err(WsError::Protocol("control frame too long"));
        }
    } else if length > max_payload as u64 {
        return Err(WsError::TooBig);
    }
    let length = usize::try_from(length).map_err(|_| WsError::TooBig)?;
    let mut key = [0; 4];
    reader.read_exact(&mut key)?;
    let mut payload = vec![0; length];
    reader.read_exact(&mut payload)?;
    apply_mask(&mut payload, key);
    Ok(Frame {
        fin,
        opcode,
        payload,
    })
}

/// The header of an unmasked, final server frame with a payload of
/// `length` bytes.
pub(crate) fn header(opcode: Opcode, length: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(10);
    out.push(0x80 | opcode.bits());
    match u16::try_from(length) {
        Ok(short) if short < 126 => out.push(short as u8),
        Ok(medium) => {
            out.push(126);
            out.extend_from_slice(&medium.to_be_bytes());
        }
        Err(_) => {
            out.push(127);
            out.extend_from_slice(&(length as u64).to_be_bytes());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 6455 section 5.7's masking key.
    const KEY: [u8; 4] = [0x37, 0xfa, 0x21, 0x3d];

    fn read(bytes: &[u8]) -> Result<Frame, WsError> {
        read_frame(&mut &bytes[..], 1 << 20)
    }

    #[test]
    fn rfc_6455_examples_server_side() {
        // A single-frame unmasked text message and an unmasked ping of
        // "Hello", as the server sends them: the header, then the text.
        assert_eq!(header(Opcode::Text, 5), [0x81, 0x05]);
        assert_eq!(header(Opcode::Ping, 5), [0x89, 0x05]);
        // 256 bytes and 64 KiB of binary data in a single frame.
        assert_eq!(header(Opcode::Binary, 256), [0x82, 0x7E, 0x01, 0x00]);
        assert_eq!(
            header(Opcode::Binary, 65536),
            [0x82, 0x7F, 0, 0, 0, 0, 0, 0x01, 0x00, 0x00]
        );
        assert_eq!(header(Opcode::Close, 125), [0x88, 125]);
        assert_eq!(header(Opcode::Binary, 65535), [0x82, 0x7E, 0xff, 0xff]);
    }

    #[test]
    fn rfc_6455_examples_client_side() {
        // A single-frame masked text message.
        let hello = [
            0x81, 0x85, 0x37, 0xfa, 0x21, 0x3d, 0x7f, 0x9f, 0x4d, 0x51, 0x58,
        ];
        let frame = read(&hello).unwrap();
        assert!(frame.fin);
        assert_eq!(frame.opcode, Opcode::Text);
        assert_eq!(frame.payload, b"Hello");
        // A masked pong carrying "Hello".
        let pong = [
            0x8a, 0x85, 0x37, 0xfa, 0x21, 0x3d, 0x7f, 0x9f, 0x4d, 0x51, 0x58,
        ];
        let frame = read(&pong).unwrap();
        assert_eq!(
            (frame.opcode, frame.payload.as_slice()),
            (Opcode::Pong, &b"Hello"[..])
        );
        // A fragmented text message, masked: "Hel" then "lo".
        let mut first = vec![0x01, 0x83];
        first.extend_from_slice(&KEY);
        let mut hel = b"Hel".to_vec();
        apply_mask(&mut hel, KEY);
        first.extend_from_slice(&hel);
        let frame = read(&first).unwrap();
        assert!(!frame.fin);
        assert_eq!(frame.opcode, Opcode::Text);
        assert_eq!(frame.payload, b"Hel");
    }

    #[test]
    fn extended_lengths() {
        for length in [126_usize, 65535, 65536] {
            let mut bytes = header(Opcode::Binary, length);
            bytes[1] |= 0x80;
            bytes.extend_from_slice(&KEY);
            let mut payload = vec![7; length];
            apply_mask(&mut payload, KEY);
            bytes.extend_from_slice(&payload);
            let frame = read(&bytes).unwrap();
            assert_eq!(frame.payload, vec![7; length]);
        }
    }

    #[test]
    fn rule_breaking_frames_are_rejected() {
        let protocol = |bytes: &[u8]| matches!(read(bytes), Err(WsError::Protocol(_)));
        assert!(
            protocol(&[0x81, 0x05, b'H', b'e', b'l', b'l', b'o']),
            "unmasked"
        );
        assert!(protocol(&[0xC1, 0x80, 0, 0, 0, 0]), "reserved bit");
        assert!(protocol(&[0x83, 0x80, 0, 0, 0, 0]), "reserved opcode");
        assert!(protocol(&[0x09, 0x80, 0, 0, 0, 0]), "fragmented ping");
        assert!(protocol(&[0x88, 0xFE, 0, 126]), "long close");
        assert!(
            protocol(&[0x82, 0xFF, 0x80, 0, 0, 0, 0, 0, 0, 0]),
            "64-bit MSB"
        );
        assert!(
            matches!(read(&[0x81, 0x85, 1, 2]), Err(WsError::Io(_))),
            "truncated"
        );
    }

    #[test]
    fn oversized_payloads_fail_before_allocating() {
        // Claims 2^62 bytes: refused from the header alone.
        let bytes = [0x82, 0xFF, 0x40, 0, 0, 0, 0, 0, 0, 0];
        assert!(matches!(
            read_frame(&mut &bytes[..], 1024),
            Err(WsError::TooBig)
        ));
        let bytes = [0x81, 0xFE, 0x04, 0x01];
        assert!(matches!(
            read_frame(&mut &bytes[..], 1024),
            Err(WsError::TooBig)
        ));
        assert_eq!(WsError::TooBig.close_code(), Some(1009));
    }
}
