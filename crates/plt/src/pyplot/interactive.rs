//! Windows and interactive mode: showing figures, redrawing them,
//! waiting for input and figure events (matplotlib's `show`, `ion`,
//! `pause`, `ginput`, `connect`, ...).

use std::cell::RefCell;
use std::time::Duration;

use super::registry::{gcf, get_fignums, with_registry};
use crate::events::{ConnectionId, Event, EventKind};
use crate::figure::Figure;
use crate::gui::{GinputOptions, GuiError, Session};

thread_local! {
    /// The windows of the figures, once shown.
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}

/// Shows every open figure in the thread's window session (started on
/// first use), runs `f` with it, then closes the figures whose windows
/// were closed. The session is not borrowed while `f` runs, so figure
/// callbacks may use pyplot.
fn with_session<R>(f: impl FnOnce(&mut Session) -> Result<R, GuiError>) -> Result<R, GuiError> {
    let stored = SESSION.with(|session| session.borrow_mut().take());
    let mut session = match stored {
        Some(session) => session,
        None => Session::new()?,
    };
    let figures: Vec<(Figure, String)> = with_registry(|registry| {
        registry
            .figures
            .iter()
            .map(|entry| (entry.figure.clone(), entry.title()))
            .collect()
    });
    let result = session.show(&figures).and_then(|()| f(&mut session));
    let closed = session.take_closed();
    with_registry(|registry| registry.remove_where(|entry| closed.contains(&entry.figure)));
    SESSION.with(|stored| *stored.borrow_mut() = Some(session));
    result
}

/// In interactive mode, shows new figures and redraws changed ones
/// (matplotlib's `draw_if_interactive`). Best effort: without a window
/// system nothing is shown, and `show` or `pause` report why.
pub fn draw_if_interactive() {
    if isinteractive() {
        let _ = with_session(|_| Ok(()));
    }
}

/// Closes the windows of `figures`. Best effort: a window system that
/// fails here takes the windows with it.
pub(super) fn close_windows(figures: &[Figure]) {
    SESSION.with(|session| {
        if let Some(session) = session.borrow_mut().as_mut() {
            for figure in figures {
                let _ = session.close(figure);
            }
        }
    });
}

/// Shows every open figure in a window (matplotlib's `show()`): outside
/// interactive mode it returns once all windows are closed, closing their
/// figures; in interactive mode it returns at once.
///
/// # Errors
///
/// When there is no window system or a window fails.
pub fn show() -> Result<(), GuiError> {
    if isinteractive() {
        return with_session(|_| Ok(()));
    }
    with_session(|session| session.run(None, || false))
}

/// Turns interactive mode on (matplotlib's `ion`): new figures open their
/// windows at once and [`show`] does not wait.
pub fn ion() {
    with_registry(|registry| registry.interactive = true);
}

/// Turns interactive mode off (matplotlib's `ioff`).
pub fn ioff() {
    with_registry(|registry| registry.interactive = false);
}

/// Whether interactive mode is on (matplotlib's `isinteractive`).
pub fn isinteractive() -> bool {
    with_registry(|registry| registry.interactive)
}

/// Redraws the shown figures that changed (matplotlib's `draw`).
///
/// # Errors
///
/// When a figure cannot be rendered.
pub fn draw() -> Result<(), GuiError> {
    let shown = SESSION.with(|session| session.borrow().is_some());
    if shown {
        with_session(|_| Ok(()))
    } else {
        Ok(())
    }
}

/// Shows the figures and keeps their windows responsive for `seconds`;
/// without figures it just sleeps (matplotlib's `pause`).
///
/// # Errors
///
/// When there is no window system or a window fails.
pub fn pause(seconds: f64) -> Result<(), GuiError> {
    let duration = Duration::try_from_secs_f64(seconds).unwrap_or(Duration::ZERO);
    if get_fignums().is_empty() {
        std::thread::sleep(duration);
        return Ok(());
    }
    with_session(|session| session.pause(duration))
}

/// Waits for up to `n` clicks in the current figure (all until stopped
/// when `n` is 0) for at most `timeout`, and returns their data
/// coordinates (matplotlib's `ginput`; see [`Session::ginput`]).
///
/// # Errors
///
/// When there is no window system or a window fails.
pub fn ginput(
    n: usize,
    timeout: Option<Duration>,
    options: GinputOptions,
) -> Result<Vec<(f64, f64)>, GuiError> {
    let figure = gcf();
    with_session(|session| session.ginput(&figure, n, timeout, options))
}

/// Waits for a key press (`Some(true)`), a click (`Some(false)`) in the
/// current figure, or `timeout` (`None`) (matplotlib's
/// `waitforbuttonpress`).
///
/// # Errors
///
/// When there is no window system or a window fails.
pub fn waitforbuttonpress(timeout: Option<Duration>) -> Result<Option<bool>, GuiError> {
    let figure = gcf();
    with_session(|session| session.wait_for_button_press(&figure, timeout))
}

/// Opens the subplot configuration tool for `targetfig`, by default the
/// current figure, in a window of its own: sliders for the margins and
/// spacing of the subplots (matplotlib's `subplot_tool`, which triggers
/// the subplots tool of the figure window's toolbar). The open figures are
/// shown first; a second call raises the tool window.
///
/// ```no_run
/// use plt::pyplot as plt;
/// plt::subplots(2, 2)?;
/// plt::subplot_tool(None)?;
/// plt::show()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When there is no window system, a window fails, or `targetfig` is not
/// a pyplot figure, which has no window ([`GuiError::NotShown`]).
pub fn subplot_tool(targetfig: Option<&Figure>) -> Result<(), GuiError> {
    let figure = targetfig.cloned().unwrap_or_else(gcf);
    with_session(|session| session.configure_subplots(&figure))
}

/// Calls `callback` on every event of `kind` in the current figure
/// (matplotlib's `connect`, which is `gcf().canvas.mpl_connect`).
pub fn connect(kind: EventKind, callback: impl FnMut(&Event) + 'static) -> ConnectionId {
    gcf().connect(kind, callback)
}

/// Removes a callback of the current figure; `false` if it was not
/// connected there (matplotlib's `disconnect`).
pub fn disconnect(id: ConnectionId) -> bool {
    gcf().disconnect(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pyplot::{close_all, figure};

    #[test]
    fn interactive_mode_toggles() {
        ioff();
        assert!(!isinteractive());
        ion();
        assert!(isinteractive());
        ioff();
        assert!(!isinteractive());
        assert!(draw().is_ok(), "nothing shown, nothing to redraw");
    }

    #[test]
    fn callbacks_attach_to_the_current_figure() {
        close_all();
        let fig = figure();
        let id = connect(EventKind::KeyPress, |_| {});
        assert!(disconnect(id));
        assert!(!disconnect(id), "already removed");
        assert_eq!(gcf(), fig);
        close_all();
    }
}
