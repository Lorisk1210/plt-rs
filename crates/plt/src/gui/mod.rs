//! Interactive figure windows (matplotlib's GUI backends and
//! `pyplot.show`), built on the `plt-ui` engine and `plt-platform`
//! windows.
//!
//! Each [`FigureWindow`] shows a figure above a toolbar — home, back,
//! forward, pan, zoom, subplot configuration, figure options and save —
//! with a readout of the data coordinates under the pointer. Dragging pans
//! or zooms as in matplotlib (see [`crate::navigation`]), the wheel zooms
//! around the pointer, and the `keymap.*` rcParams bind the usual
//! shortcuts (`h` home, `p` pan, `o` zoom, `g` grids, `l`/`k` log scales,
//! `s` save, `f` full screen, `q` close, ...). The window resizes the
//! figure with it and renders at the screen's scale factor. Saving opens a
//! dialog to pick the directory, the file name and the format (PNG, SVG,
//! PDF, PS or EPS); the figure options dialog edits the titles, limits,
//! labels and scales of the axes, the style of their curves and the
//! colormaps of their images.
//!
//! ```no_run
//! use plt::figure::Figure;
//! let fig = Figure::new();
//! fig.subplot().plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])?;
//! plt::gui::show(&[fig])?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub(crate) mod actions;
mod dialog;
mod help_dialog;
pub(crate) mod icons;
pub(crate) mod keys;
mod options;
mod save_dialog;
mod session;
mod subplot_dialog;
pub(crate) mod tools;
mod window;

use std::fmt;

pub use crate::events::GinputOptions;
pub use plt_ui::Icon;
pub use session::Session;
pub use tools::{ToolError, ToolEvent, ToolKind, ToolSpec};
pub use window::FigureWindow;

use crate::figure::{Figure, SaveError};

/// Why showing figures failed, in windows or in a browser.
#[derive(Debug)]
pub enum GuiError {
    /// The windowing system refused (no display, lost connection, ...).
    Platform(plt_platform::Error),
    /// The web backend failed (see [`crate::web`]).
    Web(crate::web::WebError),
    /// The UI could not be laid out or painted.
    Ui(plt_ui::UiError),
    /// The figure could not be rendered.
    Render(SaveError),
    /// The figure is not shown in a window (`subplot_tool`; the web
    /// backend has no windows).
    NotShown,
}

impl fmt::Display for GuiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Platform(err) => write!(f, "window system error: {err}"),
            Self::Web(err) => err.fmt(f),
            Self::Ui(err) => write!(f, "user interface error: {err}"),
            Self::Render(err) => write!(f, "cannot render the figure: {err}"),
            Self::NotShown => f.write_str("the figure is not shown in a window"),
        }
    }
}

impl std::error::Error for GuiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Platform(err) => Some(err),
            Self::Web(err) => Some(err),
            Self::Ui(err) => Some(err),
            Self::Render(err) => Some(err),
            Self::NotShown => None,
        }
    }
}

impl From<plt_platform::Error> for GuiError {
    fn from(err: plt_platform::Error) -> Self {
        Self::Platform(err)
    }
}

impl From<crate::web::WebError> for GuiError {
    fn from(err: crate::web::WebError) -> Self {
        Self::Web(err)
    }
}

impl From<plt_ui::UiError> for GuiError {
    fn from(err: plt_ui::UiError) -> Self {
        Self::Ui(err)
    }
}

impl From<SaveError> for GuiError {
    fn from(err: SaveError) -> Self {
        Self::Render(err)
    }
}

/// Shows each figure in its own window ("Figure 1", "Figure 2", ...) and
/// returns once all of them are closed (matplotlib's `pyplot.show()`).
///
/// # Errors
///
/// When there is no window system to connect to, or a window fails.
pub fn show(figures: &[Figure]) -> Result<(), GuiError> {
    let mut session = Session::new()?;
    let titled: Vec<(Figure, String)> = figures
        .iter()
        .enumerate()
        .map(|(i, figure)| (figure.clone(), format!("Figure {}", i + 1)))
        .collect();
    session.show(&titled)?;
    session.run(None, || false)
}
