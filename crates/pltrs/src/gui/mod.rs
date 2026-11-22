//! Figure windows with a toolbar, keyboard shortcuts and dialogs
//! (matplotlib's GUI backends and `pyplot.show`).
//!
//! [`show`] shows each figure in its own window. It returns when the user
//! closes all windows.
//!
//! ```no_run
//! use pltrs::figure::Figure;
//! let fig = Figure::new();
//! fig.subplot().plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])?;
//! pltrs::gui::show(&[fig])?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # The window
//!
//! A [`FigureWindow`] shows a figure below a toolbar. A status line shows
//! the data coordinates under the pointer, and the value of an image under
//! it. When the user resizes the window, the figure changes its size too.
//! The window draws the figure at the scale factor of the screen.
//!
//! The mouse controls the view ([`navigation`](crate::navigation)):
//!
//! * In pan mode, a drag moves the data. In zoom mode, a drag selects a
//!   rectangle to zoom to.
//! * The wheel zooms around the pointer while you hold Control, `x` or `y`.
//! * A drag turns 3-D axes.
//! * The back and forward buttons of the mouse step through the views.
//!
//! The figure gets the input as events ([`events`](crate::events)), so the
//! callbacks, pickers and widgets of the figure work in the window.
//!
//! # Toolbar and keyboard shortcuts
//!
//! | Action | Default key |
//! |--------|-------------|
//! | Go to the original view (home) | `h` |
//! | Go back or forward through the views | `left`, `right` |
//! | Pan, or zoom to a rectangle | `p`, `o` |
//! | Show or hide the major grid or the minor grid | `g`, `G` |
//! | Switch the y axis or the x axis to a logarithmic scale | `l`, `k` |
//! | Save the figure, or copy it to the clipboard | `s`, `ctrl+c` |
//! | Show the window full screen, or close it | `f`, `q` |
//! | Show the keyboard shortcuts | `F1` |
//! | Change the subplot spacing | |
//! | Change the figure options | |
//!
//! The `keymap.*` runtime settings change the keys ([`rc`](crate::rc)).
//! [`format_shortcut`] writes a key name as the help window shows it.
//!
//! You can use all controls from the keyboard. The canvas has the keyboard
//! focus. Ctrl+Tab moves the focus to the toolbar. There, Tab moves from
//! button to button, and Space presses the button.
//!
//! # Tools of your own
//!
//! [`Figure::add_tool`] adds a toolbar button with a keyboard shortcut. A
//! [`ToolSpec`] describes the tool. It has a name, a description, an
//! [`Icon`], shortcuts and a [`ToolKind`]: a trigger or a toggle. Toggles
//! in [`NAVIGATION_GROUP`] take turns with the pan and zoom tools.
//!
//! | Item | What it is |
//! |------|------------|
//! | [`ToolEvent`] | A use of a tool, as its callback gets it. |
//! | [`ToolChange`] | A change to the tools, for [`Figure::on_tool_change`]. |
//! | [`ToolListenerId`] | The id of an [`on_tool_change`](Figure::on_tool_change) callback. |
//! | [`ToolError`] | Why pltrs cannot add a tool. |
//!
//! # Dialogs
//!
//! | Dialog | What it does |
//! |--------|--------------|
//! | Save | Selects the directory, the file name and the format. It offers all 13 formats of [`SaveFormat`](crate::figure::SaveFormat): AVIF, EPS, GIF, JPEG, PGF, PDF, PNG, PostScript, raw RGBA, SVG, compressed SVG, TIFF and WebP. |
//! | Figure options | Changes the titles, limits, labels and scales of the axes, the style of the lines, and the colormap and interpolation of the images. |
//! | Subplot spacing | Changes the margins and the spacing of the subplots. Under constrained layout, it changes the padding and the spacing of that layout ([`SubplotLayout`], [`SubplotChange`], [`ConstrainedSpacing`]). |
//! | Keyboard shortcuts | Lists the shortcuts. |
//!
//! In a dialog, Tab and Shift+Tab move the focus. Enter presses the main
//! button, and Escape closes the dialog. Ctrl+Tab selects the next tab of
//! the figure options.
//!
//! # Platforms
//!
//! pltrs opens windows on Wayland, X11, macOS and Windows. It links to no
//! GUI toolkit. On Linux, pltrs uses Wayland if `WAYLAND_DISPLAY` is set,
//! or else X11 if `DISPLAY` is set. The environment variable
//! `PLTRS_PLATFORM` selects the platform: `wayland`, `x11`, `macos`,
//! `windows` or `headless`.
//!
//! The windows use the modern look by default. The matplotlib look gives
//! them the toolbar icons of matplotlib ([`look`](crate::look)). The
//! runtime setting `toolbar` selects the toolbar ([`ToolbarKind`]). On
//! macOS, `macosx.window_mode` selects tabs or separate windows
//! ([`MacosxWindowMode`]).
//!
//! # Where no window can open
//!
//! On Linux, [`show`] shows nothing if no display is available. On macOS
//! and Windows, it shows the figures in a web browser
//! ([`web`](crate::web)). The environment variable `MPLBACKEND` and
//! [`pyplot::switch_backend`](crate::pyplot::switch_backend) select another
//! backend.
//!
//! Tests can use windows without a display:
//!
//! * [`FigureWindow::screenshot`] gives the pixels of a window that shows a
//!   figure, with its toolbar.
//! * [`Session::headless`] runs windows without a display, for synthetic
//!   input.
//! * [`FigureWindow::capture`] gives the pixels of such a window.
//!
//! # Control windows from your program
//!
//! A [`Session`] opens windows and returns at once. Use it to update
//! windows while your program computes.
//!
//! | Method | What it does |
//! |--------|--------------|
//! | [`Session::new`], [`Session::headless`] | Start a session with or without a display. |
//! | [`Session::show`], [`Session::close`] | Open or close the windows of figures. |
//! | [`Session::run`], [`Session::pause`] | Run the windows until a time, or for a duration. |
//! | [`Session::ginput`] | Collects the points that the user clicks ([`GinputOptions`]). |
//! | [`Session::wait_for_button_press`] | Waits for a click or a key press. |
//! | [`Session::configure_subplots`] | Opens the subplot spacing dialog. |
//!
//! A [`WindowHost`] shows figures in the same windows for a program that
//! handles the input itself. pltrs (Python) uses it. The host reports
//! input ([`HostEvent`], [`CanvasInput`], [`InputKind`]), resizes and
//! toolbar clicks. The program describes the toolbar ([`ToolItem`],
//! [`ToolImage`]) and the help window ([`ShortcutGroup`]).
//!
//! # Icons and desktop integration
//!
//! Each window shows the pltrs icon ([`app_icon`]). The GNOME shell takes
//! the icons of its dash from a desktop entry instead.
//! [`install_desktop_entry`] writes this entry, and
//! [`remove_desktop_entry`] removes it ([`DesktopEntry`], [`ICON_SIDES`]).
//!
//! # Errors
//!
//! [`GuiError`] tells why a window or a browser view failed.

pub(crate) mod actions;
mod app_icon;
mod backend_rc;
mod desktop_entry;
mod dialog;
mod help_dialog;
mod host;
pub(crate) mod icons;
pub(crate) mod images;
mod input;
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
pub use desktop_entry::{
    DesktopEntry, ICON_SIDES, install_desktop_entry, install_desktop_entry_in,
    remove_desktop_entry, remove_desktop_entry_from, user_data_dir,
};
pub use host::{
    HostEvent, HostTimerId, HostedWindowId, ShortcutGroup, ToolImage, ToolItem, WindowHost,
};
pub use input::{CanvasInput, InputKind};
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
