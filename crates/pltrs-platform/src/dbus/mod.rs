//! A small D-Bus client, spoken over the session bus's Unix socket
//! without libdbus: enough to call a method and read its reply, which
//! is how desktops hand out their settings (the XDG desktop portal).
//!
//! * [`address`]: the session bus's address.
//! * [`value`]: the wire format's values and signatures.
//! * [`message`]: method calls written, replies read.
//! * [`Bus`]: the connection, authenticated with `EXTERNAL` (the
//!   process's user id, which the bus checks against the socket's
//!   credentials), then introduced with `Hello`.

mod address;
#[cfg(test)]
mod fuzz_tests;
pub(crate) mod message;
pub(crate) mod value;

use std::fmt;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::linux::net::SocketAddrExt;
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::{SocketAddr, UnixStream};
use std::time::Instant;

use address::SocketAddress;
use message::{ERROR, FIXED_HEADER, METHOD_RETURN, MethodCall, message_length};
use value::Value;

/// The longest line of the authentication exchange.
const MAX_AUTH_LINE: usize = 512;

/// What went wrong talking to the bus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BusError {
    /// No bus address could be connected to.
    NoBus,
    /// The bus refused to authenticate this process.
    Refused,
    /// Reading or writing failed, or the deadline passed.
    Io(io::ErrorKind),
    /// The bus sent something that is not D-Bus.
    Malformed,
    /// The method answered with this error.
    Reply(String),
}

impl fmt::Display for BusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoBus => f.write_str("no D-Bus session bus to connect to"),
            Self::Refused => f.write_str("the D-Bus bus refused the authentication"),
            Self::Io(kind) => write!(f, "talking to the D-Bus bus failed: {kind}"),
            Self::Malformed => f.write_str("the D-Bus bus sent a malformed message"),
            Self::Reply(name) => write!(f, "the D-Bus method failed: {name}"),
        }
    }
}

impl std::error::Error for BusError {}

impl From<io::Error> for BusError {
    fn from(error: io::Error) -> Self {
        Self::Io(error.kind())
    }
}

/// A connection to a bus.
#[derive(Debug)]
pub(crate) struct Bus {
    stream: BufReader<UnixStream>,
    /// The serial of the last message sent.
    serial: u32,
    /// When calls give up waiting.
    deadline: Instant,
}

impl Bus {
    /// Connects to the session bus, giving up at `deadline`.
    pub(crate) fn session(deadline: Instant) -> Result<Self, BusError> {
        let stream = address::session()
            .iter()
            .find_map(|address| connect(address).ok())
            .ok_or(BusError::NoBus)?;
        Self::over(stream, deadline)
    }

    /// Authenticates over `stream` and says `Hello`.
    pub(crate) fn over(stream: UnixStream, deadline: Instant) -> Result<Self, BusError> {
        let mut bus = Self {
            stream: BufReader::new(stream),
            serial: 0,
            deadline,
        };
        bus.authenticate()?;
        bus.call(&MethodCall {
            destination: "org.freedesktop.DBus",
            path: "/org/freedesktop/DBus",
            interface: "org.freedesktop.DBus",
            member: "Hello",
            strings: None,
        })?;
        Ok(bus)
    }

    /// Sets the socket's timeout to what is left before the deadline.
    fn arm(&self) -> Result<(), BusError> {
        let left = self
            .deadline
            .checked_duration_since(Instant::now())
            .filter(|left| !left.is_zero())
            .ok_or(BusError::Io(io::ErrorKind::TimedOut))?;
        self.stream.get_ref().set_read_timeout(Some(left))?;
        self.stream.get_ref().set_write_timeout(Some(left))?;
        Ok(())
    }

    /// `EXTERNAL` authentication with the process's user id.
    fn authenticate(&mut self) -> Result<(), BusError> {
        let uid = std::fs::metadata("/proc/self")?.uid();
        // The id's decimal digits, each as its ASCII code in hexadecimal
        // ('0' is 0x30 ... '9' is 0x39).
        let hex: String = uid
            .to_string()
            .chars()
            .flat_map(|digit| ['3', digit])
            .collect();
        self.arm()?;
        self.stream
            .get_mut()
            .write_all(format!("\0AUTH EXTERNAL {hex}\r\n").as_bytes())?;
        let mut line = Vec::new();
        (&mut self.stream)
            .take(MAX_AUTH_LINE as u64)
            .read_until(b'\n', &mut line)?;
        if !line.starts_with(b"OK ") {
            return Err(BusError::Refused);
        }
        self.stream.get_mut().write_all(b"BEGIN\r\n")?;
        Ok(())
    }

    /// Calls `call` and returns its reply's values, or its error.
    pub(crate) fn call(&mut self, call: &MethodCall<'_>) -> Result<Vec<Value>, BusError> {
        self.serial = self.serial.wrapping_add(1).max(1);
        let serial = self.serial;
        self.arm()?;
        self.stream.get_mut().write_all(&call.encode(serial))?;
        loop {
            let reply = self.read_message()?;
            if reply.reply_serial != Some(serial) {
                // A signal (`NameAcquired`), or a reply to nobody.
                continue;
            }
            match reply.kind {
                METHOD_RETURN => return Ok(reply.body),
                ERROR => return Err(BusError::Reply(reply.error_name.unwrap_or_default())),
                _ => {}
            }
        }
    }

    /// Reads the next whole message.
    fn read_message(&mut self) -> Result<message::Message, BusError> {
        self.arm()?;
        let mut bytes = vec![0; FIXED_HEADER];
        self.stream.read_exact(&mut bytes)?;
        let length = message_length(&bytes)?;
        bytes.resize(length, 0);
        self.stream.read_exact(&mut bytes[FIXED_HEADER..])?;
        message::parse(&bytes)
    }
}

/// Connects to a Unix-socket address.
fn connect(address: &SocketAddress) -> io::Result<UnixStream> {
    match address {
        SocketAddress::Path(path) => UnixStream::connect(path),
        SocketAddress::Abstract(name) => {
            UnixStream::connect_addr(&SocketAddr::from_abstract_name(name)?)
        }
    }
}

#[cfg(test)]
mod tests;
