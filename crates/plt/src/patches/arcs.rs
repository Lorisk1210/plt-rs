//! Elliptical arcs and annuli (matplotlib's `Arc` and `Annulus`).

use plt_core::{Affine2D, Path, PathCode, Point};

use super::{DisplayPath, Ellipse, PatchError, PatchShape, PatchStyle, ShapeError, ShapeFrame};
use crate::artist::ArtistBase;
use crate::transforms::{ResolvedTransform, unit_to_bbox};

/// Half the inverse error of the eight-segment Bézier ellipse
/// (1.89818e-6): arcs larger than this many pixels are drawn piecewise.
const LARGE_ARC: f64 = (1.0 / 1.89818e-6) * 0.5;

/// An unfilled segment of an ellipse from `theta1` to `theta2` degrees,
/// counter-clockwise (matplotlib's `Arc`).
#[derive(Clone, Debug, PartialEq)]
pub struct Arc {
    /// Centre of the ellipse.
    pub center: Point,
    /// Horizontal axis length (before rotation).
    pub width: f64,
    /// Vertical axis length (before rotation).
    pub height: f64,
    /// Counter-clockwise rotation of the ellipse in degrees.
    pub angle: f64,
    /// Start angle in degrees, relative to `angle`.
    pub theta1: f64,
    /// End angle in degrees, relative to `angle`.
    pub theta2: f64,
}

impl Arc {
    /// The whole ellipse of `width × height` around `center`.
    pub fn new(center: impl Into<Point>, width: f64, height: f64) -> Self {
        Self {
            center: center.into(),
            width,
            height,
            angle: 0.0,
            theta1: 0.0,
            theta2: 360.0,
        }
    }

    /// The same arc on an ellipse rotated by `angle` degrees.
    #[must_use]
    pub fn with_angle(self, angle: f64) -> Self {
        Self { angle, ..self }
    }

    /// The part of the arc from `theta1` to `theta2` degrees.
    #[must_use]
    pub fn between(self, theta1: f64, theta2: f64) -> Self {
        Self {
            theta1,
            theta2,
            ..self
        }
    }

    /// The angles on the unit circle that land at `theta1` and `theta2`
    /// once the circle is stretched into the ellipse
    /// (`Arc._theta_stretch`).
    #[expect(
        clippy::float_cmp,
        reason = "matplotlib compares sizes and angles exactly"
    )]
    fn stretched_thetas(&self) -> (f64, f64) {
        let full_turn = self.theta1 != self.theta2
            && self.theta1.rem_euclid(360.0) == self.theta2.rem_euclid(360.0);
        if self.width == self.height || full_turn {
            return (self.theta1, self.theta2);
        }
        let scale = self.width / self.height;
        let stretch = |theta: f64| {
            let (sin, cos) = theta.to_radians().sin_cos();
            ((scale * sin).atan2(cos).to_degrees() + 360.0) % 360.0
        };
        (stretch(self.theta1), stretch(self.theta2))
    }
}

impl PatchShape for Arc {
    fn path(&self) -> Path {
        let (theta1, theta2) = self.stretched_thetas();
        Path::arc(theta1, theta2, None)
    }

    fn patch_transform(&self) -> Affine2D {
        Ellipse {
            angle: self.angle,
            ..Ellipse::new(self.center, self.width, self.height)
        }
        .patch_transform()
    }

    /// Arcs cannot be filled.
    fn init_style(&self, style: &mut PatchStyle, _base: &mut ArtistBase) {
        style.fill = false;
    }

    /// The arc, or for an arc too large for the Bézier approximation to
    /// stay within a pixel, the pieces of it inside the axes (or figure)
    /// box, eight segments each (matplotlib's `Arc.draw`).
    fn display_paths(&self, frame: &ShapeFrame<'_>) -> Result<Vec<DisplayPath>, PatchError> {
        let placed =
            ResolvedTransform::affine(self.patch_transform()).then(frame.transform.clone());
        let (theta1, theta2) = self.stretched_thetas();
        let size = frame
            .transform
            .transform_point(Point::new(self.width, self.height))
            - frame.transform.transform_point(Point::ORIGIN);
        let pieces = if size.x < LARGE_ARC && size.y < LARGE_ARC {
            vec![Path::arc(theta1, theta2, None)]
        } else {
            visible_pieces(theta1, theta2, &placed, frame)?
        };
        Ok(pieces
            .iter()
            .map(|piece| {
                let (path, affine) = placed.split_path(piece);
                DisplayPath::filled(path.into_owned(), affine)
            })
            .collect())
    }
}

/// Where the line through `p0` and `p1` crosses the unit circle
/// (`line_circle_intersect`).
fn line_circle_intersect(p0: Point, p1: Point) -> Vec<Point> {
    let (dx, dy) = (p1.x - p0.x, p1.y - p0.y);
    let dr2 = dx * dx + dy * dy;
    let d = p0.x * p1.y - p1.x * p0.y;
    let discrim = dr2 - d * d;
    if discrim < 0.0 {
        return Vec::new();
    }
    let sign_dy = 1.0_f64.copysign(dy);
    let root = discrim.sqrt();
    vec![
        Point::new(
            (d * dy + sign_dy * dx * root) / dr2,
            (-d * dx + dy.abs() * root) / dr2,
        ),
        Point::new(
            (d * dy - sign_dy * dx * root) / dr2,
            (-d * dx - dy.abs() * root) / dr2,
        ),
    ]
}

/// Where the segment from `p0` to `p1` crosses the unit circle
/// (`segment_circle_intersect`).
fn segment_circle_intersect(p0: Point, p1: Point) -> impl Iterator<Item = Point> {
    const EPSILON: f64 = 1e-9;
    let (x0, x1) = (p0.x.min(p1.x), p0.x.max(p1.x));
    let (y0, y1) = (p0.y.min(p1.y), p0.y.max(p1.y));
    line_circle_intersect(p0, p1).into_iter().filter(move |p| {
        x0 - EPSILON < p.x && p.x < x1 + EPSILON && y0 - EPSILON < p.y && p.y < y1 + EPSILON
    })
}

/// The pieces of the unit-circle arc from `theta1` to `theta2` degrees
/// that lie inside the box of the patch's container, once `placed` maps
/// the circle onto the ellipse.
fn visible_pieces(
    theta1: f64,
    theta2: f64,
    placed: &ResolvedTransform,
    frame: &ShapeFrame<'_>,
) -> Result<Vec<Path>, PatchError> {
    let container = frame
        .parent
        .and_then(|parent| frame.layout.clip_extent(parent))
        .ok_or(PatchError::Detached)?;
    let to_circle = ResolvedTransform::affine(unit_to_bbox(&container))
        .then(placed.inverted().ok_or(PatchError::Detached)?);
    let rectangle = Path::unit_rectangle();
    let mut corners = rectangle.clone();
    for p in corners.vertices_mut() {
        *p = to_circle.transform_point(*p);
    }
    let mut thetas: Vec<f64> = corners
        .vertices()
        .windows(2)
        .flat_map(|edge| segment_circle_intersect(edge[0], edge[1]))
        .map(|p| (p.y.atan2(p.x).to_degrees() + 360.0) % 360.0)
        .filter(|&theta| theta1 < theta && theta < theta2)
        .collect();
    thetas.sort_by(f64::total_cmp);
    thetas.dedup();
    thetas.push(theta2);
    let (sin, cos) = theta1.to_radians().sin_cos();
    let mut inside = corners.contains_point(Point::new(cos, sin), None, 0.0);
    let mut last = theta1;
    let mut pieces = Vec::new();
    for theta in thetas {
        if inside {
            pieces.push(Path::arc(last, theta, Some(8)));
        }
        inside = !inside;
        last = theta;
    }
    Ok(pieces)
}

/// An elliptical ring (matplotlib's `Annulus`).
#[derive(Clone, Debug, PartialEq)]
pub struct Annulus {
    center: Point,
    semi_axes: (f64, f64),
    width: f64,
    angle: f64,
}

impl Annulus {
    /// A circular ring of outer `radius` and thickness `width`, measured
    /// inwards; fails if the ring is thicker than the radius.
    pub fn new(center: impl Into<Point>, radius: f64, width: f64) -> Result<Self, ShapeError> {
        Self::elliptical(center, (radius, radius), width, 0.0)
    }

    /// A ring whose outer edge has semi-axes `(a, b)`, rotated by `angle`
    /// degrees; fails if `width` exceeds the smaller semi-axis.
    pub fn elliptical(
        center: impl Into<Point>,
        (a, b): (f64, f64),
        width: f64,
        angle: f64,
    ) -> Result<Self, ShapeError> {
        if width > a.min(b) {
            return Err(ShapeError::AnnulusTooWide);
        }
        Ok(Self {
            center: center.into(),
            semi_axes: (a, b),
            width,
            angle,
        })
    }

    /// Centre.
    pub fn center(&self) -> Point {
        self.center
    }

    /// Outer semi-axes `(a, b)`.
    pub fn semi_axes(&self) -> (f64, f64) {
        self.semi_axes
    }

    /// Ring thickness.
    pub fn width(&self) -> f64 {
        self.width
    }

    /// Rotation in degrees.
    pub fn angle(&self) -> f64 {
        self.angle
    }

    /// Moves the centre (matplotlib's `set_center`).
    pub fn set_center(&mut self, center: impl Into<Point>) -> &mut Self {
        self.center = center.into();
        self
    }

    /// Sets the outer semi-axes `(a, b)` (matplotlib's `set_radii`); like
    /// matplotlib, the width is not checked against them.
    pub fn set_radii(&mut self, (a, b): (f64, f64)) -> &mut Self {
        self.semi_axes = (a, b);
        self
    }

    /// Sets the outer semi-axis along the unrotated x direction
    /// (matplotlib's `set_semimajor`).
    pub fn set_semimajor(&mut self, a: f64) -> &mut Self {
        self.semi_axes.0 = a;
        self
    }

    /// Sets the outer semi-axis along the unrotated y direction
    /// (matplotlib's `set_semiminor`).
    pub fn set_semiminor(&mut self, b: f64) -> &mut Self {
        self.semi_axes.1 = b;
        self
    }

    /// Sets the ring thickness (matplotlib's `set_width`); fails, leaving
    /// the ring as it was, if it exceeds the smaller semi-axis.
    ///
    /// ```
    /// use plt::patches::Annulus;
    /// let mut ring = Annulus::new((0.0, 0.0), 1.0, 0.5)?;
    /// ring.set_width(0.25)?;
    /// assert_eq!(ring.width(), 0.25);
    /// assert!(ring.set_width(2.0).is_err());
    /// # Ok::<(), plt::patches::ShapeError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`ShapeError::AnnulusTooWide`] when `width` is larger than the
    /// smaller semi-axis.
    pub fn set_width(&mut self, width: f64) -> Result<&mut Self, ShapeError> {
        let (a, b) = self.semi_axes;
        if width > a.min(b) {
            return Err(ShapeError::AnnulusTooWide);
        }
        self.width = width;
        Ok(self)
    }

    /// Sets the rotation in degrees (matplotlib's `set_angle`).
    pub fn set_angle(&mut self, angle: f64) -> &mut Self {
        self.angle = angle;
        self
    }

    fn placed(&self, vertices: impl Iterator<Item = Point>, a: f64, b: f64) -> Vec<Point> {
        let placement = Affine2D::scaling(a, b)
            .rotate_deg(self.angle)
            .translate(self.center.x, self.center.y);
        vertices.map(|p| placement.transform_point(p)).collect()
    }
}

impl PatchShape for Annulus {
    /// The outer ellipse, then the inner one backwards
    /// (`Annulus._recompute_path`).
    fn path(&self) -> Path {
        let arc = Path::arc(0.0, 360.0, None);
        let codes = arc.resolved_codes();
        let (a, b) = self.semi_axes;
        let outer = self.placed(arc.vertices().iter().copied(), a, b);
        let inner = self.placed(
            arc.vertices().iter().rev().copied(),
            a - self.width,
            b - self.width,
        );
        let mut vertices = outer.clone();
        vertices.extend(inner);
        vertices.extend([outer[0], Point::ORIGIN]);
        let mut all_codes = codes.clone();
        all_codes.push(PathCode::MoveTo);
        all_codes.extend_from_slice(&codes[1..]);
        all_codes.extend([PathCode::MoveTo, PathCode::ClosePoly]);
        Path::new(vertices, all_codes).expect("annulus codes match its vertices")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arcs_stretch_their_angles_and_are_unfilled() {
        let circle = Arc::new((0.0, 0.0), 2.0, 2.0).between(0.0, 90.0);
        assert_eq!(circle.stretched_thetas(), (0.0, 90.0));
        let wide = Arc::new((0.0, 0.0), 4.0, 2.0).between(45.0, 135.0);
        let (t1, t2) = wide.stretched_thetas();
        assert!((t1 - 2f64.atan().to_degrees()).abs() < 1e-12);
        assert!((t2 - (180.0 - 2f64.atan().to_degrees())).abs() < 1e-12);
        let whole = Arc::new((0.0, 0.0), 4.0, 2.0).between(0.0, 360.0);
        assert_eq!(whole.stretched_thetas(), (0.0, 360.0));
        let mut style = PatchStyle::from_rc(&crate::rc::RcParams::matplotlib());
        whole.init_style(&mut style, &mut ArtistBase::default());
        assert!(!style.fill);
    }

    #[test]
    fn huge_arcs_are_drawn_where_they_cross_the_axes() {
        use crate::artist::ArtistHandle;
        let fig = crate::figure::Figure::new();
        let ax = fig
            .add_axes([0.1, 0.1, 0.8, 0.8])
            .expect("a valid rectangle");
        ax.set_xlim(-1.0, 1.0)
            .and_then(|ax| ax.set_ylim(1e6 - 1.0, 1e6 + 1.0))
            .expect("finite limits");
        // A circle of radius 1e6 around the origin: only its top shows.
        let arc = ax.add_patch(Arc::new((0.0, 0.0), 2e6, 2e6));
        let extent = arc.window_extent().expect("visible");
        let axes = ax.window_extent().expect("an axes");
        assert!(extent.width() < axes.width() * 1.01, "{extent:?}");
        assert_eq!(
            line_circle_intersect(Point::new(2.0, 2.0), Point::new(3.0, 2.0)).len(),
            0
        );
        let crossings: Vec<Point> =
            segment_circle_intersect(Point::new(-2.0, 0.0), Point::new(0.0, 0.0)).collect();
        assert_eq!(crossings, [Point::new(-1.0, 0.0)]);
    }

    #[test]
    fn annulus_rings() {
        assert_eq!(
            Annulus::new((0.0, 0.0), 1.0, 2.0),
            Err(ShapeError::AnnulusTooWide)
        );
        let ring = Annulus::new((5.0, 5.0), 2.0, 0.5).unwrap();
        let path = ring.path();
        let extents = path.extents();
        assert!((extents.x0() - 3.0).abs() < 1e-9 && (extents.x1() - 7.0).abs() < 1e-9);
        let outer = Path::arc(0.0, 360.0, None).len();
        let inner_start = path.vertices()[outer];
        assert!(((inner_start - Point::new(5.0, 5.0)).length() - 1.5).abs() < 1e-9);
        assert_eq!(path.code_at(path.len() - 1), PathCode::ClosePoly);
    }

    #[test]
    fn annulus_setters_reshape_the_ring() {
        let mut ring = Annulus::new((0.0, 0.0), 0.2, 0.01).unwrap();
        ring.set_center((0.5, 0.5));
        ring.set_radii((0.5, 0.3));
        ring.set_width(0.1).unwrap();
        ring.set_angle(45.0);
        assert_eq!(
            ring,
            Annulus::elliptical((0.5, 0.5), (0.5, 0.3), 0.1, 45.0).unwrap()
        );
        ring.set_semimajor(0.05);
        assert_eq!(ring.semi_axes(), (0.05, 0.3), "semi-axes are not checked");
        assert_eq!(ring.set_width(0.2), Err(ShapeError::AnnulusTooWide));
        ring.set_semiminor(0.4);
        assert_eq!(ring.semi_axes(), (0.05, 0.4));
        assert_eq!(ring.width(), 0.1, "a rejected width is not kept");
    }
}
