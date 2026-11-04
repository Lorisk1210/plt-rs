//! What the draws of a quiver plot leave on it: matplotlib's `scale`,
//! `width` and `span` attributes, and the arrows of the last draw
//! (`get_paths()`).

use std::cell::{Cell, RefCell};

use pltrs_core::Point;

use super::{AxesFrame, QuiverGeometry, Resolved};

/// A size of quiver arrows a program can set after the plot is made
/// (matplotlib's `scale` and `width` attributes).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArrowSize {
    /// Arrow-width units per unit of arrow length.
    Scale,
    /// The shaft width, in arrow-width units.
    Width,
}

/// What the draws of a quiver plot leave on it (the attributes
/// matplotlib's `Quiver._init` and `_make_verts` set): the shaft width
/// and the arrow scale the first draw worked out where the program gave
/// none, which later draws keep, and the span of the axes and the arrow
/// outlines of the last draw.
#[derive(Clone, Debug, Default)]
pub(crate) struct DrawnArrows {
    width: Cell<Option<f64>>,
    scale: Cell<Option<f64>>,
    span: Cell<Option<f64>>,
    outlines: RefCell<Vec<Vec<Point>>>,
}

pltrs_core::archive_struct!(DrawnArrows {
    width,
    scale,
    span,
    outlines
});

/// The sizes of a quiver plot's arrows as matplotlib's attributes hold
/// them: each as the program gave it, or as the first draw worked it out
/// (`None` before), and the span of the axes at the last draw.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ArrowSizes {
    /// Arrow-width units per unit of arrow length (`scale`).
    pub scale: Option<f64>,
    /// The shaft width, in arrow-width units (`width`).
    pub width: Option<f64>,
    /// The width of the axes in arrow-width units (`span`).
    pub span: Option<f64>,
}

impl QuiverGeometry {
    /// The same arrows with another scale or shaft width (`None` works it
    /// out at the next draw), as setting matplotlib's `scale` or `width`
    /// attribute makes them.
    pub(crate) fn with_size(&self, size: ArrowSize, value: Option<f64>) -> Self {
        let mut geometry = self.clone();
        match size {
            ArrowSize::Scale => {
                geometry.sizing.scale = value;
                geometry.drawn.scale.set(None);
            }
            ArrowSize::Width => {
                geometry.sizing.width = value;
                geometry.drawn.width.set(None);
            }
        }
        geometry
    }

    /// The sizes as matplotlib's attributes hold them.
    pub(crate) fn sizes(&self) -> ArrowSizes {
        ArrowSizes {
            scale: self.sizing.scale.or(self.drawn.scale.get()),
            width: self.sizing.width.or(self.drawn.width.get()),
            span: self.drawn.span.get(),
        }
    }

    /// The arrow outlines of the last draw, in arrow-width units (NaN for
    /// masked vectors, as matplotlib's masked vertices become); none
    /// before the first.
    pub(crate) fn drawn_outlines(&self) -> Vec<Vec<Point>> {
        self.drawn.outlines.borrow().clone()
    }

    /// Records the sizes of a draw in the geometry `frame` (matplotlib's
    /// `_init`, which the plot's draw and its keys' run): the width and
    /// scale stay for later draws.
    pub(crate) fn record_sizes(&self, frame: &AxesFrame) -> Resolved {
        let resolved = self.resolve(frame);
        let drawn = &self.drawn;
        drawn.span.set(Some(resolved.span));
        if self.sizing.width.is_none() {
            drawn.width.set(Some(resolved.width));
        }
        if self.sizing.scale.is_none() {
            drawn.scale.set(Some(resolved.scale));
        }
        resolved
    }

    /// Keeps `outlines`, the arrows of a draw (NaN points for masked
    /// vectors, as matplotlib's masked vertices become).
    pub(super) fn record_outlines(&self, outlines: &[Option<[Point; 8]>]) {
        *self.drawn.outlines.borrow_mut() = outlines
            .iter()
            .map(|outline| {
                outline.map_or_else(|| vec![Point::new(f64::NAN, f64::NAN); 8], Vec::from)
            })
            .collect();
    }
}
