//! Transforms Python subclasses compute when they are drawn: matplotlib
//! reads an artist's `get_transform()` in its `draw`, so an override of it
//! (a transform that follows the axes, say) is evaluated at every draw.
//! The package gives the figure a draw hook (`set_draw_hook`) that sets the
//! native transform of each such artist from its `get_transform()` once
//! the figure is laid out (`_computed_transforms`).

use pltrs::figure::{DrawHook, Figure};

use crate::binding::prelude::*;

/// Calls `hook` (a Python callable taking nothing) whenever `figure` is
/// drawn, once it is laid out and before its artists are drawn; `None`
/// removes the hook. What the hook raises is raised by the native call
/// drawing the figure.
fn set_draw_hook(figure: Figure, hook: Option<Callback>) -> PyResult<()> {
    figure.set_draw_hook(hook.map(|hook| {
        DrawHook::new(move |_| {
            let _ = hook.call::<Owned>(());
        })
    }));
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_draw_hook", set_draw_hook)
}
