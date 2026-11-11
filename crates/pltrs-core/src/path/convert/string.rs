//! matplotlib's `_path.convert_to_string`: the path serialisation shared
//! by the SVG, PDF and PS backends.

use std::fmt::Write as _;

use super::{SketchParams, Vertex, VertexSource};
use crate::numeric::fused;
use crate::path::{Path, PathCode};
use crate::{Affine2D, Bbox, Point};

/// The tokens written for each command, in matplotlib's order
/// `[MOVETO, LINETO, CURVE3, CURVE4, CLOSEPOLY]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StringCodes<'a> {
    /// Token for moves.
    pub move_to: &'a str,
    /// Token for lines.
    pub line_to: &'a str,
    /// Token for quadratic curves; empty to write them as cubics.
    pub quad_to: &'a str,
    /// Token for cubic curves.
    pub cubic_to: &'a str,
    /// Token for closures.
    pub close: &'a str,
}

impl StringCodes<'static> {
    /// The SVG backend's tokens (prefix notation).
    pub const SVG: Self = Self {
        move_to: "M",
        line_to: "L",
        quad_to: "Q",
        cubic_to: "C",
        close: "z",
    };
    /// The PDF backend's operators (postfix notation).
    pub const PDF: Self = Self {
        move_to: "m",
        line_to: "l",
        quad_to: "",
        cubic_to: "c",
        close: "h",
    };
    /// The PostScript backend's procedures (postfix notation).
    pub const PS: Self = Self {
        move_to: "m",
        line_to: "l",
        quad_to: "",
        cubic_to: "c",
        close: "cl",
    };
}

/// The parameters of `convert_to_string`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StringOptions<'a> {
    /// Applied to every vertex.
    pub transform: Affine2D,
    /// Clip lines to this rectangle (grown by one unit); ignored unless
    /// `x0 < x1` and `y0 < y1`.
    pub clip: Option<Bbox>,
    /// Simplify with this threshold.
    pub simplify: Option<f64>,
    /// Apply the sketch filter (flattens curves).
    pub sketch: Option<SketchParams>,
    /// Digits after the decimal point (`%.Nf`, then trailing zeros and a
    /// trailing point are removed).
    pub precision: usize,
    /// Command tokens.
    pub codes: StringCodes<'a>,
    /// Write the token after the coordinates instead of before.
    pub postfix: bool,
}

impl Default for StringOptions<'static> {
    /// The SVG backend's settings.
    fn default() -> Self {
        Self {
            transform: Affine2D::IDENTITY,
            clip: None,
            simplify: None,
            sketch: None,
            precision: 6,
            codes: StringCodes::SVG,
            postfix: false,
        }
    }
}

impl Path {
    /// matplotlib's `_path.convert_to_string`: transform, NaN removal,
    /// clipping, simplification and optional sketching, then one line per
    /// command (`"M 1 2\n"`, or `"1 2 m\n"` in postfix mode).
    ///
    /// matplotlib's "malformed path codes" error cannot occur: the NaN
    /// remover always regroups a curve's vertices under its first code.
    pub fn convert_to_string(&self, options: &StringOptions<'_>) -> String {
        let clip = options.clip.filter(|r| r.x0() < r.x1() && r.y0() < r.y1());
        let sketch = options.sketch.filter(|s| s.scale != 0.0);
        let mut stream = self
            .iter_vertices()
            .transform(options.transform)
            .remove_nans(true, self.codes.is_some())
            .clip_to(clip)
            .simplify(options.simplify)
            .flatten_curves(sketch.is_some())
            .sketch(sketch);
        let mut out = String::new();
        let mut last = Point::ORIGIN;
        while let Some(Vertex { code, point }) = stream.next() {
            if code == PathCode::ClosePoly {
                out.push_str(options.codes.close);
            } else {
                let mut points = [point; 3];
                let count = code.vertex_count();
                for slot in &mut points[1..count] {
                    *slot = stream.next().map_or(Point::ORIGIN, |v| v.point);
                }
                let (token, points, count) = command(code, &options.codes, last, points, count);
                let points = &points[..count];
                if !options.postfix {
                    out.push_str(token);
                    out.push(' ');
                }
                for p in points {
                    push_number(&mut out, p.x, options.precision);
                    out.push(' ');
                    push_number(&mut out, p.y, options.precision);
                    out.push(' ');
                }
                if options.postfix {
                    out.push_str(token);
                }
                last = *points.last().expect("every drawing command has a point");
            }
            out.push('\n');
        }
        out
    }
}

/// The token and coordinates written for one command (the first `count`
/// points); quadratic curves become cubics when the quadratic token is
/// empty.
fn command<'a>(
    code: PathCode,
    codes: &StringCodes<'a>,
    last: Point,
    points: [Point; 3],
    count: usize,
) -> (&'a str, [Point; 3], usize) {
    match code {
        PathCode::MoveTo => (codes.move_to, points, count),
        PathCode::ClosePoly => (codes.close, points, 0),
        PathCode::LineTo => (codes.line_to, points, count),
        PathCode::Curve3 if codes.quad_to.is_empty() => {
            let [control, end, _] = points;
            let c1 = Point::new(
                fused(2.0 / 3.0, control.x - last.x, last.x),
                fused(2.0 / 3.0, control.y - last.y, last.y),
            );
            let c2 = Point::new(
                fused(1.0 / 3.0, end.x - last.x, c1.x),
                fused(1.0 / 3.0, end.y - last.y, c1.y),
            );
            (codes.cubic_to, [c1, c2, end], 3)
        }
        PathCode::Curve3 => (codes.quad_to, points, count),
        PathCode::Curve4 => (codes.cubic_to, points, count),
    }
}

/// Writes `value` like matplotlib's `__add_number`: correctly rounded to
/// `digits` decimals (ties to even), without trailing zeros.
fn push_number(out: &mut String, value: f64, digits: usize) {
    let start = out.len();
    // Writing to a String cannot fail.
    let _ = write!(out, "{value:.digits$}");
    if out[start..].contains('.') {
        let trimmed = out.trim_end_matches('0').trim_end_matches('.').len();
        out.truncate(trimmed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn svg_strings() {
        let path = Path::builder()
            .move_to(p(0.0, 0.0))
            .line_to(p(1.5, 2.25))
            .quad_to(p(3.0, 3.0), p(4.0, 0.0))
            .close()
            .build();
        let svg = path.convert_to_string(&StringOptions::default());
        assert_eq!(svg, "M 0 0 \nL 1.5 2.25 \nQ 3 3 4 0 \nz\n");
    }

    #[test]
    fn pdf_strings_write_quadratics_as_cubics() {
        let path = Path::builder()
            .move_to(p(0.0, 0.0))
            .quad_to(p(3.0, 3.0), p(3.0, 0.0))
            .build();
        let options = StringOptions {
            codes: StringCodes::PDF,
            postfix: true,
            ..StringOptions::default()
        };
        let pdf = path.convert_to_string(&options);
        assert_eq!(pdf, "0 0 m\n2 2 3 2 3 0 c\n");
    }

    #[test]
    fn numbers_are_trimmed() {
        let mut s = String::new();
        for v in [1.0, -0.5, 1e-7, 100.0, 2.000_000_4, -0.0, 0.007_812_5] {
            push_number(&mut s, v, 6);
            s.push(' ');
        }
        assert_eq!(s, "1 -0.5 0 100 2 -0 0.007812 ");
        s.clear();
        // Exact ties round to even, like Python's dtoa.
        for v in [100.0, 0.5, 1.5, 2.5] {
            push_number(&mut s, v, 0);
            s.push(' ');
        }
        push_number(&mut s, 0.125, 2);
        assert_eq!(s, "100 0 2 2 0.12");
    }

    #[test]
    fn curve_vertices_are_regrouped_like_matplotlib() {
        // The closure's vertex is consumed as the cubic's end point.
        let path = Path::new(
            vec![p(0.0, 0.0), p(1.0, 1.0), p(2.0, 2.0), p(3.0, 3.0)],
            vec![
                PathCode::MoveTo,
                PathCode::Curve4,
                PathCode::Curve4,
                PathCode::ClosePoly,
            ],
        )
        .expect("valid codes");
        assert_eq!(
            path.convert_to_string(&StringOptions::default()),
            "M 0 0 \nC 1 1 2 2 3 3 \n"
        );
    }
}
