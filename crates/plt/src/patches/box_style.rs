//! Box outlines drawn around a rectangle (matplotlib's `BoxStyle`), used by
//! [`FancyBboxPatch`](super::FancyBboxPatch) and the boxes of texts.

use std::str::FromStr;

use plt_core::{Affine2D, Path, PathCode, Point};

use super::StyleError;
use super::style_spec::StyleSpec;
use crate::numeric::linspace;

/// The padding of every box style when none is given.
pub const DEFAULT_PAD: f64 = 0.3;

/// The shape of a box outline, with its shape-specific parameters.
/// Lengths are fractions of the mutation size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoxShape {
    /// A rectangle.
    Square,
    /// The circle around the padded box.
    Circle,
    /// The ellipse through the corners of the padded box.
    Ellipse,
    /// A right-pointing arrow.
    RArrow(ArrowHead),
    /// A left-pointing arrow.
    LArrow(ArrowHead),
    /// A two-way arrow.
    DArrow(ArrowHead),
    /// Round corners of radius `rounding_size` (the padding when unset).
    Round {
        /// Corner radius.
        rounding_size: Option<f64>,
    },
    /// Rounded edges (by half the padding when unset).
    Round4 {
        /// Rounding of the edges.
        rounding_size: Option<f64>,
    },
    /// A sawtooth outline with teeth of `tooth_size` (half the padding
    /// when unset).
    Sawtooth {
        /// Size of the teeth.
        tooth_size: Option<f64>,
    },
    /// A rounded sawtooth outline.
    Roundtooth {
        /// Size of the teeth.
        tooth_size: Option<f64>,
    },
}

/// The head of an arrow-shaped box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArrowHead {
    /// Head width relative to the shaft width (non-negative).
    pub head_width: f64,
    /// Angle at the tip in degrees (non-zero modulo 360; negative angles
    /// point the head backwards).
    pub head_angle: f64,
}

impl Default for ArrowHead {
    fn default() -> Self {
        Self {
            head_width: 1.5,
            head_angle: 90.0,
        }
    }
}

impl ArrowHead {
    fn validated(self) -> Result<Self, StyleError> {
        if self.head_width < 0.0 {
            return Err(StyleError::InvalidValue("'head_width' must be nonnegative"));
        }
        if self.head_angle % 360.0 == 0.0 {
            return Err(StyleError::InvalidValue("'head_angle' must be nonzero"));
        }
        Ok(self)
    }
}

/// A box style: a shape and the padding around the boxed rectangle
/// (`None` uses [`DEFAULT_PAD`]; texts substitute their own default).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxStyle {
    /// The outline.
    pub shape: BoxShape,
    /// Padding as a fraction of the mutation size.
    pub pad: Option<f64>,
}

impl BoxStyle {
    /// A style of `shape` with the default padding.
    pub const fn new(shape: BoxShape) -> Self {
        Self { shape, pad: None }
    }

    /// The same style with an explicit padding.
    #[must_use]
    pub const fn with_pad(self, pad: f64) -> Self {
        Self {
            pad: Some(pad),
            ..self
        }
    }

    /// The padding in use.
    pub fn pad(&self) -> f64 {
        self.pad.unwrap_or(DEFAULT_PAD)
    }

    /// The outline around the rectangle at `(x0, y0)` of `width × height`,
    /// with lengths scaled by `mutation_size`.
    pub fn path(&self, x0: f64, y0: f64, width: f64, height: f64, mutation_size: f64) -> Path {
        let pad = self.pad() * mutation_size;
        let padded = Padded {
            x0: x0 - pad,
            y0: y0 - pad,
            dx: width + 2.0 * pad,
            dy: height + 2.0 * pad,
        };
        match self.shape {
            BoxShape::Square => padded.square(),
            BoxShape::Circle => padded.circle(),
            BoxShape::Ellipse => padded.ellipse(),
            BoxShape::RArrow(head) => padded.right_arrow(head),
            BoxShape::LArrow(head) => {
                let mut path = padded.right_arrow(head);
                for p in path.vertices_mut() {
                    p.x = 2.0 * x0 + width - p.x;
                }
                path
            }
            BoxShape::DArrow(head) => padded.double_arrow(head),
            BoxShape::Round { rounding_size } => padded.round(
                rounding_size
                    .filter(|&r| r != 0.0)
                    .map_or(pad, |r| r * mutation_size),
            ),
            BoxShape::Round4 { rounding_size } => padded.round4(
                rounding_size
                    .filter(|&r| r != 0.0)
                    .map_or(pad / 2.0, |r| r * mutation_size),
            ),
            BoxShape::Sawtooth { tooth_size } => {
                Path::closed(padded.sawtooth(self.tooth(tooth_size, mutation_size)))
            }
            BoxShape::Roundtooth { tooth_size } => {
                padded.roundtooth(self.tooth(tooth_size, mutation_size))
            }
        }
    }

    fn tooth(&self, tooth_size: Option<f64>, mutation_size: f64) -> f64 {
        tooth_size.map_or(self.pad() * 0.5 * mutation_size, |t| t * mutation_size)
    }
}

impl From<BoxShape> for BoxStyle {
    fn from(shape: BoxShape) -> Self {
        Self::new(shape)
    }
}

impl FromStr for BoxStyle {
    type Err = StyleError;

    /// Parses matplotlib's description, e.g. `"round,pad=0.2"` or
    /// `"rarrow,head_width=2"` (names are case-insensitive).
    fn from_str(text: &str) -> Result<Self, StyleError> {
        let mut spec = StyleSpec::parse(text)?;
        let pad = spec.take("pad");
        let arrow = |spec: &mut StyleSpec| {
            let defaults = ArrowHead::default();
            ArrowHead {
                head_width: spec.take("head_width").unwrap_or(defaults.head_width),
                head_angle: spec.take("head_angle").unwrap_or(defaults.head_angle),
            }
            .validated()
        };
        let shape = match spec.name.as_str() {
            "square" => BoxShape::Square,
            "circle" => BoxShape::Circle,
            "ellipse" => BoxShape::Ellipse,
            "rarrow" => BoxShape::RArrow(arrow(&mut spec)?),
            "larrow" => BoxShape::LArrow(arrow(&mut spec)?),
            "darrow" => BoxShape::DArrow(arrow(&mut spec)?),
            "round" => BoxShape::Round {
                rounding_size: spec.take("rounding_size"),
            },
            "round4" => BoxShape::Round4 {
                rounding_size: spec.take("rounding_size"),
            },
            "sawtooth" => BoxShape::Sawtooth {
                tooth_size: spec.take("tooth_size"),
            },
            "roundtooth" => BoxShape::Roundtooth {
                tooth_size: spec.take("tooth_size"),
            },
            _ => return Err(spec.unknown()),
        };
        spec.finish()?;
        Ok(Self { shape, pad })
    }
}

/// The padded rectangle every outline is built from.
struct Padded {
    x0: f64,
    y0: f64,
    dx: f64,
    dy: f64,
}

/// A closed polygon through `points` (matplotlib's `Path._create_closed`).
fn closed(points: &[(f64, f64)]) -> Path {
    Path::polygon(points.iter().map(|&(x, y)| Point::new(x, y)))
}

/// A path from `(code, x, y)` triples.
fn coded(segments: &[(PathCode, f64, f64)]) -> Path {
    let (vertices, codes) = segments
        .iter()
        .map(|&(code, x, y)| (Point::new(x, y), code))
        .unzip();
    Path::new(vertices, codes).expect("box outlines start with a move")
}

impl Padded {
    fn x1(&self) -> f64 {
        self.x0 + self.dx
    }

    fn y1(&self) -> f64 {
        self.y0 + self.dy
    }

    fn square(&self) -> Path {
        let (x0, y0, x1, y1) = (self.x0, self.y0, self.x1(), self.y1());
        closed(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
    }

    fn circle(&self) -> Path {
        let center = Point::new(self.x0 + self.dx / 2.0, self.y0 + self.dy / 2.0);
        Path::circle(center, self.dx.max(self.dy) / 2.0)
    }

    fn ellipse(&self) -> Path {
        let a = self.dx / std::f64::consts::SQRT_2;
        let b = self.dy / std::f64::consts::SQRT_2;
        Path::unit_circle().transformed(
            &Affine2D::scaling(a, b).translate(self.x0 + self.dx / 2.0, self.y0 + self.dy / 2.0),
        )
    }

    /// Mid-height, shaft edges and cotangent of the half tip angle.
    fn arrow_geometry(&self, head: ArrowHead) -> (f64, f64, f64, f64, f64) {
        let head_dy = head.head_width * self.dy;
        let mid_y = f64::midpoint(self.y0, self.y1());
        let cot = 1.0 / (head.head_angle / 2.0).to_radians().tan();
        (
            head_dy,
            mid_y,
            mid_y - head_dy / 2.0,
            mid_y + head_dy / 2.0,
            cot,
        )
    }

    fn right_arrow(&self, head: ArrowHead) -> Path {
        let (x0, y0, x1, y1, dy) = (self.x0, self.y0, self.x1(), self.y1(), self.dy);
        let (head_dy, mid_y, shaft_y0, shaft_y1, cot) = self.arrow_geometry(head);
        if cot > 0.0 {
            let tip_x = x1 + cot * dy.min(head_dy) / 2.0;
            let shaft_x = tip_x - cot * head_dy / 2.0;
            closed(&[
                (x0, y0),
                (shaft_x, y0),
                (shaft_x, shaft_y0),
                (tip_x, mid_y),
                (shaft_x, shaft_y1),
                (shaft_x, y1),
                (x0, y1),
            ])
        } else {
            let back = (-cot * (head_dy - dy).max(0.0) / 2.0).min(self.dx);
            let mid_y0 = shaft_y0.min(y0) - back / cot;
            let mid_y1 = shaft_y1.max(y1) + back / cot;
            closed(&[
                (x0, y0),
                (x1 - back, mid_y0),
                (x1, shaft_y0),
                (x1, shaft_y1),
                (x1 - back, mid_y1),
                (x0, y1),
            ])
        }
    }

    fn double_arrow(&self, head: ArrowHead) -> Path {
        let (x0, y0, x1, y1, dy) = (self.x0, self.y0, self.x1(), self.y1(), self.dy);
        let (head_dy, mid_y, shaft_y0, shaft_y1, cot) = self.arrow_geometry(head);
        if cot > 0.0 {
            let tip_x0 = x0 - cot * dy.min(head_dy) / 2.0;
            let shaft_x0 = tip_x0 + cot * head_dy / 2.0;
            let tip_x1 = x1 + cot * dy.min(head_dy) / 2.0;
            let shaft_x1 = tip_x1 - cot * head_dy / 2.0;
            closed(&[
                (shaft_x0, y1),
                (shaft_x0, shaft_y1),
                (tip_x0, mid_y),
                (shaft_x0, shaft_y0),
                (shaft_x0, y0),
                (shaft_x1, y0),
                (shaft_x1, shaft_y0),
                (tip_x1, mid_y),
                (shaft_x1, shaft_y1),
                (shaft_x1, y1),
            ])
        } else {
            let back = (-cot * (head_dy - dy).max(0.0) / 2.0).min(self.dx / 2.0);
            let mid_y0 = shaft_y0.min(y0) - back / cot;
            let mid_y1 = shaft_y1.max(y1) + back / cot;
            closed(&[
                (x0, shaft_y0),
                (x0 + back, mid_y0),
                (x1 - back, mid_y0),
                (x1, shaft_y0),
                (x1, shaft_y1),
                (x1 - back, mid_y1),
                (x0 + back, mid_y1),
                (x0, shaft_y1),
            ])
        }
    }

    /// Corners rounded by quadratic Béziers of radius `dr`.
    fn round(&self, dr: f64) -> Path {
        use PathCode::{ClosePoly, Curve3, LineTo, MoveTo};
        let (x0, y0, x1, y1) = (self.x0, self.y0, self.x1(), self.y1());
        coded(&[
            (MoveTo, x0 + dr, y0),
            (LineTo, x1 - dr, y0),
            (Curve3, x1, y0),
            (Curve3, x1, y0 + dr),
            (LineTo, x1, y1 - dr),
            (Curve3, x1, y1),
            (Curve3, x1 - dr, y1),
            (LineTo, x0 + dr, y1),
            (Curve3, x0, y1),
            (Curve3, x0, y1 - dr),
            (LineTo, x0, y0 + dr),
            (Curve3, x0, y0),
            (Curve3, x0 + dr, y0),
            (ClosePoly, x0 + dr, y0),
        ])
    }

    /// Edges bulging by cubic Béziers of rounding `dr`.
    fn round4(&self, dr: f64) -> Path {
        use PathCode::{ClosePoly, Curve4, MoveTo};
        let (x0, y0) = (self.x0 + dr, self.y0 + dr);
        let (x1, y1) = (x0 + self.dx - 2.0 * dr, y0 + self.dy - 2.0 * dr);
        coded(&[
            (MoveTo, x0, y0),
            (Curve4, x0 + dr, y0 - dr),
            (Curve4, x1 - dr, y0 - dr),
            (Curve4, x1, y0),
            (Curve4, x1 + dr, y0 + dr),
            (Curve4, x1 + dr, y1 - dr),
            (Curve4, x1, y1),
            (Curve4, x1 - dr, y1 + dr),
            (Curve4, x0 + dr, y1 + dr),
            (Curve4, x0, y1),
            (Curve4, x0 - dr, y1 - dr),
            (Curve4, x0 - dr, y0 + dr),
            (Curve4, x0, y0),
            (ClosePoly, x0, y0),
        ])
    }

    /// The sawtooth vertices, ending with a copy of the first
    /// (`Sawtooth._get_sawtooth_vertices`). Degenerate teeth (a zero or
    /// negative count) leave plain edges.
    fn sawtooth(&self, tooth_size: f64) -> Vec<Point> {
        let hsz = tooth_size / 2.0;
        let width = self.dx - tooth_size;
        let height = self.dy - tooth_size;
        let count = |extent: f64| {
            let n = ((extent - tooth_size) / (tooth_size * 2.0)).round_ties_even();
            if n.is_finite() && n > 0.0 {
                n as usize * 2
            } else {
                0
            }
        };
        let (dsx_n, dsy_n) = (count(width), count(height));
        let (x0, y0) = (self.x0 + hsz, self.y0 + hsz);
        let (x1, y1) = (x0 + width, y0 + height);
        let zigzag = |a: f64, n: usize, sign: f64| -> Vec<f64> {
            [a, a - sign * hsz, a, a + sign * hsz]
                .into_iter()
                .cycle()
                .take(4 * n)
                .take(2 * n + 2)
                .collect()
        };
        let mut xs = vec![x0];
        xs.extend(linspace(x0 + hsz, x1 - hsz, 2 * dsx_n + 1));
        xs.extend(zigzag(x1, dsy_n, -1.0));
        xs.push(x1);
        xs.extend(linspace(x1 - hsz, x0 + hsz, 2 * dsx_n + 1));
        xs.extend(zigzag(x0, dsy_n, 1.0));
        let mut ys = zigzag(y0, dsx_n, 1.0);
        ys.push(y0);
        ys.extend(linspace(y0 + hsz, y1 - hsz, 2 * dsy_n + 1));
        ys.extend(zigzag(y1, dsx_n, -1.0));
        ys.push(y1);
        ys.extend(linspace(y1 - hsz, y0 + hsz, 2 * dsy_n + 1));
        let mut points: Vec<Point> = xs
            .into_iter()
            .zip(ys)
            .map(|(x, y)| Point::new(x, y))
            .collect();
        points.push(points[0]);
        points
    }

    /// The sawtooth vertices joined by quadratic Béziers.
    fn roundtooth(&self, tooth_size: f64) -> Path {
        let mut vertices = self.sawtooth(tooth_size);
        let first = vertices[0];
        vertices.push(first);
        let curves = (vertices.len() - 1) / 2;
        let mut codes = vec![PathCode::MoveTo];
        codes.extend(std::iter::repeat_n(PathCode::Curve3, 2 * curves));
        codes.push(PathCode::ClosePoly);
        vertices.resize(codes.len(), first);
        Path::new(vertices, codes).expect("roundtooth codes match its vertices")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style(text: &str) -> BoxStyle {
        text.parse().expect("valid style")
    }

    #[test]
    fn parses_names_and_parameters() {
        assert_eq!(style("Square"), BoxStyle::new(BoxShape::Square));
        assert_eq!(
            style("round,pad=0.6,rounding_size=0.8"),
            BoxStyle::new(BoxShape::Round {
                rounding_size: Some(0.8)
            })
            .with_pad(0.6)
        );
        assert!("rarrow,head_width=-1".parse::<BoxStyle>().is_err());
        assert!("larrow,head_angle=360".parse::<BoxStyle>().is_err());
        assert!("square,rounding_size=1".parse::<BoxStyle>().is_err());
        assert!("hexagon".parse::<BoxStyle>().is_err());
    }

    #[test]
    fn square_is_padded() {
        let path = style("square,pad=0.5").path(0.0, 0.0, 4.0, 2.0, 2.0);
        let extents = path.extents();
        assert_eq!(
            (extents.x0(), extents.y0(), extents.x1(), extents.y1()),
            (-1.0, -1.0, 5.0, 3.0)
        );
        assert_eq!(path.len(), 5);
    }

    #[test]
    fn arrows_point_the_right_way() {
        let right = style("rarrow").path(0.0, 0.0, 4.0, 2.0, 1.0).extents();
        let left = style("larrow").path(0.0, 0.0, 4.0, 2.0, 1.0).extents();
        assert!((right.x1() - 5.6).abs() < 1e-12, "tip past the box");
        assert!((left.x0() + 1.6).abs() < 1e-12, "mirrored tip");
        let both = style("darrow").path(0.0, 0.0, 4.0, 2.0, 1.0).extents();
        assert!((both.x0() + 1.6).abs() < 1e-12 && (both.x1() - 5.6).abs() < 1e-12);
        let backwards = style("rarrow,head_angle=-90").path(0.0, 0.0, 4.0, 2.0, 1.0);
        assert!(backwards.extents().x1() <= 4.3 + 1e-12);
    }

    #[test]
    fn rounded_and_toothed_outlines_close() {
        for name in [
            "round",
            "round4",
            "sawtooth",
            "roundtooth",
            "circle",
            "ellipse",
        ] {
            let path = style(name).path(0.0, 0.0, 4.0, 2.0, 10.0);
            assert_eq!(path.code_at(path.len() - 1), PathCode::ClosePoly, "{name}");
        }
        let teeth = style("sawtooth").path(0.0, 0.0, 4.0, 2.0, 10.0);
        assert!(teeth.len() > 20);
    }

    #[test]
    fn degenerate_teeth_leave_plain_edges() {
        let flat = style("sawtooth,pad=0").path(0.0, 0.0, 4.0, 2.0, 10.0);
        assert!(flat.vertices().iter().all(|p| p.is_finite()));
        let tiny = style("roundtooth").path(0.0, 0.0, 0.0, 0.0, 1.0);
        assert_eq!(tiny.code_at(tiny.len() - 1), PathCode::ClosePoly);
    }
}
