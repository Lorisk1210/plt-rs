//! Interactive figures in a web browser (matplotlib's `WebAgg` backend), for
//! machines without a window system plt-rs supports: macOS, Windows, WSL
//! or a remote host.
//!
//! A [`WebServer`] is a small HTTP/1.1 and WebSocket (RFC 6455) server on
//! `std::net`. It serves matplotlib's own browser client — `mpl.js`, its
//! style sheets and toolbar icons, embedded unchanged — at matplotlib's
//! URLs: `/` shows every figure, `/<number>` one figure, and
//! `/<number>/download.<format>` saves it. The client speaks matplotlib's
//! protocol over `/<number>/ws`: it reports mouse and key input, resizes
//! and toolbar clicks; the server answers with PNG frames (whole, or only
//! the pixels that changed), the coordinate readout, the zoom rectangle,
//! cursors and the state of the toolbar buttons.
//!
//! Input reaches the figure as it does in a window ([`crate::events`]):
//! connected callbacks run, the toolbar pans and zooms
//! ([`crate::navigation`]), the wheel zooms, `keymap.*` shortcuts work
//! (`q` closes the figure), and figure timers and animations play.
//!
//! ```no_run
//! use plt::figure::Figure;
//! use plt::web::{WebOptions, serve};
//!
//! let fig = Figure::new();
//! fig.subplot().plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])?;
//! // Like matplotlib: port 8988 (or a free one nearby) on 127.0.0.1,
//! // opening a browser; returns once the figure is closed.
//! serve(&[fig], WebOptions::from_rc()?)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`crate::pyplot`] uses this backend when it is selected
//! ([`crate::pyplot::Backend::WebAgg`], or `MPLBACKEND=webagg`) and, on
//! systems without X11 (macOS, Windows), in place of windows.
//!
//! Figures are single-threaded: the thread that owns the server runs the
//! figures, and network threads pass it what clients send through bounded
//! queues. Requests and messages from the network are untrusted: malformed
//! HTTP, WebSocket frames or messages, oversized input and slow clients
//! end their connection without affecting the figures.
//!
//! Left out from matplotlib's `WebAgg`: the `url_prefix` of embedding
//! applications, the `IPython` inline display and `nbagg`, and toolbar
//! buttons for tools added with `Figure::add_tool` (their keyboard
//! shortcuts work).

mod assets;
mod browser;
mod canvas;
mod connection;
mod frames;
#[cfg(test)]
mod fuzz_tests;
mod http;
mod modern;
mod net;
mod protocol;
mod routes;
mod schedule;
mod server;
mod websocket;

use std::fmt;
use std::io;

pub use server::WebServer;

use crate::figure::Figure;

/// Why the web server failed.
#[derive(Debug)]
pub enum WebError {
    /// The address could not be resolved or listened on.
    Bind {
        /// The address asked for.
        address: String,
        /// What went wrong.
        source: io::Error,
    },
    /// Every port tried was in use (matplotlib: "an available port could
    /// not be found").
    NoFreePort {
        /// The address asked for.
        address: String,
        /// The first port tried.
        port: u16,
    },
    /// The `webagg.port` rcParam is not a port number.
    InvalidPort(i64),
    /// A server thread could not be started.
    Thread(io::Error),
    /// Another figure is served under this number.
    NumberTaken(u32),
    /// No web browser could be started.
    Browser(io::Error),
}

impl fmt::Display for WebError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bind { address, source } => {
                write!(f, "the web server cannot listen on {address}: {source}")
            }
            Self::NoFreePort { address, port } => write!(
                f,
                "the web server could not be started because no port near {port} is free on {address}"
            ),
            Self::InvalidPort(port) => write!(f, "{port} is not a port number"),
            Self::Thread(err) => write!(f, "cannot start a web server thread: {err}"),
            Self::NumberTaken(number) => write!(f, "another figure is served as figure {number}"),
            Self::Browser(err) => write!(f, "cannot open a web browser: {err}"),
        }
    }
}

impl std::error::Error for WebError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Bind { source, .. } => Some(source),
            Self::Thread(err) | Self::Browser(err) => Some(err),
            Self::NoFreePort { .. } | Self::InvalidPort(_) | Self::NumberTaken(_) => None,
        }
    }
}

/// Where the server listens and whether it opens a browser (matplotlib's
/// `webagg.*` rcParams).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebOptions {
    /// The address or host name to listen on (`webagg.address`); listening
    /// on another address than `127.0.0.1` makes the figures reachable
    /// from other machines.
    pub address: String,
    /// The port (`webagg.port`); 0 lets the system pick a free one.
    pub port: u16,
    /// How many ports to try when the port is in use: the next four, then
    /// random ones nearby (`webagg.port_retries`; at least the port itself
    /// is tried).
    pub port_retries: u32,
    /// Whether [`serve`] and pyplot open the figures in a browser
    /// (`webagg.open_in_browser`).
    pub open_in_browser: bool,
}

impl WebOptions {
    /// Listening on `address` and `port`, without retries or a browser.
    pub fn new(address: impl Into<String>, port: u16) -> Self {
        Self {
            address: address.into(),
            port,
            port_retries: 0,
            open_in_browser: false,
        }
    }

    /// The options the `webagg.*` rcParams give: by default port 8988 on
    /// `127.0.0.1`, with 50 retries, opening a browser.
    ///
    /// # Errors
    ///
    /// [`WebError::InvalidPort`] when `webagg.port` is out of range.
    pub fn from_rc() -> Result<Self, WebError> {
        let rc = crate::rc::params();
        let port =
            u16::try_from(rc.webagg_port).map_err(|_| WebError::InvalidPort(rc.webagg_port))?;
        Ok(Self {
            address: rc.webagg_address.clone(),
            port,
            port_retries: u32::try_from(rc.webagg_port_retries.max(0)).unwrap_or(u32::MAX),
            open_in_browser: rc.webagg_open_in_browser,
        })
    }
}

/// Serves `figures` ("Figure 1", "Figure 2", ...) until every one is
/// closed from its page (`keymap.quit`, `q` by default) — or the program
/// is interrupted, as matplotlib's server is with Ctrl+C. The address is
/// printed, and opened in a browser if `options` say so.
///
/// # Errors
///
/// When the server cannot start.
pub fn serve(figures: &[Figure], options: WebOptions) -> Result<(), WebError> {
    let open_in_browser = options.open_in_browser;
    let mut server = WebServer::bind(options)?;
    for figure in figures {
        server.add_figure(figure);
    }
    browser::announce(&server.url(), open_in_browser);
    server.run(None, || false);
    Ok(())
}

/// Prints where `server` serves its figures and, if asked, opens them in
/// a browser (what pyplot does when it starts the server).
pub(crate) fn announce(server: &WebServer, open_browser: bool) {
    browser::announce(&server.url(), open_browser);
}
