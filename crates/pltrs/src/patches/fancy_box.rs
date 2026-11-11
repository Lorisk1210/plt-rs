//! `FancyBboxPatch`: a styled box around a rectangle.

use pltrs_core::{Path, Point};

use super::PatchShape;
use super::box_style::{BoxShape, BoxStyle};
use crate::numeric::numpy_arith::Exceptions;

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

pltrs_core::archive_struct!(FancyBboxPatch {
    xy,
    width,
    height,
    boxstyle,
    mutation_scale,
    mutation_aspect
});

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

    fn runs_program(&self) -> bool {
        matches!(self.boxstyle.shape, BoxShape::Program(_))
    }

    fn path(&self) -> Path {
        let mut path = self.squeezed_path();
        for p in path.vertices_mut() {
            p.y *= self.mutation_aspect;
        }
        path
    }

    fn warn_drawing(&self) {
        self.warn_making_path();
    }

    /// matplotlib's `get_path` stretches the box back with numpy
    /// (`vertices * [1, aspect]`), which warns where an infinite aspect
    /// meets a coordinate it cannot multiply (zero times infinity). A
    /// style of the program's is not asked for its box again.
    fn warn_making_path(&self) {
        let aspect = self.mutation_aspect;
        if !aspect.is_infinite() || self.runs_program() {
            return;
        }
        let mut exceptions = Exceptions::default();
        for p in self.squeezed_path().vertices() {
            exceptions.note(p.y * aspect, p.y, aspect);
        }
        exceptions.warn("multiply");
    }
}

impl FancyBboxPatch {
    /// The box of the style around the rectangle with its y coordinates
    /// squeezed by the mutation aspect, which [`path`](PatchShape::path)
    /// stretches back.
    fn squeezed_path(&self) -> Path {
        let aspect = self.mutation_aspect;
        self.boxstyle.path(
            self.xy.x,
            self.xy.y / aspect,
            self.width,
            self.height / aspect,
            self.mutation_scale,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_scales_the_padding() {
        let square: BoxStyle = "square,pad=0.5".parse().unwrap();
        let unit = FancyBboxPatch::new((0.0, 0.0), 2.0, 2.0, square.clone()).path();
        assert_eq!(unit.extents().x0(), -0.5);
        let doubled = FancyBboxPatch::new((0.0, 0.0), 2.0, 2.0, square)
            .with_mutation_scale(2.0)
            .path();
        assert_eq!(doubled.extents().x0(), -1.0);
    }

    #[test]
    fn an_infinite_aspect_warns_as_numpy_multiplies() {
        let square: BoxStyle = "square,pad=0".parse().unwrap();
        let warned = |aspect: f64| {
            let patch = FancyBboxPatch::new((0.0, 0.0), 2.0, 4.0, square.clone())
                .with_mutation_aspect(aspect);
            crate::warnings::capturing_runtime(|| patch.warn_making_path()).1
        };
        assert_eq!(warned(3.0), [""; 0]);
        assert_eq!(warned(f64::NAN), [""; 0]);
        // The bottom of the box, squeezed to zero, is stretched back.
        assert_eq!(
            warned(f64::INFINITY),
            ["invalid value encountered in multiply"]
        );
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
