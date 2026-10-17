//! Interactive figure windows (matplotlib's GUI backends and
//! `pyplot.show`), built on the `plt-ui` engine and `plt-platform`
//! windows.
//!
//! Each [`FigureWindow`] shows a figure above a toolbar — home, back,
//! forward, pan, zoom and save — with a readout of the data coordinates
//! under the pointer. Dragging pans or zooms as in matplotlib (see
//! [`crate::navigation`]), the wheel zooms around the pointer, and the
//! `keymap.*` rcParams bind the usual shortcuts (`h` home, `p` pan, `o`
//! zoom, `g` grids, `l`/`k` log scales, `s` save, `q` close, ...). The
//! window resizes the figure with it and renders at the screen's scale
//! factor.
//!
//! ```no_run
//! use plt::figure::Figure;
//! let fig = Figure::new();
//! fig.subplot().plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])?;
//! plt::gui::show(&[fig])?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod actions;
mod icons;
mod keys;
mod window;

use std::fmt;

use plt_platform::{Event, EventLoop};

pub use window::FigureWindow;

use crate::figure::{Figure, SaveError};

/// Why a figure window failed.
#[derive(Debug)]
pub enum GuiError {
    /// The windowing system refused (no display, lost connection, ...).
    Platform(plt_platform::Error),
    /// The UI could not be laid out or painted.
    Ui(plt_ui::UiError),
    /// The figure could not be rendered.
    Render(SaveError),
}

impl fmt::Display for GuiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Platform(err) => write!(f, "window system error: {err}"),
            Self::Ui(err) => write!(f, "user interface error: {err}"),
            Self::Render(err) => write!(f, "cannot render the figure: {err}"),
        }
    }
}

impl std::error::Error for GuiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Platform(err) => Some(err),
            Self::Ui(err) => Some(err),
            Self::Render(err) => Some(err),
        }
    }
}

impl From<plt_platform::Error> for GuiError {
    fn from(err: plt_platform::Error) -> Self {
        Self::Platform(err)
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
    let mut event_loop = EventLoop::new()?;
    let mut windows: Vec<FigureWindow> = Vec::new();
    let mut started = false;
    let mut failure: Option<GuiError> = None;
    event_loop.run(|cx, event| {
        let result = match event {
            Event::Started => {
                started = true;
                figures.iter().enumerate().try_for_each(|(i, figure)| {
                    windows.push(FigureWindow::open(
                        cx,
                        figure,
                        &format!("Figure {}", i + 1),
                    )?);
                    Ok(())
                })
            }
            Event::Timer(timer) => windows
                .iter_mut()
                .find(|window| window.owns_timer(timer))
                .map_or(Ok(()), |window| window.tick(cx, timer)),
            Event::Window { window, event } => {
                match windows.iter_mut().position(|w| w.id() == window) {
                    Some(index) => match windows[index].handle(cx, &event) {
                        Ok(true) => Ok(()),
                        Ok(false) => {
                            windows.remove(index);
                            cx.close_window(window).map_err(GuiError::from)
                        }
                        Err(err) => Err(err),
                    },
                    None => Ok(()),
                }
            }
            _ => Ok(()),
        };
        if let Err(err) = result {
            failure = Some(err);
        }
        if failure.is_some() || (started && windows.is_empty()) {
            cx.exit();
        }
    })?;
    failure.map_or(Ok(()), Err)
}
