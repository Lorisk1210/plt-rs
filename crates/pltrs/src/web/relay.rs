//! matplotlib's `WebAgg` server relaying its clients to a program
//! ([`WebRelay`]).

use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::mpsc::{Receiver, Sender, SyncSender};
use std::time::Duration;

use super::net::{ConnectionId, DownloadError, Inbound, Listener, Outbound, ServedFigure};
use super::{WebError, WebOptions};

/// How many queued messages one [`WebRelay::poll`] takes at most.
const MAX_BATCH: usize = 256;

/// The close code for a WebSocket the program closes ("going away").
const GOING_AWAY: u16 = 1001;

/// What the clients of a [`WebRelay`] did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelayEvent {
    /// A page opened the WebSocket `connection` of figure `figure`.
    Opened {
        /// The connection, for [`WebRelay::send_text`] and the others.
        connection: u64,
        /// The figure number from the URL.
        figure: u32,
    },
    /// The client of `connection` sent the message `text` (matplotlib's
    /// JSON messages).
    Message {
        /// The connection.
        connection: u64,
        /// The message, as it came.
        text: String,
    },
    /// The WebSocket `connection` closed.
    Closed {
        /// The connection.
        connection: u64,
    },
    /// A client asks for figure `figure` saved in `format` (a file
    /// extension: `png`, `svg`, `pdf`, ...); answer with
    /// [`WebRelay::answer_download`].
    Download {
        /// The request, for the answer.
        request: u64,
        /// The figure number from the URL.
        figure: u32,
        /// The format's file extension.
        format: String,
    },
}

/// An open WebSocket.
#[derive(Debug)]
struct Peer {
    id: ConnectionId,
    outbox: SyncSender<Outbound>,
    stream: TcpStream,
}

/// A download waiting for the program.
type PendingDownload = (u64, Sender<Result<Vec<u8>, DownloadError>>);

/// matplotlib's `WebAgg` server for a program that runs the figures
/// itself (pltrs (Python)'s `webagg` backend, whose `FigureManagerWebAgg`s
/// speak matplotlib's protocol): the pages, scripts, style sheets and
/// icons are served as [`WebServer`](super::WebServer) serves them, while
/// what the clients' WebSocket connections send, and the downloads they
/// ask for, go to the program as they come, and the program sends the
/// clients what it wants.
///
/// Network threads accept the connections and answer plain requests;
/// [`WebRelay::poll`] hands over what the WebSocket connections send. A
/// client that falls behind what is sent to it is disconnected. Dropping
/// the relay stops the server and closes its connections.
#[derive(Debug)]
pub struct WebRelay {
    listener: Listener,
    inbound: Receiver<Inbound>,
    peers: Vec<Peer>,
    downloads: Vec<PendingDownload>,
    next_download: u64,
}

impl WebRelay {
    /// Starts a server listening as `options` say, which serves `script`
    /// as the client script (`/js/mpl.js`: matplotlib's
    /// `FigureManagerWebAgg.get_javascript()`).
    ///
    /// # Errors
    ///
    /// When the address cannot be bound, no port is free, or a thread
    /// cannot be started.
    pub fn bind(options: &WebOptions, script: String) -> Result<Self, WebError> {
        let (listener, inbound) = Listener::start(options, script, true)?;
        Ok(Self {
            listener,
            inbound,
            peers: Vec::new(),
            downloads: Vec::new(),
            next_download: 0,
        })
    }

    /// The address the server listens on, as this machine reaches it.
    pub fn local_addr(&self) -> SocketAddr {
        self.listener.local_addr()
    }

    /// Serves pages for the figures `figures`, each a number and whether
    /// it is drawn on a dark background (which the modern chrome shows in
    /// its dark theme); the other numbers' pages and WebSocket connections are not
    /// found.
    pub fn set_figures(&self, figures: &[(u32, bool)]) {
        let figures = figures
            .iter()
            .map(|&(number, dark)| ServedFigure { number, dark })
            .collect();
        self.listener.shared().set_figures(figures);
    }

    /// Waits up to `timeout` (forever with `None`) for what clients send,
    /// and returns it, oldest first.
    pub fn poll(&mut self, timeout: Option<Duration>) -> Vec<RelayEvent> {
        let first = match timeout {
            Some(timeout) => self.inbound.recv_timeout(timeout).ok(),
            None => self.inbound.recv().ok(),
        };
        let more = std::iter::from_fn(|| self.inbound.try_recv().ok()).take(MAX_BATCH);
        let batch: Vec<Inbound> = first.into_iter().chain(more).collect();
        batch
            .into_iter()
            .filter_map(|inbound| self.relay(inbound))
            .collect()
    }

    /// What a connection thread sent, as the program sees it.
    fn relay(&mut self, inbound: Inbound) -> Option<RelayEvent> {
        match inbound {
            Inbound::Opened {
                id,
                figure,
                outbox,
                stream,
            } => {
                self.peers.push(Peer { id, outbox, stream });
                Some(RelayEvent::Opened {
                    connection: id,
                    figure,
                })
            }
            Inbound::Text { id, text } => Some(RelayEvent::Message {
                connection: id,
                text,
            }),
            // A relay's connections pass their messages on as they came.
            Inbound::Message { .. } => None,
            Inbound::Closed { id } => {
                let known = self.peers.iter().any(|peer| peer.id == id);
                self.peers.retain(|peer| peer.id != id);
                known.then_some(RelayEvent::Closed { connection: id })
            }
            Inbound::Download {
                figure,
                format,
                reply,
            } => {
                self.next_download += 1;
                let request = self.next_download;
                self.downloads.push((request, reply));
                Some(RelayEvent::Download {
                    request,
                    figure,
                    format: format.extension().to_owned(),
                })
            }
        }
    }

    /// Sends the text message `text` to the client of `connection`;
    /// returns whether it is connected. A client that falls behind is
    /// disconnected.
    pub fn send_text(&mut self, connection: u64, text: &str) -> bool {
        self.send(connection, Outbound::Text(text.into()))
    }

    /// Sends the binary message `data` (a frame) to the client of
    /// `connection`, as [`WebRelay::send_text`] sends text.
    pub fn send_binary(&mut self, connection: u64, data: &[u8]) -> bool {
        self.send(connection, Outbound::Binary(data.into()))
    }

    fn send(&mut self, connection: u64, outbound: Outbound) -> bool {
        let Some(index) = self.peers.iter().position(|peer| peer.id == connection) else {
            return false;
        };
        if self.peers[index].outbox.try_send(outbound).is_ok() {
            return true;
        }
        let peer = self.peers.remove(index);
        // The connection may be gone already.
        let _ = peer.stream.shutdown(Shutdown::Both);
        false
    }

    /// Closes the WebSocket `connection`, if it is open.
    pub fn close(&mut self, connection: u64) {
        if let Some(index) = self.peers.iter().position(|peer| peer.id == connection) {
            let peer = self.peers.remove(index);
            let _ = peer.outbox.try_send(Outbound::Close(GOING_AWAY));
        }
    }

    /// Answers the download `request` with the file, or with "not found"
    /// (`None`: the figure is gone, or cannot be saved in the format).
    pub fn answer_download(&mut self, request: u64, file: Option<Vec<u8>>) {
        if let Some(index) = self.downloads.iter().position(|(id, _)| *id == request) {
            let (_, reply) = self.downloads.remove(index);
            // The client may have given up waiting.
            let _ = reply.send(file.ok_or(DownloadError::NoFigure));
        }
    }
}
