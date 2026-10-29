//! WebSocket messages (RFC 6455 sections 5.4 and 5.5): fragments joined
//! into whole messages, control frames interleaved, and the close frame's
//! status code and reason checked.

use std::io::Read;

use super::frame::{Frame, Opcode, WsError, read_frame};

/// A whole message from a client.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Message {
    /// A text message.
    Text(String),
    /// A ping, with the payload the pong must echo.
    Ping(Vec<u8>),
    /// A pong.
    Pong,
    /// The client starts or answers the closing handshake, with its status
    /// code, if any.
    Close(Option<u16>),
}

/// Joins a client's frames into messages of at most `max_message` bytes.
#[derive(Debug)]
pub(crate) struct MessageReader {
    max_message: usize,
    /// The fragments of a text message received so far.
    partial: Option<Vec<u8>>,
}

impl MessageReader {
    /// A reader accepting messages of up to `max_message` bytes.
    pub(crate) fn new(max_message: usize) -> Self {
        Self {
            max_message,
            partial: None,
        }
    }

    /// Reads frames until a message is complete; control frames are
    /// returned as they come, also between the fragments of a message.
    ///
    /// # Errors
    ///
    /// When the connection fails or the client breaks the protocol (see
    /// [`WsError::close_code`] for the code to close with).
    pub(crate) fn next(&mut self, reader: &mut impl Read) -> Result<Message, WsError> {
        loop {
            let received = self.partial.as_ref().map_or(0, Vec::len);
            let frame = read_frame(reader, self.max_message - received)?;
            if let Some(message) = self.accept(frame)? {
                return Ok(message);
            }
        }
    }

    /// Takes one frame; returns the message it completes, if any.
    fn accept(&mut self, frame: Frame) -> Result<Option<Message>, WsError> {
        let Frame {
            fin,
            opcode,
            payload,
        } = frame;
        match opcode {
            Opcode::Ping => Ok(Some(Message::Ping(payload))),
            Opcode::Pong => Ok(Some(Message::Pong)),
            Opcode::Close => close_code(&payload).map(|code| Some(Message::Close(code))),
            Opcode::Binary => Err(WsError::Unsupported),
            Opcode::Text if self.partial.is_some() => {
                Err(WsError::Protocol("a new message within a fragmented one"))
            }
            Opcode::Continuation if self.partial.is_none() => {
                Err(WsError::Protocol("a continuation without a message"))
            }
            Opcode::Text | Opcode::Continuation => self.extend(payload, fin),
        }
    }

    /// Adds a fragment; the last one completes the text.
    fn extend(&mut self, payload: Vec<u8>, fin: bool) -> Result<Option<Message>, WsError> {
        let mut text = match self.partial.take() {
            Some(mut text) => {
                text.extend_from_slice(&payload);
                text
            }
            None => payload,
        };
        if !fin {
            // Fail early on bytes that can never become UTF-8.
            if let Err(err) = std::str::from_utf8(&text)
                && err.error_len().is_some()
            {
                return Err(WsError::InvalidUtf8);
            }
            self.partial = Some(text);
            return Ok(None);
        }
        text.shrink_to_fit();
        String::from_utf8(text)
            .map(|text| Some(Message::Text(text)))
            .map_err(|_| WsError::InvalidUtf8)
    }
}

/// The status code of a close frame's payload: none, or a code that may
/// be sent (RFC 6455 section 7.4) followed by a UTF-8 reason.
fn close_code(payload: &[u8]) -> Result<Option<u16>, WsError> {
    let Some((code, reason)) = payload.split_first_chunk::<2>() else {
        return if payload.is_empty() {
            Ok(None)
        } else {
            Err(WsError::Protocol("one-byte close payload"))
        };
    };
    let code = u16::from_be_bytes(*code);
    let valid = matches!(code, 1000..=1003 | 1007..=1014 | 3000..=4999);
    if !valid {
        return Err(WsError::Protocol("invalid close code"));
    }
    std::str::from_utf8(reason).map_err(|_| WsError::InvalidUtf8)?;
    Ok(Some(code))
}

#[cfg(test)]
mod tests {
    use super::super::frame::apply_mask;
    use super::*;

    /// A masked client frame.
    fn frame(first: u8, payload: &[u8]) -> Vec<u8> {
        let key = [1, 2, 3, 4];
        let length = u8::try_from(payload.len()).expect("short test payloads");
        let mut out = vec![first, 0x80 | length];
        out.extend_from_slice(&key);
        let mut masked = payload.to_vec();
        apply_mask(&mut masked, key);
        out.extend_from_slice(&masked);
        out
    }

    fn messages(bytes: &[u8], max: usize) -> Vec<Result<Message, String>> {
        let mut reader = MessageReader::new(max);
        let mut input = bytes;
        let mut out = Vec::new();
        while !input.is_empty() {
            match reader.next(&mut input) {
                Ok(message) => out.push(Ok(message)),
                Err(err) => {
                    out.push(Err(err.to_string()));
                    break;
                }
            }
        }
        out
    }

    #[test]
    fn fragments_join_around_control_frames() {
        // RFC 6455's fragmented "Hello", with a ping in the middle.
        let mut bytes = frame(0x01, b"Hel");
        bytes.extend(frame(0x89, b"hi"));
        bytes.extend(frame(0x80, b"lo"));
        bytes.extend(frame(0x8A, b""));
        assert_eq!(
            messages(&bytes, 1024),
            [
                Ok(Message::Ping(b"hi".to_vec())),
                Ok(Message::Text("Hello".into())),
                Ok(Message::Pong),
            ]
        );
    }

    #[test]
    fn utf8_may_split_across_fragments() {
        let text = "é".as_bytes();
        let mut bytes = frame(0x01, &text[..1]);
        bytes.extend(frame(0x80, &text[1..]));
        assert_eq!(messages(&bytes, 16), [Ok(Message::Text("é".into()))]);
    }

    #[test]
    fn close_frames_carry_valid_codes() {
        let close = |payload: &[u8]| messages(&frame(0x88, payload), 16);
        assert_eq!(close(b""), [Ok(Message::Close(None))]);
        assert_eq!(
            close(&[0x03, 0xE8, b'o', b'k']),
            [Ok(Message::Close(Some(1000)))]
        );
        assert_eq!(close(&[0x0F, 0xA0]), [Ok(Message::Close(Some(4000)))]);
        for bad in [
            &[0x03][..],
            &[0x03, 0xED],
            &[0x00, 0x10],
            &[0x03, 0xE8, 0xff],
        ] {
            assert!(close(bad)[0].is_err(), "{bad:?}");
        }
    }

    #[test]
    fn broken_sequences_are_rejected() {
        let continuation = frame(0x80, b"x");
        assert!(messages(&continuation, 16)[0].is_err());
        let mut nested = frame(0x01, b"a");
        nested.extend(frame(0x81, b"b"));
        assert!(messages(&nested, 16)[0].is_err());
        assert!(messages(&frame(0x82, b"bin"), 16)[0].is_err());
        assert!(messages(&frame(0x81, &[0xff, 0xfe]), 16)[0].is_err());
        assert!(
            messages(&frame(0x01, &[0xff]), 16)[0].is_err(),
            "fails early"
        );
    }

    #[test]
    fn messages_are_limited_across_fragments() {
        let mut bytes = frame(0x01, b"12345678");
        bytes.extend(frame(0x80, b"9"));
        let result = messages(&bytes, 8);
        assert_eq!(result, [Err(WsError::TooBig.to_string())]);
        assert_eq!(messages(&frame(0x81, b"12345678"), 8).len(), 1);
    }
}
