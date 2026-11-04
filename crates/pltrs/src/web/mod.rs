//! Interactive figures in a web browser (matplotlib's `WebAgg` backend), for
//! machines where pltrs's windows cannot open: WSL, a remote host.
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
//! use pltrs::figure::Figure;
//! use pltrs::web::{WebOptions, serve};
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
//! macOS and Windows, in place of windows that cannot open.
//!
//! Figures are single-threaded: the thread that owns the server runs the
//! figures, and network threads pass it what clients send through bounded
//! queues. Requests and messages from the network are untrusted: malformed
//! HTTP, WebSocket frames or messages, oversized input and slow clients
//! end their connection without affecting the figures.
//!
//! Left out from matplotlib's `WebAgg`: the `IPython` inline display and
//! `nbagg`, and toolbar buttons for tools added with `Figure::add_tool`
//! (their keyboard shortcuts work).
//!
//! A [`WebRelay`] is the same server for programs that run matplotlib's
//! side of the protocol themselves (pltrs (Python)'s `webagg` backend):
//! it serves the pages and files, and hands over what the WebSocket
//! connections send and the downloads asked for ([`RelayEvent`]),
//! sending the clients what the program wants. [`client_files`] gives the files of the
//! browser client, for programs serving them themselves.

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
mod relay;
mod routes;
mod schedule;
mod server;
mod websocket;

use std::fmt;
use std::io;

pub use relay::{RelayEvent, WebRelay};
pub use server::WebServer;

use crate::figure::Figure;

/// matplotlib's client script (its `web_backend/js/mpl.js`), as the pages
/// load it before the toolbar items and download formats are appended:
/// for programs embedding the client themselves (matplotlib's
/// `FigureManagerWebAgg.get_javascript`).
pub fn client_script() -> &'static str {
    assets::MPL_JS
}

/// The files of matplotlib's browser client, as matplotlib's
/// `web_backend` directory holds them (its `get_static_file_path`): the
/// pages (Tornado templates), the style sheets, `mpl.js`,
/// `mpl_tornado.js` and the notebook's `nbagg_mpl.js`, each with its path
/// in the directory (`"js/mpl.js"`), and their license.
pub fn client_files() -> impl Iterator<Item = (&'static str, &'static [u8])> {
    assets::client_files()
}

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
    /// The path every URL of the server starts with, for an application
    /// serving the figures among its own pages: `/` and a path, without
    /// a final `/` (matplotlib's `url_prefix`); empty for the root.
    pub url_prefix: String,
}

impl WebOptions {
    /// Listening on `address` and `port`, without retries or a browser.
    pub fn new(address: impl Into<String>, port: u16) -> Self {
        Self {
            address: address.into(),
            port,
            port_retries: 0,
            open_in_browser: false,
            url_prefix: String::new(),
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
            url_prefix: String::new(),
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
