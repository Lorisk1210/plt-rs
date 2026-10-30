//! Vector fields drawn as arrows and wind barbs (matplotlib.quiver).
//!
//! * [`VectorField`] — positions and vector components, with optional
//!   values mapped to colours (the positional `X, Y, U, V, C` of `quiver`
//!   and `barbs`); non-finite components are masked.
//! * [`QuiverOptions`] — how arrows are sized: [`QuiverUnits`] for the
//!   arrow width, the `scale` and its units, [`Angles`], the [`QuiverArrowShape`]
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
//! use pltrs::numeric::{Grid, linspace, meshgrid};
//! use pltrs::prelude::*;
//! use pltrs::quiver::{QuiverOptions, VectorField};
//!
//! let (xx, yy) = meshgrid(&linspace(-1.0, 1.0, 5), &linspace(-1.0, 1.0, 5));
//! let u = yy.map(|y| -y);
//! let field = VectorField::on_mesh(&xx, &yy, &u, &xx);
//! let ax = Figure::new().subplot();
//! let q = ax.quiver(field, QuiverOptions::default())?;
//! assert_eq!(q.len(), 25);
//! # Ok::<(), pltrs::axes::PlotError>(())
//! ```

mod arrows;
mod barbs;
mod field;
mod geometry;
mod key;

pub use arrows::{Pivot, QuiverArrowShape};
pub use barbs::{BarbIncrements, BarbPivot, BarbSizes, BarbsOptions};
pub use field::VectorField;
pub use geometry::{Angles, QuiverUnits};
pub use key::{KeyCoordinates, LabelPosition, QuiverKey, QuiverKeyOptions};

pub(crate) use barbs::{BarbFlip, barb_outlines};
pub(crate) use field::retained;
pub(crate) use geometry::{ArrowSizing, QuiverGeometry};
pub(crate) use key::{KeyVector, QuiverKeyArtist};

use crate::artist::{ArtistHandle, RawHandle, TypedHandle};
use crate::axes::PlotError;
use crate::collections::{Collection, CollectionArtist, CollectionProps};
use crate::colors::Color;
use crate::transforms::Transform;

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
    pub shape: QuiverArrowShape,
    /// Which part of the arrow sits on its position.
    pub pivot: Pivot,
    /// Face colour of the arrows unless colours are mapped (`pltrs.ink`,
    /// black, by default).
    pub color: Color,
    /// Further collection properties (edges, alpha, z-order, ...).
    pub props: CollectionProps,
    /// The coordinates of the positions (`None` = the axes' data
    /// coordinates; matplotlib's `transform`). Positions count for the
    /// data limits where the transform puts them when it passes through
    /// the data coordinates last, after a fixed affine map; `angles="xy"`
    /// still follows the data coordinates.
    pub transform: Option<Transform>,
}

impl Default for QuiverOptions {
    fn default() -> Self {
        Self {
            scale: None,
            scale_units: None,
            units: QuiverUnits::Width,
            angles: Angles::Uv,
            width: None,
            shape: QuiverArrowShape::default(),
            pivot: Pivot::Tail,
            color: crate::rc::params().pltrs_ink.clone(),
            props: CollectionProps::default(),
            transform: None,
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

impl TypedHandle for Quiver {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        let is_quiver = raw.is::<CollectionArtist>()
            && raw.with::<CollectionArtist, _>(|c| c.draw_time_state::<QuiverGeometry>().is_some());
        is_quiver.then(|| Self::new(Collection::from_raw(raw.clone())))
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

    /// Replaces the vector components, one pair per arrow or one value for
    /// all (matplotlib's `set_UVC`); arrows with a non-finite component are
    /// hidden. An automatic scale is worked out from the components the
    /// arrows are drawn with.
    ///
    /// ```
    /// use pltrs::numeric::{Grid, linspace, meshgrid};
    /// use pltrs::prelude::*;
    /// use pltrs::quiver::{QuiverOptions, VectorField};
    /// let (x, y) = meshgrid(&linspace(0.0, 1.0, 2), &linspace(0.0, 1.0, 2));
    /// let ones = Grid::from_fn(2, 2, |_, _| 1.0);
    /// let ax = Figure::new().subplot();
    /// let q = ax.quiver(VectorField::on_mesh(&x, &y, &ones, &ones), QuiverOptions::default())?;
    /// q.set_uvc(&[0.0], &[1.0])?;
    /// assert!(q.set_uvc(&[0.0, 1.0], &[1.0]).is_err());
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `u` or `v` has neither one value nor one per arrow.
    pub fn set_uvc(&self, u: &[f64], v: &[f64]) -> Result<&Self, PlotError> {
        let n = self.len();
        let scalar = u.len() == 1 && v.len() == 1;
        let components = (
            field::broadcast("U", u.to_vec(), n)?,
            field::broadcast("V", v.to_vec(), n)?,
        );
        self.collection
            .raw()
            .with_mut::<CollectionArtist, _>(|artist| {
                let replaced = artist
                    .draw_time_state::<QuiverGeometry>()
                    .map(|geometry| geometry.with_components(components, scalar));
                if let Some(geometry) = replaced {
                    artist.draw_time = Some(Box::new(geometry));
                }
            });
        Ok(self)
    }
}
