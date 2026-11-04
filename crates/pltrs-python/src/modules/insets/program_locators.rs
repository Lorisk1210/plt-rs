//! Axes placed by locators of the program's (matplotlib's
//! `set_axes_locator` with any callable of the axes and the renderer).
//!
//! matplotlib calls such a locator whenever it draws the axes. Python code
//! cannot run while pltrs (Rust) lays the figure out (it would need the
//! figure, which is busy), so the package calls the locators of a figure
//! from its layout hook, which runs before every layout of a draw, and
//! places each axes where its locator says until the next one
//! (`set_located_position`).

use std::ops::ControlFlow;

use pltrs::axes::{Axes, AxesLocatorFn};
use pltrs::figure::{DrawHook, Figure};
use pltrs::transforms::Bbox;

use crate::binding::prelude::*;

/// Places `axes` at `bounds` (`(x0, y0, width, height)` in fractions of
/// its figure) before its aspect is applied, at every layout until it is
/// placed again, or at its original position for `None` (what a locator
/// returning `None` asks for).
fn set_located_position(axes: Axes, bounds: Option<(f64, f64, f64, f64)>) -> PyResult<()> {
    let position = bounds.map(|(x0, y0, width, height)| Bbox::from_bounds(x0, y0, width, height));
    axes.set_axes_locator(Some(AxesLocatorFn::at(position)));
    Ok(())
}

/// Calls `hook` (a Python callable taking nothing) whenever `figure` is
/// laid out to be drawn, before its layout; `None` removes the hook. What
/// the hook raises is raised by the native call drawing the figure, and
/// stops the draw there, as an exception in matplotlib's draw does.
fn set_layout_hook(figure: Figure, hook: Option<Callback>) -> PyResult<()> {
    figure.set_layout_hook(hook.map(|hook| {
        DrawHook::stopping(move |_| match hook.call::<Owned>(()) {
            Some(_) => ControlFlow::Continue(()),
            None => ControlFlow::Break(()),
        })
    }));
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_located_position", set_located_position)?;
    module.add("set_layout_hook", set_layout_hook)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn located_positions_replace_the_original_one_until_cleared() {
        let ax = Figure::new().add_axes([0.1, 0.1, 0.8, 0.8]).unwrap();
        set_located_position(ax.clone(), Some((0.2, 0.3, 0.4, 0.5))).unwrap();
        let (x0, y0, width, height) = ax.position().bounds();
        assert_eq!((x0, y0, height), (0.2, 0.3, 0.5));
        assert!((width - 0.4).abs() < 1e-12);
        set_located_position(ax.clone(), None).unwrap();
        assert_eq!(ax.position().bounds(), (0.1, 0.1, 0.8, 0.8));
    }
}
