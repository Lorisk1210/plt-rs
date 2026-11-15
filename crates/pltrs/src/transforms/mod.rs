//! Coordinate systems and transformations (matplotlib.transforms).
//!
//! matplotlib links transforms into a lazily-invalidated graph. pltrs
//! instead describes *which* coordinate system something lives in with the
//! [`Transform`] expression type and resolves it against the current figure
//! state at draw time into a [`ResolvedTransform`]: a chain of non-affine
//! steps (scales, projections) followed by a single trailing affine map, the
//! same split matplotlib uses so renderers only ever see affine transforms.
//!
//! Transform expressions answer matplotlib's sub-tree queries
//! ([`Transform::contains_branch`] and
//! [`Transform::contains_branch_separately`]) structurally.
//!
//! The concrete building blocks of matplotlib's transform tree live here
//! too, with matplotlib's exact arithmetic: box-to-box maps
//! ([`bbox_transform`] and friends), [`blended_affine`],
//! [`scaled_translation`] and the units of [`Transform::offset_copy`], the
//! mutable data-limits box [`TrackedBbox`], the box following another
//! with some edges locked [`LockableBbox`], and the scalar helpers
//! [`nonsingular`] and [`interval_contains`] (with its open and close
//! variants). `Affine2D` and `Bbox` themselves come from `pltrs_core`.

mod bbox_maps;
mod blend;
mod branch;
mod custom;
mod evaluations;
mod interval;
mod leaves;
mod lockable_bbox;
mod nonsingular;
mod offset;
pub(crate) mod program_calls;
mod resolved;
mod to_data;
pub(crate) use to_data::NoFigure;
mod tracked_bbox;

use std::fmt;

pub use bbox_maps::{bbox_to_unit, bbox_transform, bbox_transform_from, unit_to_bbox};
pub use blend::blended_affine;
pub use custom::CustomMap;
pub use evaluations::{DrawnTick, Evaluation, EvaluationHook, is_tame, is_tame_span};
pub use interval::{interval_contains, interval_contains_close, interval_contains_open};
pub use leaves::{AxesLeaves, FrameLeaves, ProjectionLeaves, TreeLeaves};
pub use lockable_bbox::LockableBbox;
pub use nonsingular::nonsingular;
pub use offset::{OffsetUnits, scaled_translation};
pub use pltrs_core::{Affine2D, Anchor, Bbox, Point};
pub use resolved::{AxisMap, PointMap, ResolvedTransform, Step};
pub use tracked_bbox::TrackedBbox;

use crate::artist::ArtistId;
use crate::ticker::AxisName;

/// A coordinate system, expressed relative to figure/axes geometry.
#[derive(Clone, Debug, PartialEq)]
pub enum Transform {
    /// Display pixels (origin bottom-left).
    Display,
    /// A fixed affine map into display pixels.
    Affine(Affine2D),
    /// Fractions of a figure or subfigure frame (`fig.transFigure`).
    Figure(ArtistId),
    /// Fractions of a root figure's own box, its size at its resolution
    /// from the canvas origin, which a print cut to a window (a tight
    /// box) leaves where it is (matplotlib's `BboxTransformTo(fig.bbox)`
    /// taken outside a print: the box `bbox_inches` makes through
    /// `dpi_scale_trans`, which `adjust_bbox` replaces rather than moves).
    FigureBox(ArtistId),
    /// Physical inches from the canvas origin (`fig.dpi_scale_trans`).
    Inches,
    /// Fractions of an axes (`ax.transAxes`).
    Axes(ArtistId),
    /// Data coordinates of an axes (`ax.transData`).
    Data(ArtistId),
    /// Takes x from the first transform and y from the second
    /// (`blended_transform_factory`).
    Blended(Box<Transform>, Box<Transform>),
    /// Applies the first transform, then the second (`a + b`).
    Composite(Box<Transform>, Box<Transform>),
    /// Translation by the image of the point `(dx, dy)` under `units`
    /// (`ScaledTranslation`; see [`scaled_translation`]).
    Translation {
        /// Horizontal displacement.
        dx: f64,
        /// Vertical displacement.
        dy: f64,
        /// Coordinate system the displacement is measured in.
        units: Box<Transform>,
    },
    /// The inverse of a transform.
    Inverted(Box<Transform>),
    /// The affine part of a transform without its translation, for
    /// displacements such as collection offsets (`AffineDeltaTransform`).
    AffineDelta(Box<Transform>),
    /// A non-affine map of the plane defined by the program (a matplotlib
    /// `Transform` subclass); see [`Transform::custom`].
    Custom(CustomMap),
}

pltrs_core::archive_enum!(Transform {
    Display,
    Affine(v0),
    Figure(v0),
    FigureBox(v0),
    Inches,
    Axes(v0),
    Data(v0),
    Blended(v0, v1),
    Composite(v0, v1),
    Translation { dx, dy, units },
    Inverted(v0),
    AffineDelta(v0),
    Custom(v0),
});

impl Transform {
    /// `x` in `x_coords`, `y` in `y_coords`.
    pub fn blended(x_coords: Self, y_coords: Self) -> Self {
        Self::Blended(Box::new(x_coords), Box::new(y_coords))
    }

    /// The non-affine map `map` (matplotlib's own `Transform` subclasses),
    /// typically followed by a standard coordinate system:
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::transforms::{Point, PointMap, Transform};
    /// #[derive(Debug)]
    /// struct Swap;
    /// impl PointMap for Swap {
    ///     fn forward(&self, p: Point) -> Point { Point::new(p.y, p.x) }
    ///     fn inverse(&self, p: Point) -> Point { Point::new(p.y, p.x) }
    /// }
    /// let fig = Figure::new();
    /// let swapped = fig.resolve_transform(&Transform::custom(Swap))?;
    /// assert!(!swapped.is_affine());
    /// assert_eq!(swapped.transform_point(Point::new(1.0, 2.0)), Point::new(2.0, 1.0));
    /// # Ok::<(), pltrs::transforms::TransformError>(())
    /// ```
    pub fn custom(map: impl PointMap + 'static) -> Self {
        Self::Custom(CustomMap::new(map))
    }

    /// Applies `self`, then `next` (matplotlib's `self + next`).
    #[must_use]
    pub fn then(self, next: Self) -> Self {
        Self::Composite(Box::new(self), Box::new(next))
    }

    /// `self` followed by an offset of `(dx, dy)` in `units` (matplotlib's
    /// `offset_copy`): dots translate directly, points and inches become a
    /// [`Transform::Translation`] in inches, scaled by the figure's dpi.
    #[must_use]
    pub fn offset_copy(self, dx: f64, dy: f64, units: OffsetUnits) -> Self {
        let (dx, dy) = match units {
            OffsetUnits::Dots => return self.then(Self::Affine(Affine2D::translation(dx, dy))),
            OffsetUnits::Points => (dx / 72.0, dy / 72.0),
            OffsetUnits::Inches => (dx, dy),
        };
        self.then(Self::Translation {
            dx,
            dy,
            units: Box::new(Self::Inches),
        })
    }

    /// `self` followed by an offset of `(dx, dy)` points.
    #[must_use]
    pub fn offset_points(self, dx: f64, dy: f64) -> Self {
        self.offset_copy(dx, dy, OffsetUnits::Points)
    }

    /// `self` followed by an offset of `(dx, dy)` inches.
    #[must_use]
    pub fn offset_inches(self, dx: f64, dy: f64) -> Self {
        self.offset_copy(dx, dy, OffsetUnits::Inches)
    }

    /// The inverse transform.
    #[must_use]
    pub fn inverted(self) -> Self {
        match self {
            Self::Inverted(inner) => *inner,
            other => Self::Inverted(Box::new(other)),
        }
    }

    /// The axes and its data axis when `x` and `y` blend one axes' data
    /// coordinates along one axis with its axes fractions along the other
    /// (the axis transforms [`Axes::trans_xaxis`] and
    /// [`Axes::trans_yaxis`] build).
    ///
    /// [`Axes::trans_xaxis`]: crate::axes::Axes::trans_xaxis
    /// [`Axes::trans_yaxis`]: crate::axes::Axes::trans_yaxis
    fn axis_blend(x: &Self, y: &Self) -> Option<(ArtistId, AxisName)> {
        match (x, y) {
            (Self::Data(a), Self::Axes(b)) if a == b => Some((*a, AxisName::X)),
            (Self::Axes(a), Self::Data(b)) if a == b => Some((*a, AxisName::Y)),
            _ => None,
        }
    }

    /// Resolves the expression against the current figure geometry.
    pub fn resolve(&self, ctx: &dyn TransformContext) -> Result<ResolvedTransform, TransformError> {
        Ok(match self {
            Self::Display => ResolvedTransform::identity(),
            Self::Affine(affine) => ResolvedTransform::affine(*affine),
            Self::Inches => {
                let dpi = ctx.dpi();
                ResolvedTransform::affine(Affine2D::scaling(dpi, dpi))
            }
            Self::Figure(frame) => {
                let bbox = ctx
                    .frame_bbox(*frame)
                    .ok_or(TransformError::MissingArtist(*frame))?;
                ResolvedTransform::affine(unit_to_bbox(&bbox))
            }
            Self::FigureBox(frame) => {
                let bbox = ctx
                    .frame_bbox(*frame)
                    .ok_or(TransformError::MissingArtist(*frame))?;
                let own = Bbox::from_bounds(0.0, 0.0, bbox.width(), bbox.height());
                ResolvedTransform::affine(unit_to_bbox(&own))
            }
            Self::Axes(axes) => {
                let bbox = ctx
                    .axes_bbox(*axes)
                    .ok_or(TransformError::MissingArtist(*axes))?;
                ResolvedTransform::affine(unit_to_bbox(&bbox))
            }
            Self::Data(axes) => ctx
                .data_transform(*axes)
                .ok_or(TransformError::MissingArtist(*axes))?,
            Self::Blended(x, y) => match Self::axis_blend(x, y)
                .and_then(|(axes, data_axis)| ctx.axis_transform(axes, data_axis))
            {
                Some(axis) => axis,
                None => ResolvedTransform::blend(x.resolve(ctx)?, y.resolve(ctx)?),
            },
            Self::Composite(first, second) => first.resolve(ctx)?.then(second.resolve(ctx)?),
            Self::Translation { dx, dy, units } => {
                ResolvedTransform::affine(scaled_translation(*dx, *dy, &units.resolve(ctx)?))
            }
            Self::Inverted(inner) => inner
                .resolve(ctx)?
                .inverted()
                .ok_or(TransformError::Singular)?,
            Self::AffineDelta(inner) => {
                let [a, b, c, d, _, _] = inner.resolve(ctx)?.affine_part().values();
                ResolvedTransform::affine(Affine2D::from_values(a, b, c, d, 0.0, 0.0))
            }
            Self::Custom(map) => {
                ResolvedTransform::with_step(Step::Plane(map.map()), Affine2D::IDENTITY)
            }
        })
    }
}

/// The geometry a [`Transform`] is resolved against.
pub trait TransformContext {
    /// Resolution of the canvas in dots per inch.
    fn dpi(&self) -> f64;
    /// Display-space extent of a figure or subfigure frame.
    fn frame_bbox(&self, frame: ArtistId) -> Option<Bbox>;
    /// Display-space extent of an axes.
    fn axes_bbox(&self, axes: ArtistId) -> Option<Bbox>;
    /// Complete data-to-display transform of an axes.
    fn data_transform(&self, axes: ArtistId) -> Option<ResolvedTransform>;
    /// The transform of an axes that is data along `data_axis` and axes
    /// fractions across it, when the axes' projection defines its own
    /// (matplotlib's `get_xaxis_transform` and `get_yaxis_transform`);
    /// `None` leaves it to the plain blend of the two coordinate systems.
    fn axis_transform(&self, _axes: ArtistId, _data_axis: AxisName) -> Option<ResolvedTransform> {
        None
    }
}

/// Why a transform could not be resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformError {
    /// The transform refers to an artist that no longer exists.
    MissingArtist(ArtistId),
    /// The transform has no inverse.
    Singular,
    /// An affine blend was given a child with rotation or shear.
    NotSeparable,
}

impl fmt::Display for TransformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingArtist(id) => write!(f, "transform refers to removed artist {id}"),
            Self::Singular => f.write_str("transform is not invertible"),
            Self::NotSeparable => f.write_str("both children of an affine blend must be separable"),
        }
    }
}

impl std::error::Error for TransformError {}

impl crate::figure::Figure {
    /// `transform` as it maps points in the figure's current layout
    /// (matplotlib's `transform.transform(...)` on a live transform such as
    /// `ax.transData`); the figure is laid out first.
    ///
    /// # Errors
    ///
    /// When the transform refers to a removed artist.
    pub fn resolve_transform(
        &self,
        transform: &Transform,
    ) -> Result<ResolvedTransform, TransformError> {
        let state = self.state_cell().prepared();
        transform.resolve(&*state)
    }

    /// `transform` as it maps points in the figure's state of now, without
    /// laying the figure out first.
    ///
    /// # Errors
    ///
    /// When the transform refers to a removed artist.
    pub fn resolve_transform_as_is(
        &self,
        transform: &Transform,
    ) -> Result<ResolvedTransform, TransformError> {
        transform.resolve(&*self.state())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A figure without artists: only its resolution is known.
    struct Dpi(f64);

    impl TransformContext for Dpi {
        fn dpi(&self) -> f64 {
            self.0
        }
        fn frame_bbox(&self, _: ArtistId) -> Option<Bbox> {
            None
        }
        fn axes_bbox(&self, _: ArtistId) -> Option<Bbox> {
            None
        }
        fn data_transform(&self, _: ArtistId) -> Option<ResolvedTransform> {
            None
        }
    }

    fn resolve_affine(t: &Transform, dpi: f64) -> Affine2D {
        t.resolve(&Dpi(dpi)).expect("resolvable").affine_part()
    }

    #[test]
    fn offset_copy_units() {
        let base = Affine2D::scaling(2.0, 3.0);
        let offset = |units| Transform::Affine(base).offset_copy(36.0, 72.0, units);
        assert_eq!(
            resolve_affine(&offset(OffsetUnits::Dots), 100.0),
            base.translate(36.0, 72.0)
        );
        assert_eq!(
            resolve_affine(&offset(OffsetUnits::Points), 100.0),
            base.translate(50.0, 100.0)
        );
        assert_eq!(
            resolve_affine(&offset(OffsetUnits::Inches), 100.0),
            base.translate(3600.0, 7200.0)
        );
        assert_eq!(
            Transform::Display.offset_points(36.0, 0.0),
            Transform::Display.offset_inches(0.5, 0.0)
        );
    }

    #[test]
    fn affine_deltas_drop_the_translation() {
        let base = Affine2D::scaling(2.0, 3.0).translate(5.0, 7.0);
        let delta = Transform::AffineDelta(Box::new(Transform::Affine(base)));
        assert_eq!(resolve_affine(&delta, 72.0), Affine2D::scaling(2.0, 3.0));
    }

    #[test]
    fn scaled_translations_map_the_offset_as_a_point() {
        let units = Affine2D::scaling(100.0, 100.0).translate(5.0, 7.0);
        let translation = Transform::Translation {
            dx: 1.0,
            dy: 1.0,
            units: Box::new(Transform::Affine(units)),
        };
        assert_eq!(
            resolve_affine(&translation, 72.0),
            Affine2D::translation(105.0, 107.0)
        );
    }
}
