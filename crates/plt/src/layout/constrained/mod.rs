//! Constrained layout (matplotlib's `_constrained_layout.py`): margins
//! around every grid cell as large as the decorations of its axes, room
//! for colorbars, and the cells sharing what is left.
//!
//! Subfigures get grids of their own, nested in the cells of their
//! parent's grid, so their titles, labels and axes are laid out too.
//!
//! The pass runs twice, because decorations change size once the axes
//! moved (tick labels come and go). Each time it measures every grid-placed
//! axes and its colorbars, raises the margins of their cells, makes
//! submerged interior margins match, solves the grids and moves the
//! subfigures, axes and colorbars into their cells, unless some cell
//! collapsed.

mod colorbars;
mod compress;
mod margins;
mod reposition;
mod tree;

use crate::figure::FigureState;
use crate::rc::RcParams;
use tree::{FIGURE, LayoutTree};

/// The parameters of constrained layout (matplotlib's
/// `ConstrainedLayoutEngine`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConstrainedLayout {
    /// Padding left and right of the decorations, in inches.
    pub w_pad: f64,
    /// Padding above and below the decorations, in inches.
    pub h_pad: f64,
    /// Space between columns, as a fraction of the figure width shared
    /// among the gaps (used when larger than the padding).
    pub wspace: f64,
    /// Space between rows, as a fraction of the figure height.
    pub hspace: f64,
    /// The part of the figure laid out, `[left, bottom, width, height]` in
    /// figure fractions.
    pub rect: [f64; 4],
    /// Close the gaps fixed-aspect axes leave in their cells afterwards
    /// (matplotlib's `layout="compressed"`).
    pub compress: bool,
}

impl ConstrainedLayout {
    /// The `figure.constrained_layout.*` rc parameters over the whole
    /// figure.
    pub fn from_rc(rc: &RcParams) -> Self {
        Self {
            w_pad: rc.figure_constrained_layout_w_pad,
            h_pad: rc.figure_constrained_layout_h_pad,
            wspace: rc.figure_constrained_layout_wspace,
            hspace: rc.figure_constrained_layout_hspace,
            rect: [0.0, 0.0, 1.0, 1.0],
            compress: false,
        }
    }
}

impl Default for ConstrainedLayout {
    /// [`ConstrainedLayout::from_rc`] with the current rc parameters.
    fn default() -> Self {
        Self::from_rc(&crate::rc::params())
    }
}

/// The paddings in figure fractions and the spacings.
#[derive(Clone, Copy, Debug)]
struct Pads {
    w_pad: f64,
    h_pad: f64,
    wspace: f64,
    hspace: f64,
}

/// matplotlib's `do_constrained_layout` on the figure.
pub(super) fn execute(fig: &mut FigureState, params: &ConstrainedLayout) {
    let (width, height) = fig.size_inches;
    let pads = Pads {
        w_pad: params.w_pad / width,
        h_pad: params.h_pad / height,
        wspace: params.wspace,
        hspace: params.hspace,
    };
    let Some(mut tree) = LayoutTree::new(fig, params.rect) else {
        return;
    };
    for _ in 0..2 {
        tree.make_layout_margins(fig, FIGURE, &pads);
        tree.make_margin_suptitles(fig, FIGURE, &pads);
        tree.match_submerged_margins(FIGURE);
        tree.grids.solve();
        if tree.no_collapsed_axes() {
            tree.reposition_axes(fig, FIGURE, false);
            if params.compress && tree.compress_fixed_aspect(fig) {
                tree.grids.solve();
                if tree.no_collapsed_axes() {
                    tree.reposition_axes(fig, FIGURE, true);
                }
                tree.place_suptitle(fig, pads.h_pad);
            }
        }
        tree.reset_margins();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn titles_labels_and_suptitle_get_room() {
        crate::rc::context(titles_labels_and_suptitle_get_room_unhinted);
    }

    /// The body of `titles_labels_and_suptitle_get_room` under the spec's
    /// reference configuration, which turns text hinting off.
    fn titles_labels_and_suptitle_get_room_unhinted() {
        crate::rc::set("text.hinting", "none").expect("valid setting");
        let fig = Figure::new();
        fig.set_layout_engine(Some(super::super::LayoutEngine::Constrained(
            ConstrainedLayout::from_rc(&RcParams::matplotlib()),
        )));
        let axs = fig.subplots(2, 2).unwrap();
        for (i, ax) in axs.as_slice().iter().enumerate() {
            ax.set_xlim(0.0, 10.0).set_ylim(-1.0, 1.0);
            ax.set_title(format!("title {i}"));
            ax.set_xlabel("x label").set_ylabel("y label");
        }
        fig.suptitle("suptitle");
        fig.layout();
        // matplotlib 3.11 under the spec's reference configuration.
        let expected = [
            [0.104_818, 0.572_328, 0.374_852, 0.323_126],
            [0.604_818, 0.572_328, 0.374_852, 0.323_126],
            [0.104_818, 0.098_38, 0.374_852, 0.323_126],
            [0.604_818, 0.098_38, 0.374_852, 0.323_126],
        ];
        for (ax, expected) in axs.as_slice().iter().zip(expected) {
            let (x, y, w, h) = ax.position().bounds();
            for (actual, expected) in [x, y, w, h].into_iter().zip(expected) {
                assert!((actual - expected).abs() < 1e-6, "{actual} vs {expected}");
            }
        }
    }

    #[test]
    fn empty_suptitles_take_no_room() {
        let positions = |suptitle: Option<&str>| {
            let fig = Figure::new();
            fig.set_layout_engine(Some(super::super::LayoutEngine::Constrained(
                ConstrainedLayout::from_rc(&RcParams::matplotlib()),
            )));
            let ax = fig.subplot();
            if let Some(text) = suptitle {
                fig.suptitle(text);
            }
            fig.layout();
            ax.position()
        };
        assert_eq!(positions(Some("")), positions(None));
        assert!(positions(Some("title")).y1() < positions(None).y1());
    }
}
