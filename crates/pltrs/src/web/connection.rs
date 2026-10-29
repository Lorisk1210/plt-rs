//! One client connection: HTTP requests answered in turn until the client
//! leaves or the connection becomes a figure's WebSocket, whose messages
//! are then parsed and passed to the figures' thread while a writer
//! thread sends what the figure answers.

use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread;
use std::time::{Duration, Instant};

use super::http::{self, HttpError, Method, Request, Response, Status};
use super::net::{ConnectionId, Inbound, Outbound, Shared};
use super::protocol::ClientMessage;
use super::routes::{self, Route};
use super::websocket::{self, Message, MessageReader, Opcode, WsError, frame};

/// How long a client has to send a complete request, waiting included.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// How long writing to a client may block.
const WRITE_TIMEOUT: Duration = Duration::from_secs(30);

/// How long, and how much of, what a refused client still sends is read
/// before the connection closes.
const LINGER: (Duration, u64) = (Duration::from_secs(1), 1 << 20);

/// The largest WebSocket message a client may send.
pub(super) const MAX_MESSAGE: usize = 64 * 1024;

/// How many frames may wait for a WebSocket's writer; a client that falls
/// further behind is disconnected.
pub(crate) const OUTBOX_CAPACITY: usize = 256;

/// A socket read with an optional deadline for the whole of what is being
/// read, so that a client trickling bytes cannot hold a request open.
#[derive(Debug)]
struct Input {
    stream: TcpStream,
    deadline: Option<Instant>,
}

impl Read for Input {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let timeout = match self.deadline {
            None => None,
            Some(deadline) => {
                let left = deadline.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    return Err(io::ErrorKind::TimedOut.into());
                }
                Some(left)
            }
        };
        self.stream.set_read_timeout(timeout)?;
        self.stream.read(buf)
    }
}

/// Serves a connection until it closes.
pub(crate) fn serve(stream: TcpStream, shared: &Shared, id: ConnectionId) {
    // Small messages go out at once; without these the connection is just
    // slower or less robust.
    let _ = stream.set_nodelay(true);
    let _ = stream.set_write_timeout(Some(WRITE_TIMEOUT));
    let Ok(read_half) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(Input {
        stream: read_half,
        deadline: None,
    });
    let mut writer = stream;
    loop {
        reader.get_mut().deadline = Some(Instant::now() + REQUEST_TIMEOUT);
        let request = match next_request(&mut reader) {
            Ok(Some(request)) => request,
            Ok(None) => return,
            Err(err) => {
                if let Some(status) = err.status() {
                    refuse(reader, writer, &Response::error(status));
                }
                return;
            }
        };
        if request.method == Method::Other {
            let refusal =
                Response::error(Status::MethodNotAllowed).with_header("Allow", "GET, HEAD");
            refuse(reader, writer, &refusal);
            return;
        }
        let route = routes::route(&request.path);
        if let Route::WebSocket(figure) = route
            && websocket::is_upgrade(&request)
        {
            upgrade(reader, writer, &request, figure, shared, id);
            return;
        }
        let response = routes::respond(route, &request, shared);
        let keep_alive = request.keep_alive();
        let written = response.write_to(&mut writer, request.method == Method::Head, keep_alive);
        if written.is_err() || !keep_alive {
            return;
        }
    }
}

/// Answers with an error and closes the connection, reading what the
/// client still sends for a moment first: closing with unread input would
/// reset the connection and lose the answer.
fn refuse(mut reader: BufReader<Input>, mut writer: TcpStream, response: &Response) {
    if response.write_to(&mut writer, false, false).is_err() {
        return;
    }
    let _ = writer.shutdown(Shutdown::Write);
    let (time, bytes) = LINGER;
    reader.get_mut().deadline = Some(Instant::now() + time);
    let _ = io::copy(&mut reader.take(bytes), &mut io::sink());
}

/// The next request head, `None` once the client is done.
fn next_request(reader: &mut BufReader<Input>) -> Result<Option<Request>, HttpError> {
    http::read_head(reader)?
        .map(|head| http::parse_request(&head))
        .transpose()
}

/// Turns the connection into figure `figure`'s WebSocket and serves it.
fn upgrade(
    mut reader: BufReader<Input>,
    mut writer: TcpStream,
    request: &Request,
    figure: u32,
    shared: &Shared,
    id: ConnectionId,
) {
    if !shared.serves(figure) {
        refuse(reader, writer, &Response::error(Status::NotFound));
        return;
    }
    let accepted = match websocket::handshake(request) {
        Ok(accepted) => accepted,
        Err(refusal) => {
            refuse(reader, writer, &refusal);
            return;
        }
    };
    let Ok(closer) = writer.try_clone() else {
        return;
    };
    if accepted.write_to(&mut writer, false, true).is_err() {
        return;
    }
    let (outbox, queue) = mpsc::sync_channel(OUTBOX_CAPACITY);
    let spawned = thread::Builder::new()
        .name("pltrs-web-writer".into())
        .spawn(move || write_frames(writer, &queue));
    if spawned.is_err() {
        return;
    }
    let Ok(stream) = closer.try_clone() else {
        return;
    };
    let opened = Inbound::Opened {
        id,
        figure,
        outbox: outbox.clone(),
        stream,
    };
    if shared.send(opened).is_ok() {
        if matches!(
            read_messages(&mut reader, &outbox, shared, id),
            Err(WsError::Io(_))
        ) {
            // The client is gone: stop the writer too.
            let _ = closer.shutdown(Shutdown::Both);
        }
        let _ = shared.send(Inbound::Closed { id });
    }
}

/// Passes the client's messages to the figures' thread and answers pings
/// until the closing handshake, a failure or a protocol violation (which
/// is answered with a close frame).
fn read_messages(
    reader: &mut BufReader<Input>,
    outbox: &SyncSender<Outbound>,
    shared: &Shared,
    id: ConnectionId,
) -> Result<(), WsError> {
    let result = relay_messages(reader, outbox, shared, id);
    if let Err(err) = &result
        && let Some(code) = err.close_code()
    {
        let _ = outbox.try_send(Outbound::Close(code));
    }
    result
}

/// The loop of [`read_messages`].
fn relay_messages(
    reader: &mut BufReader<Input>,
    outbox: &SyncSender<Outbound>,
    shared: &Shared,
    id: ConnectionId,
) -> Result<(), WsError> {
    let mut messages = MessageReader::new(MAX_MESSAGE);
    loop {
        match next_message(reader, &mut messages)? {
            Message::Text(text) => {
                let message = ClientMessage::parse(&text)
                    .map_err(|err| WsError::BadMessage(err.to_string()))?;
                if shared.send(Inbound::Message { id, message }).is_err() {
                    // The server stopped.
                    return Ok(());
                }
            }
            Message::Ping(payload) => outbox
                .try_send(Outbound::Pong(payload))
                .map_err(|_| WsError::BadMessage("pings faster than pongs are read".into()))?,
            Message::Pong => {}
            Message::Close(code) => {
                let _ = outbox.try_send(Outbound::Close(code.unwrap_or(1000)));
                return Ok(());
            }
        }
    }
}

/// The client's next message: an idle page may stay silent indefinitely,
/// but a message, once begun, must arrive within [`REQUEST_TIMEOUT`].
fn next_message(
    reader: &mut BufReader<Input>,
    messages: &mut MessageReader,
) -> Result<Message, WsError> {
    reader.get_mut().deadline = None;
    if reader.fill_buf()?.is_empty() {
        return Err(WsError::Io(io::ErrorKind::UnexpectedEof.into()));
    }
    reader.get_mut().deadline = Some(Instant::now() + REQUEST_TIMEOUT);
    messages.next(reader)
}

/// Writes what the figures' thread queues as frames, until a close frame,
/// a failure, or the queue's end.
fn write_frames(stream: TcpStream, queue: &Receiver<Outbound>) {
    let Ok(closer) = stream.try_clone() else {
        return;
    };
    let mut out = BufWriter::new(stream);
    while let Ok(first) = queue.recv() {
        let mut next = Some(first);
        let mut closing = false;
        let mut written = Ok(());
        while let Some(outbound) = next.take() {
            closing = matches!(outbound, Outbound::Close(_));
            written = write_frame(&mut out, &outbound);
            if written.is_err() || closing {
                break;
            }
            next = queue.try_recv().ok();
        }
        if written.and_then(|()| out.flush()).is_err() || closing {
            break;
        }
    }
    // A close frame ends the connection, as does a failure or the server
    // forgetting it.
    let _ = closer.shutdown(Shutdown::Both);
}

/// Writes one server frame.
fn write_frame(out: &mut impl Write, outbound: &Outbound) -> io::Result<()> {
    let status;
    let (opcode, payload): (Opcode, &[u8]) = match outbound {
        Outbound::Text(text) => (Opcode::Text, text.as_bytes()),
        Outbound::Binary(data) => (Opcode::Binary, data),
        Outbound::Pong(payload) => (Opcode::Pong, payload),
        Outbound::Close(code) => {
            status = code.to_be_bytes();
            (Opcode::Close, &status)
        }
    };
    out.write_all(&frame::header(opcode, payload.len()))?;
    out.write_all(payload)
}
