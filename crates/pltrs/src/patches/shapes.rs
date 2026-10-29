//! Patch geometries: each shape supplies a path and the transform placing
//! it, exactly as the corresponding matplotlib patch class.

use pltrs_core::{Affine2D, Bbox, Path, PathCode, Point};

use super::PatchShape;

/// Where a [`Rectangle`] rotates around.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum RotationPoint {
    /// The anchor corner `xy`.
    #[default]
    Xy,
    /// The centre of the rectangle.
    Center,
    /// An arbitrary point.
    At(Point),
}

/// An axis-aligned (optionally rotated) rectangle anchored at `xy`.
#[derive(Clone, Debug, PartialEq)]
pub struct Rectangle {
    /// Anchor corner.
    pub xy: Point,
    /// Extent along x (may be negative).
    pub width: f64,
    /// Extent along y (may be negative).
    pub height: f64,
    /// Counter-clockwise rotation in degrees.
    pub angle: f64,
    /// Centre of the rotation.
    pub rotation_point: RotationPoint,
    /// Height-to-width ratio of a display unit square in data units, so
    /// that rotations look right on axes with unequal scales (matplotlib's
    /// `_aspect_ratio_correction`, set by the rectangle selector).
    pub aspect_ratio_correction: f64,
}

impl Rectangle {
    /// A rectangle from `xy` spanning `width × height`.
    pub fn new(xy: impl Into<Point>, width: f64, height: f64) -> Self {
        Self {
            xy: xy.into(),
            width,
            height,
            angle: 0.0,
            rotation_point: RotationPoint::Xy,
            aspect_ratio_correction: 1.0,
        }
    }

    /// The same rectangle rotated by `angle` degrees about `point`.
    #[must_use]
    pub fn rotated(self, angle: f64, point: RotationPoint) -> Self {
        Self {
            angle,
            rotation_point: point,
            ..self
        }
    }

    /// The unrotated extent.
    pub fn bbox(&self) -> Bbox {
        Bbox::from_bounds(self.xy.x, self.xy.y, self.width, self.height)
    }

    /// The corners, rotated: `xy` first, then counter-clockwise for a
    /// positive width and height (matplotlib's `get_corners`).
    pub fn corners(&self) -> [Point; 4] {
        placed(self, [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)])
    }

    /// The centre, rotated (matplotlib's `get_center`).
    pub fn center(&self) -> Point {
        let [center] = placed(self, [(0.5, 0.5)]);
        center
    }
}

/// `points` of a shape's own coordinates, mapped by its patch transform.
fn placed<const N: usize>(shape: &impl PatchShape, points: [(f64, f64); N]) -> [Point; N] {
    let transform = shape.patch_transform();
    points.map(|p| transform.transform_point(p.into()))
}

impl PatchShape for Rectangle {
    fn path(&self) -> Path {
        Path::unit_rectangle()
    }

    fn clips_to_bounding_box(&self) -> bool {
        true
    }

    fn patch_transform(&self) -> Affine2D {
        let bbox = self.bbox();
        let pivot = match self.rotation_point {
            RotationPoint::Xy => bbox.p0,
            RotationPoint::Center => Point::new(
                bbox.x0() + (bbox.x1() - bbox.x0()) / 2.0,
                bbox.y0() + (bbox.y1() - bbox.y0()) / 2.0,
            ),
            RotationPoint::At(p) => p,
        };
        let correction = self.aspect_ratio_correction;
        crate::transforms::unit_to_bbox(&bbox).then(
            Affine2D::translation(-pivot.x, -pivot.y)
                .scale(1.0, correction)
                .rotate_deg(self.angle)
                .scale(1.0, 1.0 / correction)
                .translate(pivot.x, pivot.y),
        )
    }

    fn skips_autoscale(&self) -> bool {
        self.width == 0.0 && self.height == 0.0
    }
}

/// An ellipse with full axis lengths `width` and `height`.
#[derive(Clone, Debug, PartialEq)]
pub struct Ellipse {
    /// Centre.
    pub center: Point,
    /// Full horizontal diameter (before rotation).
    pub width: f64,
    /// Full vertical diameter (before rotation).
    pub height: f64,
    /// Counter-clockwise rotation in degrees.
    pub angle: f64,
    /// Height-to-width ratio of a display unit square in data units (see
    /// [`Rectangle::aspect_ratio_correction`]).
    pub aspect_ratio_correction: f64,
}

impl Ellipse {
    /// An ellipse centred on `center`.
    pub fn new(center: impl Into<Point>, width: f64, height: f64) -> Self {
        Self {
            center: center.into(),
            width,
            height,
            angle: 0.0,
            aspect_ratio_correction: 1.0,
        }
    }

    /// The corners of the rotated box the ellipse is inscribed in,
    /// counter-clockwise from the one at `(-width, -height) / 2`
    /// (matplotlib's `get_corners`).
    pub fn corners(&self) -> [Point; 4] {
        placed(self, [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)])
    }

    /// The ends of the major axis (matplotlib's `get_vertices`; the
    /// horizontal axis when the ellipse is round).
    pub fn vertices(&self) -> [Point; 2] {
        if self.width < self.height {
            placed(self, [(0.0, 1.0), (0.0, -1.0)])
        } else {
            placed(self, [(1.0, 0.0), (-1.0, 0.0)])
        }
    }

    /// The ends of the minor axis (matplotlib's `get_co_vertices`).
    pub fn co_vertices(&self) -> [Point; 2] {
        if self.width < self.height {
            placed(self, [(1.0, 0.0), (-1.0, 0.0)])
        } else {
            placed(self, [(0.0, 1.0), (0.0, -1.0)])
        }
    }
}

impl PatchShape for Ellipse {
    fn path(&self) -> Path {
        Path::unit_circle()
    }

    fn patch_transform(&self) -> Affine2D {
        let correction = self.aspect_ratio_correction;
        Affine2D::identity()
            .scale(self.width * 0.5, self.height * 0.5 * correction)
            .rotate_deg(self.angle)
            .scale(1.0, 1.0 / correction)
            .translate(self.center.x, self.center.y)
    }
}

/// A circle (an [`Ellipse`] with equal axes).
#[derive(Clone, Debug, PartialEq)]
pub struct Circle {
    /// Centre.
    pub center: Point,
    /// Radius.
    pub radius: f64,
}

impl Circle {
    /// A circle of `radius` around `center`.
    pub fn new(center: impl Into<Point>, radius: f64) -> Self {
        Self {
            center: center.into(),
            radius,
        }
    }
}

impl PatchShape for Circle {
    fn path(&self) -> Path {
        Path::unit_circle()
    }

    fn patch_transform(&self) -> Affine2D {
        Ellipse::new(self.center, self.radius * 2.0, self.radius * 2.0).patch_transform()
    }
}

/// A polygon through the given vertices, open or closed.
#[derive(Clone, Debug, PartialEq)]
pub struct Polygon {
    vertices: Vec<Point>,
    closed: bool,
}

impl Polygon {
    /// A polygon through `vertices`. Closed polygons gain a closing vertex
    /// unless the last vertex already repeats the first; open polygons drop
    /// such a repeated vertex (matplotlib's `Polygon.set_xy`).
    pub fn new<I: IntoIterator<Item = Point>>(vertices: I, closed: bool) -> Self {
        let mut vertices: Vec<Point> = vertices.into_iter().collect();
        let n = vertices.len();
        if closed {
            if n == 1 || (n > 1 && vertices[0] != vertices[n - 1]) {
                vertices.push(vertices[0]);
            }
        } else if n > 2 && vertices[0] == vertices[n - 1] {
            vertices.pop();
        }
        Self { vertices, closed }
    }

    /// The stored vertices (including the closing vertex of closed polygons).
    pub fn vertices(&self) -> &[Point] {
        &self.vertices
    }

    /// Whether the polygon is closed.
    pub fn is_closed(&self) -> bool {
        self.closed
    }
}

impl PatchShape for Polygon {
    fn path(&self) -> Path {
        if self.closed {
            Path::closed(self.vertices.clone())
        } else {
            Path::polyline(self.vertices.clone())
        }
    }
}

/// A pie wedge (or annular sector when `width` is set).
#[derive(Clone, Debug, PartialEq)]
pub struct Wedge {
    /// Centre.
    pub center: Point,
    /// Outer radius.
    pub r: f64,
    /// Start angle in degrees.
    pub theta1: f64,
    /// End angle in degrees.
    pub theta2: f64,
    /// Ring width for annular sectors; `None` draws a full wedge.
    pub width: Option<f64>,
}

impl Wedge {
    /// A wedge of radius `r` from `theta1` to `theta2` degrees.
    pub fn new(center: impl Into<Point>, r: f64, theta1: f64, theta2: f64) -> Self {
        Self {
            center: center.into(),
            r,
            theta1,
            theta2,
            width: None,
        }
    }
}

impl PatchShape for Wedge {
    fn path(&self) -> Path {
        let full_turn = ((self.theta2 - self.theta1) - 360.0).abs() <= 1e-12;
        let (theta1, theta2, connector) = if full_turn {
            (0.0, 360.0, PathCode::MoveTo)
        } else {
            (self.theta1, self.theta2, PathCode::LineTo)
        };
        let arc = Path::arc(theta1, theta2, None);
        let arc_codes = arc.resolved_codes();
        let (vertices, codes) = self.width.map_or_else(
            || wedge_outline(&arc, arc_codes.clone(), connector),
            |width| annulus_outline(&arc, &arc_codes, (self.r - width) / self.r, connector),
        );
        let placed = vertices
            .into_iter()
            .map(|p| p * self.r + self.center)
            .collect();
        Path::new(placed, codes).expect("wedge codes are well formed")
    }
}

/// The outline of a wedge without a hole: the arc, then the centre joined
/// by `connector` (a move for a full turn, so that no radius is drawn),
/// closed.
fn wedge_outline(
    arc: &Path,
    arc_codes: Vec<PathCode>,
    connector: PathCode,
) -> (Vec<Point>, Vec<PathCode>) {
    let mut vertices = arc.vertices().to_vec();
    vertices.extend([Point::ORIGIN, Point::ORIGIN]);
    let mut codes = arc_codes;
    codes.extend([connector, PathCode::ClosePoly]);
    (vertices, codes)
}

/// The outline of an annular sector: the outer arc, then the inner arc
/// (scaled by `inner`) backwards, joined by `connector`.
fn annulus_outline(
    arc: &Path,
    arc_codes: &[PathCode],
    inner: f64,
    connector: PathCode,
) -> (Vec<Point>, Vec<PathCode>) {
    let mut vertices = arc.vertices().to_vec();
    vertices.extend(arc.vertices().iter().rev().map(|&p| p * inner));
    vertices.push(Point::ORIGIN);
    let mut codes = arc_codes.to_vec();
    codes.push(connector);
    codes.extend_from_slice(&arc_codes[1..]);
    codes.push(PathCode::ClosePoly);
    (vertices, codes)
}

/// A regular polygon with `sides` vertices on a circle of `radius`.
#[derive(Clone, Debug, PartialEq)]
pub struct RegularPolygon {
    /// Centre.
    pub center: Point,
    /// Number of vertices.
    pub sides: usize,
    /// Distance from the centre to each vertex.
    pub radius: f64,
    /// Rotation in radians.
    pub orientation: f64,
}

impl RegularPolygon {
    /// A regular polygon pointing up.
    pub fn new(center: impl Into<Point>, sides: usize, radius: f64) -> Self {
        Self {
            center: center.into(),
            sides,
            radius,
            orientation: 0.0,
        }
    }
}

impl PatchShape for RegularPolygon {
    fn path(&self) -> Path {
        Path::unit_regular_polygon(self.sides)
    }

    fn patch_transform(&self) -> Affine2D {
        Affine2D::identity()
            .scale(self.radius, self.radius)
            .rotate(self.orientation)
            .translate(self.center.x, self.center.y)
    }
}

/// A regular polygon approximating a circle (matplotlib's `CirclePolygon`).
#[derive(Clone, Debug, PartialEq)]
pub struct CirclePolygon {
    /// Centre.
    pub center: Point,
    /// Distance from the centre to each vertex.
    pub radius: f64,
    /// Number of vertices.
    pub resolution: usize,
}

impl CirclePolygon {
    /// A 20-gon of `radius` around `center`.
    pub fn new(center: impl Into<Point>, radius: f64) -> Self {
        Self {
            center: center.into(),
            radius,
            resolution: 20,
        }
    }

    /// The same circle with `resolution` vertices.
    #[must_use]
    pub fn with_resolution(self, resolution: usize) -> Self {
        Self { resolution, ..self }
    }

    fn polygon(&self) -> RegularPolygon {
        RegularPolygon::new(self.center, self.resolution, self.radius)
    }
}

impl PatchShape for CirclePolygon {
    fn path(&self) -> Path {
        self.polygon().path()
    }

    fn patch_transform(&self) -> Affine2D {
        self.polygon().patch_transform()
    }
}

/// An arbitrary path (matplotlib's `PathPatch`); draws an edge by default.
#[derive(Clone, Debug, PartialEq)]
pub struct PathShape(pub Path);

impl PatchShape for PathShape {
    const EDGE_DEFAULT: bool = true;

    fn path(&self) -> Path {
        self.0.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placed<S: PatchShape>(shape: &S) -> Vec<Point> {
        shape
            .path()
            .transformed(&shape.patch_transform())
            .vertices()
            .to_vec()
    }

    #[test]
    fn rectangle_corners_and_rotation() {
        let rect = Rectangle::new((1.0, 2.0), 3.0, 4.0);
        assert_eq!(placed(&rect)[2], Point::new(4.0, 6.0));
        let turned = rect.rotated(90.0, RotationPoint::Xy);
        let far = placed(&turned)[1];
        assert!((far - Point::new(1.0, 5.0)).length() < 1e-12);
        let centered = Rectangle::new((0.0, 0.0), 2.0, 2.0).rotated(180.0, RotationPoint::Center);
        assert!((placed(&centered)[0] - Point::new(2.0, 2.0)).length() < 1e-12);
    }

    #[test]
    fn corners_centres_and_axis_ends() {
        let p = Point::new;
        let rect = Rectangle::new((10.0, 20.0), 1.0, 2.0);
        assert_eq!(
            rect.corners(),
            [p(10.0, 20.0), p(11.0, 20.0), p(11.0, 22.0), p(10.0, 22.0)]
        );
        assert_eq!(rect.center(), p(10.5, 21.0));
        let turned = rect.clone().rotated(90.0, RotationPoint::Xy);
        let close = |a: Point, b: Point| (a - b).length() < 1e-12;
        assert!(close(turned.corners()[2], p(8.0, 21.0)));
        assert!(close(turned.center(), p(9.0, 20.5)));
        let ellipse = Ellipse::new((10.5, 21.0), 1.0, 2.0);
        assert_eq!(ellipse.corners(), rect.corners());
        assert_eq!(ellipse.vertices(), [p(10.5, 22.0), p(10.5, 20.0)]);
        assert_eq!(ellipse.co_vertices(), [p(11.0, 21.0), p(10.0, 21.0)]);
        let wide = Ellipse::new((0.0, 0.0), 4.0, 2.0);
        assert_eq!(wide.vertices(), [p(2.0, 0.0), p(-2.0, 0.0)]);
        assert_eq!(wide.co_vertices(), [p(0.0, 1.0), p(0.0, -1.0)]);
    }

    #[test]
    fn polygon_closing_rules() {
        let p = Point::new;
        assert_eq!(
            Polygon::new([p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)], true)
                .vertices()
                .len(),
            4
        );
        assert_eq!(
            Polygon::new([p(0.0, 0.0), p(1.0, 0.0), p(0.0, 0.0)], true)
                .vertices()
                .len(),
            3
        );
        assert_eq!(
            Polygon::new([p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 0.0)], false)
                .vertices()
                .len(),
            3
        );
    }

    #[test]
    fn wedges_and_annuli() {
        let wedge = Wedge::new((0.0, 0.0), 2.0, 0.0, 90.0);
        let path = wedge.path();
        assert_eq!(path.code_at(path.len() - 1), PathCode::ClosePoly);
        assert_eq!(path.vertices()[path.len() - 2], Point::ORIGIN);
        assert_eq!(path.code_at(path.len() - 2), PathCode::LineTo);
        // A full turn moves to the centre instead of drawing a radius.
        let disc = Wedge::new((0.0, 0.0), 2.0, 90.0, 450.0).path();
        assert_eq!(disc.code_at(disc.len() - 2), PathCode::MoveTo);
        assert_eq!(disc.code_at(disc.len() - 1), PathCode::ClosePoly);
        assert_eq!(disc.vertices()[0], Point::new(2.0, 0.0));
        let ring = Wedge {
            width: Some(1.0),
            ..Wedge::new((0.0, 0.0), 2.0, 0.0, 360.0)
        };
        let ring_path = ring.path();
        let inner_start = ring_path.vertices()[ring_path.len() / 2];
        assert!((inner_start.length() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn circles_scale_the_unit_circle() {
        let extents = Circle::new((5.0, 5.0), 2.0)
            .path()
            .transformed(&Circle::new((5.0, 5.0), 2.0).patch_transform())
            .extents();
        assert!((extents.xmin() - 3.0).abs() < 1e-12 && (extents.ymax() - 7.0).abs() < 1e-12);
    }
}
