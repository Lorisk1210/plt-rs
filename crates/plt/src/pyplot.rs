//! A stateful interface in the style of `matplotlib.pyplot`: each thread
//! has a list of numbered figures, a current figure and, in it, a current
//! axes; the functions here act on them.
//!
//! ```
//! use plt::pyplot as plt;
//! plt::plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])?;
//! plt::title("Squares");
//! plt::xlabel("x");
//! let png = plt::gcf().to_png()?;
//! assert!(!png.is_empty());
//! plt::close_all();
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Everything else is reached through the objects: [`gca`] returns the
//! current [`Axes`] with the full plotting API, [`gcf`] the current
//! [`Figure`].
//!
//! [`show`] opens a window per figure and waits until they are closed. In
//! interactive mode ([`ion`]) new figures open their windows at once and
//! [`show`] returns immediately; [`pause`] keeps the windows responsive
//! for a while, and [`ginput`] and [`waitforbuttonpress`] wait for clicks
//! and keys in them.

use std::cell::RefCell;
use std::path::Path;
use std::time::Duration;

use crate::artist::ArtistHandle;
use crate::axes::{Axes, PlotError};
use crate::collections::Collection;
use crate::figure::{Figure, SaveError};
use crate::gridspec::{GridSpec, GridSpecError};
use crate::gui::{GinputOptions, GuiError, Session};
use crate::lines::Line2D;
use crate::numeric::Grid;
use crate::scale::ScaleError;
use crate::text::Text;
use crate::units::AxisValues;

/// The figures of one thread, by number, and the current one.
#[derive(Default)]
struct Registry {
    figures: Vec<(u32, Figure)>,
    current: Option<u32>,
    /// Whether figures show as they are made (matplotlib's interactive
    /// mode).
    interactive: bool,
}

impl Registry {
    fn get(&self, num: u32) -> Option<&Figure> {
        self.figures
            .iter()
            .find(|(n, _)| *n == num)
            .map(|(_, fig)| fig)
    }

    /// Registers `figure` under the lowest free number and makes it
    /// current.
    fn add(&mut self, figure: Figure) -> u32 {
        let mut used: Vec<u32> = self.figures.iter().map(|(n, _)| *n).collect();
        used.sort_unstable();
        let num = used.iter().zip(1..).find(|(used, n)| *used != n).map_or(
            u32::try_from(used.len()).map_or(u32::MAX, |len| len + 1),
            |(_, n)| n,
        );
        self.figures.push((num, figure));
        self.current = Some(num);
        num
    }
}

thread_local! {
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry::default());
    /// The windows of the figures, once shown.
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}

fn with_registry<R>(f: impl FnOnce(&mut Registry) -> R) -> R {
    REGISTRY.with(|registry| f(&mut registry.borrow_mut()))
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
            .map(|(num, fig)| (fig.clone(), format!("Figure {num}")))
            .collect()
    });
    let result = session.show(&figures).and_then(|()| f(&mut session));
    let closed = session.take_closed();
    with_registry(|registry| {
        registry.figures.retain(|(_, fig)| !closed.contains(fig));
        if registry
            .current
            .is_some_and(|num| registry.get(num).is_none())
        {
            registry.current = registry.figures.last().map(|(n, _)| *n);
        }
    });
    SESSION.with(|stored| *stored.borrow_mut() = Some(session));
    result
}

/// In interactive mode, shows new figures and redraws changed ones
/// (matplotlib's `draw_if_interactive`). Best effort: without a window
/// system nothing is shown, and `show` or `pause` report why.
fn draw_if_interactive() {
    if isinteractive() {
        let _ = with_session(|_| Ok(()));
    }
}

/// A new figure with the default size, made current (matplotlib's
/// `figure()`).
pub fn figure() -> Figure {
    let fig = Figure::new();
    with_registry(|registry| registry.add(fig.clone()));
    draw_if_interactive();
    fig
}

/// The figure numbered `num`, made current; created if it does not exist
/// (matplotlib's `figure(num)`).
pub fn figure_num(num: u32) -> Figure {
    with_registry(|registry| {
        let fig = if let Some(fig) = registry.get(num) {
            fig.clone()
        } else {
            let fig = Figure::new();
            registry.figures.push((num, fig.clone()));
            fig
        };
        registry.current = Some(num);
        fig
    })
}

/// The numbers of the open figures, in creation order.
pub fn fignums() -> Vec<u32> {
    with_registry(|registry| registry.figures.iter().map(|(n, _)| *n).collect())
}

/// The current figure, created if there is none (matplotlib's `gcf`).
pub fn gcf() -> Figure {
    let current = with_registry(|registry| registry.current.and_then(|n| registry.get(n).cloned()));
    current.unwrap_or_else(figure)
}

/// The current axes of the current figure, created if there is none
/// (matplotlib's `gca`).
pub fn gca() -> Axes {
    gcf().gca()
}

/// Makes `ax` the current axes and its figure the current figure, if that
/// figure is managed here (matplotlib's `sca`).
pub fn sca(ax: &Axes) {
    with_registry(|registry| {
        if let Some((num, fig)) = registry
            .figures
            .iter()
            .find(|(_, fig)| fig.contains_artist(ax))
        {
            fig.sca(ax);
            registry.current = Some(*num);
        }
    });
}

/// A new current figure with a grid of axes (matplotlib's `subplots`).
///
/// # Errors
///
/// When the grid has no cells.
pub fn subplots(nrows: usize, ncols: usize) -> Result<(Figure, Grid<Axes>), GridSpecError> {
    let fig = figure();
    let axes = fig.subplots(nrows, ncols)?;
    Ok((fig, axes))
}

/// The axes in cell `index` (1-based, row-major) of an `nrows × ncols`
/// grid of the current figure, reusing an axes already there (matplotlib's
/// `subplot`); it becomes the current axes.
///
/// # Errors
///
/// When the grid is empty or `index` is outside it.
pub fn subplot(nrows: usize, ncols: usize, index: usize) -> Result<Axes, GridSpecError> {
    let fig = gcf();
    let spec = GridSpec::new(nrows, ncols)?.numbered(index)?;
    let existing = fig
        .axes()
        .into_iter()
        .find(|ax| ax.subplot_spec().as_ref() == Some(&spec));
    let ax = match existing {
        Some(ax) => ax,
        None => fig.add_subplot_spec(&spec)?,
    };
    fig.sca(&ax);
    Ok(ax)
}

/// Closes the current figure and its window (matplotlib's `close()`).
pub fn close() {
    let closed = with_registry(|registry| {
        let num = registry.current.take()?;
        let figure = registry.get(num).cloned();
        registry.figures.retain(|(n, _)| *n != num);
        registry.current = registry.figures.last().map(|(n, _)| *n);
        figure
    });
    close_windows(closed.as_slice());
}

/// Closes every figure and window (matplotlib's `close("all")`); the
/// interactive mode stays.
pub fn close_all() {
    let closed = with_registry(|registry| {
        registry.current = None;
        std::mem::take(&mut registry.figures)
    });
    let figures: Vec<Figure> = closed.into_iter().map(|(_, fig)| fig).collect();
    close_windows(&figures);
}

/// Closes the windows of `figures`. Best effort: a window system that
/// fails here takes the windows with it.
fn close_windows(figures: &[Figure]) {
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
    if fignums().is_empty() {
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

/// Saves the current figure as PNG (matplotlib's `savefig`).
///
/// # Errors
///
/// When the figure cannot be rendered or written.
pub fn savefig(path: impl AsRef<Path>) -> Result<(), SaveError> {
    gcf().savefig(path)
}

/// Plots `y` against `x` on the current axes.
///
/// # Errors
///
/// As [`Axes::plot`].
pub fn plot(x: impl Into<AxisValues>, y: impl Into<AxisValues>) -> Result<Line2D, PlotError> {
    gca().plot(x, y)
}

/// Plots with a format string such as `"r--o"` on the current axes.
///
/// # Errors
///
/// As [`Axes::plot_fmt`].
pub fn plot_fmt(
    x: impl Into<AxisValues>,
    y: impl Into<AxisValues>,
    format: &str,
) -> Result<Line2D, PlotError> {
    gca().plot_fmt(x, y, format)
}

/// Scatters markers on the current axes.
///
/// # Errors
///
/// As [`Axes::scatter`].
pub fn scatter(
    x: impl Into<AxisValues>,
    y: impl Into<AxisValues>,
) -> Result<Collection, PlotError> {
    gca().scatter(x, y)
}

/// Sets the title of the current axes.
pub fn title(text: impl Into<String>) -> Text {
    gca().set_title(text)
}

/// Sets the x label of the current axes.
pub fn xlabel(text: impl Into<String>) {
    gca().set_xlabel(text);
}

/// Sets the y label of the current axes.
pub fn ylabel(text: impl Into<String>) {
    gca().set_ylabel(text);
}

/// Sets the x limits of the current axes.
pub fn xlim(left: f64, right: f64) {
    gca().set_xlim(left, right);
}

/// Sets the y limits of the current axes.
pub fn ylim(bottom: f64, top: f64) {
    gca().set_ylim(bottom, top);
}

/// Sets the x scale of the current axes by name.
///
/// # Errors
///
/// When no scale is registered under `name`.
pub fn xscale(name: &str) -> Result<(), ScaleError> {
    gca().set_xscale(name).map(|_| ())
}

/// Sets the y scale of the current axes by name.
///
/// # Errors
///
/// When no scale is registered under `name`.
pub fn yscale(name: &str) -> Result<(), ScaleError> {
    gca().set_yscale(name).map(|_| ())
}

/// Shows or hides the major grid of the current axes.
pub fn grid(visible: bool) {
    gca().grid(visible);
}

/// Sets the title of the current figure (matplotlib's `suptitle`).
pub fn suptitle(text: impl Into<String>) -> Text {
    gcf().suptitle(text)
}

/// Clears the current axes (matplotlib's `cla`).
pub fn cla() {
    gca().clear();
}

/// Clears the current figure (matplotlib's `clf`).
pub fn clf() {
    gcf().clear();
}

/// Removes `ax` from its figure, or the current axes when `None`
/// (matplotlib's `delaxes`).
pub fn delaxes(ax: Option<&Axes>) {
    match ax {
        Some(ax) => ax.remove(),
        None => gca().remove(),
    }
}

/// Whether figure `num` is open (matplotlib's `fignum_exists`).
pub fn fignum_exists(num: u32) -> bool {
    fignums().contains(&num)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn figures_are_numbered_and_current() {
        close_all();
        let first = figure();
        let second = figure();
        assert_eq!(fignums(), [1, 2]);
        assert_eq!(gcf(), second);
        assert_eq!(figure_num(1), first);
        assert_eq!(gcf(), first);
        close();
        assert_eq!(fignums(), [2]);
        assert_eq!(figure(), figure_num(1), "the lowest free number is reused");
        close_all();
        assert_eq!(fignums(), [] as [u32; 0]);
    }

    #[test]
    fn plotting_goes_to_the_current_axes() {
        close_all();
        let line = plot(vec![0.0, 1.0], vec![0.0, 2.0]).unwrap();
        let ax = gca();
        assert!(gcf().contains_artist(&line));
        assert_eq!(gcf().axes(), std::slice::from_ref(&ax));
        let right = subplot(1, 2, 2).unwrap();
        assert_eq!(gca(), right);
        assert_eq!(subplot(1, 2, 2).unwrap(), right, "subplot reuses the cell");
        sca(&ax);
        assert_eq!(gca(), ax);
        title("t");
        assert_eq!(ax.title(crate::axes::TitleLocation::Center), "t");
        close_all();
    }

    #[test]
    fn clearing_and_removing() {
        close_all();
        plot(vec![0.0, 1.0], vec![0.0, 2.0]).unwrap();
        cla();
        assert_eq!(gca().lines(), Vec::<crate::lines::Line2D>::new());
        delaxes(None);
        assert_eq!(gcf().axes(), Vec::<Axes>::new());
        subplot(2, 1, 1).unwrap();
        clf();
        assert_eq!(gcf().axes(), Vec::<Axes>::new());
        assert!(fignum_exists(1) && !fignum_exists(2));
        close_all();
    }

    #[test]
    fn subplots_make_a_new_figure() {
        close_all();
        let (fig, axes) = subplots(2, 2).unwrap();
        assert_eq!(gcf(), fig);
        assert_eq!(axes.as_slice().len(), 4);
        assert_eq!(gca(), axes[(1, 1)], "the last axes added is current");
        close_all();
    }
}
