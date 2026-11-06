//! Interactive figure windows (matplotlib's GUI backends and
//! `pyplot.show`), built on the `pltrs-ui` engine and `pltrs-platform`
//! windows.
//!
//! Each [`FigureWindow`] shows a figure above a toolbar — home, back,
//! forward, pan, zoom, subplot configuration, figure options and save,
//! with matplotlib's own icons in matplotlib's [look](crate::look) —
//! with a readout of the data coordinates under the pointer. Dragging pans
//! or zooms as in matplotlib (see [`crate::navigation`]), the wheel zooms
//! around the pointer with Control (or `x` or `y`) held, and the
//! `keymap.*` rcParams bind the usual
//! shortcuts (`h` home, `p` pan, `o` zoom, `g` grids, `l`/`k` log scales,
//! `s` save, `f` full screen, `q` close, ...), and the mouse's back and
//! forward buttons step through the views. The window resizes the
//! figure with it and renders at the screen's scale factor. Saving opens a
//! dialog to pick the directory, the file name and the format (every
//! format matplotlib's Qt dialog offers, by its name there); the figure
//! options dialog edits the titles, limits, labels and scales of the axes,
//! the style of their curves and the colormaps and interpolation of their
//! images; the subplot tool adjusts the subplot parameters as matplotlib's
//! Qt tool does, and under constrained layout that layout's padding and
//! spacing, or hands the subplots over to the parameters.
//!
//! Everything works from the keyboard too, following Qt and GTK. The
//! canvas has the focus, as in matplotlib's Qt backend, and takes it back
//! when clicked; Ctrl+Tab moves the focus on to the toolbar, whose buttons
//! Tab steps through and Space presses, while the keys they leave unused
//! still reach the canvas. In the dialogs, the first field has the focus,
//! Tab and Shift+Tab move it, Enter presses the primary button (Save, OK,
//! Close) unless the focused control uses Enter, Escape closes, and
//! Ctrl+Tab switches the figure options' tabs.
//!
//! ```no_run
//! use pltrs::figure::Figure;
//! let fig = Figure::new();
//! fig.subplot().plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])?;
//! pltrs::gui::show(&[fig])?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`FigureWindow::screenshot`] draws such a window off screen, with no
//! display, and returns what it shows; [`FigureWindow::capture`] reads a
//! window of a headless event loop driven by synthetic input.
//!
//! A [`WindowHost`] shows figures in the same windows for programs that
//! handle their figures' input themselves, as matplotlib's backends do
//! (pltrs (Python)'s native backend): it reports input, resizes and
//! toolbar clicks, and leaves navigation and callbacks to the program.
//!
//! Every window shows pltrs's [`app_icon`], drawn at the sizes and in the
//! layout each platform asks for (in the Dock on macOS).

pub(crate) mod actions;
mod app_icon;
mod backend_rc;
mod dialog;
mod help_dialog;
mod host;
pub(crate) mod icons;
pub(crate) mod images;
pub(crate) mod keys;
mod options;
mod save_dialog;
mod session;
mod subplot_dialog;
mod tool_icons;
pub(crate) mod tools;
mod window;

use std::fmt;

pub use crate::events::GinputOptions;
pub use app_icon::app_icon;
pub use backend_rc::{MacosxWindowMode, ToolbarKind};
pub use host::{
    CanvasInput, HostEvent, HostTimerId, HostedWindowId, InputKind, ShortcutGroup, ToolImage,
    ToolItem, WindowHost,
};
pub use keys::format_shortcut;
pub use pltrs_ui::Icon;
pub use session::Session;
pub use subplot_dialog::{ConstrainedSpacing, SubplotChange, SubplotLayout};
pub use tools::{
    NAVIGATION_GROUP, ToolChange, ToolError, ToolEvent, ToolKind, ToolListenerId, ToolSpec,
};
pub use window::FigureWindow;

use crate::figure::{CanvasError, Figure, SaveError};

/// Why showing figures failed, in windows or in a browser.
#[derive(Debug)]
pub enum GuiError {
    /// The windowing system refused (no display, lost connection, ...).
    Platform(pltrs_platform::Error),
    /// The web backend failed (see [`crate::web`]).
    Web(crate::web::WebError),
    /// The UI could not be laid out or painted.
    Ui(pltrs_ui::UiError),
    /// The figure could not be rendered.
    Render(SaveError),
    /// The figure's canvas could not be drawn or shown.
    Canvas(CanvasError),
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
            Self::Canvas(err) => write!(f, "cannot show the figure: {err}"),
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
            Self::Canvas(err) => Some(err),
            Self::NotShown => None,
        }
    }
}

impl From<pltrs_platform::Error> for GuiError {
    fn from(err: pltrs_platform::Error) -> Self {
        Self::Platform(err)
    }
}

impl From<crate::web::WebError> for GuiError {
    fn from(err: crate::web::WebError) -> Self {
        Self::Web(err)
    }
}

impl From<pltrs_ui::UiError> for GuiError {
    fn from(err: pltrs_ui::UiError) -> Self {
        Self::Ui(err)
    }
}

impl From<CanvasError> for GuiError {
    fn from(err: CanvasError) -> Self {
        Self::Canvas(err)
    }
}

impl From<SaveError> for GuiError {
    fn from(err: SaveError) -> Self {
        Self::Render(err)
    }
}

/// Shows each figure in its own window ("Figure 1", "Figure 2", ...) and
/// returns once all of them are closed (matplotlib's `pyplot.show()`),
/// with the backend [`pyplot::show`](crate::pyplot::show) uses, which
/// falls back the same way: where windows cannot open, the figures are
/// not shown at all on Linux (a machine without a display) and served to
/// a web browser on macOS and Windows. `MPLBACKEND` and
/// [`pyplot::switch_backend`](crate::pyplot::switch_backend) choose
/// another backend.
///
/// # Errors
///
/// When the backend cannot start (no window system and no fallback, or a
/// busy web port), or a window fails.
pub fn show(figures: &[Figure]) -> Result<(), GuiError> {
    crate::pyplot::show_until_closed(figures)
}
