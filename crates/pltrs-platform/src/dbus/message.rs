//! D-Bus messages: method calls written, and the replies, errors and
//! signals the bus sends read back ("Message Format" of the D-Bus
//! specification).

use super::BusError;
use super::value::{Reader, Value, Writer, types};

/// The largest message read; the bus sends this client only replies
/// and the odd signal, all small.
pub(crate) const MAX_MESSAGE: usize = 1 << 20;
/// The fixed part of a header, up to the length of its fields.
pub(crate) const FIXED_HEADER: usize = 16;

/// `METHOD_CALL`.
const METHOD_CALL: u8 = 1;
/// `METHOD_RETURN`.
pub(crate) const METHOD_RETURN: u8 = 2;
/// `ERROR`.
pub(crate) const ERROR: u8 = 3;

/// Header field codes.
mod field {
    pub(super) const PATH: u8 = 1;
    pub(super) const INTERFACE: u8 = 2;
    pub(super) const MEMBER: u8 = 3;
    pub(super) const ERROR_NAME: u8 = 4;
    pub(super) const REPLY_SERIAL: u8 = 5;
    pub(super) const DESTINATION: u8 = 6;
    pub(super) const SIGNATURE: u8 = 8;
}

/// A method call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MethodCall<'a> {
    /// The bus name of the service.
    pub(crate) destination: &'a str,
    /// The object.
    pub(crate) path: &'a str,
    /// The interface.
    pub(crate) interface: &'a str,
    /// The method.
    pub(crate) member: &'a str,
    /// The arguments: an array of strings, if any (all this client
    /// passes).
    pub(crate) strings: Option<&'a [&'a str]>,
}

impl MethodCall<'_> {
    /// The message, numbered `serial`.
    pub(crate) fn encode(&self, serial: u32) -> Vec<u8> {
        let mut body = Writer::default();
        if let Some(strings) = self.strings {
            body.strings(strings);
        }
        let mut message = Writer::default();
        for byte in [b'l', METHOD_CALL, 0, 1] {
            message.u8(byte);
        }
        message.u32(u32::try_from(body.bytes.len()).expect("bodies are small"));
        message.u32(serial);
        message.u32(0);
        let fields_start = message.bytes.len();
        let mut fields: Vec<(u8, &str, &str)> = vec![
            (field::PATH, "o", self.path),
            (field::INTERFACE, "s", self.interface),
            (field::MEMBER, "s", self.member),
            (field::DESTINATION, "s", self.destination),
        ];
        if self.strings.is_some() {
            fields.push((field::SIGNATURE, "g", "as"));
        }
        for (code, signature, value) in fields {
            message.align(8);
            message.u8(code);
            message.signature(signature);
            if signature == "g" {
                message.signature(value);
            } else {
                message.string(value);
            }
        }
        let fields_length = u32::try_from(message.bytes.len() - fields_start).expect("short");
        message.bytes[fields_start - 4..fields_start].copy_from_slice(&fields_length.to_le_bytes());
        message.align(8);
        message.bytes.extend_from_slice(&body.bytes);
        message.bytes
    }
}

/// A message read from the bus.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Message {
    /// `METHOD_RETURN`, `ERROR`, ...
    pub(crate) kind: u8,
    /// The call this answers, for replies and errors.
    pub(crate) reply_serial: Option<u32>,
    /// The error's name, for errors.
    pub(crate) error_name: Option<String>,
    /// The values of the body.
    pub(crate) body: Vec<Value>,
}

/// Whether the message is big-endian, from its first byte.
fn big_endian(first: u8) -> Result<bool, BusError> {
    match first {
        b'l' => Ok(false),
        b'B' => Ok(true),
        _ => Err(BusError::Malformed),
    }
}

/// The whole length of the message whose first [`FIXED_HEADER`] bytes
/// are `head`, checked against [`MAX_MESSAGE`].
pub(crate) fn message_length(head: &[u8]) -> Result<usize, BusError> {
    let mut reader = Reader::new(head, 4, big_endian(head[0])?);
    let body = reader.u32()? as usize;
    reader.u32()?;
    let fields = reader.u32()? as usize;
    let length = FIXED_HEADER
        .checked_add(fields)
        .map(|header| header.next_multiple_of(8))
        .and_then(|header| header.checked_add(body))
        .ok_or(BusError::Malformed)?;
    if length > MAX_MESSAGE {
        return Err(BusError::Malformed);
    }
    Ok(length)
}

/// Parses a whole message.
pub(crate) fn parse(bytes: &[u8]) -> Result<Message, BusError> {
    let head = bytes.get(..FIXED_HEADER).ok_or(BusError::Malformed)?;
    if message_length(head)? != bytes.len() || head[3] != 1 {
        return Err(BusError::Malformed);
    }
    let big = big_endian(head[0])?;
    let mut reader = Reader::new(bytes, 12, big);
    let Value::Array(fields) = reader.value(b"a(yv)")? else {
        return Err(BusError::Malformed);
    };
    reader.align(8)?;
    let mut message = Message {
        kind: head[1],
        reply_serial: None,
        error_name: None,
        body: Vec::new(),
    };
    let mut signature = String::new();
    for header_field in &fields {
        let Value::Struct(parts) = header_field else {
            return Err(BusError::Malformed);
        };
        let (Some(Value::Byte(code)), Some(Value::Variant(value))) = (parts.first(), parts.get(1))
        else {
            return Err(BusError::Malformed);
        };
        match (*code, &**value) {
            (field::REPLY_SERIAL, &Value::UInt(serial)) => {
                message.reply_serial = u32::try_from(serial).ok();
            }
            (field::ERROR_NAME, Value::Str(name)) => message.error_name = Some(name.clone()),
            (field::SIGNATURE, Value::Str(body)) => signature.clone_from(body),
            _ => {}
        }
    }
    for value in types(signature.as_bytes())? {
        message.body.push(reader.value(value)?);
    }
    if reader.pos() != bytes.len() {
        return Err(BusError::Malformed);
    }
    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    const READ_ALL: MethodCall<'static> = MethodCall {
        destination: "org.freedesktop.portal.Desktop",
        path: "/org/freedesktop/portal/desktop",
        interface: "org.freedesktop.portal.Settings",
        member: "ReadAll",
        strings: Some(&["org.gnome.desktop.wm.preferences"]),
    };

    #[test]
    fn method_calls_are_written_as_the_specification_lays_them_out() {
        let bytes = READ_ALL.encode(7);
        assert_eq!(&bytes[..4], b"l\x01\x00\x01");
        assert_eq!(message_length(&bytes[..FIXED_HEADER]), Ok(bytes.len()));
        assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 7);
        // Read back as a message: the header's fields and the body.
        let message = parse(&bytes).unwrap();
        assert_eq!(message.kind, METHOD_CALL);
        assert_eq!(
            message.body,
            [Value::Array(vec![Value::Str(
                "org.gnome.desktop.wm.preferences".into()
            )])]
        );
        let hello = MethodCall {
            destination: "org.freedesktop.DBus",
            path: "/org/freedesktop/DBus",
            interface: "org.freedesktop.DBus",
            member: "Hello",
            strings: None,
        };
        let bytes = hello.encode(1);
        assert_eq!(bytes.len() % 8, 0, "no body after the padded header");
        assert_eq!(parse(&bytes).unwrap().body, []);
    }

    /// The bus's reply to `Hello`, laid out as the specification says:
    /// little-endian, its body a string (the name `:1.42`), to serial 1,
    /// with the header fields buses send (the sender too).
    const HELLO_REPLY: &[u8] = &[
        b'l', 2, 1, 1, 10, 0, 0, 0, 1, 0, 0, 0, 61, 0, 0, 0, // fixed header
        6, 1, b's', 0, 5, 0, 0, 0, b':', b'1', b'.', b'4', b'2', 0, 0, 0, // DESTINATION
        5, 1, b'u', 0, 1, 0, 0, 0, // REPLY_SERIAL
        8, 1, b'g', 0, 1, b's', 0, 0, // SIGNATURE
        7, 1, b's', 0, 20, 0, 0, 0, // SENDER, its length ...
        b'o', b'r', b'g', b'.', b'f', b'r', b'e', b'e', b'd', b'e', b's', b'k', b't', b'o', b'p',
        b'.', b'D', b'B', b'u', b's', 0, 0, 0, 0, // ... and name, padding
        5, 0, 0, 0, b':', b'1', b'.', b'4', b'2', 0, // the body
    ];

    #[test]
    fn replies_are_read_with_their_header_fields() {
        let message = parse(HELLO_REPLY).unwrap();
        assert_eq!(message.kind, METHOD_RETURN);
        assert_eq!(message.reply_serial, Some(1));
        assert_eq!(message.error_name, None);
        assert_eq!(message.body, [Value::Str(":1.42".into())]);
    }

    #[test]
    fn broken_messages_are_errors() {
        let mut wrong_endian = HELLO_REPLY.to_vec();
        wrong_endian[0] = b'x';
        let mut wrong_version = HELLO_REPLY.to_vec();
        wrong_version[3] = 2;
        let mut long_body = HELLO_REPLY.to_vec();
        long_body[4] = 11;
        let mut huge = HELLO_REPLY.to_vec();
        huge[7] = 0x7f;
        for bytes in [
            &wrong_endian[..],
            &wrong_version,
            &long_body,
            &huge,
            &HELLO_REPLY[..HELLO_REPLY.len() - 1],
            &HELLO_REPLY[..10],
        ] {
            assert!(parse(bytes).is_err(), "{bytes:?}");
        }
        assert_eq!(
            message_length(&huge[..FIXED_HEADER]),
            Err(BusError::Malformed)
        );
    }
}
