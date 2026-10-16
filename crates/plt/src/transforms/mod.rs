//! Coordinate systems and transformations (matplotlib.transforms).
//!
//! matplotlib links transforms into a lazily-invalidated graph. plt-rs
//! instead describes *which* coordinate system something lives in with the
//! [`Transform`] expression type and resolves it against the current figure
//! state at draw time into a [`ResolvedTransform`]: a chain of non-affine
//! steps (scales, projections) followed by a single trailing affine map, the
//! same split matplotlib uses so renderers only ever see affine transforms.

mod nonsingular;
mod resolved;

use std::fmt;

pub use plt_core::{Affine2D, Bbox, Point};
pub use nonsingular::nonsingular;
pub use resolved::{AxisMap, PointMap, ResolvedTransform, Step};

use crate::artist::ArtistId;

/// A coordinate system, expressed relative to figure/axes geometry.
#[derive(Clone, Debug, PartialEq)]
pub enum Transform {
    /// Display pixels (origin bottom-left).
    Display,
    /// A fixed affine map into display pixels.
    Affine(Affine2D),
    /// Fractions of a figure or subfigure frame (`fig.transFigure`).
    Figure(ArtistId),
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
    /// Translation by `(dx, dy)` measured in the units of `units`
    /// (`ScaledTranslation`).
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
}

impl Transform {
    /// `x` in `x_coords`, `y` in `y_coords`.
    pub fn blended(x_coords: Self, y_coords: Self) -> Self {
        Self::Blended(Box::new(x_coords), Box::new(y_coords))
    }

    /// Applies `self`, then `next` (matplotlib's `self + next`).
    #[must_use]
    pub fn then(self, next: Self) -> Self {
        Self::Composite(Box::new(self), Box::new(next))
    }

    /// `self` followed by an offset of `(dx, dy)` points
    /// (matplotlib's `offset_copy(..., units='points')`).
    #[must_use]
    pub fn offset_points(self, dx: f64, dy: f64) -> Self {
        self.offset_inches(dx / 72.0, dy / 72.0)
    }

    /// `self` followed by an offset of `(dx, dy)` inches.
    #[must_use]
    pub fn offset_inches(self, dx: f64, dy: f64) -> Self {
        self.then(Self::Translation {
            dx,
            dy,
            units: Box::new(Self::Inches),
        })
    }

    /// The inverse transform.
    #[must_use]
    pub fn inverted(self) -> Self {
        match self {
            Self::Inverted(inner) => *inner,
            other => Self::Inverted(Box::new(other)),
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
            Self::Axes(axes) => {
                let bbox = ctx
                    .axes_bbox(*axes)
                    .ok_or(TransformError::MissingArtist(*axes))?;
                ResolvedTransform::affine(unit_to_bbox(&bbox))
            }
            Self::Data(axes) => ctx
                .data_transform(*axes)
                .ok_or(TransformError::MissingArtist(*axes))?,
            Self::Blended(x, y) => ResolvedTransform::blend(x.resolve(ctx)?, y.resolve(ctx)?),
            Self::Composite(first, second) => first.resolve(ctx)?.then(second.resolve(ctx)?),
            Self::Translation { dx, dy, units } => {
                let units = units.resolve(ctx)?;
                let origin = units.transform_point(Point::ORIGIN);
                let moved = units.transform_point(Point::new(*dx, *dy));
                let delta = moved - origin;
                ResolvedTransform::affine(Affine2D::translation(delta.x, delta.y))
            }
            Self::Inverted(inner) => inner
                .resolve(ctx)?
                .inverted()
                .ok_or(TransformError::Singular)?,
        })
    }
}

/// The affine map taking the unit square onto `bbox` (`BboxTransformTo`).
pub fn unit_to_bbox(bbox: &Bbox) -> Affine2D {
    Affine2D::scaling(bbox.width(), bbox.height()).translate(bbox.x0(), bbox.y0())
}

/// The affine map taking `bbox` onto the unit square (`BboxTransformFrom`).
/// Degenerate boxes map everything to the origin.
pub fn bbox_to_unit(bbox: &Bbox) -> Affine2D {
    let (w, h) = (bbox.width(), bbox.height());
    let sx = if w == 0.0 { 0.0 } else { 1.0 / w };
    let sy = if h == 0.0 { 0.0 } else { 1.0 / h };
    Affine2D::translation(-bbox.x0(), -bbox.y0()).scale(sx, sy)
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
}

/// Why a transform could not be resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformError {
    /// The transform refers to an artist that no longer exists.
    MissingArtist(ArtistId),
    /// The transform has no inverse.
    Singular,
}

impl fmt::Display for TransformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingArtist(id) => write!(f, "transform refers to removed artist {id}"),
            Self::Singular => f.write_str("transform is not invertible"),
        }
    }
}

impl std::error::Error for TransformError {}
