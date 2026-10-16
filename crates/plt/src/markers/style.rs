//! Resolved marker geometry (matplotlib's `MarkerStyle`).

use plt_core::{Affine2D, Path, PathCode, Point};
use plt_render::{CapStyle, JoinStyle};

use super::spec::{FillStyle, Marker, MarkerError, MarkerSymbol, PolygonStyle};

/// A marker resolved into the paths and transforms used to draw it.
///
/// Marker paths live in a unit coordinate system (a marker of size 1 spans
/// about one unit); [`transform`](Self::transform) maps them there and the
/// line or collection then scales by the marker size in pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct MarkerStyle {
    marker: Marker,
    fillstyle: FillStyle,
    user_transform: Option<Affine2D>,
    user_capstyle: Option<CapStyle>,
    user_joinstyle: Option<JoinStyle>,
    shape: Shape,
}

/// The output of one of matplotlib's `_set_*` marker functions.
#[derive(Clone, Debug, PartialEq)]
struct Shape {
    path: Path,
    transform: Affine2D,
    alt: Option<(Path, Affine2D)>,
    snap_threshold: Option<f64>,
    joinstyle: JoinStyle,
    capstyle: CapStyle,
    filled: bool,
}

impl MarkerStyle {
    /// Resolves `marker` with the given fill style.
    pub fn new(marker: impl Into<Marker>, fillstyle: FillStyle) -> Result<Self, MarkerError> {
        Self::with_styles(marker, fillstyle, None, None)
    }

    /// Resolves `marker`, overriding the marker's own cap and join styles.
    ///
    /// As in matplotlib, a join style override only applies to markers that
    /// choose a join style themselves (polygons, stars, carets, ...); round
    /// markers always use round joins.
    pub fn with_styles(
        marker: impl Into<Marker>,
        fillstyle: FillStyle,
        capstyle: Option<CapStyle>,
        joinstyle: Option<JoinStyle>,
    ) -> Result<Self, MarkerError> {
        let marker = marker.into();
        let shape = Shape::build(&marker, fillstyle, capstyle, joinstyle)?;
        Ok(Self {
            marker,
            fillstyle,
            user_transform: None,
            user_capstyle: capstyle,
            user_joinstyle: joinstyle,
            shape,
        })
    }

    /// The marker specification.
    pub fn marker(&self) -> &Marker {
        &self.marker
    }

    /// The fill style.
    pub fn fillstyle(&self) -> FillStyle {
        self.fillstyle
    }

    /// The primary path (the face for filled markers).
    pub fn path(&self) -> &Path {
        &self.shape.path
    }

    /// Transform of [`path`](Self::path), including any user transform.
    pub fn transform(&self) -> Affine2D {
        self.with_user(self.shape.transform)
    }

    /// The alternate half of a half-filled marker.
    pub fn alt_path(&self) -> Option<&Path> {
        self.shape.alt.as_ref().map(|(path, _)| path)
    }

    /// Transform of [`alt_path`](Self::alt_path), including any user transform.
    pub fn alt_transform(&self) -> Option<Affine2D> {
        self.shape.alt.as_ref().map(|(_, t)| self.with_user(*t))
    }

    /// The user-supplied part of the transform.
    pub fn user_transform(&self) -> Option<Affine2D> {
        self.user_transform
    }

    /// Join style used when stroking the marker.
    pub fn joinstyle(&self) -> JoinStyle {
        self.shape.joinstyle
    }

    /// Cap style used when stroking the marker.
    pub fn capstyle(&self) -> CapStyle {
        self.shape.capstyle
    }

    /// Whether the marker has a face to fill.
    pub fn is_filled(&self) -> bool {
        self.shape.filled
    }

    /// Marker size (in pixels) at or above which the marker is snapped to
    /// the pixel grid; `None` never snaps, infinity always defers to the
    /// renderer's automatic snapping.
    pub fn snap_threshold(&self) -> Option<f64> {
        self.shape.snap_threshold
    }

    /// Whether the marker draws anything at all.
    pub fn is_visible(&self) -> bool {
        !self.shape.path.is_empty()
    }

    /// A copy with `transform` appended to the user transform.
    #[must_use]
    pub fn transformed(&self, transform: Affine2D) -> Self {
        let mut marker = self.clone();
        marker.user_transform = Some(self.user_transform.map_or(transform, |t| t.then(transform)));
        marker
    }

    /// A copy rotated by `degrees`.
    #[must_use]
    pub fn rotated_deg(&self, degrees: f64) -> Self {
        self.transformed(Affine2D::identity().rotate_deg(degrees))
    }

    /// A copy rotated by `radians`.
    #[must_use]
    pub fn rotated(&self, radians: f64) -> Self {
        self.transformed(Affine2D::identity().rotate(radians))
    }

    /// A copy scaled by `(sx, sy)`.
    #[must_use]
    pub fn scaled(&self, sx: f64, sy: f64) -> Self {
        self.transformed(Affine2D::scaling(sx, sy))
    }

    /// The user cap-style override.
    pub fn user_capstyle(&self) -> Option<CapStyle> {
        self.user_capstyle
    }

    /// The user join-style override.
    pub fn user_joinstyle(&self) -> Option<JoinStyle> {
        self.user_joinstyle
    }

    fn with_user(&self, transform: Affine2D) -> Affine2D {
        self.user_transform
            .map_or(transform, |user| transform.then(user))
    }
}

impl Shape {
    fn build(
        marker: &Marker,
        fill: FillStyle,
        user_cap: Option<CapStyle>,
        user_join: Option<JoinStyle>,
    ) -> Result<Self, MarkerError> {
        let mut shape = Self {
            path: Path::default(),
            transform: Affine2D::IDENTITY,
            alt: None,
            snap_threshold: None,
            joinstyle: JoinStyle::Round,
            capstyle: user_cap.unwrap_or(CapStyle::Butt),
            filled: fill != FillStyle::None,
        };
        let join = |default: JoinStyle| user_join.unwrap_or(default);
        match marker {
            Marker::Symbol(symbol) => shape.symbol(*symbol, fill, join),
            Marker::Polygon {
                sides,
                style,
                angle,
            } => {
                if *sides == 0 {
                    return Err(MarkerError::NoSides);
                }
                shape.path = match style {
                    PolygonStyle::Regular => Path::unit_regular_polygon(*sides),
                    PolygonStyle::Star => Path::unit_regular_star(*sides, 0.5),
                    PolygonStyle::Asterisk => Path::unit_regular_asterisk(*sides),
                };
                shape.joinstyle = join(if *style == PolygonStyle::Regular {
                    JoinStyle::Miter
                } else {
                    JoinStyle::Bevel
                });
                if *style == PolygonStyle::Asterisk {
                    shape.filled = false;
                }
                shape.transform = Affine2D::scaling(0.5, 0.5).rotate_deg(*angle);
            }
            Marker::Vertices(points) => shape.custom(Path::polyline(points.clone()))?,
            Marker::Path(path) => shape.custom(path.clone())?,
        }
        Ok(shape)
    }

    /// `_set_custom_marker`: normalise so the largest coordinate is 0.5.
    ///
    /// Like numpy's `max`, a NaN coordinate poisons the scale, and an
    /// all-zero shape yields an infinite one; the scaling is applied to the
    /// identity matrix so off-diagonal entries become NaN exactly as in
    /// matplotlib.
    fn custom(&mut self, path: Path) -> Result<(), MarkerError> {
        let rescale = path
            .vertices()
            .iter()
            .flat_map(|p| [p.x.abs(), p.y.abs()])
            .reduce(|a, b| {
                if a.is_nan() || b.is_nan() {
                    f64::NAN
                } else {
                    a.max(b)
                }
            })
            .ok_or(MarkerError::EmptyShape)?;
        let factor = 0.5 / rescale;
        self.transform = Affine2D::IDENTITY.scale(factor, factor);
        self.path = path;
        Ok(())
    }

    fn symbol(
        &mut self,
        symbol: MarkerSymbol,
        fill: FillStyle,
        join: impl Fn(JoinStyle) -> JoinStyle,
    ) {
        use MarkerSymbol as S;
        match symbol {
            S::Nothing => self.filled = false,
            S::Circle => self.circle(1.0, fill),
            S::Point => self.circle(0.5, fill),
            S::Pixel => {
                self.path = Path::unit_rectangle();
                self.transform = Affine2D::translation(-0.49999, -0.49999);
            }
            S::TriangleUp => self.triangle(0.0, 0, fill, join(JoinStyle::Miter)),
            S::TriangleDown => self.triangle(180.0, 2, fill, join(JoinStyle::Miter)),
            S::TriangleLeft => self.triangle(90.0, 3, fill, join(JoinStyle::Miter)),
            S::TriangleRight => self.triangle(270.0, 1, fill, join(JoinStyle::Miter)),
            S::Square => self.square(fill, join(JoinStyle::Miter)),
            S::Diamond => self.diamond(fill, join(JoinStyle::Miter)),
            S::ThinDiamond => {
                self.diamond(fill, join(JoinStyle::Miter));
                // matplotlib's alternate transform aliases the primary one,
                // so the extra scaling applies to both halves.
                let thin = |t: Affine2D| t.scale(0.6, 1.0);
                self.transform = thin(self.transform);
                if let Some((_, alt)) = self.alt.as_mut() {
                    *alt = thin(*alt);
                }
            }
            S::Pentagon => self.pentagon(fill, join(JoinStyle::Miter)),
            S::Star => self.star(fill, join(JoinStyle::Bevel)),
            S::Hexagon1 => self.hexagon1(fill, join(JoinStyle::Miter)),
            S::Hexagon2 => self.hexagon2(fill, join(JoinStyle::Miter)),
            S::Octagon => self.octagon(fill, join(JoinStyle::Miter)),
            S::VLine => self.line_marker(0.0),
            S::HLine => self.line_marker(90.0),
            S::TickLeft => self.tick(horizontal_tick(), Affine2D::scaling(-1.0, 1.0)),
            S::TickRight => self.tick(horizontal_tick(), Affine2D::IDENTITY),
            S::TickUp => self.tick(vertical_tick(), Affine2D::IDENTITY),
            S::TickDown => self.tick(vertical_tick(), Affine2D::scaling(1.0, -1.0)),
            S::TriDown => self.tri(0.0),
            S::TriUp => self.tri(180.0),
            S::TriLeft => self.tri(270.0),
            S::TriRight => self.tri(90.0),
            S::CaretDown => self.caret(caret_path(), 0.0, join(JoinStyle::Miter)),
            S::CaretUp => self.caret(caret_path(), 180.0, join(JoinStyle::Miter)),
            S::CaretLeft => self.caret(caret_path(), 270.0, join(JoinStyle::Miter)),
            S::CaretRight => self.caret(caret_path(), 90.0, join(JoinStyle::Miter)),
            S::CaretDownBase => self.caret(caret_base_path(), 0.0, join(JoinStyle::Miter)),
            S::CaretUpBase => self.caret(caret_base_path(), 180.0, join(JoinStyle::Miter)),
            S::CaretLeftBase => self.caret(caret_base_path(), 270.0, join(JoinStyle::Miter)),
            S::CaretRightBase => self.caret(caret_base_path(), 90.0, join(JoinStyle::Miter)),
            S::Plus => self.unfilled(plus_path(), 1.0),
            S::X => self.unfilled(x_path(), 3.0),
            S::PlusFilled => self.filled_cross(fill, join(JoinStyle::Miter), plus_filled_paths()),
            S::XFilled => self.filled_cross(fill, join(JoinStyle::Miter), x_filled_paths()),
        }
    }

    fn circle(&mut self, size: f64, fill: FillStyle) {
        self.transform = Affine2D::scaling(0.5 * size, 0.5 * size);
        self.snap_threshold = Some(f64::INFINITY);
        if fill.is_half() {
            let half = Path::unit_circle_righthalf();
            self.transform = self
                .transform
                .rotate_deg(rotation(fill, [180.0, 0.0, 90.0, 270.0]));
            self.alt = Some((half.clone(), self.transform.rotate_deg(180.0)));
            self.path = half;
        } else {
            self.path = Path::unit_circle();
        }
    }

    fn triangle(&mut self, rotation_deg: f64, skip: usize, fill: FillStyle, join: JoinStyle) {
        self.transform = Affine2D::scaling(0.5, 0.5).rotate_deg(rotation_deg);
        self.snap_threshold = Some(5.0);
        if fill.is_half() {
            let parts = [
                Path::polygon(points(&[[0.0, 1.0], [-0.6, -0.2], [0.6, -0.2]])),
                Path::polygon(points(&[[0.0, 1.0], [0.0, -1.0], [-1.0, -1.0]])),
                Path::polygon(points(&[
                    [-0.6, -0.2],
                    [0.6, -0.2],
                    [1.0, -1.0],
                    [-1.0, -1.0],
                ])),
                Path::polygon(points(&[[0.0, 1.0], [0.0, -1.0], [1.0, -1.0]])),
            ];
            let (main, alt) = match fill {
                FillStyle::Top => (0, 2),
                FillStyle::Bottom => (2, 0),
                FillStyle::Left => (1, 3),
                _ => (3, 1),
            };
            self.path = parts[(main + skip) % 4].clone();
            self.alt = Some((parts[(alt + skip) % 4].clone(), self.transform));
        } else {
            self.path = Path::polygon(points(&[[0.0, 1.0], [-1.0, -1.0], [1.0, -1.0]]));
        }
        self.joinstyle = join;
    }

    fn square(&mut self, fill: FillStyle, join: JoinStyle) {
        self.transform = Affine2D::translation(-0.5, -0.5);
        self.snap_threshold = Some(2.0);
        if fill.is_half() {
            self.path = Path::polyline(points(&[
                [0.0, 0.0],
                [1.0, 0.0],
                [1.0, 0.5],
                [0.0, 0.5],
                [0.0, 0.0],
            ]));
            let alt = Path::polyline(points(&[
                [0.0, 0.5],
                [1.0, 0.5],
                [1.0, 1.0],
                [0.0, 1.0],
                [0.0, 0.5],
            ]));
            self.transform = self
                .transform
                .rotate_deg(rotation(fill, [270.0, 90.0, 180.0, 0.0]));
            self.alt = Some((alt, self.transform));
        } else {
            self.path = Path::unit_rectangle();
        }
        self.joinstyle = join;
    }

    fn diamond(&mut self, fill: FillStyle, join: JoinStyle) {
        self.transform = Affine2D::translation(-0.5, -0.5).rotate_deg(45.0);
        self.snap_threshold = Some(5.0);
        if fill.is_half() {
            self.path = Path::polyline(points(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]]));
            let alt = Path::polyline(points(&[[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [0.0, 0.0]]));
            self.transform = self
                .transform
                .rotate_deg(rotation(fill, [180.0, 0.0, 90.0, 270.0]));
            self.alt = Some((alt, self.transform));
        } else {
            self.path = Path::unit_rectangle();
        }
        self.joinstyle = join;
    }

    fn pentagon(&mut self, fill: FillStyle, join: JoinStyle) {
        self.transform = Affine2D::scaling(0.5, 0.5);
        self.snap_threshold = Some(5.0);
        let polygon = Path::unit_regular_polygon(5);
        if fill.is_half() {
            let v = polygon.vertices();
            let y = (1.0 + 5f64.sqrt()) / 4.0;
            let top = Path::polyline(vec![v[0], v[1], v[4], v[0]]);
            let bottom = Path::polyline(vec![v[1], v[2], v[3], v[4], v[1]]);
            let left = Path::polyline(vec![v[0], v[1], v[2], Point::new(0.0, -y), v[0]]);
            let right = Path::polyline(vec![v[0], v[4], v[3], Point::new(0.0, -y), v[0]]);
            self.halves(fill, [top, bottom, left, right]);
        } else {
            self.path = polygon;
        }
        self.joinstyle = join;
    }

    fn star(&mut self, fill: FillStyle, join: JoinStyle) {
        self.transform = Affine2D::scaling(0.5, 0.5);
        self.snap_threshold = Some(5.0);
        let polygon = Path::unit_regular_star(5, 0.381_966);
        if fill.is_half() {
            let v = polygon.vertices();
            let pick = |ranges: &[std::ops::Range<usize>]| {
                Path::polyline(
                    ranges
                        .iter()
                        .flat_map(|r| v[r.clone()].iter().copied())
                        .collect(),
                )
            };
            let top = pick(&[0..4, 7..10, 0..1]);
            let bottom = pick(&[3..8, 3..4]);
            let left = pick(&[0..6, 0..1]);
            let right = pick(&[0..1, 5..10, 0..1]);
            self.halves(fill, [top, bottom, left, right]);
        } else {
            self.path = polygon;
        }
        self.joinstyle = join;
    }

    fn hexagon1(&mut self, fill: FillStyle, join: JoinStyle) {
        self.transform = Affine2D::scaling(0.5, 0.5);
        let polygon = Path::unit_regular_polygon(6);
        if fill.is_half() {
            let v = polygon.vertices();
            let x = (5.0 * std::f64::consts::PI / 6.0).cos().abs();
            let top = Path::polyline(vec![
                Point::new(-x, 0.0),
                v[1],
                v[0],
                v[5],
                Point::new(x, 0.0),
            ]);
            let bottom = Path::polyline(vec![
                Point::new(-x, 0.0),
                v[2],
                v[3],
                v[4],
                Point::new(x, 0.0),
            ]);
            let left = Path::polyline(v[0..4].to_vec());
            let right = Path::polyline(vec![v[0], v[5], v[4], v[3]]);
            self.halves(fill, [top, bottom, left, right]);
        } else {
            self.path = polygon;
        }
        self.joinstyle = join;
    }

    fn hexagon2(&mut self, fill: FillStyle, join: JoinStyle) {
        self.transform = Affine2D::scaling(0.5, 0.5).rotate_deg(30.0);
        let polygon = Path::unit_regular_polygon(6);
        if fill.is_half() {
            let v = polygon.vertices();
            let (x, y) = (3f64.sqrt() / 4.0, 0.75);
            let top = Path::polyline(vec![v[1], v[0], v[5], v[4], v[1]]);
            let bottom = Path::polyline(v[1..5].to_vec());
            let left = Path::polyline(vec![
                Point::new(x, y),
                v[0],
                v[1],
                v[2],
                Point::new(-x, -y),
                Point::new(x, y),
            ]);
            let right =
                Path::polyline(vec![Point::new(x, y), v[5], v[4], v[3], Point::new(-x, -y)]);
            self.halves(fill, [top, bottom, left, right]);
        } else {
            self.path = polygon;
        }
        self.joinstyle = join;
    }

    fn octagon(&mut self, fill: FillStyle, join: JoinStyle) {
        self.transform = Affine2D::scaling(0.5, 0.5);
        self.snap_threshold = Some(5.0);
        if fill.is_half() {
            let x = 2f64.sqrt() / 4.0;
            let half = Path::polyline(points(&[
                [0.0, -1.0],
                [0.0, 1.0],
                [-x, 1.0],
                [-1.0, x],
                [-1.0, -x],
                [-x, -1.0],
                [0.0, -1.0],
            ]));
            self.transform = self
                .transform
                .rotate_deg(rotation(fill, [0.0, 180.0, 270.0, 90.0]));
            self.alt = Some((half.clone(), self.transform.rotate_deg(180.0)));
            self.path = half;
        } else {
            self.transform = self.transform.rotate_deg(22.5);
            self.path = Path::unit_regular_polygon(8);
        }
        self.joinstyle = join;
    }

    /// Assigns `(top, bottom, left, right)` halves by fill style, sharing
    /// the primary transform with the alternate half.
    fn halves(&mut self, fill: FillStyle, [top, bottom, left, right]: [Path; 4]) {
        let (main, alt) = match fill {
            FillStyle::Top => (top, bottom),
            FillStyle::Bottom => (bottom, top),
            FillStyle::Left => (left, right),
            _ => (right, left),
        };
        self.path = main;
        self.alt = Some((alt, self.transform));
    }

    fn line_marker(&mut self, rotation_deg: f64) {
        self.unfilled(Path::polyline(points(&[[0.0, -1.0], [0.0, 1.0]])), 1.0);
        self.transform = self.transform.rotate_deg(rotation_deg);
    }

    fn tick(&mut self, path: Path, transform: Affine2D) {
        self.transform = transform;
        self.snap_threshold = Some(1.0);
        self.filled = false;
        self.path = path;
    }

    fn tri(&mut self, rotation_deg: f64) {
        let path = Path::new(
            points(&[
                [0.0, 0.0],
                [0.0, -1.0],
                [0.0, 0.0],
                [0.8, 0.5],
                [0.0, 0.0],
                [-0.8, 0.5],
            ]),
            [PathCode::MoveTo, PathCode::LineTo].repeat(3),
        )
        .expect("tri marker codes are well formed");
        self.unfilled(path, 5.0);
        self.transform = self.transform.rotate_deg(rotation_deg);
    }

    fn caret(&mut self, path: Path, rotation_deg: f64, join: JoinStyle) {
        self.unfilled(path, 3.0);
        self.transform = self.transform.rotate_deg(rotation_deg);
        self.joinstyle = join;
    }

    fn unfilled(&mut self, path: Path, snap: f64) {
        self.transform = Affine2D::scaling(0.5, 0.5);
        self.snap_threshold = Some(snap);
        self.filled = false;
        self.path = path;
    }

    fn filled_cross(&mut self, fill: FillStyle, join: JoinStyle, (full, half): (Path, Path)) {
        self.transform = Affine2D::IDENTITY;
        self.snap_threshold = Some(5.0);
        self.joinstyle = join;
        if fill.is_half() {
            self.transform = self
                .transform
                .rotate_deg(rotation(fill, [90.0, 270.0, 0.0, 180.0]));
            self.alt = Some((half.clone(), self.transform.rotate_deg(180.0)));
            self.path = half;
        } else {
            self.path = full;
        }
    }
}

/// Picks the rotation (degrees) for a half fill style from a table given
/// in the order `[left, right, top, bottom]`.
fn rotation(fill: FillStyle, [left, right, top, bottom]: [f64; 4]) -> f64 {
    match fill {
        FillStyle::Left => left,
        FillStyle::Top => top,
        FillStyle::Bottom => bottom,
        _ => right,
    }
}

fn points(coords: &[[f64; 2]]) -> Vec<Point> {
    coords.iter().map(|&p| Point::from(p)).collect()
}

fn horizontal_tick() -> Path {
    Path::polyline(points(&[[0.0, 0.0], [1.0, 0.0]]))
}

fn vertical_tick() -> Path {
    Path::polyline(points(&[[-0.0, 0.0], [-0.0, 1.0]]))
}

fn caret_path() -> Path {
    Path::polyline(points(&[[-1.0, 1.5], [0.0, 0.0], [1.0, 1.5]]))
}

fn caret_base_path() -> Path {
    Path::polyline(points(&[[-1.0, 0.0], [0.0, -1.5], [1.0, 0.0]]))
}

fn two_strokes(coords: &[[f64; 2]; 4]) -> Path {
    Path::new(
        points(coords),
        vec![
            PathCode::MoveTo,
            PathCode::LineTo,
            PathCode::MoveTo,
            PathCode::LineTo,
        ],
    )
    .expect("two-stroke marker codes are well formed")
}

fn plus_path() -> Path {
    two_strokes(&[[-1.0, 0.0], [1.0, 0.0], [0.0, -1.0], [0.0, 1.0]])
}

fn x_path() -> Path {
    two_strokes(&[[-1.0, -1.0], [1.0, 1.0], [-1.0, 1.0], [1.0, -1.0]])
}

fn scaled_polygon(coords: &[[f64; 2]], divisor: f64) -> Path {
    Path::polygon(
        coords
            .iter()
            .map(|&[x, y]| Point::new(x / divisor, y / divisor)),
    )
}

fn plus_filled_paths() -> (Path, Path) {
    (
        scaled_polygon(
            &[
                [-1.0, -3.0],
                [1.0, -3.0],
                [1.0, -1.0],
                [3.0, -1.0],
                [3.0, 1.0],
                [1.0, 1.0],
                [1.0, 3.0],
                [-1.0, 3.0],
                [-1.0, 1.0],
                [-3.0, 1.0],
                [-3.0, -1.0],
                [-1.0, -1.0],
            ],
            6.0,
        ),
        scaled_polygon(
            &[
                [3.0, 0.0],
                [3.0, 1.0],
                [1.0, 1.0],
                [1.0, 3.0],
                [-1.0, 3.0],
                [-1.0, 1.0],
                [-3.0, 1.0],
                [-3.0, 0.0],
            ],
            6.0,
        ),
    )
}

fn x_filled_paths() -> (Path, Path) {
    (
        scaled_polygon(
            &[
                [-1.0, -2.0],
                [0.0, -1.0],
                [1.0, -2.0],
                [2.0, -1.0],
                [1.0, 0.0],
                [2.0, 1.0],
                [1.0, 2.0],
                [0.0, 1.0],
                [-1.0, 2.0],
                [-2.0, 1.0],
                [-1.0, 0.0],
                [-2.0, -1.0],
            ],
            4.0,
        ),
        scaled_polygon(
            &[
                [1.0, 0.0],
                [2.0, 1.0],
                [1.0, 2.0],
                [0.0, 1.0],
                [-1.0, 2.0],
                [-2.0, 1.0],
                [-1.0, 0.0],
            ],
            4.0,
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style(marker: MarkerSymbol, fill: FillStyle) -> MarkerStyle {
        MarkerStyle::new(marker, fill).expect("valid marker")
    }

    #[test]
    fn circle_is_filled_with_round_join() {
        let m = style(MarkerSymbol::Circle, FillStyle::Full);
        assert!(m.is_filled());
        assert_eq!(m.joinstyle(), JoinStyle::Round);
        assert_eq!(m.transform(), Affine2D::scaling(0.5, 0.5));
        assert_eq!(m.snap_threshold(), Some(f64::INFINITY));
        assert!(m.alt_path().is_none());
    }

    #[test]
    fn user_joinstyle_only_overrides_markers_that_choose_one() {
        let circle = MarkerStyle::with_styles(
            MarkerSymbol::Circle,
            FillStyle::Full,
            None,
            Some(JoinStyle::Miter),
        )
        .expect("valid");
        assert_eq!(circle.joinstyle(), JoinStyle::Round);
        let square = MarkerStyle::with_styles(
            MarkerSymbol::Square,
            FillStyle::Full,
            None,
            Some(JoinStyle::Bevel),
        )
        .expect("valid");
        assert_eq!(square.joinstyle(), JoinStyle::Bevel);
    }

    #[test]
    fn half_filled_markers_have_alternates() {
        let m = style(MarkerSymbol::Circle, FillStyle::Top);
        assert!(m.alt_path().is_some());
        let alt = m.alt_transform().expect("alternate");
        let p = alt.transform_point(Point::new(1.0, 0.0));
        assert!(
            (p - Point::new(0.0, -0.5)).length() < 1e-12,
            "alt half is rotated 180°"
        );
    }

    #[test]
    fn thin_diamond_scales_both_halves() {
        let m = style(MarkerSymbol::ThinDiamond, FillStyle::Left);
        assert_eq!(m.transform(), m.alt_transform().expect("alternate"));
    }

    #[test]
    fn unfilled_markers() {
        for symbol in [
            MarkerSymbol::Plus,
            MarkerSymbol::X,
            MarkerSymbol::TriUp,
            MarkerSymbol::CaretDown,
        ] {
            assert!(!style(symbol, FillStyle::Full).is_filled(), "{symbol:?}");
        }
        assert!(!style(MarkerSymbol::Nothing, FillStyle::Full).is_visible());
    }

    #[test]
    fn custom_vertices_are_normalised() {
        let m = MarkerStyle::new(
            Marker::Vertices(vec![Point::new(-2.0, 0.0), Point::new(4.0, 1.0)]),
            FillStyle::Full,
        )
        .expect("valid");
        assert_eq!(m.transform(), Affine2D::scaling(0.125, 0.125));
        assert_eq!(
            MarkerStyle::new(Marker::Vertices(Vec::new()), FillStyle::Full),
            Err(MarkerError::EmptyShape)
        );
    }

    #[test]
    fn user_transforms_compose() {
        let m = style(MarkerSymbol::Square, FillStyle::Full)
            .scaled(2.0, 2.0)
            .rotated_deg(90.0);
        let p = m.transform().transform_point(Point::new(1.0, 0.5));
        assert!((p - Point::new(0.0, 1.0)).length() < 1e-12);
    }

    #[test]
    fn polygon_tuples() {
        let star = MarkerStyle::new(Marker::polygon(5, PolygonStyle::Star, 0.0), FillStyle::Full)
            .expect("valid");
        assert_eq!(star.joinstyle(), JoinStyle::Bevel);
        assert_eq!(star.path().len(), 11);
        let asterisk = MarkerStyle::new(
            Marker::polygon(4, PolygonStyle::Asterisk, 45.0),
            FillStyle::Full,
        )
        .expect("valid");
        assert!(!asterisk.is_filled());
        assert_eq!(
            MarkerStyle::new(
                Marker::polygon(0, PolygonStyle::Regular, 0.0),
                FillStyle::Full
            ),
            Err(MarkerError::NoSides)
        );
    }
}
