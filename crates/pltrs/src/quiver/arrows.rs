//! The outline of one quiver arrow in arrow-width units (matplotlib's
//! `Quiver._h_arrows`).

use std::f64::consts::PI;

use pltrs_core::Point;

/// Which part of an arrow sits on its position.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Pivot {
    /// The tail (the arrow points away from its position).
    #[default]
    Tail,
    /// The middle of the arrow.
    Middle,
    /// The tip (the arrow points at its position).
    Tip,
}

/// Head proportions and the treatment of short arrows, all in arrow-width
/// units except the multiples `minshaft` (matplotlib's `headwidth`,
/// `headlength`, `headaxislength`, `minshaft` and `minlength`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuiverArrowShape {
    /// Head width as a multiple of the shaft width.
    pub head_width: f64,
    /// Head length as a multiple of the shaft width.
    pub head_length: f64,
    /// Head length where it meets the shaft, as a multiple of the shaft
    /// width (0 gives a triangular head).
    pub head_axis_length: f64,
    /// Arrows shorter than this many head lengths are scaled down as a
    /// whole instead of losing their shaft.
    pub min_shaft: f64,
    /// Arrows shorter than this (in widths) are drawn as a hexagonal dot
    /// of this diameter.
    pub min_length: f64,
}

impl Default for QuiverArrowShape {
    fn default() -> Self {
        Self {
            head_width: 3.0,
            head_length: 5.0,
            head_axis_length: 4.5,
            min_shaft: 1.0,
            min_length: 1.0,
        }
    }
}

/// The order in which the four half-arrow corners are visited: tail, head
/// base, head corner and tip on one side, then back on the other side.
const OUTLINE_ORDER: [usize; 8] = [0, 1, 2, 3, 2, 1, 0, 0];

/// Largest arrow length in widths; longer arrows overflow the rasterizer.
const MAX_LENGTH: f64 = 65536.0;

impl QuiverArrowShape {
    /// The eight outline vertices of a horizontal arrow `length` widths
    /// long pointing along +x, placed by `pivot`.
    pub(crate) fn outline(&self, length: f64, pivot: Pivot) -> [Point; 8] {
        let length = length.clamp(0.0, MAX_LENGTH);
        let min_shaft = self.min_shaft * self.head_length;
        // Too short for a shaft (only possible for a positive minimum):
        // shrink the whole minimal arrow instead.
        let (corners_x, shrink) = if length < min_shaft {
            let shrink = length / min_shaft;
            (self.corners_x(min_shaft).map(|x| shrink * x), shrink)
        } else {
            (self.corners_x(length), 1.0)
        };
        let half_y = [0.5, 0.5, 0.5 * self.head_width, 0.0];
        let mut outline = OUTLINE_ORDER.map(|i| Point::new(corners_x[i], half_y[i]));
        for (k, p) in outline.iter_mut().enumerate() {
            let side = if (3..7).contains(&k) { -p.y } else { p.y };
            p.y = shrink * side;
        }
        let tip = outline[3].x;
        match pivot {
            Pivot::Tail => {}
            Pivot::Middle => outline.iter_mut().for_each(|p| p.x -= 0.5 * tip),
            Pivot::Tip => outline.iter_mut().for_each(|p| p.x -= tip),
        }
        if length < self.min_length {
            return self.dot();
        }
        outline
    }

    /// The x of the tail, head base, head corner and tip of an arrow
    /// `length` widths long.
    fn corners_x(&self, length: f64) -> [f64; 4] {
        [
            0.0,
            length - self.head_axis_length,
            length - self.head_length,
            length,
        ]
    }

    /// A hexagon of diameter `min_length` centred on the position, closed
    /// by repeating its first corner twice (matplotlib's "heptagonal dot").
    fn dot(&self) -> [Point; 8] {
        std::array::from_fn(|i| {
            let theta = i as f64 * (PI / 3.0);
            Point::new(
                theta.cos() * self.min_length * 0.5,
                theta.sin() * self.min_length * 0.5,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xs(outline: &[Point; 8]) -> Vec<f64> {
        outline.iter().map(|p| p.x).collect()
    }

    #[test]
    fn long_arrows_have_a_shaft_and_a_head() {
        let outline = QuiverArrowShape::default().outline(10.0, Pivot::Tail);
        assert_eq!(xs(&outline), [0.0, 5.5, 5.0, 10.0, 5.0, 5.5, 0.0, 0.0]);
        let ys: Vec<f64> = outline.iter().map(|p| p.y).collect();
        assert_eq!(ys, [0.5, 0.5, 1.5, -0.0, -1.5, -0.5, -0.5, 0.5]);
    }

    #[test]
    fn pivots_move_the_arrow_along_its_axis() {
        let shape = QuiverArrowShape::default();
        let middle = shape.outline(10.0, Pivot::Middle);
        assert_eq!(middle[0].x, -5.0);
        assert_eq!(middle[3].x, 5.0);
        let tip = shape.outline(10.0, Pivot::Tip);
        assert_eq!(tip[3].x, 0.0);
        assert_eq!(tip[0].x, -10.0);
    }

    #[test]
    fn short_arrows_shrink_and_tiny_ones_become_dots() {
        let shape = QuiverArrowShape::default();
        // 2.5 widths is half the minimal shaft-less arrow (5 widths).
        let short = shape.outline(2.5, Pivot::Tail);
        assert_eq!(xs(&short), [0.0, 0.25, 0.0, 2.5, 0.0, 0.25, 0.0, 0.0]);
        assert_eq!(short[2].y, 0.75);
        let dot = shape.outline(0.5, Pivot::Tip);
        assert_eq!(dot[0], Point::new(0.5, 0.0));
        assert!((dot[3].x + 0.5).abs() < 1e-15);
        assert!((dot[7].x - dot[1].x).abs() < 1e-15, "closed hexagon");
    }

    #[test]
    fn degenerate_lengths() {
        let shape = QuiverArrowShape::default();
        let zero = shape.outline(0.0, Pivot::Tail);
        assert!(zero.iter().all(|p| p.x.abs() <= 0.5), "a dot");
        let huge = shape.outline(1e9, Pivot::Tail);
        assert_eq!(huge[3].x, MAX_LENGTH);
        let nan = shape.outline(f64::NAN, Pivot::Tail);
        assert!(nan[3].x.is_nan());
    }
}
