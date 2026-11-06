//! The client against a bus played by a thread over a socket pair, and
//! against the session bus, when there is one.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::thread;
use std::time::{Duration, Instant};

use super::message::{self, FIXED_HEADER, MethodCall, message_length};
use super::value::{Value, Writer};
use super::{Bus, BusError};

fn soon() -> Instant {
    Instant::now() + Duration::from_secs(5)
}

/// A reply (`METHOD_RETURN`) or error to `serial`, with `body` of
/// `signature` written by `write`.
fn reply(
    serial: u32,
    error: Option<&str>,
    signature: &str,
    write: impl FnOnce(&mut Writer),
) -> Vec<u8> {
    let mut body = Writer::default();
    write(&mut body);
    let mut message = Writer::default();
    let kind = if error.is_some() { 3 } else { 2 };
    for byte in [b'l', kind, 1, 1] {
        message.u8(byte);
    }
    message.u32(u32::try_from(body.bytes.len()).unwrap());
    message.u32(1000 + serial);
    message.u32(0);
    let start = message.bytes.len();
    message.u8(5);
    message.signature("u");
    message.u32(serial);
    if !signature.is_empty() {
        message.align(8);
        message.u8(8);
        message.signature("g");
        message.signature(signature);
    }
    if let Some(name) = error {
        message.align(8);
        message.u8(4);
        message.signature("s");
        message.string(name);
    }
    let length = u32::try_from(message.bytes.len() - start).unwrap();
    message.bytes[start - 4..start].copy_from_slice(&length.to_le_bytes());
    message.align(8);
    message.bytes.extend_from_slice(&body.bytes);
    message.bytes
}

/// A signal nobody asked for (`NameAcquired`), which the client skips.
fn signal() -> Vec<u8> {
    let mut bytes = reply(0, None, "s", |body| body.string(":1.7"));
    bytes[1] = 4;
    bytes
}

/// Reads a message the client sent; returns its serial and the message.
fn receive(stream: &mut BufReader<UnixStream>) -> (u32, message::Message) {
    let mut bytes = vec![0; FIXED_HEADER];
    stream.read_exact(&mut bytes).unwrap();
    bytes.resize(message_length(&bytes).unwrap(), 0);
    stream.read_exact(&mut bytes[FIXED_HEADER..]).unwrap();
    let serial = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    (serial, message::parse(&bytes).unwrap())
}

/// Plays a bus: authenticates, answers `Hello`, then answers each later
/// call with what `answer` writes for it.
fn fake_bus(answer: impl Fn(u32, &message::Message) -> Vec<u8> + Send + 'static) -> UnixStream {
    let (client, server) = UnixStream::pair().unwrap();
    thread::spawn(move || {
        let mut stream = BufReader::new(server);
        let mut line = Vec::new();
        stream.read_until(b'\n', &mut line).unwrap();
        assert!(line.starts_with(b"\0AUTH EXTERNAL "), "{line:?}");
        let hex = &line[15..line.len() - 2];
        assert!(!hex.is_empty() && hex.iter().all(u8::is_ascii_hexdigit));
        stream
            .get_mut()
            .write_all(b"OK 0123456789abcdef\r\n")
            .unwrap();
        line.clear();
        stream.read_until(b'\n', &mut line).unwrap();
        assert_eq!(line, b"BEGIN\r\n");
        let (serial, _) = receive(&mut stream);
        let hello = reply(serial, None, "s", |body| body.string(":1.7"));
        stream.get_mut().write_all(&hello).unwrap();
        stream.get_mut().write_all(&signal()).unwrap();
        loop {
            if stream.fill_buf().map_or(true, <[u8]>::is_empty) {
                return;
            }
            let (serial, call) = receive(&mut stream);
            let answer = answer(serial, &call);
            if stream.get_mut().write_all(&answer).is_err() {
                return;
            }
        }
    });
    client
}

const READ: MethodCall<'static> = MethodCall {
    destination: "org.freedesktop.portal.Desktop",
    path: "/org/freedesktop/portal/desktop",
    interface: "org.freedesktop.portal.Settings",
    member: "ReadAll",
    strings: Some(&["org.gnome.desktop.wm.preferences"]),
};

#[test]
fn calls_are_answered_after_the_bus_says_hello() {
    let stream = fake_bus(|serial, call| {
        assert_eq!(
            call.body,
            [Value::Array(vec![Value::Str(
                "org.gnome.desktop.wm.preferences".into()
            )])]
        );
        // {"org.gnome.desktop.wm.preferences": {"button-layout": <"x">}}
        reply(serial, None, "a{sa{sv}}", |body| {
            body.u32(0);
            let outer = body.bytes.len();
            body.align(8);
            body.string("org.gnome.desktop.wm.preferences");
            body.u32(0);
            let inner_length_at = body.bytes.len() - 4;
            body.align(8);
            let inner = body.bytes.len();
            body.string("button-layout");
            body.signature("s");
            body.string("x");
            let inner_length = u32::try_from(body.bytes.len() - inner).unwrap();
            body.bytes[inner_length_at..inner_length_at + 4]
                .copy_from_slice(&inner_length.to_le_bytes());
            let outer_length = u32::try_from(body.bytes.len() - outer.next_multiple_of(8)).unwrap();
            body.bytes[outer - 4..outer].copy_from_slice(&outer_length.to_le_bytes());
        })
    });
    let mut bus = Bus::over(stream, soon()).unwrap();
    let values = bus.call(&READ).unwrap();
    let layout = values[0]
        .get("org.gnome.desktop.wm.preferences")
        .and_then(|keys| keys.get("button-layout"))
        .and_then(Value::as_str);
    assert_eq!(layout, Some("x"));
}

#[test]
fn errors_and_silence_are_failures() {
    let stream = fake_bus(|serial, _| {
        reply(
            serial,
            Some("org.freedesktop.DBus.Error.UnknownMethod"),
            "s",
            |body| {
                body.string("no such method");
            },
        )
    });
    let mut bus = Bus::over(stream, soon()).unwrap();
    assert_eq!(
        bus.call(&READ),
        Err(BusError::Reply(
            "org.freedesktop.DBus.Error.UnknownMethod".into()
        ))
    );
    // A bus that never answers: the deadline ends the wait.
    let (client, _server) = UnixStream::pair().unwrap();
    let started = Instant::now();
    let silent = Bus::over(client, Instant::now() + Duration::from_millis(100));
    assert!(matches!(silent, Err(BusError::Io(_))), "{silent:?}");
    assert!(started.elapsed() < Duration::from_secs(2));
    // A bus refusing the authentication.
    let (client, server) = UnixStream::pair().unwrap();
    thread::spawn(move || {
        let mut stream = BufReader::new(server);
        let mut line = Vec::new();
        stream.read_until(b'\n', &mut line).unwrap();
        stream
            .get_mut()
            .write_all(b"REJECTED EXTERNAL\r\n")
            .unwrap();
    });
    assert_eq!(Bus::over(client, soon()).err(), Some(BusError::Refused));
}

#[test]
fn the_session_bus_answers_when_there_is_one() {
    if std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_none() {
        return;
    }
    let Ok(mut bus) = Bus::session(soon()) else {
        return;
    };
    let id = bus
        .call(&MethodCall {
            destination: "org.freedesktop.DBus",
            path: "/org/freedesktop/DBus",
            interface: "org.freedesktop.DBus",
            member: "GetId",
            strings: None,
        })
        .expect("the bus has an id");
    let id = id.first().and_then(Value::as_str).unwrap_or_default();
    assert_eq!(id.len(), 32, "{id:?}");
    assert!(id.bytes().all(|b| b.is_ascii_hexdigit()));
}
