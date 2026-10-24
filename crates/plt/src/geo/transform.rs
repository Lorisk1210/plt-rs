//! The transform stack of geographic axes: the projection with path
//! subdivision and its affine onto the axes box (matplotlib's
//! `transProjection + transAffine`), the outline of the map, and the
//! coordinates gridlines are drawn in.

use std::f64::consts::{PI, TAU};
use std::rc::Rc;

use plt_core::{Affine2D, Path, Point};

use super::{GeoProjection, GeoState};
use crate::transforms::{PointMap, ResolvedTransform, Step};

/// Pieces every segment of a path is cut into before it is projected
/// (matplotlib's `GeoAxes.RESOLUTION`).
const RESOLUTION: usize = 75;

/// A map projection as a plane map that subdivides paths (matplotlib's
/// `_GeoTransform`).
#[derive(Debug)]
struct GeoMap(GeoProjection);

impl PointMap for GeoMap {
    fn forward(&self, point: Point) -> Point {
        self.0.forward(point)
    }

    fn inverse(&self, point: Point) -> Point {
        self.0.inverse(point)
    }

    /// matplotlib's `transform_path_non_affine`: every segment is cut into
    /// [`RESOLUTION`] pieces whatever the path's own interpolation hint.
    fn transform_path(&self, path: &Path) -> Path {
        let mut mapped = path.interpolated(RESOLUTION);
        for p in mapped.vertices_mut() {
            *p = self.forward(*p);
        }
        mapped
    }
}

/// The outline of the map in axes coordinates: the circle inscribed in
/// the unit square, an ellipse once stretched onto the axes box
/// (matplotlib's `_gen_axes_patch` and `_gen_axes_spines`).
pub(crate) fn outline() -> Path {
    Path::circle(Point::new(0.5, 0.5), 0.5)
}

/// From `(fraction of the width, latitude)` to data: latitude gridlines
/// span every longitude (matplotlib's `yaxis_stretch`).
pub(crate) fn latitude_span() -> Affine2D {
    Affine2D::scaling(TAU, 1.0).translate(-PI, 0.0)
}

impl GeoState {
    /// From data to the unit square of the axes (matplotlib's
    /// `transProjection + transAffine`).
    pub(crate) fn to_axes(&self) -> ResolvedTransform {
        ResolvedTransform::with_step(
            Step::Plane(Rc::new(GeoMap(self.projection))),
            self.projection.affine(),
        )
    }

    /// From `(longitude, fraction of the height)` to data: longitude
    /// gridlines stop at the longitude cap (matplotlib's
    /// `_xaxis_pretransform`).
    pub(crate) fn longitude_span(&self) -> Affine2D {
        let cap = self.longitude_cap;
        Affine2D::scaling(1.0, cap * 2.0).translate(0.0, -cap)
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::FRAC_PI_2;

    use super::*;

    #[test]
    fn the_map_fills_the_axes_box() {
        let geo = GeoState::new(GeoProjection::Hammer);
        let to_axes = geo.to_axes();
        let west = to_axes.transform_point(Point::new(-PI, 0.0));
        let north = to_axes.transform_point(Point::new(0.0, FRAC_PI_2));
        assert!(west.x.abs() < 1e-12 && (west.y - 0.5).abs() < 1e-12);
        assert!((north.x - 0.5).abs() < 1e-12 && (north.y - 1.0).abs() < 1e-12);
        let back = to_axes
            .inverted()
            .unwrap()
            .transform_point(Point::new(0.7, 0.6));
        let again = to_axes.transform_point(back);
        assert!((again.x - 0.7).abs() < 1e-9 && (again.y - 0.6).abs() < 1e-9);
    }

    #[test]
    fn paths_are_subdivided_before_projection() {
        let to_axes = GeoState::new(GeoProjection::Mollweide).to_axes();
        let segment = Path::polyline(vec![Point::new(-1.0, 0.5), Point::new(1.0, 0.5)]);
        let (mapped, _) = to_axes.split_path(&segment);
        assert_eq!(mapped.len(), RESOLUTION + 1);
        let (points, _) = to_axes.split_points(&segment);
        assert_eq!(points.len(), 2);
    }

    #[test]
    fn gridline_spans() {
        let geo = GeoState {
            longitude_cap: 1.0,
            ..GeoState::new(GeoProjection::Aitoff)
        };
        let span = geo.longitude_span();
        assert_eq!(
            span.transform_point(Point::new(0.3, 0.0)),
            Point::new(0.3, -1.0)
        );
        assert_eq!(
            span.transform_point(Point::new(0.3, 1.0)),
            Point::new(0.3, 1.0)
        );
        let stretch = latitude_span();
        assert_eq!(
            stretch.transform_point(Point::new(0.0, 0.2)),
            Point::new(-PI, 0.2)
        );
        assert_eq!(
            stretch.transform_point(Point::new(1.0, 0.2)),
            Point::new(PI, 0.2)
        );
        let bounds = outline().extents();
        assert!((bounds.x0()).abs() < 1e-3 && (bounds.y1() - 1.0).abs() < 1e-3);
    }
}
