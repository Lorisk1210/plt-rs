//! `figures/cases_patches.py`, split by section.

mod collections;
mod fancy;
mod hatching;
mod shapes;
mod styles;

pub(crate) use collections::CASES as COLLECTIONS;
pub(crate) use fancy::CASES as FANCY;
pub(crate) use hatching::CASES as HATCHING;
pub(crate) use shapes::CASES as SHAPES;
pub(crate) use styles::CASES as STYLES;

use plt::prelude::*;
use plt::rc::Aspect;

/// `_canvas(fig, xlim, ylim, equal)`: one axes with fixed limits,
/// optionally with equal data aspect.
fn canvas(fig: &Figure, xlim: (f64, f64), ylim: (f64, f64), equal: bool) -> Axes {
    let ax = fig.subplot();
    ax.set_xlim(xlim.0, xlim.1).set_ylim(ylim.0, ylim.1);
    if equal {
        ax.set_aspect(Aspect::Equal)
            .expect("an equal aspect is always valid");
    }
    ax
}

/// `_canvas(fig)`: limits `(0, 10)` on both axes, equal aspect.
fn square_canvas(fig: &Figure) -> Axes {
    canvas(fig, (0.0, 10.0), (0.0, 10.0), true)
}
