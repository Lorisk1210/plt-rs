//! Transforms Python subclasses compute when they are drawn: matplotlib
//! reads an artist's `get_transform()` in its `draw`, so an override of it
//! (a transform that follows the axes, say) is evaluated at every draw.
//! The package gives the figure a draw hook (`set_draw_hook`) that sets the
//! native transform of each such artist from its `get_transform()` once
//! the figure is laid out (`_computed_transforms`).

use std::ops::ControlFlow;

use pltrs::figure::{DrawHook, Figure};

use crate::binding::prelude::*;

/// Calls `hook` (a Python callable taking nothing) whenever `figure` is
/// drawn, once it is laid out and before its artists are drawn; `None`
/// removes the hook. What the hook raises is raised by the native call
/// drawing the figure, and stops the draw there, as an exception in
/// matplotlib's draw does.
fn set_draw_hook(figure: Figure, hook: Option<Callback>) -> PyResult<()> {
    figure.set_draw_hook(hook.map(|hook| {
        DrawHook::stopping(move |_| match hook.call::<Owned>(()) {
            Some(_) => ControlFlow::Continue(()),
            None => ControlFlow::Break(()),
        })
    }));
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_draw_hook", set_draw_hook)
}
