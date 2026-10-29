//! Gouraud-shaded triangles, emulated with three overlapping linear
//! gradients over a triangle of the average colour
//! (`draw_gouraud_triangles`, `_draw_gouraud_triangle`).

#![expect(
    clippy::float_cmp,
    reason = "matplotlib tests for exactly axis-parallel edges"
)]

use pltrs_core::{Affine2D, Point, Rgba};

use super::SvgRenderer;
use super::style::css;
use crate::gc::GraphicsContext;
use crate::vector::short;

/// The mean of three colours (numpy's `np.average(colors, axis=0)`).
fn average(colors: &[Rgba; 3]) -> Rgba {
    let mean = |f: fn(&Rgba) -> f64| colors.iter().map(f).sum::<f64>() / 3.0;
    Rgba::new(mean(|c| c.r), mean(|c| c.g), mean(|c| c.b), mean(|c| c.a))
}

/// Where the gradient from `p1` ends: the foot of the perpendicular from
/// `p1` onto the line through `p2` and `p3`.
fn gradient_end(p1: Point, p2: Point, p3: Point) -> Point {
    if p2.x == p3.x {
        Point::new(p2.x, p1.y)
    } else if p2.y == p3.y {
        Point::new(p1.x, p2.y)
    } else {
        let m1 = (p2.y - p3.y) / (p2.x - p3.x);
        let b1 = p2.y - m1 * p2.x;
        let m2 = -(1.0 / m1);
        let b2 = p1.y - m2 * p1.x;
        let x = (-b1 + b2) / (m1 - m2);
        Point::new(x, m2 * x + b2)
    }
}

impl SvgRenderer {
    /// Writes the colour-correction filters on the first use.
    fn write_gouraud_filters(&mut self) {
        if self.has_gouraud {
            return;
        }
        self.has_gouraud = true;
        let w = &mut self.writer;
        w.start("filter", &[("id", "colorAdd".to_owned())]);
        w.element(
            "feComposite",
            "",
            &[
                ("in", "SourceGraphic".to_owned()),
                ("in2", "BackgroundImage".to_owned()),
                ("operator", "arithmetic".to_owned()),
                ("k2", "1".to_owned()),
                ("k3", "1".to_owned()),
            ],
        );
        w.end(true);
        w.start("filter", &[("id", "colorMat".to_owned())]);
        w.element(
            "feColorMatrix",
            "",
            &[
                ("type", "matrix".to_owned()),
                (
                    "values",
                    "1 0 0 0 0 \n0 1 0 0 0 \n0 0 1 0 0 \n1 1 1 1 0 \n0 0 0 0 1 ".to_owned(),
                ),
            ],
        );
        w.end(true);
    }

    /// The three gradients of triangle `n`, each from one corner (in its
    /// colour) to the opposite edge (transparent, in the average colour).
    fn write_gradients(&mut self, n: usize, points: [Point; 3], colors: &[Rgba; 3], avg: Rgba) {
        self.writer.start("defs", &[]);
        for i in 0..3 {
            let (p1, p2, p3) = (points[i], points[(i + 1) % 3], points[(i + 2) % 3]);
            let end = gradient_end(p1, p2, p3);
            self.writer.start(
                "linearGradient",
                &[
                    ("id", format!("GR{n:x}_{i}")),
                    ("gradientUnits", "userSpaceOnUse".to_owned()),
                    ("x1", short(p1.x)),
                    ("y1", short(p1.y)),
                    ("x2", short(end.x)),
                    ("y2", short(end.y)),
                ],
            );
            self.writer.element(
                "stop",
                "",
                &[
                    ("offset", "1".to_owned()),
                    (
                        "style",
                        css(&[
                            ("stop-color", avg.to_hex(false)),
                            ("stop-opacity", short(colors[i].a)),
                        ]),
                    ),
                ],
            );
            self.writer.element(
                "stop",
                "",
                &[
                    ("offset", "0".to_owned()),
                    (
                        "style",
                        css(&[
                            ("stop-color", colors[i].to_hex(false)),
                            ("stop-opacity", "0".to_owned()),
                        ]),
                    ),
                ],
            );
            self.writer.end(true);
        }
        self.writer.end(true);
    }

    /// One triangle in SVG coordinates.
    fn gouraud_triangle(&mut self, points: [Point; 3], colors: &[Rgba; 3]) {
        let avg = average(colors);
        if avg.a == 0.0 {
            return;
        }
        let n = self.gradients;
        self.write_gradients(n, points, colors, avg);
        // matplotlib's loop leaves the corners rotated: the path starts at
        // the third.
        let [a, b, c] = [points[2], points[0], points[1]];
        let d = format!(
            "M {},{} L {},{} {},{} Z",
            short(a.x),
            short(a.y),
            short(b.x),
            short(b.y),
            short(c.x),
            short(c.y)
        );
        let crisp = || ("shape-rendering", "crispEdges".to_owned());
        self.writer.element(
            "path",
            "",
            &[
                ("d", d.clone()),
                ("fill", avg.to_hex(false)),
                ("fill-opacity", "1".to_owned()),
                crisp(),
            ],
        );
        self.writer.start(
            "g",
            &[
                ("stroke", "none".to_owned()),
                ("stroke-width", "0".to_owned()),
                crisp(),
                ("filter", "url(#colorMat)".to_owned()),
            ],
        );
        self.writer.element(
            "path",
            "",
            &[
                ("d", d.clone()),
                ("fill", format!("url(#GR{n:x}_0)")),
                crisp(),
            ],
        );
        for i in 1..3 {
            self.writer.element(
                "path",
                "",
                &[
                    ("d", d.clone()),
                    ("fill", format!("url(#GR{n:x}_{i})")),
                    ("filter", "url(#colorAdd)".to_owned()),
                    crisp(),
                ],
            );
        }
        self.writer.end(true);
        self.gradients += 1;
    }

    /// `draw_gouraud_triangles`.
    pub(super) fn svg_gouraud_triangles(
        &mut self,
        gc: &GraphicsContext,
        triangles: &[[Point; 3]],
        colors: &[[Rgba; 3]],
        transform: Affine2D,
    ) {
        let clip: Vec<_> = self.clip_attribute(gc).into_iter().collect();
        self.writer.start("g", &clip);
        self.write_gouraud_filters();
        let to_svg = transform.then(self.flip());
        for (triangle, colors) in triangles.iter().zip(colors) {
            self.gouraud_triangle(triangle.map(|p| to_svg.transform_point(p)), colors);
        }
        self.writer.end(true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_ends_are_perpendicular_feet() {
        let foot = gradient_end(
            Point::new(0.0, 0.0),
            Point::new(2.0, 0.0),
            Point::new(0.0, 2.0),
        );
        assert!((foot.x - 1.0).abs() < 1e-12 && (foot.y - 1.0).abs() < 1e-12);
        assert_eq!(
            gradient_end(
                Point::new(5.0, 1.0),
                Point::new(2.0, 0.0),
                Point::new(2.0, 9.0)
            ),
            Point::new(2.0, 1.0)
        );
        let avg = average(&[Rgba::BLACK, Rgba::WHITE, Rgba::TRANSPARENT]);
        assert!((avg.a - 2.0 / 3.0).abs() < 1e-12);
    }
}
