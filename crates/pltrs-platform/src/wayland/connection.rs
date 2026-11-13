//! A connection to the compositor: object ids and their interfaces,
//! sending requests, and turning received bytes and descriptors into
//! [`Event`]s.

use std::collections::{HashMap, VecDeque};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::sync::Arc;

use super::protocol::core::{self, DISPLAY};
use super::protocol::{Event, Interface};
use super::transport::{self, SharedOutput};
use super::wire::{Args, HEADER_LEN, Header, MAX_MESSAGE_LEN, Request};
use crate::backend::Wait;
use crate::blocking::Blocking;
use crate::error::{Error, WaylandError};

/// The first id of objects the compositor creates.
const SERVER_ID_BASE: u32 = 0xff00_0000;
/// The most descriptors kept waiting for their messages; more means the
/// compositor sends descriptors without messages.
const MAX_QUEUED_FDS: usize = 1024;

/// The id the waker's `wl_display.sync` callbacks always use. Compositors
/// insist that new ids follow each other without gaps, so the connection
/// claims it with the first request it sends.
pub(crate) const WAKE_CALLBACK: u32 = 2;

/// An event with the object that sent it and the object's interface
/// (which may be destroyed by the time the event is handled).
#[derive(Debug)]
pub(crate) struct Received {
    /// The sender.
    pub(crate) object: u32,
    /// The sender's interface.
    pub(crate) interface: Interface,
    /// The event.
    pub(crate) event: Event,
}

/// One connection to the compositor.
#[derive(Debug)]
pub(crate) struct Connection {
    stream: Arc<UnixStream>,
    output: SharedOutput,
    /// Received bytes not yet parsed.
    input: Vec<u8>,
    /// Received descriptors not yet claimed by a message.
    fds: VecDeque<OwnedFd>,
    /// The interface of every live object.
    objects: HashMap<u32, Interface>,
    /// Client ids freed by `wl_display.delete_id`.
    free_ids: Vec<u32>,
    next_id: u32,
}

impl Connection {
    /// Wraps a connected socket, queuing the request that claims
    /// [`WAKE_CALLBACK`] (its callback is done like a wake-up's).
    pub(crate) fn new(stream: UnixStream) -> Result<Self, Error> {
        let stream = Arc::new(stream);
        let objects = HashMap::from([
            (DISPLAY, Interface::Display),
            (WAKE_CALLBACK, Interface::Callback),
        ]);
        let mut conn = Self {
            output: SharedOutput::new(Arc::clone(&stream)),
            stream,
            input: Vec::new(),
            fds: VecDeque::new(),
            objects,
            free_ids: Vec::new(),
            next_id: WAKE_CALLBACK + 1,
        };
        conn.send(core::sync(WAKE_CALLBACK))?;
        Ok(conn)
    }

    /// The output, shared with the waker.
    pub(crate) fn output(&self) -> SharedOutput {
        self.output.clone()
    }

    /// Allocates an id for a new object of `interface`.
    pub(crate) fn new_id(&mut self, interface: Interface) -> Result<u32, Error> {
        let id = match self.free_ids.pop() {
            Some(id) => id,
            None if self.next_id < SERVER_ID_BASE => {
                self.next_id += 1;
                self.next_id - 1
            }
            None => return Err(Error::ResourceIdsExhausted),
        };
        self.objects.insert(id, interface);
        Ok(id)
    }

    /// The interface of a live object.
    pub(crate) fn interface(&self, id: u32) -> Option<Interface> {
        self.objects.get(&id).copied()
    }

    /// Queues a request.
    pub(crate) fn send(&mut self, request: Request) -> Result<(), Error> {
        let (bytes, fds) = request.finish();
        self.output.lock().send(&bytes, &fds)
    }

    /// Queues a destructor request for an object the compositor created,
    /// forgetting the object at once (only client ids are confirmed with
    /// `delete_id`).
    pub(crate) fn destroy_server_object(&mut self, request: Request, id: u32) -> Result<(), Error> {
        self.objects.remove(&id);
        self.send(request)
    }

    /// Writes every queued request.
    pub(crate) fn flush(&mut self) -> Result<(), Error> {
        self.output.lock().flush()
    }

    /// Waits as `wait` allows for data (through `blocking`), then returns
    /// every complete event received, in order. Fatal protocol errors and
    /// a closed connection are errors.
    pub(crate) fn read_events(
        &mut self,
        wait: Wait,
        blocking: Blocking<'_>,
    ) -> Result<Vec<Received>, Error> {
        self.flush()?;
        let mut events = Vec::new();
        let mut wait = wait;
        loop {
            self.parse_available(&mut events)?;
            if !events.is_empty() && wait != Wait::Poll {
                wait = Wait::Poll;
            }
            if !self.receive(wait, blocking)? {
                return Ok(events);
            }
        }
    }

    /// Sends `wl_display.sync` and reads until it is done; returns the
    /// events received meanwhile.
    pub(crate) fn round_trip(&mut self) -> Result<Vec<Received>, Error> {
        let callback = self.new_id(Interface::Callback)?;
        self.send(core::sync(callback))?;
        self.flush()?;
        let mut events = Vec::new();
        loop {
            self.parse_available(&mut events)?;
            if let Some(at) = events.iter().position(|received| {
                received.object == callback && matches!(received.event, Event::Done)
            }) {
                events.remove(at);
                return Ok(events);
            }
            self.receive(Wait::Forever, Blocking::DIRECT)?;
        }
    }

    /// Reads once (a read that may wait, through `blocking`); returns
    /// whether anything arrived.
    fn receive(&mut self, wait: Wait, blocking: Blocking<'_>) -> Result<bool, Error> {
        let mut chunk = [0; MAX_MESSAGE_LEN];
        let mut fds = Vec::new();
        let stream = &self.stream;
        let received = if wait == Wait::Poll {
            transport::read(stream, &mut chunk, &mut fds, wait)
        } else {
            blocking.run(|| transport::read(stream, &mut chunk, &mut fds, wait))
        };
        let received = match received {
            Ok(received) => received,
            Err(error) => return Err(self.fail(error)),
        };
        self.fds.extend(fds);
        if self.fds.len() > MAX_QUEUED_FDS {
            return Err(self.fail(Error::Malformed("descriptors without messages")));
        }
        match received {
            None => Ok(false),
            Some(0) => Err(self.fail(Error::ConnectionClosed)),
            Some(n) => {
                self.input.extend_from_slice(&chunk[..n]);
                Ok(true)
            }
        }
    }

    /// Parses the complete messages received so far into `events`.
    fn parse_available(&mut self, events: &mut Vec<Received>) -> Result<(), Error> {
        let mut input = std::mem::take(&mut self.input);
        let mut at = 0;
        let result = loop {
            let header = match Header::parse(&input[at..]) {
                Ok(Some(header)) if at + header.len <= input.len() => header,
                Ok(_) => break Ok(()),
                Err(error) => break Err(error),
            };
            let body = &input[at + HEADER_LEN..at + header.len];
            at += header.len;
            match self.parse_message(header, body) {
                Ok(Some(received)) => events.push(received),
                Ok(None) => {}
                Err(error) => break Err(error),
            }
        };
        input.drain(..at);
        self.input = input;
        result.map_err(|error| self.fail(error))
    }

    /// Parses one message, handling the ones about objects themselves.
    fn parse_message(&mut self, header: Header, body: &[u8]) -> Result<Option<Received>, Error> {
        // Messages for objects destroyed meanwhile are dropped; only
        // client objects carry descriptors, and those stay known until
        // the compositor confirms their destruction.
        let Some(interface) = self.interface(header.object) else {
            return Ok(None);
        };
        let mut args = Args::new(body, &mut self.fds);
        let event = Event::parse(interface, header.opcode, &mut args)?;
        let event = match event {
            Event::Error {
                object,
                code,
                message,
            } => {
                return Err(Error::Wayland(WaylandError {
                    object,
                    code,
                    message,
                }));
            }
            Event::DeleteId(id) => {
                if id != WAKE_CALLBACK && id < SERVER_ID_BASE && self.objects.remove(&id).is_some()
                {
                    self.free_ids.push(id);
                }
                return Ok(None);
            }
            Event::DataOffer(id) if id >= SERVER_ID_BASE => {
                self.objects.insert(id, Interface::DataOffer);
                Event::DataOffer(id)
            }
            Event::DataOffer(_) => {
                return Err(Error::Malformed("an offer with a client object id"));
            }
            event => event,
        };
        Ok(Some(Received {
            object: header.object,
            interface,
            event,
        }))
    }

    /// Closes the output after a fatal error, so that the waker stops
    /// writing, and returns the error.
    fn fail(&self, error: Error) -> Error {
        self.output.lock().close();
        error
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.output.lock().close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wayland::sys;

    /// A message as the compositor sends it.
    fn event(object: u32, opcode: u16, args: impl FnOnce(Request) -> Request) -> Vec<u8> {
        args(Request::new(object, opcode)).finish().0
    }

    fn pair() -> (Connection, UnixStream) {
        let (ours, theirs) = UnixStream::pair().unwrap();
        (Connection::new(ours).unwrap(), theirs)
    }

    #[test]
    fn ids_are_allocated_and_reused_after_deletion() {
        let (mut conn, server) = pair();
        let first = conn.new_id(Interface::Surface).unwrap();
        let second = conn.new_id(Interface::Buffer).unwrap();
        assert_eq!((first, second), (3, 4));
        assert_eq!(conn.interface(4), Some(Interface::Buffer));
        let delete = event(DISPLAY, 1, |r| r.uint(first));
        let wake_delete = event(DISPLAY, 1, |r| r.uint(WAKE_CALLBACK));
        sys::send(&server, &[delete, wake_delete].concat(), &[]).unwrap();
        assert!(
            conn.read_events(Wait::Poll, Blocking::DIRECT)
                .unwrap()
                .is_empty()
        );
        assert_eq!(conn.interface(first), None);
        assert_eq!(conn.interface(WAKE_CALLBACK), Some(Interface::Callback));
        assert_eq!(conn.new_id(Interface::Viewport).unwrap(), first);
    }

    #[test]
    fn events_are_parsed_by_interface_and_split_messages_wait() {
        let (mut conn, server) = pair();
        let registry = conn.new_id(Interface::Registry).unwrap();
        let global = event(registry, 0, |r| r.uint(9).string("wl_shm").uint(2));
        sys::send(&server, &global[..10], &[]).unwrap();
        assert!(
            conn.read_events(Wait::Poll, Blocking::DIRECT)
                .unwrap()
                .is_empty(),
            "half a message"
        );
        sys::send(&server, &global[10..], &[]).unwrap();
        let events = conn.read_events(Wait::Forever, Blocking::DIRECT).unwrap();
        assert!(matches!(
            &events[..],
            [Received { object, interface: Interface::Registry, event: Event::Global { name: 9, interface, version: 2 } }]
                if *object == registry && interface == "wl_shm"
        ));
        let unknown = event(77, 0, |r| r.uint(1));
        sys::send(&server, &unknown, &[]).unwrap();
        assert!(
            conn.read_events(Wait::Poll, Blocking::DIRECT)
                .unwrap()
                .is_empty(),
            "unknown objects"
        );
    }

    #[test]
    fn descriptors_reach_their_events() {
        let (mut conn, server) = pair();
        let keyboard = conn.new_id(Interface::Keyboard).unwrap();
        let keymap = event(keyboard, 0, |r| r.uint(1).uint(12));
        let (reader, _writer) = std::io::pipe().unwrap();
        let fd = OwnedFd::from(reader);
        sys::send(&server, &keymap, &[std::os::fd::AsFd::as_fd(&fd)]).unwrap();
        let events = conn.read_events(Wait::Forever, Blocking::DIRECT).unwrap();
        assert!(matches!(
            &events[..],
            [Received {
                event: Event::Keymap {
                    format: 1,
                    size: 12,
                    ..
                },
                ..
            }]
        ));
        let missing = event(keyboard, 0, |r| r.uint(1).uint(12));
        sys::send(&server, &missing, &[]).unwrap();
        assert!(matches!(
            conn.read_events(Wait::Forever, Blocking::DIRECT),
            Err(Error::Malformed(_))
        ));
    }

    #[test]
    fn protocol_errors_and_closing_are_fatal() {
        let (mut conn, server) = pair();
        let error = event(DISPLAY, 0, |r| r.uint(5).uint(2).string("bad"));
        sys::send(&server, &error, &[]).unwrap();
        assert_eq!(
            conn.read_events(Wait::Forever, Blocking::DIRECT)
                .unwrap_err(),
            Error::Wayland(WaylandError {
                object: 5,
                code: 2,
                message: String::from("bad")
            })
        );
        assert_eq!(
            conn.send(core::sync(9)),
            Err(Error::EventLoopClosed),
            "the output closed"
        );
        let (mut conn, server) = pair();
        drop(server);
        assert_eq!(
            conn.read_events(Wait::Forever, Blocking::DIRECT)
                .unwrap_err(),
            Error::ConnectionClosed
        );
    }

    #[test]
    fn round_trips_return_the_events_before_done() {
        let (mut conn, server) = pair();
        let compositor = std::thread::spawn(move || {
            let mut buf = [0; 64];
            let mut fds = Vec::new();
            let n = sys::receive(&server, &mut buf, &mut fds, true).unwrap();
            let words: Vec<u32> = buf[..n]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|&w| u32::from_ne_bytes(w))
                .collect();
            assert_eq!(
                words,
                [DISPLAY, 12 << 16, WAKE_CALLBACK, DISPLAY, 12 << 16, 3]
            );
            let callback = words[5];
            let ping = event(DISPLAY, 1, |r| r.uint(99));
            let done = event(callback, 0, |r| r.uint(0));
            sys::send(&server, &[ping, done].concat(), &[]).unwrap();
            server
        });
        assert!(conn.round_trip().unwrap().is_empty());
        drop(compositor.join().unwrap());
    }
}
