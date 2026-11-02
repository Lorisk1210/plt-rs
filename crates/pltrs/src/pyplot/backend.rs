//! How pyplot shows figures (matplotlib's backends, `switch_backend`,
//! `get_backend` and the `MPLBACKEND` environment variable).

use std::cell::Cell;
use std::fmt;
use std::str::FromStr;

use super::interactive::forget_session;

/// How pyplot shows figures (matplotlib's backend).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Backend {
    /// pltrs's figure windows ([`crate::gui`]). When they cannot open and
    /// the `backend_fallback` rcParam is on (the default), the web backend
    /// takes their place on macOS and Windows (away from the main thread
    /// of a macOS process, say), and [`Backend::Agg`] on Linux systems
    /// without a display, as matplotlib falls back to `agg` on headless
    /// machines.
    #[default]
    Windows,
    /// Figures in a web browser (matplotlib's `webagg`, see
    /// [`crate::web`]), at the address the `webagg.*` rcParams give.
    WebAgg,
    /// No display (matplotlib's `agg` and its other non-interactive
    /// backends): `show` and `draw` do nothing, `pause` sleeps, and
    /// `ginput` and `waitforbuttonpress` return at once with nothing.
    Agg,
}

/// The matplotlib backend names of pltrs's figure windows: its GUI
/// backends.
const WINDOW_NAMES: [&str; 13] = [
    "gtk3agg",
    "gtk3cairo",
    "gtk4agg",
    "gtk4cairo",
    "macosx",
    "qt5agg",
    "qt5cairo",
    "qtagg",
    "qtcairo",
    "tkagg",
    "tkcairo",
    "wxagg",
    "wxcairo",
];

/// The matplotlib backend names that display nothing.
const NON_INTERACTIVE_NAMES: [&str; 7] = ["agg", "cairo", "pdf", "pgf", "ps", "svg", "template"];

/// A backend name matplotlib does not know.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownBackend(pub String);

impl fmt::Display for UnknownBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} is not a known backend", self.0)
    }
}

impl std::error::Error for UnknownBackend {}

impl FromStr for Backend {
    type Err = UnknownBackend;

    /// A matplotlib backend name, ignoring case: `webagg`; `agg` (or
    /// another non-interactive backend); or a GUI backend (`qtagg`,
    /// `tkagg`, `macosx`, ...), whose part pltrs's windows play.
    fn from_str(name: &str) -> Result<Self, UnknownBackend> {
        let lower = name.to_ascii_lowercase();
        if lower == "webagg" {
            Ok(Self::WebAgg)
        } else if NON_INTERACTIVE_NAMES.contains(&lower.as_str()) {
            Ok(Self::Agg)
        } else if WINDOW_NAMES.contains(&lower.as_str()) {
            Ok(Self::Windows)
        } else {
            Err(UnknownBackend(name.to_owned()))
        }
    }
}

thread_local! {
    /// The thread's backend, once chosen.
    static BACKEND: Cell<Option<Backend>> = const { Cell::new(None) };
}

/// The backend pyplot shows figures with (matplotlib's `get_backend`): the
/// one chosen with [`switch_backend`], else the one the `MPLBACKEND`
/// environment variable names, else [`Backend::Windows`]; after a
/// fallback, the backend that took over.
pub fn get_backend() -> Backend {
    BACKEND.get().unwrap_or_else(|| {
        let backend = std::env::var("MPLBACKEND")
            .ok()
            .and_then(|name| name.parse().ok())
            .unwrap_or_default();
        BACKEND.set(Some(backend));
        backend
    })
}

/// Shows figures with `backend` from now on (matplotlib's
/// `switch_backend`): the figures shown so far leave their windows or web
/// pages, and the next [`show`](super::show) shows them the new way. Unlike
/// matplotlib, the figures stay open.
///
/// ```
/// use pltrs::pyplot as plt;
/// use pltrs::pyplot::Backend;
/// plt::switch_backend(Backend::Agg);
/// plt::plot([0.0, 1.0], [1.0, 0.0])?;
/// plt::show()?; // nothing to display
/// assert_eq!(plt::get_backend(), Backend::Agg);
/// plt::close_all();
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn switch_backend(backend: Backend) {
    forget_session();
    BACKEND.set(Some(backend));
}

/// Records the backend that started, which a fallback may have changed,
/// keeping what is shown.
pub(super) fn record_backend(backend: Backend) {
    BACKEND.set(Some(backend));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_follow_matplotlib() {
        assert_eq!("webagg".parse(), Ok(Backend::WebAgg));
        assert_eq!("WebAgg".parse(), Ok(Backend::WebAgg));
        assert_eq!("Agg".parse(), Ok(Backend::Agg));
        assert_eq!("svg".parse(), Ok(Backend::Agg));
        assert_eq!("QtAgg".parse(), Ok(Backend::Windows));
        assert_eq!("macosx".parse(), Ok(Backend::Windows));
        assert_eq!(
            "module://mine".parse::<Backend>(),
            Err(UnknownBackend("module://mine".into()))
        );
    }

    #[test]
    fn switching_is_per_thread() {
        switch_backend(Backend::Agg);
        assert_eq!(get_backend(), Backend::Agg);
        let other = std::thread::spawn(|| {
            switch_backend(Backend::WebAgg);
            get_backend()
        });
        assert_eq!(other.join().ok(), Some(Backend::WebAgg));
        assert_eq!(get_backend(), Backend::Agg);
    }
}
