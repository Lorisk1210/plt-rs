//! The runtime warnings numpy issues as matplotlib's `Text._get_layout`
//! stacks, aligns and rotates the lines of a text. The renderers measure
//! mathtext lines as numpy scalars (`np.ceil` of the box's extents) and
//! other lines as Python floats, so the layout's arithmetic is numpy's
//! wherever a mathtext line's metrics reach it: infinities meeting there
//! (a line spacing of `inf`, a NaN-size font) warn once per operation,
//! each time matplotlib lays the text out. pltrs (Rust) lays the block out
//! with plain floats ([`layout_block`](pltrs_text::layout_block)); this
//! replays matplotlib's operations, in its order, for their warnings only,
//! where matplotlib lays a text out.

use pltrs_text::{
    BlockStyle, FontHeightMetrics, HAlign, LineMetrics, LineSpacing, RotationMode, VAlign,
};

use crate::numeric::numpy_arith;

/// A number of matplotlib's layout: its value and whether it is a numpy
/// scalar, whose arithmetic warns (a Python float's never does).
#[derive(Clone, Copy, Debug)]
struct Scalar {
    value: f64,
    numpy: bool,
}

impl Scalar {
    /// A Python float.
    const fn python(value: f64) -> Self {
        Self {
            value,
            numpy: false,
        }
    }

    /// The result of an operation on `self` and `other`: numpy's (with
    /// its warning) if either is a numpy scalar.
    fn op(self, other: Self, plain: fn(f64, f64) -> f64, numpy: fn(f64, f64) -> f64) -> Self {
        let is_numpy = self.numpy || other.numpy;
        let operation = if is_numpy { numpy } else { plain };
        Self {
            value: operation(self.value, other.value),
            numpy: is_numpy,
        }
    }

    fn add(self, other: Self) -> Self {
        self.op(other, |a, b| a + b, numpy_arith::add)
    }

    fn sub(self, other: Self) -> Self {
        self.op(other, |a, b| a - b, numpy_arith::subtract)
    }

    fn mul(self, other: Self) -> Self {
        self.op(other, |a, b| a * b, numpy_arith::multiply)
    }

    fn div(self, other: Self) -> Self {
        self.op(other, |a, b| a / b, numpy_arith::divide)
    }

    /// `self / 2` (`/ 2.0` and `/ 2` alike).
    fn half(self) -> Self {
        self.div(Self::python(2.0))
    }
}

/// Python's `max(items)`: the first item no later item is greater than.
fn py_max(items: impl IntoIterator<Item = Scalar>) -> Option<Scalar> {
    items
        .into_iter()
        .reduce(|best, item| if item.value > best.value { item } else { best })
}

/// Python's `min(items)`: the first item no later item is less than.
fn py_min(items: impl IntoIterator<Item = Scalar>) -> Option<Scalar> {
    items
        .into_iter()
        .reduce(|best, item| if item.value < best.value { item } else { best })
}

/// matplotlib's `_rotate_point`: Python's cosine and sine of the angle
/// times the coordinates (no rotation for zero).
fn rotate(angle: f64, x: Scalar, y: Scalar) -> (Scalar, Scalar) {
    if angle == 0.0 {
        return (x, y);
    }
    let radians = angle.to_radians();
    let (cos, sin) = (Scalar::python(radians.cos()), Scalar::python(radians.sin()));
    (cos.mul(x).sub(sin.mul(y)), sin.mul(x).add(cos.mul(y)))
}

/// One line as matplotlib's layout receives it: its metrics, and whether
/// they are numpy scalars (a mathtext line's).
#[derive(Clone, Copy, Debug)]
pub(super) struct MeasuredLine {
    /// Width, height and descent.
    pub(super) metrics: LineMetrics,
    /// Whether the renderer gave them as numpy scalars.
    pub(super) numpy: bool,
}

/// Issues numpy's warnings for matplotlib's `Text._get_layout` of `lines`
/// with the font's height metrics `font` and `style` (its line spacing
/// normal for TeX, as matplotlib's is). Nothing warns while every number
/// is finite, which skips the replay.
pub(super) fn warn_laying_out(lines: &[MeasuredLine], font: FontHeightMetrics, style: &BlockStyle) {
    let spacing = match style.linespacing {
        LineSpacing::Normal => 0.0,
        LineSpacing::Factor(factor) => factor,
    };
    let finite = lines.iter().all(|line| {
        let LineMetrics {
            width,
            height,
            descent,
        } = line.metrics;
        width.is_finite() && height.is_finite() && descent.is_finite()
    }) && [
        font.ascent,
        font.descent,
        font.line_gap,
        spacing,
        style.rotation,
    ]
    .iter()
    .all(|value| value.is_finite());
    if finite || !lines.iter().any(|line| line.numpy) {
        return;
    }
    replay(lines, font, style);
}

/// The stacking of `_get_layout`: per line its width, ascent and descent
/// and its baseline, then the distance from the top to the last baseline
/// and the last descent.
struct Stacked {
    lines: Vec<(Scalar, Scalar)>,
    baseline: Scalar,
    descent: Scalar,
}

/// matplotlib's loop over the lines.
fn stack(lines: &[MeasuredLine], font: FontHeightMetrics, spacing: LineSpacing) -> Stacked {
    let (min_ascent, min_descent) = (Scalar::python(font.ascent), Scalar::python(font.descent));
    let line_gap = Scalar::python(if lines.len() == 1 { 0.0 } else { font.line_gap });
    let mut this_y = Scalar::python(0.0);
    let mut stacked = Stacked {
        lines: Vec::with_capacity(lines.len()),
        baseline: this_y,
        descent: this_y,
    };
    for line in lines {
        let number = |value: f64| Scalar {
            value,
            numpy: line.numpy,
        };
        let (w, h, d) = (
            number(line.metrics.width),
            number(line.metrics.height),
            number(line.metrics.descent),
        );
        let mut a = h.sub(d);
        let mut d = d;
        match spacing {
            LineSpacing::Normal => {
                a = py_max([a, min_ascent]).unwrap_or(a).add(line_gap.half());
                d = py_max([d, min_descent]).unwrap_or(d).add(line_gap.half());
            }
            LineSpacing::Factor(factor) => {
                let line_height = Scalar::python(factor).mul(min_ascent.add(min_descent));
                let leading = line_height.sub(a.add(d));
                a = a.add(leading.half());
                d = d.add(leading.half());
            }
        }
        stacked.baseline = a.sub(this_y);
        this_y = this_y.sub(a);
        stacked.lines.push((w, this_y));
        this_y = this_y.sub(d);
        stacked.descent = d;
    }
    stacked
}

/// matplotlib's arithmetic after the loop: the box, its rotation, the
/// alignment offsets and the positions they move.
fn replay(lines: &[MeasuredLine], font: FontHeightMetrics, style: &BlockStyle) {
    let stacked = stack(lines, font, style.linespacing);
    let zero = Scalar::python(0.0);
    let Some(width) = py_max(stacked.lines.iter().map(|&(w, _)| w)) else {
        return;
    };
    let last_y = stacked.lines.last().map_or(zero, |&(_, y)| y);
    let (xmin, xmax, ymax, ymin) = (zero, width, zero, last_y.sub(stacked.descent));
    let offset_layout: Vec<(Scalar, Scalar)> = stacked
        .lines
        .iter()
        .map(|&(w, y)| match style.multialign {
            HAlign::Left => (zero, y),
            HAlign::Center => (zero.add(width.half()).sub(w.half()), y),
            HAlign::Right => (zero.add(width).sub(w), y),
        })
        .collect();
    let corners = [(xmin, ymin), (xmin, ymax), (xmax, ymax), (xmax, ymin)];
    // `size_horiz`, kept for boxes drawn around the text.
    xmax.sub(xmin);
    ymax.sub(ymin);
    let angle = style.rotation;
    let rotated = corners.map(|(x, y)| rotate(angle, x, y));
    let xs = rotated.map(|(x, _)| x);
    let ys = rotated.map(|(_, y)| y);
    let (rxmin, rxmax) = (py_min(xs).unwrap_or(zero), py_max(xs).unwrap_or(zero));
    let (rymin, rymax) = (py_min(ys).unwrap_or(zero), py_max(ys).unwrap_or(zero));
    let width_rot = rxmax.sub(rxmin);
    let height_rot = rymax.sub(rymin);
    let (offset_x, offset_y) = if style.rotation_mode == RotationMode::Anchor {
        let ((x0, y0), (x1, y1)) = (corners[0], corners[2]);
        let ox = match style.halign {
            HAlign::Left => x0,
            HAlign::Right => x1,
            HAlign::Center => x0.add(x1).half(),
        };
        let oy = match style.valign {
            VAlign::Bottom => y0,
            VAlign::Top => y1,
            VAlign::Center => y0.add(y1).half(),
            VAlign::Baseline => y1.sub(stacked.baseline),
            VAlign::CenterBaseline => y1.sub(stacked.baseline.half()),
        };
        rotate(angle, ox, oy)
    } else {
        let (halign, valign) = style.rotated_alignment();
        let ox = match halign {
            HAlign::Left => rxmin,
            HAlign::Right => rxmax,
            HAlign::Center => rxmin.add(rxmax).half(),
        };
        let oy = match valign {
            VAlign::Bottom => rymin,
            VAlign::Top => rymax,
            VAlign::Center => rymin.add(rymax).half(),
            VAlign::Baseline => rymin.add(stacked.descent),
            VAlign::CenterBaseline => rymin.add(height_rot).sub(stacked.baseline.half()),
        };
        (ox, oy)
    };
    let (x0, y0) = (rxmin.sub(offset_x), rymin.sub(offset_y));
    // `Bbox.from_bounds`, which takes the far corner as `x0 + width`.
    x0.add(width_rot);
    y0.add(height_rot);
    for (x, y) in offset_layout {
        let (x, y) = rotate(angle, x, y);
        x.sub(offset_x);
        y.sub(offset_y);
    }
    let (x, y) = rotated[0];
    x.sub(offset_x);
    y.sub(offset_y);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::warnings::capturing_runtime;

    const FONT: FontHeightMetrics = FontHeightMetrics {
        ascent: 9.0,
        descent: 2.0,
        line_gap: 1.0,
    };

    fn math(height: f64) -> MeasuredLine {
        MeasuredLine {
            metrics: LineMetrics {
                width: 16.0,
                height,
                descent: 3.0,
            },
            numpy: true,
        }
    }

    fn warned(lines: &[MeasuredLine], style: &BlockStyle) -> Vec<String> {
        capturing_runtime(|| warn_laying_out(lines, FONT, style)).1
    }

    #[test]
    fn an_infinite_spacing_of_a_math_line_warns_as_matplotlib() {
        // matplotlib 3.11: `$0^{\circ}$` at rotation 180, anchored at its
        // centre and baseline, with `linespacing=inf`.
        let style = BlockStyle {
            rotation: 180.0,
            halign: HAlign::Center,
            valign: VAlign::Baseline,
            rotation_mode: RotationMode::Anchor,
            linespacing: LineSpacing::Factor(f64::INFINITY),
            ..BlockStyle::default()
        };
        let warned = warned(&[math(13.0)], &style);
        let count = |kind: &str| warned.iter().filter(|w| w.ends_with(kind)).count();
        assert_eq!(count("scalar add"), 2, "{warned:?}");
        assert_eq!(count("scalar subtract"), 4, "{warned:?}");
        assert_eq!(warned.len(), 6, "{warned:?}");
    }

    #[test]
    fn finite_layouts_and_python_floats_warn_nothing() {
        let style = BlockStyle {
            linespacing: LineSpacing::Factor(f64::INFINITY),
            ..BlockStyle::default()
        };
        assert_eq!(
            warned(&[math(13.0)], &BlockStyle::default()),
            Vec::<String>::new()
        );
        let plain = MeasuredLine {
            numpy: false,
            ..math(13.0)
        };
        assert_eq!(warned(&[plain], &style), Vec::<String>::new());
    }
}
