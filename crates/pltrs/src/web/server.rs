//! The web server as the figures' thread sees it: the served figures, the
//! WebSocket connections open on them, and the loop that runs client messages,
//! figure timers and redraws (matplotlib's `WebAggApplication` with its
//! figure managers).

use std::convert::Infallible;
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender};
use std::time::{Duration, Instant};

use pltrs_codec::base64;

use super::canvas::{Next, Output, WebCanvas};
use super::net::{ConnectionId, DownloadError, Inbound, Listener, Outbound, ServedFigure};
use super::protocol::ClientMessage;
use super::{WebError, WebOptions, assets, browser, modern};
use crate::events::{Event, GinputOptions, wait};
use crate::figure::Figure;

/// The close code for a WebSocket whose figure closed ("going away").
const GOING_AWAY: u16 = 1001;

/// How many queued messages are handled before timers get a turn.
const MAX_BATCH: usize = 256;

/// An open WebSocket.
#[derive(Debug)]
struct Peer {
    id: ConnectionId,
    /// The figure number it shows.
    figure: u32,
    outbox: SyncSender<Outbound>,
    stream: TcpStream,
    /// Whether the client takes binary frames.
    binary: bool,
}

/// A frame or event encoded once for every client of a figure.
enum Encoded {
    Text(Arc<str>),
    Png(Arc<[u8]>),
}

/// Serves figures to web browsers, like matplotlib's `WebAgg` backend: each
/// figure has a page with matplotlib's toolbar, where it pans, zooms,
/// reports coordinates and receives mouse and key events, and a page
/// shows every figure.
///
/// Network threads accept connections and parse requests; the figures
/// stay on the thread that owns the server, and run only while
/// [`WebServer::run`] (or [`WebServer::pause`]) is running. Dropping the
/// server stops it and closes its connections.
///
/// ```no_run
/// use pltrs::figure::Figure;
/// use pltrs::web::{WebOptions, WebServer};
///
/// let fig = Figure::new();
/// fig.subplot().plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])?;
/// let mut server = WebServer::bind(WebOptions::new("127.0.0.1", 8988))?;
/// let number = server.add_figure(&fig);
/// println!("open {}", server.figure_url(number));
/// server.run(None, || false); // until the figure is closed (`q`)
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug)]
pub struct WebServer {
    listener: Listener,
    inbound: Receiver<Inbound>,
    canvases: Vec<(u32, WebCanvas)>,
    peers: Vec<Peer>,
    /// Figures closed from a browser, not yet reported.
    closed: Vec<Figure>,
}

impl WebServer {
    /// Starts a server listening as `options` say (opening no browser: see
    /// [`WebServer::open_in_browser`]); the client script preselects the
    /// `savefig.format` download format.
    ///
    /// # Errors
    ///
    /// When the address cannot be bound, no port is free, or a thread
    /// cannot be started.
    pub fn bind(options: WebOptions) -> Result<Self, WebError> {
        let script = assets::script(&crate::rc::params().savefig_format);
        let (listener, inbound) = Listener::start(&options, script)?;
        Ok(Self {
            listener,
            inbound,
            canvases: Vec::new(),
            peers: Vec::new(),
            closed: Vec::new(),
        })
    }

    /// The address the server listens on, as this machine reaches it.
    pub fn local_addr(&self) -> SocketAddr {
        self.listener.local_addr()
    }

    /// The page of every figure.
    pub fn url(&self) -> String {
        format!("http://{}/", self.local_addr())
    }

    /// The page of figure `number`.
    pub fn figure_url(&self, number: u32) -> String {
        format!("{}{number}", self.url())
    }

    /// Opens [`WebServer::url`] in the system's web browser (`$BROWSER`,
    /// else `xdg-open`, `open` or `start`).
    ///
    /// # Errors
    ///
    /// When no browser could be started.
    pub fn open_in_browser(&self) -> Result<(), WebError> {
        browser::open(&self.url()).map_err(WebError::Browser)
    }

    /// Serves `figure` under the lowest free number, titled `Figure
    /// <number>`, and returns the number; a figure already served keeps
    /// its number.
    pub fn add_figure(&mut self, figure: &Figure) -> u32 {
        if let Some(number) = self.number_of(figure) {
            return number;
        }
        let number = (1..=u32::MAX)
            .find(|n| !self.canvases.iter().any(|(used, _)| used == n))
            .expect("fewer than u32::MAX figures");
        self.insert(number, figure, &format!("Figure {number}"));
        number
    }

    /// Serves `figure` as figure `number` titled `title` (pyplot's
    /// numbering); a figure already served keeps its number, takes the
    /// title, and its pages loaded from now on follow its background.
    ///
    /// # Errors
    ///
    /// [`WebError::NumberTaken`] when another figure has the number.
    pub fn add_figure_as(
        &mut self,
        number: u32,
        figure: &Figure,
        title: &str,
    ) -> Result<(), WebError> {
        if let Some((_, canvas)) = self.canvases.iter_mut().find(|(_, c)| c.figure() == figure) {
            canvas.set_title(title);
            self.publish();
            return Ok(());
        }
        if self.canvas(number).is_some() {
            return Err(WebError::NumberTaken(number));
        }
        self.insert(number, figure, title);
        Ok(())
    }

    fn insert(&mut self, number: u32, figure: &Figure, title: &str) {
        self.canvases.push((number, WebCanvas::new(figure, title)));
        self.publish();
    }

    /// Tells the network threads which figures are served, and which are
    /// drawn on dark backgrounds now (a page chooses its theme when it
    /// loads, as a window does when it opens).
    fn publish(&self) {
        let figures = self
            .canvases
            .iter()
            .map(|(number, canvas)| ServedFigure {
                number: *number,
                dark: modern::is_dark(canvas.figure()),
            })
            .collect();
        self.listener.shared().set_figures(figures);
    }

    /// The number `figure` is served under.
    pub fn number_of(&self, figure: &Figure) -> Option<u32> {
        self.canvases
            .iter()
            .find_map(|(number, canvas)| (canvas.figure() == figure).then_some(*number))
    }

    /// Whether `figure` is served.
    pub fn shows(&self, figure: &Figure) -> bool {
        self.number_of(figure).is_some()
    }

    /// Whether no figure is served.
    pub fn is_empty(&self) -> bool {
        self.canvases.is_empty()
    }

    /// Stops serving `figure` (matplotlib's `pyplot.close`); its pages
    /// keep the last frame. Returns whether it was served.
    pub fn close(&mut self, figure: &Figure) -> bool {
        self.number_of(figure)
            .map(|number| self.remove(number))
            .is_some()
    }

    /// The figures closed from a browser (`keymap.quit`) since the last
    /// call.
    pub fn take_closed(&mut self) -> Vec<Figure> {
        std::mem::take(&mut self.closed)
    }

    /// Serves until `deadline` passes (forever when `None`), `done`
    /// returns `true` after a message, or the last figure is closed.
    pub fn run(&mut self, deadline: Option<Instant>, mut done: impl FnMut() -> bool) {
        self.run_while(deadline, |server| !done() && !server.is_empty());
    }

    /// Serves for `duration` (matplotlib's `pause`).
    pub fn pause(&mut self, duration: Duration) {
        self.run(Instant::now().checked_add(duration), || false);
    }

    /// Collects up to `n` points clicked in `figure` (all until stopped
    /// when `n` is 0), within `timeout` (matplotlib's `ginput`; see
    /// [`Session::ginput`](crate::gui::Session::ginput)).
    pub fn ginput(
        &mut self,
        figure: &Figure,
        n: usize,
        timeout: Option<Duration>,
        options: GinputOptions,
    ) -> Vec<(f64, f64)> {
        let points = wait::ginput(figure, n, timeout, options, &mut |deadline, done| {
            self.run_while_shown(figure, deadline, done);
            Ok::<(), Infallible>(())
        });
        points.unwrap_or_else(|never| match never {})
    }

    /// Waits for a key press (`Some(true)`) or a click (`Some(false)`) in
    /// `figure`, or `timeout` (`None`) (matplotlib's `waitforbuttonpress`).
    pub fn wait_for_button_press(
        &mut self,
        figure: &Figure,
        timeout: Option<Duration>,
    ) -> Option<bool> {
        let pressed = wait::wait_for_button_press(figure, timeout, &mut |deadline, done| {
            self.run_while_shown(figure, deadline, done);
            Ok::<(), Infallible>(())
        });
        pressed.unwrap_or_else(|never| match never {})
    }

    /// Serves until `done`, `deadline`, or `figure` closes.
    fn run_while_shown(
        &mut self,
        figure: &Figure,
        deadline: Option<Instant>,
        done: &mut dyn FnMut() -> bool,
    ) {
        self.run_while(deadline, |server| !done() && server.shows(figure));
    }

    /// Runs timers and client messages while `keep_going` holds, until
    /// `deadline`.
    fn run_while(&mut self, deadline: Option<Instant>, mut keep_going: impl FnMut(&Self) -> bool) {
        loop {
            let now = Instant::now();
            for (_, canvas) in &mut self.canvases {
                canvas.fire_timers(now);
            }
            self.flush();
            if !keep_going(self) || deadline.is_some_and(|deadline| now >= deadline) {
                return;
            }
            let timers = self.canvases.iter().filter_map(|(_, c)| c.next_timer());
            let wake = timers.chain(deadline).min();
            let first = match wake {
                Some(wake) => match self
                    .inbound
                    .recv_timeout(wake.saturating_duration_since(Instant::now()))
                {
                    Ok(message) => Some(message),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => return,
                },
                None => match self.inbound.recv() {
                    Ok(message) => Some(message),
                    Err(_) => return,
                },
            };
            let more = std::iter::from_fn(|| self.inbound.try_recv().ok()).take(MAX_BATCH);
            let batch: Vec<Inbound> = first.into_iter().chain(more).collect();
            for message in batch {
                self.dispatch(message);
            }
        }
    }

    fn canvas(&self, number: u32) -> Option<&WebCanvas> {
        self.canvases
            .iter()
            .find_map(|(n, canvas)| (*n == number).then_some(canvas))
    }

    fn canvas_mut(&mut self, number: u32) -> Option<&mut WebCanvas> {
        self.canvases
            .iter_mut()
            .find_map(|(n, canvas)| (*n == number).then_some(canvas))
    }

    /// Handles what a connection thread sent.
    fn dispatch(&mut self, message: Inbound) {
        match message {
            Inbound::Opened {
                id,
                figure,
                outbox,
                stream,
            } => match self.canvas_mut(figure) {
                Some(canvas) => {
                    canvas.connected();
                    self.peers.push(Peer {
                        id,
                        figure,
                        outbox,
                        stream,
                        binary: true,
                    });
                }
                // The figure closed meanwhile.
                None => {
                    let _ = outbox.try_send(Outbound::Close(GOING_AWAY));
                }
            },
            Inbound::Message { id, message } => self.message(id, message),
            Inbound::Closed { id } => self.peers.retain(|peer| peer.id != id),
            Inbound::Download {
                figure,
                format,
                reply,
            } => {
                let saved = self
                    .canvas(figure)
                    .map_or(Err(DownloadError::NoFigure), |canvas| {
                        canvas.download(format).map_err(|_| DownloadError::Failed)
                    });
                // The client may have given up waiting.
                let _ = reply.send(saved);
            }
        }
    }

    /// A message from the client of connection `id`.
    fn message(&mut self, id: ConnectionId, message: ClientMessage) {
        let Some(peer) = self.peers.iter_mut().find(|peer| peer.id == id) else {
            return;
        };
        // Binary support is the client's own; everything else is the
        // figure's, shared by its clients.
        if let ClientMessage::SupportsBinary(binary) = message {
            peer.binary = binary;
            return;
        }
        let number = peer.figure;
        let Some(canvas) = self.canvas_mut(number) else {
            return;
        };
        match canvas.handle(message) {
            Next::Continue => {}
            Next::Close => self.close_from_browser(number),
            Next::CloseAll => {
                let numbers: Vec<u32> = self.canvases.iter().map(|(n, _)| *n).collect();
                for number in numbers {
                    self.close_from_browser(number);
                }
            }
        }
    }

    /// Closes figure `number` as its window would: the figure gets a close
    /// event and is reported by [`WebServer::take_closed`].
    fn close_from_browser(&mut self, number: u32) {
        if let Some(canvas) = self.remove(number) {
            canvas.figure().process_event(&Event::Close);
            self.closed.push(canvas.figure().clone());
        }
    }

    /// Stops serving figure `number` and closes its WebSocket connections.
    fn remove(&mut self, number: u32) -> Option<WebCanvas> {
        let index = self.canvases.iter().position(|(n, _)| *n == number)?;
        let (_, canvas) = self.canvases.remove(index);
        for peer in self.peers.iter().filter(|peer| peer.figure == number) {
            let _ = peer.outbox.try_send(Outbound::Close(GOING_AWAY));
        }
        self.peers.retain(|peer| peer.figure != number);
        self.publish();
        Some(canvas)
    }

    /// Asks the clients of changed figures for a draw and sends every
    /// figure's outputs to its clients.
    fn flush(&mut self) {
        for index in 0..self.canvases.len() {
            let (number, canvas) = &mut self.canvases[index];
            let number = *number;
            let watched = self.peers.iter().any(|peer| peer.figure == number);
            if watched {
                canvas.idle();
            }
            let outputs = canvas.take_outputs();
            if watched {
                self.send(number, outputs);
            }
        }
    }

    /// Queues `outputs` for every client of figure `number`; clients that
    /// fall behind are disconnected.
    fn send(&mut self, number: u32, outputs: Vec<Output>) {
        if outputs.is_empty() {
            return;
        }
        let encoded: Vec<Encoded> = outputs
            .into_iter()
            .map(|output| match output {
                Output::Event(event) => Encoded::Text(event.to_json().into()),
                Output::Frame(png) => Encoded::Png(png.into()),
            })
            .collect();
        let mut behind = Vec::new();
        for peer in self.peers.iter().filter(|peer| peer.figure == number) {
            for item in &encoded {
                let outbound = match item {
                    Encoded::Text(text) => Outbound::Text(Arc::clone(text)),
                    Encoded::Png(png) if peer.binary => Outbound::Binary(Arc::clone(png)),
                    Encoded::Png(png) => {
                        let uri = format!("data:image/png;base64,{}", base64::encode(png));
                        Outbound::Text(uri.into())
                    }
                };
                if peer.outbox.try_send(outbound).is_err() {
                    behind.push(peer.id);
                    break;
                }
            }
        }
        for id in behind {
            if let Some(index) = self.peers.iter().position(|peer| peer.id == id) {
                let peer = self.peers.remove(index);
                // The connection may be gone already.
                let _ = peer.stream.shutdown(Shutdown::Both);
            }
        }
    }
}
