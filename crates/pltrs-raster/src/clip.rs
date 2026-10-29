//! Clip rectangles and clip masks.

use crate::mask::Mask;

/// An integer pixel rectangle `[x0, x1) × [y0, y1)` in device space
/// (y down) that restricts drawing. It may extend beyond the target.
///
/// For path rendering the box clips the geometry itself (Agg's rasterizer
/// clip box, whose far edges are exclusive). matplotlib's `RendererAgg`
/// applies the *same* rectangle to markers and unmasked images through
/// Agg's `renderer_base`, which treats the far edges inclusively; use
/// [`ClipBox::inclusive`] to reproduce that.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClipBox {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

impl ClipBox {
    /// The box spanned by two corners (in any order).
    pub fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        Self {
            x0: x0.min(x1),
            y0: y0.min(y1),
            x1: x0.max(x1),
            y1: y0.max(y1),
        }
    }

    /// The whole of a `width × height` target.
    pub fn full(width: u32, height: u32) -> Self {
        Self::new(0, 0, saturate(width), saturate(height))
    }

    /// Converts a matplotlib clip rectangle — display coordinates with y
    /// up, `(x0, y0)` lower left — exactly as `RendererAgg::set_clipbox`
    /// does: edges are rounded with `floor(v + 0.5)`, then the left and
    /// bottom edges are raised to at least 0 and the right and top edges
    /// lowered to at most the canvas size.
    pub fn from_matplotlib_cliprect(
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        width: u32,
        height: u32,
    ) -> Self {
        let round = |v: f64| (v + 0.5).floor() as i32;
        let h = f64::from(height);
        Self::new(
            round(x0).max(0),
            round(h - y0).max(0),
            round(x1).min(saturate(width)),
            round(h - y1).min(saturate(height)),
        )
    }

    /// The box grown by one pixel to the right and bottom, i.e. with the
    /// far edges included as Agg's `renderer_base::clip_box` treats them.
    #[must_use]
    pub fn inclusive(self) -> Self {
        Self {
            x1: self.x1.saturating_add(1),
            y1: self.y1.saturating_add(1),
            ..self
        }
    }

    /// Left edge (inclusive).
    pub const fn x0(&self) -> i32 {
        self.x0
    }

    /// Top edge (inclusive).
    pub const fn y0(&self) -> i32 {
        self.y0
    }

    /// Right edge (exclusive).
    pub const fn x1(&self) -> i32 {
        self.x1
    }

    /// Bottom edge (exclusive).
    pub const fn y1(&self) -> i32 {
        self.y1
    }

    /// The intersection with a `width × height` target, if not empty.
    pub(crate) fn within(self, width: u32, height: u32) -> Option<Self> {
        let b = Self {
            x0: self.x0.max(0),
            y0: self.y0.max(0),
            x1: self.x1.min(saturate(width)),
            y1: self.y1.min(saturate(height)),
        };
        (b.x0 < b.x1 && b.y0 < b.y1).then_some(b)
    }
}

fn saturate(v: u32) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
}

/// The clipping applied to one drawing operation.
///
/// `mask`, when present, must have the target's dimensions; each pixel's
/// anti-aliasing cover is scaled by it as Agg's `alpha_mask_gray8` does
/// (`(255 + cover·mask) >> 8`).
#[derive(Clone, Copy, Debug, Default)]
pub struct Clip<'a> {
    /// Optional clip rectangle; `None` means the whole target.
    pub rect: Option<ClipBox>,
    /// Optional clip mask (a rasterized clip path).
    pub mask: Option<&'a Mask>,
}

impl<'a> Clip<'a> {
    /// No clipping beyond the target's bounds.
    pub const NONE: Clip<'static> = Clip {
        rect: None,
        mask: None,
    };

    /// Clipping to a rectangle.
    pub const fn rect(rect: ClipBox) -> Self {
        Self {
            rect: Some(rect),
            mask: None,
        }
    }

    /// The same clip with a mask added.
    #[must_use]
    pub const fn with_mask(self, mask: &'a Mask) -> Self {
        Self {
            mask: Some(mask),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalises_and_intersects() {
        let b = ClipBox::new(5, 7, 1, 2);
        assert_eq!((b.x0(), b.y0(), b.x1(), b.y1()), (1, 2, 5, 7));
        assert_eq!(b.within(3, 10), Some(ClipBox::new(1, 2, 3, 7)));
        assert_eq!(ClipBox::new(4, 4, 6, 6).within(3, 3), None);
        assert_eq!(b.inclusive().x1(), 6);
    }

    #[test]
    fn matplotlib_cliprect_rounding() {
        // Display box (10.4, 5.5)–(30.5, 20.2) on a 40×30 canvas.
        let b = ClipBox::from_matplotlib_cliprect(10.4, 5.5, 30.5, 20.2, 40, 30);
        assert_eq!((b.x0(), b.y0(), b.x1(), b.y1()), (10, 10, 31, 25));
        let full = ClipBox::from_matplotlib_cliprect(-5.0, -5.0, 100.0, 100.0, 40, 30);
        assert_eq!(full, ClipBox::new(0, -70, 40, 35));
        assert_eq!(full.within(40, 30), Some(ClipBox::full(40, 30)));
    }

    #[test]
    fn clip_builders() {
        let mask = Mask::new(2, 2).unwrap();
        let clip = Clip::rect(ClipBox::full(2, 2)).with_mask(&mask);
        assert!(clip.mask.is_some());
        assert!(Clip::NONE.rect.is_none());
    }
}
