//! Vector fields drawn as arrows and wind barbs (matplotlib.quiver).
//!
//! * [`VectorField`] — positions and vector components, with optional
//!   values mapped to colours (the positional `X, Y, U, V, C` of `quiver`
//!   and `barbs`); non-finite components are masked.
//! * [`QuiverOptions`] — how arrows are sized: [`QuiverUnits`] for the
//!   arrow width, the `scale` and its units, [`Angles`], the [`ArrowShape`]
//!   and the [`Pivot`]. The arrows are polygons in arrow-width units that
//!   are rebuilt from the axes geometry whenever the figure is drawn, as
//!   in matplotlib's `Quiver.draw`.
//! * [`Quiver`] — what `Axes::quiver` returns: a collection handle plus
//!   the field's size; [`QuiverKey`] — a labelled reference arrow
//!   (`Axes::quiverkey`).
//! * [`BarbsOptions`] — wind barbs: [`BarbIncrements`] decide how many
//!   flags, full and half barbs a magnitude gets, [`BarbSizes`] their
//!   proportions (`Axes::barbs`).
//!
//! ```
//! use plt::numeric::{Grid, linspace, meshgrid};
//! use plt::prelude::*;
//! use plt::quiver::{QuiverOptions, VectorField};
//!
//! let (xx, yy) = meshgrid(&linspace(-1.0, 1.0, 5), &linspace(-1.0, 1.0, 5));
//! let u = yy.map(|y| -y);
//! let field = VectorField::on_mesh(&xx, &yy, &u, &xx);
//! let ax = Figure::new().subplot();
//! let q = ax.quiver(field, QuiverOptions::default())?;
//! assert_eq!(q.len(), 25);
//! # Ok::<(), plt::axes::PlotError>(())
//! ```

mod arrows;
mod barbs;
mod field;
mod geometry;
mod key;

pub use arrows::{ArrowShape, Pivot};
pub use barbs::{BarbIncrements, BarbPivot, BarbSizes, BarbsOptions};
pub use field::VectorField;
pub use geometry::{Angles, QuiverUnits};
pub use key::{KeyCoordinates, LabelPosition, QuiverKey, QuiverKeyOptions};

pub(crate) use barbs::{BarbFlip, barb_outlines};
pub(crate) use field::retained;
pub(crate) use geometry::{ArrowSizing, QuiverGeometry};
pub(crate) use key::{KeyVector, QuiverKeyArtist};

use crate::artist::{ArtistHandle, RawHandle};
use crate::collections::{Collection, CollectionProps};
use crate::colors::Color;

/// Options of `Axes::quiver` (matplotlib's `quiver` keywords).
#[derive(Clone, Debug, PartialEq)]
pub struct QuiverOptions {
    /// Data units per arrow length unit: larger values give shorter
    /// arrows (`None` = autoscaled from the mean vector length).
    pub scale: Option<f64>,
    /// Units of the arrow length relative to `scale` (`None` = the
    /// arrow-width `units`).
    pub scale_units: Option<QuiverUnits>,
    /// Units of the arrow dimensions except the length.
    pub units: QuiverUnits,
    /// How the arrow directions are computed.
    pub angles: Angles,
    /// Shaft width in `units` (`None` = from the axes size and the number
    /// of arrows).
    pub width: Option<f64>,
    /// Head proportions and the handling of short arrows.
    pub shape: ArrowShape,
    /// Which part of the arrow sits on its position.
    pub pivot: Pivot,
    /// Face colour of the arrows unless colours are mapped.
    pub color: Color,
    /// Further collection properties (edges, alpha, z-order, ...).
    pub props: CollectionProps,
}

impl Default for QuiverOptions {
    fn default() -> Self {
        Self {
            scale: None,
            scale_units: None,
            units: QuiverUnits::Width,
            angles: Angles::Uv,
            width: None,
            shape: ArrowShape::default(),
            pivot: Pivot::Tail,
            color: Color::from("k"),
            props: CollectionProps::default(),
        }
    }
}

/// A handle to a quiver plot (matplotlib's `Quiver`): a collection of
/// arrow polygons, one per position.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Quiver {
    collection: Collection,
}

impl ArtistHandle for Quiver {
    fn raw(&self) -> &RawHandle {
        self.collection.raw()
    }
}

impl Quiver {
    pub(crate) fn new(collection: Collection) -> Self {
        Self { collection }
    }

    /// The arrows as a collection, for styling.
    pub fn collection(&self) -> &Collection {
        &self.collection
    }

    /// The number of arrow positions (matplotlib's `N`).
    pub fn len(&self) -> usize {
        self.collection.offsets().len()
    }

    /// Whether the plot has no arrows.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
