//! Maps from a coordinate system into the data coordinates of an axes
//! (matplotlib's `transform - ax.transData`), with which artists placed
//! through a transform extend the data limits.

use pltrs_core::{Affine2D, Bbox};

use super::{ResolvedTransform, Transform, TransformContext};
use crate::artist::ArtistId;

/// A context without a figure, for transforms that need no geometry.
pub(crate) struct NoFigure;

impl TransformContext for NoFigure {
    fn dpi(&self) -> f64 {
        // Never asked: only transforms without physical units are
        // resolved against this context.
        f64::NAN
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

impl Transform {
    /// Whether resolving the transform needs the figure: its frames, axes,
    /// resolution or data transforms.
    fn needs_figure(&self) -> bool {
        match self {
            Self::Display | Self::Affine(_) | Self::Custom(_) => false,
            Self::Figure(_)
            | Self::FigureBox(_)
            | Self::Inches
            | Self::Axes(_)
            | Self::Data(_)
            | Self::Translation { .. } => true,
            Self::Blended(a, b) | Self::Composite(a, b) => a.needs_figure() || b.needs_figure(),
            Self::Inverted(inner) | Self::AffineDelta(inner) => inner.needs_figure(),
        }
    }

    /// The transform as a map of the plane when it needs no figure:
    /// affine and program-defined maps and what is built from them.
    fn detached(&self) -> Option<ResolvedTransform> {
        if self.needs_figure() {
            return None;
        }
        self.resolve(&NoFigure).ok()
    }

    /// The map from these coordinates into the data coordinates of `axes`
    /// (matplotlib's `transform - ax.transData`), for a transform that
    /// passes through them last: what it applies before the data
    /// transform, when that needs no figure. A blend maps each direction
    /// by its own component and leaves a direction that does not end in
    /// the data coordinates as it is (its values do not count as data).
    /// `None` when no direction ends in the data coordinates, or when what
    /// comes before them needs the figure.
    pub(crate) fn to_data(&self, axes: ArtistId) -> Option<ResolvedTransform> {
        match self {
            Self::Data(id) => (*id == axes).then(ResolvedTransform::identity),
            Self::Composite(first, last) => {
                let rest = last.to_data(axes)?;
                Some(first.detached()?.then(rest))
            }
            Self::Blended(x, y) => match (x.to_data(axes), y.to_data(axes)) {
                (None, None) => None,
                (x, y) => Some(ResolvedTransform::blend(
                    x.unwrap_or_else(ResolvedTransform::identity),
                    y.unwrap_or_else(ResolvedTransform::identity),
                )),
            },
            _ => None,
        }
    }

    /// [`Transform::to_data`] when it is affine.
    pub(crate) fn affine_before_data(&self, axes: ArtistId) -> Option<Affine2D> {
        self.to_data(axes)
            .filter(ResolvedTransform::is_affine)
            .map(|to_data| to_data.affine_part())
    }
}

#[cfg(test)]
mod tests {
    use pltrs_core::Point;

    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::transforms::PointMap;

    /// A non-affine shift of the plane.
    #[derive(Debug)]
    struct Shift(f64);

    impl PointMap for Shift {
        fn forward(&self, p: Point) -> Point {
            Point::new(p.x + self.0, p.y + self.0)
        }
        fn inverse(&self, p: Point) -> Point {
            Point::new(p.x - self.0, p.y - self.0)
        }
    }

    fn scale(s: f64) -> Transform {
        Transform::Affine(Affine2D::scaling(s, s))
    }

    #[test]
    fn chains_map_through_everything_before_the_data_transform() {
        let ax = Figure::new().subplot();
        let chain = scale(2.0)
            .then(Transform::custom(Shift(10.0)))
            .then(ax.trans_data());
        let to_data = chain.to_data(ax.id()).expect("ends in the data");
        assert!(!to_data.is_affine());
        assert_eq!(
            to_data.transform_point(Point::new(1.0, 2.0)),
            Point::new(12.0, 14.0)
        );
        assert_eq!(chain.affine_before_data(ax.id()), None);
    }

    #[test]
    fn nested_affine_chains_fold_into_one_map() {
        let ax = Figure::new().subplot();
        let chain = scale(2.0).then(scale(3.0)).then(ax.trans_data());
        assert_eq!(
            chain.affine_before_data(ax.id()),
            Some(Affine2D::scaling(6.0, 6.0))
        );
    }

    #[test]
    fn blends_map_only_their_data_direction() {
        let ax = Figure::new().subplot();
        let blend = Transform::blended(ax.trans_axes(), scale(10.0).then(ax.trans_data()));
        let to_data = blend.to_data(ax.id()).expect("y ends in the data");
        assert_eq!(
            to_data.transform_point(Point::new(0.5, -5.0)),
            Point::new(0.5, -50.0)
        );
    }

    #[test]
    fn other_coordinates_have_no_map() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let other = fig.subplot();
        assert!(ax.trans_axes().to_data(ax.id()).is_none());
        assert!(other.trans_data().to_data(ax.id()).is_none());
        let through_axes = ax.trans_axes().then(ax.trans_data());
        assert!(through_axes.to_data(ax.id()).is_none(), "needs the figure");
    }
}
