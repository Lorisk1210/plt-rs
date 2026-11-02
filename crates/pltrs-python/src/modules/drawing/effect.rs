//! Path effects of classes of the program's (matplotlib's
//! `AbstractPathEffect` subclasses): while pltrs (Rust) draws a path of an
//! artist with such an effect, it calls the package's
//! `effect_draw_path(effect, target, gc, (path, matrix, fill))`, which
//! runs the effect's `draw_path` with the renderer of the draw recording
//! into a drawing, and draws that drawing in the path's place.

use pltrs::artist::{Drawing, Renderer};
use pltrs::patheffects::{CustomEffect, CustomPathEffect, Effect};
use pltrs_core::{Affine2D, Path, Rgba};
use pltrs_render::GraphicsContext;

use super::super::backends::PyGc;
use super::super::path::path_arrays;
use super::super::transforms::matrix;
use super::painter::{taken, target};
use crate::binding::prelude::*;
use crate::binding::{lookup, report};

/// A path effect of a class of the program's: the Python object.
#[derive(Debug)]
pub(in crate::modules) struct PythonEffect {
    effect: Owned,
}

impl PythonEffect {
    /// The effect of the Python object `effect`.
    pub(in crate::modules) fn of(effect: &Bound<'_>) -> Self {
        Self {
            effect: effect.clone().unbind(),
        }
    }

    /// The effect's Python object.
    pub(in crate::modules) fn object(&self) -> &Owned {
        &self.effect
    }

    /// Runs the effect's `draw_path` for `path` drawn onto `renderer`:
    /// the drawing it drew, and the graphics context `gc` as the effect
    /// left it; `None` when it raised (its exception raises from the
    /// native call drawing).
    fn record(
        &self,
        renderer: &dyn Renderer,
        gc: &GraphicsContext,
        path: &Path,
        affine: &Affine2D,
        fill: Option<Rgba>,
    ) -> Option<(Drawing, GraphicsContext)> {
        Python::with_gil(|py| {
            let call: Callback = lookup(py, "effect_draw_path").ok()?.extract().ok()?;
            let geometry = (
                path_arrays(path, false).ok()?,
                matrix(*affine).ok()?,
                fill.map(|c| (c.r, c.g, c.b, c.a)),
            );
            let (drawing, gc): (Native<Drawing>, PyGc) = call.call((
                self.effect.clone_ref(py),
                target(renderer, 0, true),
                PyGc(gc.clone()),
                geometry,
            ))?;
            Some((taken(&drawing), gc.0))
        })
    }
}

impl CustomPathEffect for PythonEffect {
    fn draw_path(
        &self,
        renderer: &mut dyn Renderer,
        gc: &mut GraphicsContext,
        path: &Path,
        affine: &Affine2D,
        fill: Option<Rgba>,
    ) {
        let Some((drawing, left)) = self.record(renderer, gc, path, affine, fill) else {
            return;
        };
        *gc = left;
        let problems = drawing.draw_onto(renderer);
        if !problems.is_empty() {
            report(pltrs::figure::DrawErrors(problems).into());
        }
    }
}

/// The effect of the Python object `effect`, of a class of the program's.
pub(in crate::modules) fn python_effect(effect: &Bound<'_>) -> Effect {
    Effect::Custom(CustomEffect::new(PythonEffect::of(effect)))
}
