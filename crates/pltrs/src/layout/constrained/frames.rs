//! The conversions of constrained layout between pixels, figure fractions
//! and the fractions of a (sub)figure, with matplotlib's arithmetic: its
//! inverted `transFigure`, and the affine maps it composes for
//! `transSubfigure - transFigure` and `transFigure - transSubfigure`
//! (the identity for the figure itself), so that margins and positions
//! come out as matplotlib's to the last bit.

use pltrs_core::Bbox;

use crate::artist::ArtistId;
use crate::figure::FigureState;
use crate::transforms::TransformContext;

/// `bbox` in pixels as figure fractions, as matplotlib's inverted
/// `transFigure` maps it: times the reciprocals of the figure's size.
pub(super) fn pixels_to_figure(fig: &FigureState, bbox: &Bbox) -> Bbox {
    let (width, height) = fig.size_pixels();
    Scaling {
        sx: 1.0 / width,
        tx: 0.0,
        sy: 1.0 / height,
        ty: 0.0,
    }
    .apply(bbox)
}

/// The affine map `x ↦ sx·x + tx`, `y ↦ sy·y + ty`, rounded as
/// matplotlib's `affine_transform` rounds it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Scaling {
    sx: f64,
    tx: f64,
    sy: f64,
    ty: f64,
}

impl Scaling {
    fn apply(&self, bbox: &Bbox) -> Bbox {
        let x = |v: f64| self.sx * v + self.tx;
        let y = |v: f64| self.sy * v + self.ty;
        Bbox::from_extents(x(bbox.x0()), y(bbox.y0()), x(bbox.x1()), y(bbox.y1()))
    }
}

/// How a frame's fractions map to figure fractions and back.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum FrameMap {
    /// The figure itself: matplotlib's subtraction of a transform from
    /// itself is the identity.
    Figure,
    /// A subfigure, whose maps compose its box in pixels with the
    /// figure's.
    Subfigure {
        to_figure: Scaling,
        to_frame: Scaling,
    },
}

impl FrameMap {
    /// The maps of the frame `id`.
    ///
    /// For a subfigure spanning `(l, b, w, h)` pixels in a figure of
    /// `W × H` pixels, `transSubfigure - transFigure` is the product of
    /// the inverted `transFigure` (whose entries numpy computes as
    /// `1/W`, `1/H`) and the subfigure's `BboxTransformTo`;
    /// `transFigure - transSubfigure` the product of the subfigure's
    /// inverse (`1/w` and `(0 - l)·(1/w)`, as numpy's `inv` gives them)
    /// and `transFigure`.
    pub(super) fn of(fig: &FigureState, id: ArtistId) -> Option<Self> {
        if id == fig.root {
            return Some(Self::Figure);
        }
        let frame = fig.frame_bbox(id)?;
        let (width, height) = fig.size_pixels();
        let (l, b, w, h) = frame.bounds();
        let to_figure = Scaling {
            sx: 1.0 / width * w,
            tx: 1.0 / width * l,
            sy: 1.0 / height * h,
            ty: 1.0 / height * b,
        };
        let to_frame = Scaling {
            sx: 1.0 / w * width,
            tx: (0.0 - l) * (1.0 / w),
            sy: 1.0 / h * height,
            ty: (0.0 - b) * (1.0 / h),
        };
        Some(Self::Subfigure {
            to_figure,
            to_frame,
        })
    }

    /// The maps of the frame holding the axes `axes`.
    pub(super) fn of_axes(fig: &FigureState, axes: ArtistId) -> Option<Self> {
        Self::of(fig, fig.store.get(axes)?.base().parent?)
    }

    /// `bbox` in the frame's fractions as figure fractions (matplotlib's
    /// `transSubfigure - transFigure`).
    pub(super) fn to_figure(self, bbox: &Bbox) -> Bbox {
        match self {
            Self::Figure => *bbox,
            Self::Subfigure { to_figure, .. } => to_figure.apply(bbox),
        }
    }

    /// `bbox` in figure fractions as fractions of the frame (matplotlib's
    /// `transFigure - transSubfigure`).
    pub(super) fn to_frame(self, bbox: &Bbox) -> Bbox {
        match self {
            Self::Figure => *bbox,
            Self::Subfigure { to_frame, .. } => to_frame.apply(bbox),
        }
    }

    /// The paddings `(w_pad, h_pad)` in the frame's fractions: the size of
    /// the box from the origin to the paddings, mapped to the frame.
    pub(super) fn local_pads(self, w_pad: f64, h_pad: f64) -> (f64, f64) {
        let pads = self.to_frame(&Bbox::from_extents(0.0, 0.0, w_pad, h_pad));
        (pads.width(), pads.height())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::{Figure, SubfigureOptions};

    #[test]
    fn frames_map_as_matplotlib_composes_them() {
        let fig = Figure::with_size(6.4, 4.8, 100.0);
        let parts = fig.subfigures(1, 3, SubfigureOptions::default()).unwrap();
        let state = fig.raw().figure_state().borrow();
        let map = FrameMap::of(&state, parts[(0, 1)].raw().id()).unwrap();
        // matplotlib 3.11: the middle one of three subfigures.
        let local = Bbox::from_extents(0.125, 0.11, 0.9, 0.88);
        let figure = map.to_figure(&local);
        assert_eq!(
            [figure.x0(), figure.x1()].map(f64::to_bits),
            [0x3FD8_0000_0000_0000, 0x3FE4_4444_4444_4444]
        );
        let back = map.to_frame(&figure);
        assert!((back.x0() - local.x0()).abs() < 1e-15);
        assert_eq!(FrameMap::Figure.to_frame(&local), local);
        assert_eq!(FrameMap::Figure.local_pads(0.1, 0.2), (0.1, 0.2));
    }
}
