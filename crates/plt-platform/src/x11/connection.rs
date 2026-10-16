//! A connection to an X server: the setup handshake, resource ids, and the
//! demultiplexing of the replies, errors and events read from the socket.
//!
//! Requests go through the [`SharedOutput`], which numbers them. Messages
//! from the server carry the low 16 bits of the sequence number of the
//! request they answer (or of the last request processed, for events);
//! [`Connection`] widens them to 64 bits by tracking the last one seen,
//! which is unambiguous because the output makes the server answer at
//! least once every 2¹⁶ requests.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use super::auth::Credentials;
use super::display::DisplayName;
use super::output::SharedOutput;
use super::reply;
use super::request::{self, Query, Request};
use super::setup::{self, Setup};
use super::transport::{Stream, Wait};
use super::wire::{u16_at, u32_at};
use crate::error::{Error, X11Error};

/// Bytes requested from the socket per read.
const READ_CHUNK: usize = 64 * 1024;
/// Length of errors, events and reply headers.
const MESSAGE_LEN: usize = 32;
/// Message type of errors.
const ERROR: u8 = 0;
/// Message type of replies.
const REPLY: u8 = 1;
/// The one event without a sequence number.
const KEYMAP_NOTIFY: u8 = 11;
/// Extension events longer than 32 bytes (not selected by this crate).
const GENERIC_EVENT: u8 = 35;

/// A raw 32-byte event.
pub(crate) type RawEvent = [u8; MESSAGE_LEN];

/// Identifies a reply still to be collected with [`Connection::reply`].
#[must_use = "the reply must be collected"]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReplyCookie(u64);

/// Hands out resource ids from the range the server assigned.
#[derive(Debug, Clone, PartialEq, Eq)]
struct IdAllocator {
    base: u32,
    mask: u32,
    /// Distance between consecutive ids: the lowest bit of the mask.
    step: u32,
    /// Number of ids handed out so far.
    issued: u32,
}

impl IdAllocator {
    fn new(base: u32, mask: u32) -> Self {
        Self {
            base,
            mask,
            step: mask.isolate_lowest_one(),
            issued: 0,
        }
    }

    fn next(&mut self) -> Result<u32, Error> {
        let offset = self
            .issued
            .checked_add(1)
            .and_then(|n| n.checked_mul(self.step))
            .filter(|&offset| offset <= self.mask)
            .ok_or(Error::ResourceIdsExhausted)?;
        self.issued += 1;
        Ok(self.base | offset)
    }
}

/// An open connection to an X server.
#[derive(Debug)]
pub(crate) struct Connection {
    stream: Arc<Stream>,
    output: SharedOutput,
    setup: Setup,
    ids: IdAllocator,
    max_request_bytes: usize,
    /// Received bytes not yet parsed into messages.
    input: Vec<u8>,
    /// Widened sequence number of the last message received.
    last_sequence: u64,
    /// Requests whose reply or error is awaited.
    awaited: HashSet<u64>,
    /// Replies and errors of awaited requests, by sequence number.
    replies: HashMap<u64, Result<Vec<u8>, X11Error>>,
    events: VecDeque<RawEvent>,
    /// Errors of requests nobody waits for.
    async_errors: VecDeque<X11Error>,
}

impl Connection {
    /// Connects to the server named by `name`, authenticating with the
    /// user's Xauthority cookie, and enables BIG-REQUESTS if available.
    pub(crate) fn connect(name: &DisplayName) -> Result<Self, Error> {
        let (stream, credentials) = Stream::connect(name)?;
        Self::handshake(stream, credentials.as_ref())
    }

    /// Performs the connection setup over an open stream.
    fn handshake(stream: Stream, credentials: Option<&Credentials>) -> Result<Self, Error> {
        stream.write_all(&setup::encode_request(credentials))?;
        let mut header = [0; setup::REPLY_HEADER_LEN];
        stream.read_exact(&mut header)?;
        let mut reply = header.to_vec();
        reply.resize(header.len() + setup::reply_body_len(header), 0);
        stream.read_exact(&mut reply[header.len()..])?;
        let setup = setup::parse_reply(&reply)?;
        let stream = Arc::new(stream);
        let max_request_bytes = usize::from(setup.maximum_request_length) * 4;
        let mut connection = Self {
            output: SharedOutput::new(Arc::clone(&stream), max_request_bytes),
            stream,
            ids: IdAllocator::new(setup.resource_id_base, setup.resource_id_mask),
            setup,
            max_request_bytes,
            input: Vec::new(),
            last_sequence: 0,
            awaited: HashSet::new(),
            replies: HashMap::new(),
            events: VecDeque::new(),
            async_errors: VecDeque::new(),
        };
        connection.enable_big_requests()?;
        Ok(connection)
    }

    /// Raises the request size limit with the BIG-REQUESTS extension.
    fn enable_big_requests(&mut self) -> Result<(), Error> {
        let Some(major) = self.extension("BIG-REQUESTS")? else {
            return Ok(());
        };
        let reply = self.round_trip(&request::big_requests_enable(major))?;
        let words = reply::big_requests_enable(&reply)?;
        self.max_request_bytes = usize::try_from(words).map_or(usize::MAX, |w| w.saturating_mul(4));
        self.output
            .lock()
            .set_max_request_bytes(self.max_request_bytes);
        Ok(())
    }

    /// The major opcode of an extension, `None` if the server lacks it.
    pub(crate) fn extension(&mut self, name: &str) -> Result<Option<u8>, Error> {
        let reply = self.round_trip(&request::query_extension(name))?;
        reply::query_extension(&reply)
    }

    /// The server's setup information.
    pub(crate) fn setup(&self) -> &Setup {
        &self.setup
    }

    /// The sending half, for wakers on other threads.
    pub(crate) fn output(&self) -> SharedOutput {
        self.output.clone()
    }

    /// The largest request the server accepts, in bytes.
    pub(crate) fn max_request_bytes(&self) -> usize {
        self.max_request_bytes
    }

    /// Lowers the request size limit, to exercise request splitting.
    #[cfg(test)]
    pub(crate) fn limit_request_bytes(&mut self, bytes: usize) {
        self.max_request_bytes = self.max_request_bytes.min(bytes);
        self.output
            .lock()
            .set_max_request_bytes(self.max_request_bytes);
    }

    /// A fresh resource id for a window, GC, font, ...
    pub(crate) fn generate_id(&mut self) -> Result<u32, Error> {
        self.ids.next()
    }

    /// Queues a request without waiting. Should it fail, the error is
    /// reported by [`Connection::take_async_error`].
    pub(crate) fn send(&mut self, request: &Request) -> Result<(), Error> {
        self.output.lock().send(request.encoded(), false).map(drop)
    }

    /// Sends a request and waits until the server has processed it,
    /// returning its error if it failed.
    pub(crate) fn send_checked(&mut self, request: &Request) -> Result<(), Error> {
        let sequence = self.output.lock().send(request.encoded(), false)?;
        self.awaited.insert(sequence);
        // Replies arrive in request order: once this one is in, any error
        // of the checked request has arrived too.
        self.round_trip(&request::get_input_focus())?;
        self.awaited.remove(&sequence);
        match self.replies.remove(&sequence) {
            None => Ok(()),
            Some(Err(error)) => Err(Error::X11(error)),
            Some(Ok(_)) => Err(Error::Malformed("reply to a request that has none")),
        }
    }

    /// Sends a query; collect its reply with [`Connection::reply`].
    pub(crate) fn query(&mut self, query: &Query) -> Result<ReplyCookie, Error> {
        let sequence = self.output.lock().send(query.encoded(), true)?;
        self.awaited.insert(sequence);
        Ok(ReplyCookie(sequence))
    }

    /// Waits for the reply of a query sent earlier.
    pub(crate) fn reply(&mut self, cookie: ReplyCookie) -> Result<Vec<u8>, Error> {
        self.flush()?;
        loop {
            if let Some(result) = self.replies.remove(&cookie.0) {
                return result.map_err(Error::X11);
            }
            self.receive(Wait::Forever)?;
        }
    }

    /// Sends a query and waits for its reply.
    pub(crate) fn round_trip(&mut self, query: &Query) -> Result<Vec<u8>, Error> {
        let cookie = self.query(query)?;
        self.reply(cookie)
    }

    /// Writes all queued requests.
    pub(crate) fn flush(&mut self) -> Result<(), Error> {
        self.output.lock().flush()
    }

    /// Flushes, then returns the queued events, first waiting as `wait`
    /// allows if there are none (and no pending error).
    pub(crate) fn wait_for_events(&mut self, wait: Wait) -> Result<VecDeque<RawEvent>, Error> {
        self.flush()?;
        while self.events.is_empty() && self.async_errors.is_empty() {
            if !self.receive(wait)? || wait == Wait::Poll {
                break;
            }
        }
        Ok(std::mem::take(&mut self.events))
    }

    /// The oldest error of a request sent with [`Connection::send`].
    pub(crate) fn take_async_error(&mut self) -> Option<X11Error> {
        self.async_errors.pop_front()
    }

    /// Reads once from the socket and parses the complete messages.
    /// Returns `false` if nothing arrived in time.
    fn receive(&mut self, wait: Wait) -> Result<bool, Error> {
        let start = self.input.len();
        self.input.resize(start + READ_CHUNK, 0);
        let result = if wait == Wait::Poll {
            // Polling makes the socket briefly non-blocking; keep wakers
            // from writing meanwhile.
            let _output = self.output.lock();
            self.stream.read(&mut self.input[start..], wait)
        } else {
            self.stream.read(&mut self.input[start..], wait)
        };
        let received = result.as_ref().map_or(0, |n| n.unwrap_or(0));
        self.input.truncate(start + received);
        match result {
            Err(error) => Err(Error::io("reading from the X server", &error)),
            Ok(None) => Ok(false),
            Ok(Some(0)) => Err(Error::ConnectionClosed),
            Ok(Some(_)) => {
                self.parse_input()?;
                Ok(true)
            }
        }
    }

    fn parse_input(&mut self) -> Result<(), Error> {
        let input = std::mem::take(&mut self.input);
        let mut offset = 0;
        let result = loop {
            match message_len(&input[offset..]) {
                Ok(Some(len)) => {
                    self.dispatch(&input[offset..offset + len]);
                    offset += len;
                }
                Ok(None) => break Ok(()),
                Err(error) => break Err(error),
            }
        };
        self.input = input;
        self.input.drain(..offset);
        result
    }

    /// Routes one complete message (at least 32 bytes long).
    fn dispatch(&mut self, message: &[u8]) {
        let Some(header) = message.first_chunk::<MESSAGE_LEN>() else {
            return;
        };
        match header[0] {
            ERROR => {
                let error = X11Error {
                    code: header[1],
                    sequence: self.widen(header),
                    bad_value: u32_at(header, 4),
                    minor_opcode: u16_at(header, 8),
                    major_opcode: header[10],
                };
                if self.awaited.remove(&error.sequence) {
                    self.replies.insert(error.sequence, Err(error));
                } else {
                    self.async_errors.push_back(error);
                }
            }
            REPLY => {
                let sequence = self.widen(header);
                if self.awaited.remove(&sequence) {
                    self.replies.insert(sequence, Ok(message.to_vec()));
                }
            }
            code => {
                if code & 0x7f != KEYMAP_NOTIFY {
                    self.widen(header);
                }
                if code & 0x7f != GENERIC_EVENT {
                    self.events.push_back(*header);
                }
            }
        }
    }

    /// Widens the 16-bit sequence number of a message.
    fn widen(&mut self, header: &[u8; MESSAGE_LEN]) -> u64 {
        let low = u16_at(header, 2);
        // Truncation intended: only the low 16 bits are compared.
        let delta = low.wrapping_sub(self.last_sequence as u16);
        self.last_sequence += u64::from(delta);
        self.last_sequence
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.output.lock().close();
    }
}

/// The length of the message at the start of `bytes`, or `None` if it has
/// not fully arrived.
fn message_len(bytes: &[u8]) -> Result<Option<usize>, Error> {
    let Some(header) = bytes.first_chunk::<MESSAGE_LEN>() else {
        return Ok(None);
    };
    let len = if header[0] == REPLY || header[0] & 0x7f == GENERIC_EVENT {
        usize::try_from(u32_at(header, 4))
            .ok()
            .and_then(|words| words.checked_mul(4))
            .and_then(|extra| extra.checked_add(MESSAGE_LEN))
            .ok_or(Error::Malformed("reply length overflows"))?
    } else {
        MESSAGE_LEN
    };
    Ok((bytes.len() >= len).then_some(len))
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;
    use std::thread;

    use super::*;
    use crate::x11::fixtures;

    /// The server side of a socket pair, speaking just enough protocol.
    struct FakeServer(UnixStream);

    impl FakeServer {
        /// Reads one request and returns its opcode.
        fn read_request(&mut self) -> u8 {
            let mut header = [0; 4];
            self.0.read_exact(&mut header).unwrap();
            let words = usize::from(u16::from_le_bytes([header[2], header[3]]));
            let mut rest = vec![0; words * 4 - 4];
            self.0.read_exact(&mut rest).unwrap();
            header[0]
        }

        fn write(&mut self, message: &[u8]) {
            self.0.write_all(message).unwrap();
        }
    }

    fn message(kind: u8, detail: u8, sequence: u16, fields: &[u32]) -> Vec<u8> {
        let mut out = vec![kind, detail];
        out.extend(sequence.to_le_bytes());
        for field in fields {
            out.extend(field.to_le_bytes());
        }
        out.resize(MESSAGE_LEN, 0);
        out
    }

    /// A connection to a fake `XWayland` server without BIG-REQUESTS.
    fn connect() -> (Connection, FakeServer) {
        let (client, server) = UnixStream::pair().unwrap();
        let handle = thread::spawn(move || {
            let mut server = FakeServer(server);
            let mut setup = [0; 12];
            server.0.read_exact(&mut setup).unwrap();
            let auth_len = |at: usize| {
                let len = usize::from(u16::from_le_bytes([setup[at], setup[at + 1]]));
                len.div_ceil(4) * 4
            };
            let mut auth = vec![0; auth_len(6) + auth_len(8)];
            server.0.read_exact(&mut auth).unwrap();
            server.write(fixtures::SETUP);
            assert_eq!(server.read_request(), 98, "QueryExtension");
            server.write(&message(REPLY, 0, 1, &[0, 0]));
            server
        });
        let connection = Connection::handshake(Stream::Unix(client), None).unwrap();
        (connection, handle.join().unwrap())
    }

    #[test]
    fn handshake_reads_setup() {
        let (connection, _server) = connect();
        assert_eq!(connection.setup().screens.len(), 1);
        assert_eq!(connection.max_request_bytes(), 0xffff * 4);
    }

    #[test]
    fn replies_errors_and_events_are_demultiplexed() {
        let (mut connection, mut server) = connect();
        let first = connection.query(&request::intern_atom("A")).unwrap();
        let second = connection.query(&request::intern_atom("B")).unwrap();
        connection.send(&request::map_window(1)).unwrap();
        connection.flush().unwrap();
        for _ in 0..3 {
            server.read_request();
        }
        // Sequence 1 was the handshake's QueryExtension; then come the two
        // queries (2, 3) and MapWindow (4).
        server.write(&message(12, 0, 1, &[0x42])); // Expose
        server.write(&message(REPLY, 0, 2, &[0, 7]));
        server.write(&message(ERROR, 5, 3, &[0, 0]));
        server.write(&message(ERROR, 3, 4, &[1, 0x0008_0000]));
        assert_eq!(
            connection.reply(second),
            Err(Error::X11(X11Error {
                code: 5,
                sequence: 3,
                bad_value: 0,
                major_opcode: 0,
                minor_opcode: 0,
            }))
        );
        assert_eq!(reply::intern_atom(&connection.reply(first).unwrap()), Ok(7));
        let error = connection.take_async_error().unwrap();
        assert_eq!((error.code, error.bad_value, error.major_opcode), (3, 1, 8));
        let events = connection.wait_for_events(Wait::Poll).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0][4], 0x42);
    }

    #[test]
    fn checked_requests_report_their_error() {
        let (mut connection, mut server) = connect();
        let replier = thread::spawn(move || {
            assert_eq!(server.read_request(), 4, "DestroyWindow");
            assert_eq!(server.read_request(), 43, "GetInputFocus");
            server.write(&message(ERROR, 3, 2, &[0x99, 0x0004_0000]));
            server.write(&message(REPLY, 0, 3, &[]));
            server
        });
        let result = connection.send_checked(&request::destroy_window(0x99));
        assert!(matches!(result, Err(Error::X11(X11Error { code: 3, .. }))));
        let mut server = replier.join().unwrap();
        let replier = thread::spawn(move || {
            server.read_request();
            server.read_request();
            server.write(&message(REPLY, 0, 5, &[]));
            server
        });
        assert_eq!(connection.send_checked(&request::map_window(1)), Ok(()));
        drop(replier.join().unwrap());
    }

    #[test]
    fn sequence_numbers_widen_across_wraparound() {
        let (mut connection, _server) = connect();
        connection.last_sequence = 0xfff0;
        assert_eq!(
            connection.widen(&message(12, 0, 0xfffe, &[]).try_into().unwrap()),
            0xfffe
        );
        assert_eq!(
            connection.widen(&message(12, 0, 0x0003, &[]).try_into().unwrap()),
            0x1_0003
        );
        assert_eq!(
            connection.widen(&message(12, 0, 0x0003, &[]).try_into().unwrap()),
            0x1_0003
        );
    }

    #[test]
    fn long_replies_wait_for_all_their_bytes() {
        let header = message(REPLY, 0, 1, &[2]);
        assert_eq!(message_len(&header), Ok(None));
        let mut full = header.clone();
        full.extend([0; 8]);
        assert_eq!(message_len(&full), Ok(Some(40)));
        assert_eq!(message_len(&message(12, 0, 1, &[])), Ok(Some(32)));
        assert_eq!(message_len(&[1; 31]), Ok(None));
    }

    #[test]
    fn closed_connections_are_reported() {
        let (mut connection, server) = connect();
        drop(server);
        assert_eq!(
            connection.wait_for_events(Wait::Forever),
            Err(Error::ConnectionClosed)
        );
    }

    #[test]
    fn resource_ids_follow_the_mask() {
        let mut ids = IdAllocator::new(0x0020_0000, 0x001f_ffff);
        assert_eq!(ids.next(), Ok(0x0020_0001));
        assert_eq!(ids.next(), Ok(0x0020_0002));
        let mut ids = IdAllocator::new(0x0400_0000, 0x0000_0030);
        assert_eq!(ids.next(), Ok(0x0400_0010));
        assert_eq!(ids.next(), Ok(0x0400_0020));
        assert_eq!(ids.next(), Ok(0x0400_0030));
        assert_eq!(ids.next(), Err(Error::ResourceIdsExhausted));
    }
}
