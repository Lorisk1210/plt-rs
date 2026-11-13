//! Message parsing: headers and arguments of random bytes, events of
//! every interface with made-up arguments, and byte streams cut anywhere
//! read by a connection.

use std::collections::VecDeque;
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::net::UnixStream;

use super::{Arg, args, message};
use crate::backend::Wait;
use crate::blocking::Blocking;
use crate::fuzz::fuzz;
use crate::wayland::connection::Connection;
use crate::wayland::protocol::{Event, Interface};
use crate::wayland::sys;
use crate::wayland::wire::{Args, HEADER_LEN, Header};

/// Every interface with events.
const INTERFACES: [Interface; 17] = [
    Interface::Display,
    Interface::Registry,
    Interface::Callback,
    Interface::Buffer,
    Interface::Surface,
    Interface::Output,
    Interface::Seat,
    Interface::Pointer,
    Interface::Keyboard,
    Interface::DataDevice,
    Interface::DataOffer,
    Interface::DataSource,
    Interface::WmBase,
    Interface::XdgSurface,
    Interface::Toplevel,
    Interface::Decoration,
    Interface::FractionalScale,
];

#[test]
fn headers_and_arguments() {
    fuzz(
        "wayland wire",
        3_000,
        |g| {
            (0..g.len(64))
                .map(|_| g.below(256) as u8)
                .collect::<Vec<u8>>()
        },
        |bytes| {
            if let Ok(Some(header)) = Header::parse(bytes) {
                assert!(header.len >= HEADER_LEN && header.len % 4 == 0);
            }
            let mut fds = VecDeque::new();
            let mut args = Args::new(bytes, &mut fds);
            for _ in 0..8 {
                let _ = args.uint();
                let _ = args.optional_string();
                let _ = args.array();
                let _ = args.fixed();
            }
            assert!(args.fd().is_err());
        },
    );
}

/// An event to parse: an interface, an opcode and arguments, perhaps
/// with a descriptor.
#[derive(Debug)]
struct Made {
    interface: Interface,
    opcode: u16,
    args: Vec<Arg>,
    fd: bool,
}

#[test]
fn events_of_every_interface() {
    fuzz(
        "wayland events",
        5_000,
        |g| Made {
            interface: g.pick(&INTERFACES),
            opcode: g.below(12) as u16,
            args: args(g),
            fd: g.chance(0.3),
        },
        |made| {
            let (bytes, _) = message(1, made.opcode, &made.args).finish();
            let mut fds = VecDeque::new();
            if made.fd {
                let (reader, _) = std::io::pipe().expect("a pipe");
                fds.push_back(OwnedFd::from(reader));
            }
            let mut args = Args::new(&bytes[HEADER_LEN..], &mut fds);
            if let Ok(Event::Keymap { size, .. }) =
                Event::parse(made.interface, made.opcode, &mut args)
            {
                assert!(made.fd, "a keymap needs its descriptor ({size})");
            }
        },
    );
}

/// A stream of messages for the objects of a connection, cut and
/// corrupted.
#[derive(Debug)]
struct Stream {
    messages: Vec<(usize, u16, Vec<Arg>)>,
    fds: usize,
    corruption: crate::fuzz::Corruption,
}

/// The bytes of messages to objects 1 (the display) onwards.
fn encode(messages: &[(usize, u16, Vec<Arg>)]) -> Vec<u8> {
    messages
        .iter()
        .flat_map(|(object, opcode, args)| message(*object as u32 + 1, *opcode, args).finish().0)
        .collect()
}

#[test]
fn streams_read_by_a_connection() {
    fuzz(
        "wayland streams",
        1_500,
        |g| {
            let messages = (0..g.len(12))
                .map(|_| (g.below(INTERFACES.len() + 2), g.below(12) as u16, args(g)))
                .collect::<Vec<_>>();
            let corruption = g.corrupt(&encode(&messages));
            Stream {
                messages,
                fds: g.below(4),
                corruption,
            }
        },
        |stream| {
            let (ours, theirs) = UnixStream::pair().expect("a socket pair");
            let mut conn = Connection::new(ours).expect("the first request queues");
            for interface in &INTERFACES[1..] {
                conn.new_id(*interface).expect("ids are left");
            }
            let bytes = stream.corruption.apply(&encode(&stream.messages));
            let pipes: Vec<_> = (0..stream.fds)
                .map(|_| std::io::pipe().expect("a pipe").0)
                .collect();
            let fds: Vec<_> = pipes.iter().map(AsFd::as_fd).collect();
            for (index, chunk) in bytes.chunks(512).enumerate() {
                let attached = if index == 0 { &fds[..] } else { &[] };
                sys::send(&theirs, chunk, attached).expect("the connection reads");
            }
            drop(theirs);
            for _ in 0..4 {
                if conn.read_events(Wait::Poll, Blocking::DIRECT).is_err() {
                    break;
                }
            }
        },
    );
}
