//! `FancyBboxPatch`: a styled box around a rectangle.

use pltrs_core::{Path, Point};

use super::PatchShape;
use super::box_style::{BoxShape, BoxStyle};

/// A box of [`BoxStyle`] drawn around the rectangle at `xy` (matplotlib's
/// `FancyBboxPatch`); draws an edge by default.
#[derive(Clone, Debug, PartialEq)]
pub struct FancyBboxPatch {
    /// Lower-left corner of the boxed rectangle.
    pub xy: Point,
    /// Width of the boxed rectangle.
    pub width: f64,
    /// Height of the boxed rectangle.
    pub height: f64,
    /// The outline.
    pub boxstyle: BoxStyle,
    /// Scale of the style's lengths (padding, rounding, ...).
    pub mutation_scale: f64,
    /// The rectangle's height is divided by this before the box is built
    /// and the box is stretched back by it, so that horizontal and vertical
    /// padding can differ.
    pub mutation_aspect: f64,
}

impl FancyBboxPatch {
    /// A `boxstyle` box around the rectangle at `xy`, with unit mutation
    /// scale and aspect.
    pub fn new(xy: impl Into<Point>, width: f64, height: f64, boxstyle: BoxStyle) -> Self {
        Self {
            xy: xy.into(),
            width,
            height,
            boxstyle,
            mutation_scale: 1.0,
            mutation_aspect: 1.0,
        }
    }

    /// matplotlib's default box: round corners.
    pub fn round(xy: impl Into<Point>, width: f64, height: f64) -> Self {
        Self::new(
            xy,
            width,
            height,
            BoxStyle::new(BoxShape::Round {
                rounding_size: None,
            }),
        )
    }

    /// The same box with another mutation scale.
    #[must_use]
    pub fn with_mutation_scale(self, mutation_scale: f64) -> Self {
        Self {
            mutation_scale,
            ..self
        }
    }

    /// The same box with another mutation aspect.
    #[must_use]
    pub fn with_mutation_aspect(self, mutation_aspect: f64) -> Self {
        Self {
            mutation_aspect,
            ..self
        }
    }
}

impl PatchShape for FancyBboxPatch {
    const EDGE_DEFAULT: bool = true;

    fn path(&self) -> Path {
        let aspect = self.mutation_aspect;
        let mut path = self.boxstyle.path(
            self.xy.x,
            self.xy.y / aspect,
            self.width,
            self.height / aspect,
            self.mutation_scale,
        );
        for p in path.vertices_mut() {
            p.y *= aspect;
        }
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_scales_the_padding() {
        let square: BoxStyle = "square,pad=0.5".parse().unwrap();
        let unit = FancyBboxPatch::new((0.0, 0.0), 2.0, 2.0, square).path();
        assert_eq!(unit.extents().x0(), -0.5);
        let doubled = FancyBboxPatch::new((0.0, 0.0), 2.0, 2.0, square)
            .with_mutation_scale(2.0)
            .path();
        assert_eq!(doubled.extents().x0(), -1.0);
    }

    #[test]
    fn aspect_stretches_vertical_padding() {
        let square: BoxStyle = "square,pad=0.5".parse().unwrap();
        let tall = FancyBboxPatch::new((0.0, 3.0), 2.0, 4.0, square)
            .with_mutation_aspect(3.0)
            .path()
            .extents();
        assert_eq!((tall.x0(), tall.y0()), (-0.5, 1.5));
        assert!((tall.y1() - 8.5).abs() < 1e-12);
    }
}
