//! The Wayland wire format: messages of native-endian 32-bit words, a
//! header (object id, then length and opcode) followed by arguments.
//!
//! Integers, object ids and fixed-point numbers take one word; strings
//! and arrays a length word and their bytes padded to a word boundary;
//! file descriptors travel beside the bytes, in the socket's ancillary
//! data, in the order their messages use them.

use std::collections::VecDeque;
use std::os::fd::OwnedFd;

use crate::error::Error;

/// Bytes of a message header.
pub(crate) const HEADER_LEN: usize = 8;
/// The longest message a peer accepts (libwayland's buffer size).
pub(crate) const MAX_MESSAGE_LEN: usize = 4096;
/// The longest string argument sent, in bytes, leaving room for the
/// header and the other arguments of its message.
pub(crate) const MAX_STRING_LEN: usize = 2048;

/// A 24.8 fixed-point number as a float.
pub(crate) fn fixed_to_f64(raw: i32) -> f64 {
    f64::from(raw) / 256.0
}

/// A float as a 24.8 fixed-point number (saturating), for tests playing
/// the compositor.
#[cfg(test)]
pub(crate) fn fixed_from_f64(value: f64) -> i32 {
    (value * 256.0).round() as i32
}

/// The longest prefix of `text` that is a valid Wayland string: at most
/// [`MAX_STRING_LEN`] bytes, ending on a character boundary, before any
/// NUL.
pub(crate) fn clamp_string(text: &str) -> &str {
    let text = text.split('\0').next().unwrap_or_default();
    let mut end = text.len().min(MAX_STRING_LEN);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// A message to send, built argument by argument.
#[derive(Debug)]
pub(crate) struct Request {
    bytes: Vec<u8>,
    fds: Vec<OwnedFd>,
}

impl Request {
    /// A request with opcode `opcode` to object `object`.
    pub(crate) fn new(object: u32, opcode: u16) -> Self {
        let mut bytes = Vec::with_capacity(32);
        bytes.extend(object.to_ne_bytes());
        bytes.extend(u32::from(opcode).to_ne_bytes());
        Self {
            bytes,
            fds: Vec::new(),
        }
    }

    /// Appends a signed integer.
    #[must_use]
    pub(crate) fn int(mut self, value: i32) -> Self {
        self.bytes.extend(value.to_ne_bytes());
        self
    }

    /// Appends an unsigned integer (also object ids, new ids and enums).
    #[must_use]
    pub(crate) fn uint(mut self, value: u32) -> Self {
        self.bytes.extend(value.to_ne_bytes());
        self
    }

    /// Appends a fixed-point number (events, for tests playing the
    /// compositor).
    #[cfg(test)]
    #[must_use]
    pub(crate) fn fixed(self, value: f64) -> Self {
        self.int(fixed_from_f64(value))
    }

    /// Appends a string, cut to a valid Wayland string (see
    /// [`clamp_string`]).
    #[must_use]
    pub(crate) fn string(mut self, text: &str) -> Self {
        let text = clamp_string(text);
        // The length counts the terminating NUL; `clamp_string` keeps it
        // far below `u32::MAX`.
        self.bytes.extend((text.len() as u32 + 1).to_ne_bytes());
        self.bytes.extend(text.as_bytes());
        self.bytes.push(0);
        self.pad();
        self
    }

    /// Appends an array of bytes (events, for tests playing the
    /// compositor).
    #[cfg(test)]
    #[must_use]
    pub(crate) fn array(mut self, data: &[u8]) -> Self {
        let len = u32::try_from(data.len()).unwrap_or(u32::MAX);
        self.bytes.extend(len.to_ne_bytes());
        self.bytes.extend(data);
        self.pad();
        self
    }

    /// Attaches a file descriptor, which the peer receives a copy of.
    #[must_use]
    pub(crate) fn fd(mut self, fd: OwnedFd) -> Self {
        self.fds.push(fd);
        self
    }

    fn pad(&mut self) {
        self.bytes.resize(self.bytes.len().next_multiple_of(4), 0);
    }

    /// The encoded message and its file descriptors.
    pub(crate) fn finish(mut self) -> (Vec<u8>, Vec<OwnedFd>) {
        // Strings are clamped and requests carry few arguments, so every
        // message fits the 16-bit length.
        let len = self.bytes.len().min(usize::from(u16::MAX)) as u32;
        let opcode = u32::from_ne_bytes(self.bytes[4..8].try_into().expect("a header"));
        self.bytes[4..8].copy_from_slice(&(len << 16 | opcode).to_ne_bytes());
        (self.bytes, self.fds)
    }
}

/// The header of a received message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Header {
    /// The object the message is for (events: from).
    pub(crate) object: u32,
    /// The event or request number.
    pub(crate) opcode: u16,
    /// The whole message's length in bytes, header included.
    pub(crate) len: usize,
}

impl Header {
    /// The header at the start of `bytes`, or `None` when fewer than
    /// [`HEADER_LEN`] bytes arrived. Lengths that are too short, not
    /// whole words or beyond [`MAX_MESSAGE_LEN`] are errors.
    pub(crate) fn parse(bytes: &[u8]) -> Result<Option<Self>, Error> {
        let Some(header) = bytes.get(..HEADER_LEN) else {
            return Ok(None);
        };
        let object = u32::from_ne_bytes(header[..4].try_into().expect("4 bytes"));
        let word = u32::from_ne_bytes(header[4..].try_into().expect("4 bytes"));
        let len = (word >> 16) as usize;
        if len < HEADER_LEN || !len.is_multiple_of(4) || len > MAX_MESSAGE_LEN {
            return Err(Error::Malformed("invalid Wayland message length"));
        }
        Ok(Some(Self {
            object,
            opcode: (word & 0xffff) as u16,
            len,
        }))
    }
}

/// Reads the arguments of a received message in order, taking file
/// descriptors from the connection's queue.
#[derive(Debug)]
pub(crate) struct Args<'a> {
    body: &'a [u8],
    at: usize,
    fds: &'a mut VecDeque<OwnedFd>,
}

impl<'a> Args<'a> {
    /// The arguments in `body` (the message without its header).
    pub(crate) fn new(body: &'a [u8], fds: &'a mut VecDeque<OwnedFd>) -> Self {
        Self { body, at: 0, fds }
    }

    fn word(&mut self) -> Result<[u8; 4], Error> {
        let word = self
            .body
            .get(self.at..self.at + 4)
            .ok_or(Error::Malformed("Wayland message too short"))?;
        self.at += 4;
        Ok(word.try_into().expect("4 bytes"))
    }

    /// A signed integer.
    pub(crate) fn int(&mut self) -> Result<i32, Error> {
        self.word().map(i32::from_ne_bytes)
    }

    /// An unsigned integer, enum or (possibly null, 0) object id.
    pub(crate) fn uint(&mut self) -> Result<u32, Error> {
        self.word().map(u32::from_ne_bytes)
    }

    /// A fixed-point number.
    pub(crate) fn fixed(&mut self) -> Result<f64, Error> {
        self.int().map(fixed_to_f64)
    }

    /// The bytes of an array or string: a length word, then the data
    /// padded to a word boundary.
    fn blob(&mut self) -> Result<&'a [u8], Error> {
        let len = self.uint()? as usize;
        let padded = len.next_multiple_of(4);
        let data = self
            .at
            .checked_add(padded)
            .and_then(|end| self.body.get(self.at..end))
            .ok_or(Error::Malformed("Wayland array beyond its message"))?;
        self.at += padded;
        Ok(&data[..len])
    }

    /// An array of bytes.
    pub(crate) fn array(&mut self) -> Result<&'a [u8], Error> {
        self.blob()
    }

    /// A string that may be null; invalid UTF-8 is replaced.
    pub(crate) fn optional_string(&mut self) -> Result<Option<String>, Error> {
        let bytes = self.blob()?;
        match bytes.split_last() {
            None => Ok(None),
            Some((0, text)) => Ok(Some(String::from_utf8_lossy(text).into_owned())),
            Some(_) => Err(Error::Malformed("Wayland string without its NUL")),
        }
    }

    /// A string; a null string is an error.
    pub(crate) fn string(&mut self) -> Result<String, Error> {
        self.optional_string()?
            .ok_or(Error::Malformed("null Wayland string"))
    }

    /// The next file descriptor received.
    pub(crate) fn fd(&mut self) -> Result<OwnedFd, Error> {
        self.fds.pop_front().ok_or(Error::Malformed(
            "Wayland message without its file descriptor",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(bytes: &[u8]) -> Vec<u32> {
        bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&w| u32::from_ne_bytes(w))
            .collect()
    }

    #[test]
    fn requests_encode_header_and_arguments() {
        let (bytes, fds) = Request::new(3, 2)
            .int(-1)
            .uint(7)
            .fixed(1.5)
            .string("ab")
            .array(&[1, 2, 3, 4, 5])
            .finish();
        assert!(fds.is_empty());
        assert_eq!(bytes.len(), 40);
        let encoded = words(&bytes);
        assert_eq!(&encoded[..2], [3, 40 << 16 | 2]);
        assert_eq!(&encoded[2..5], [u32::MAX, 7, 384]);
        assert_eq!(encoded[5], 3, "the string length counts the NUL");
        assert_eq!(&bytes[24..28], b"ab\0\0");
        assert_eq!(encoded[7], 5);
        assert_eq!(&bytes[32..40], [1, 2, 3, 4, 5, 0, 0, 0]);
        let (empty, _) = Request::new(1, 0).string("").finish();
        assert_eq!(words(&empty), [1, 16 << 16, 1, 0], "the NUL, padded");
    }

    #[test]
    fn strings_are_clamped() {
        assert_eq!(clamp_string("a\0b"), "a");
        let long = "é".repeat(MAX_STRING_LEN);
        let clamped = clamp_string(&long);
        assert!(clamped.len() <= MAX_STRING_LEN && clamped.len().is_multiple_of(2));
        let (bytes, _) = Request::new(1, 0).string(&long).finish();
        assert!(bytes.len() <= MAX_MESSAGE_LEN);
    }

    #[test]
    fn headers_are_validated() {
        let header = |object: u32, len: u32, opcode: u32| {
            let mut bytes = object.to_ne_bytes().to_vec();
            bytes.extend((len << 16 | opcode).to_ne_bytes());
            bytes
        };
        assert_eq!(
            Header::parse(&header(5, 12, 3)).unwrap(),
            Some(Header {
                object: 5,
                opcode: 3,
                len: 12
            })
        );
        assert_eq!(Header::parse(&[0; 7]).unwrap(), None);
        assert!(
            Header::parse(&header(1, 4, 0)).is_err(),
            "shorter than a header"
        );
        assert!(Header::parse(&header(1, 10, 0)).is_err(), "not whole words");
        assert!(Header::parse(&header(1, 8192, 0)).is_err(), "too long");
    }

    #[test]
    fn arguments_decode_and_reject_truncation() {
        let (bytes, _) = Request::new(1, 0)
            .int(-5)
            .fixed(-0.25)
            .string("hé")
            .uint(0)
            .array(&[9])
            .finish();
        let mut fds = VecDeque::new();
        let mut args = Args::new(&bytes[HEADER_LEN..], &mut fds);
        assert_eq!(args.int().unwrap(), -5);
        assert!((args.fixed().unwrap() + 0.25).abs() < 1e-9);
        assert_eq!(args.string().unwrap(), "hé");
        assert_eq!(args.optional_string().unwrap(), None, "length 0 is null");
        assert_eq!(args.array().unwrap(), [9]);
        assert!(args.uint().is_err());
        assert!(args.fd().is_err());

        let lying = [100_u32.to_ne_bytes(), *b"abcd"].concat();
        assert!(Args::new(&lying, &mut fds).array().is_err());
        let huge = u32::MAX.to_ne_bytes();
        assert!(Args::new(&huge, &mut fds).array().is_err());
        let unterminated = [4_u32.to_ne_bytes(), *b"abcd"].concat();
        assert!(Args::new(&unterminated, &mut fds).string().is_err());
        let null = 0_u32.to_ne_bytes();
        assert!(Args::new(&null, &mut fds).string().is_err());
    }

    #[test]
    fn fixed_point_conversions() {
        assert!((fixed_to_f64(256) - 1.0).abs() < f64::EPSILON);
        assert!((fixed_to_f64(-128) + 0.5).abs() < f64::EPSILON);
        assert_eq!(fixed_from_f64(2.5), 640);
        assert_eq!(fixed_from_f64(f64::INFINITY), i32::MAX);
        assert_eq!(fixed_from_f64(f64::NAN), 0);
    }
}
