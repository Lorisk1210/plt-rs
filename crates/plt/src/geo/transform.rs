//! The transform stack of geographic axes: the projection with path
//! subdivision, its affine onto the axes box (matplotlib's
//! `transProjection + transAffine + transAxes`), the outline of the map,
//! and the coordinates gridlines are drawn in.

use std::f64::consts::{PI, TAU};
use std::rc::Rc;

use plt_core::{Affine2D, Bbox, Path, Point};

use super::{GeoProjection, GeoState};
use crate::spines::SpineSide;
use crate::transforms::{PointMap, ResolvedTransform, Step, unit_to_bbox};

/// Pieces every segment of a path is cut into before it is projected
/// (matplotlib's `GeoAxes.RESOLUTION`).
const RESOLUTION: usize = 75;

/// The spine drawing the outline of geographic axes (matplotlib's
/// `spines['geo']`); the other three spines are not drawn.
pub(crate) const OUTLINE_SPINE: SpineSide = SpineSide::Top;

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
    /// The complete data → display transform for axes placed at
    /// `display` (matplotlib's `transData`).
    pub(crate) fn data_transform(&self, display: &Bbox) -> ResolvedTransform {
        ResolvedTransform::with_step(
            Step::Plane(Rc::new(GeoMap(self.projection))),
            self.projected_to_display(display),
        )
    }

    /// From projected coordinates to display (matplotlib's `transAffine +
    /// transAxes`).
    pub(crate) fn projected_to_display(&self, display: &Bbox) -> Affine2D {
        self.projection.affine().then(unit_to_bbox(display))
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
    fn data_transform_fills_the_axes_box() {
        let geo = GeoState::new(GeoProjection::Hammer);
        let display = Bbox::from_extents(10.0, 20.0, 410.0, 220.0);
        let data = geo.data_transform(&display);
        let west = data.transform_point(Point::new(-PI, 0.0));
        let north = data.transform_point(Point::new(0.0, FRAC_PI_2));
        assert!((west.x - 10.0).abs() < 1e-9 && (west.y - 120.0).abs() < 1e-9);
        assert!((north.x - 210.0).abs() < 1e-9 && (north.y - 220.0).abs() < 1e-9);
        let back = data
            .inverted()
            .unwrap()
            .transform_point(Point::new(300.0, 150.0));
        let again = data.transform_point(back);
        assert!((again.x - 300.0).abs() < 1e-6 && (again.y - 150.0).abs() < 1e-6);
    }

    #[test]
    fn paths_are_subdivided_before_projection() {
        let geo = GeoState::new(GeoProjection::Mollweide);
        let data = geo.data_transform(&Bbox::UNIT);
        let segment = Path::polyline(vec![Point::new(-1.0, 0.5), Point::new(1.0, 0.5)]);
        let (mapped, _) = data.split_path(&segment);
        assert_eq!(mapped.len(), RESOLUTION + 1);
        let (points, _) = data.split_points(&segment);
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
