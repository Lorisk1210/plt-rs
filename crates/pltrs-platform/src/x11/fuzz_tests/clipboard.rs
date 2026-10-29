//! Data from other clients and the environment: clipboard text as the
//! property replies of a paste carry it (UTF-8, Latin-1, not text at all,
//! in any format, with lying lengths, and the `INCR` type of incremental
//! transfers), Xauthority files and display names.

use super::{reply, word};
use crate::fuzz::{Corruption, Gen, fuzz};
use crate::x11::atoms::predefined;
use crate::x11::auth::{self, FAMILY_INTERNET, FAMILY_INTERNET6, FAMILY_LOCAL, MIT_MAGIC_COOKIE};
use crate::x11::display::DisplayName;
use crate::x11::paste::decode_text;
use crate::x11::reply as parse;

/// The atoms `UTF8_STRING` and `INCR` of the paste replies.
const UTF8_STRING: u32 = 300;
const INCR: u32 = 301;

/// The value of a pasted property.
#[derive(Debug)]
enum Value {
    /// Text in UTF-8.
    Text(String),
    /// Bytes, valid as Latin-1 and not as UTF-8.
    Bytes(Vec<u8>),
    /// The size an `INCR` transfer announces.
    Incremental(u32),
}

/// The reply to a paste's `GetProperty`.
#[derive(Debug)]
struct Pasted {
    kind: u32,
    format: u8,
    value: Value,
    /// A unit count that disagrees with the value, if any.
    claimed: Option<u32>,
    corruption: Option<Corruption>,
}

impl Pasted {
    fn generate(g: &mut Gen) -> Self {
        let value = match g.below(3) {
            0 => Value::Text(g.soup(&["text", "é", "😀", "\u{0}", "\r\n", " "], 8)),
            1 => Value::Bytes((0..g.len(40)).map(|_| g.below(256) as u8).collect()),
            _ => Value::Incremental(word(g)),
        };
        let other = word(g);
        let pasted = Self {
            kind: g.pick(&[
                UTF8_STRING,
                predefined::STRING,
                INCR,
                predefined::ATOM,
                0,
                other,
            ]),
            format: g.pick(&[8, 8, 0, 16, 32, 7, 255]),
            value,
            claimed: g.chance(0.15).then(|| word(g)),
            corruption: None,
        };
        let corruption = g.chance(0.15).then(|| g.corrupt(&pasted.bytes()));
        Self {
            corruption,
            ..pasted
        }
    }

    fn bytes(&self) -> Vec<u8> {
        let value = match &self.value {
            Value::Text(text) => text.clone().into_bytes(),
            Value::Bytes(bytes) => bytes.clone(),
            Value::Incremental(size) => size.to_le_bytes().to_vec(),
        };
        let unit = usize::from(self.format / 8).max(1);
        let units = self
            .claimed
            .unwrap_or_else(|| u32::try_from(value.len() / unit).unwrap_or(u32::MAX));
        let bytes = reply(self.format, 7, &[self.kind, 0, units], &value);
        match &self.corruption {
            Some(corruption) => corruption.apply(&bytes),
            None => bytes,
        }
    }
}

#[test]
fn clipboard_properties() {
    fuzz("x11 clipboard", 3_000, Pasted::generate, |pasted| {
        let bytes = pasted.bytes();
        let Ok(property) = parse::property(&bytes) else {
            return;
        };
        let text = decode_text(&property, INCR);
        if property.kind == INCR || property.format != 8 {
            assert_eq!(text, None, "no text from {property:?}");
        }
    });
}

/// One Xauthority entry's fields.
#[derive(Debug)]
struct Entry {
    family: u16,
    address: Vec<u8>,
    number: Vec<u8>,
    name: Vec<u8>,
    data: Vec<u8>,
}

impl Entry {
    fn generate(g: &mut Gen) -> Self {
        let field = |g: &mut Gen, pieces: &[&[u8]]| -> Vec<u8> {
            if g.chance(0.8) {
                g.pick(pieces).to_vec()
            } else {
                (0..g.len(20)).map(|_| g.below(256) as u8).collect()
            }
        };
        Self {
            family: g.pick(&[
                FAMILY_INTERNET,
                FAMILY_INTERNET6,
                FAMILY_LOCAL,
                0xFFFF,
                1,
                254,
            ]),
            address: field(g, &[b"host", b"", &[127, 0, 0, 1], b"fedora.fritz.box"]),
            number: field(g, &[b"0", b"", b"1", b"65535", b"99999"]),
            name: field(g, &[MIT_MAGIC_COOKIE, b"XDM-AUTHORIZATION-1", b""]),
            data: field(g, &[&[0xAB; 16], b"", &[0; 1]]),
        }
    }

    fn bytes(&self) -> Vec<u8> {
        let mut out = self.family.to_be_bytes().to_vec();
        for field in [&self.address, &self.number, &self.name, &self.data] {
            let len = u16::try_from(field.len()).unwrap_or(u16::MAX);
            out.extend_from_slice(&len.to_be_bytes());
            out.extend_from_slice(field);
        }
        out
    }
}

/// An Xauthority file and the server to find credentials for.
#[derive(Debug)]
struct Lookup {
    entries: Vec<Entry>,
    corruption: Option<Corruption>,
    family: u16,
    address: Vec<u8>,
    display: u16,
}

#[test]
fn xauthority_files() {
    fuzz(
        "x11 xauthority",
        3_000,
        |g| {
            let entries: Vec<Entry> = (0..g.len(5)).map(|_| Entry::generate(g)).collect();
            let bytes: Vec<u8> = entries.iter().flat_map(Entry::bytes).collect();
            Lookup {
                corruption: g.chance(0.4).then(|| g.corrupt(&bytes)),
                entries,
                family: g.pick(&[FAMILY_INTERNET, FAMILY_INTERNET6, FAMILY_LOCAL, 7]),
                address: g.pick(&[&b"host"[..], b"", &[127, 0, 0, 1]]).to_vec(),
                display: g.pick(&[0, 1, 9, 65535]),
            }
        },
        |lookup| {
            let bytes: Vec<u8> = lookup.entries.iter().flat_map(Entry::bytes).collect();
            let bytes = match &lookup.corruption {
                Some(corruption) => corruption.apply(&bytes),
                None => bytes,
            };
            let entries = auth::parse(&bytes);
            assert!(
                entries.len() <= bytes.len() / 10,
                "entries take 10 bytes or more"
            );
            if let Some(credentials) =
                auth::find(&entries, lookup.family, &lookup.address, lookup.display)
            {
                assert_eq!(credentials.name, MIT_MAGIC_COOKIE);
            }
        },
    );
}

/// Display names, and pieces of them.
const DISPLAYS: [&str; 10] = [
    ":0",
    ":1.2",
    "unix:3",
    "localhost:10.0",
    "tcp/example.org:2",
    "[::1]:0",
    "unix/anything:4.1",
    "/private/tmp/com.apple.launchd.abc/org.xquartz:0",
    "host::0",
    "decnet/host:0",
];

const DISPLAY_PIECES: [&str; 12] = [
    ":", ".", "/", "unix", "tcp", "inet6", "local", "65535", "65536", "0", "::", " ",
];

#[test]
fn display_names() {
    fuzz(
        "x11 display names",
        3_000,
        |g| g.text(&DISPLAYS, &DISPLAY_PIECES),
        |name| {
            if let Ok(display) = DisplayName::parse(name) {
                assert!(
                    display
                        .local_socket_path()
                        .ends_with(&display.display.to_string())
                );
            }
        },
    );
}
