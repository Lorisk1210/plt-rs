//! Arrows that follow any path, with open or filled heads and brackets at
//! their ends (matplotlib's `ArrowStyle._Curve` family: `-`, `->`, `<|-|>`,
//! `]-[`, `|-|`, ...).

use pltrs_core::bezier::{get_cos_sin, get_normal_points};
use pltrs_core::{Affine2D, Path, PathCode, Point};

use super::StyledPath;

/// What an end of a curve arrow carries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CurveEnd {
    /// Nothing.
    #[default]
    Plain,
    /// An open head (`<` or `>`).
    Head,
    /// A filled head (`<|` or `|>`).
    FilledHead,
    /// A bracket (`]`, `[` or, with zero length, `|`).
    Bracket,
}

/// The size and orientation of the bracket at one end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bracket {
    /// Width, relative to the scale.
    pub width: f64,
    /// Length of the bracket's arms, relative to the scale (0 draws a bar).
    pub length: f64,
    /// Counter-clockwise rotation in degrees (0 is perpendicular to the line).
    pub angle: f64,
    /// Scale of width and length; `None` uses the mutation size.
    pub scale: Option<f64>,
}

impl Default for Bracket {
    fn default() -> Self {
        Self {
            width: 1.0,
            length: 0.2,
            angle: 0.0,
            scale: None,
        }
    }
}

/// An arrow along any path (matplotlib's `_Curve` subclasses).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurveArrow {
    /// What the start carries.
    pub begin: CurveEnd,
    /// What the end carries.
    pub end: CurveEnd,
    /// Head length, relative to the mutation size.
    pub head_length: f64,
    /// Head width, relative to the mutation size.
    pub head_width: f64,
    /// The bracket at the start.
    pub bracket_a: Bracket,
    /// The bracket at the end.
    pub bracket_b: Bracket,
}

impl CurveArrow {
    /// An arrow with the given ends and matplotlib's default sizes.
    pub fn new(begin: CurveEnd, end: CurveEnd) -> Self {
        Self {
            begin,
            end,
            head_length: 0.4,
            head_width: 0.2,
            bracket_a: Bracket::default(),
            bracket_b: Bracket::default(),
        }
    }

    /// The head at the tip `(x1, y1)` of the segment from `(x0, y0)`, and
    /// the shift that keeps the projecting line cap from overshooting the
    /// tip (`_Curve._get_arrow_wedge`).
    fn head(from: Point, tip: Point, head: HeadGeometry, linewidth: f64) -> (Vec<Point>, Point) {
        let (mut dx, mut dy) = (from.x - tip.x, from.y - tip.y);
        let mut distance = dx.hypot(dy);
        let pad_projected = 0.5 * linewidth / head.sin;
        if distance == 0.0 {
            distance = 1.0;
        }
        let shift = Point::new(pad_projected * dx / distance, pad_projected * dy / distance);
        dx = dx / distance * head.dist;
        dy = dy / distance * head.dist;
        let (cos, sin) = (head.cos, head.sin);
        let side1 = Point::new(cos * dx + sin * dy, -sin * dx + cos * dy);
        let side2 = Point::new(cos * dx - sin * dy, sin * dx + cos * dy);
        let base = tip + shift;
        (vec![base + side1, base, base + side2], shift)
    }

    /// A bracket at `end` facing away from `toward` (`_Curve._get_bracket`).
    fn bracket(end: Point, toward: Point, width: f64, length: f64, angle: f64) -> Path {
        let direction = get_cos_sin(toward, end);
        let (p1, p2) = get_normal_points(end, direction, width);
        let arm = Point::new(length * direction.cos, length * direction.sin);
        let mut vertices = vec![p1 + arm, p1, p2, p2 + arm];
        if angle != 0.0 {
            let rotation = Affine2D::identity().rotate_deg_around(end.x, end.y, angle);
            for v in &mut vertices {
                *v = rotation.transform_point(*v);
            }
        }
        Path::polyline(vertices)
    }

    /// The shaft and the ends along `path` (`_Curve.transmute`).
    pub(super) fn transmute(&self, path: &Path, mutation_size: f64, linewidth: f64) -> StyledPath {
        let vertices = path.vertices();
        let n = vertices.len();
        if n < 2 {
            return StyledPath::Many(vec![(path.clone(), false)]);
        }
        let head_length = self.head_length * mutation_size;
        let head_width = self.head_width * mutation_size;
        let dist = head_length.hypot(head_width);
        let geometry = HeadGeometry {
            dist,
            cos: head_length / dist,
            sin: head_width / dist,
        };
        let is_head = |end: CurveEnd| matches!(end, CurveEnd::Head | CurveEnd::FilledHead);
        let (p0, p1) = (vertices[0], vertices[1]);
        let (p2, p3) = (vertices[n - 2], vertices[n - 1]);
        let begin_head =
            (is_head(self.begin) && p0 != p1).then(|| Self::head(p1, p0, geometry, linewidth));
        let end_head =
            (is_head(self.end) && p2 != p3).then(|| Self::head(p2, p3, geometry, linewidth));

        let mut shaft = vertices.to_vec();
        if let Some((_, shift)) = &begin_head {
            shaft[0] = p0 + *shift;
        }
        if let Some((_, shift)) = &end_head {
            shaft[n - 1] = p3 + *shift;
        }
        let shaft = match path.codes() {
            Some(codes) => Path::new(shaft, codes.to_vec()).expect("the shaft keeps the codes"),
            None => Path::polyline(shaft),
        };
        let mut paths = vec![(shaft, false)];
        let scale = |bracket: &Bracket| bracket.scale.unwrap_or(mutation_size);
        let ends = [
            (self.begin, begin_head, p0, p1, &self.bracket_a),
            (self.end, end_head, p3, p2, &self.bracket_b),
        ];
        for (kind, head, tip, toward, bracket) in ends {
            match (kind, head) {
                (CurveEnd::FilledHead, Some((head, _))) => {
                    paths.push((closed_head(head), true));
                }
                (_, Some((head, _))) => paths.push((Path::polyline(head), false)),
                (CurveEnd::Bracket, None) => {
                    let scale = scale(bracket);
                    paths.push((
                        Self::bracket(
                            tip,
                            toward,
                            bracket.width * scale,
                            bracket.length * scale,
                            bracket.angle,
                        ),
                        false,
                    ));
                }
                _ => {}
            }
        }
        StyledPath::Many(paths)
    }
}

/// The length and opening of a head.
#[derive(Clone, Copy, Debug)]
struct HeadGeometry {
    dist: f64,
    cos: f64,
    sin: f64,
}

/// The three head vertices closed into a triangle (closing vertex at the
/// origin, as matplotlib stores it).
fn closed_head(head: Vec<Point>) -> Path {
    let mut vertices = head;
    vertices.push(Point::ORIGIN);
    let codes = vec![
        PathCode::MoveTo,
        PathCode::LineTo,
        PathCode::LineTo,
        PathCode::ClosePoly,
    ];
    Path::new(vertices, codes).expect("a closed head has four vertices")
}
