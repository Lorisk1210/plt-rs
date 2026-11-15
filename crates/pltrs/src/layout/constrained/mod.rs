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
//! collapsed. The artists the program measures (see
//! [`ArtistPainter::measure`](crate::artist::ArtistPainter::measure)) are
//! measured again before the second pass, where the first left the axes.

mod colorbars;
mod compress;
mod frames;
mod margins;
mod reposition;
mod tree;

use super::kiwi::SolverError;
use crate::figure::FigureState;
use crate::numeric::numpy_arith;
use crate::rc::RcParams;
use tree::FIGURE;

pub(super) use tree::LayoutTree;

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

pltrs_core::archive_struct!(ConstrainedLayout {
    w_pad,
    h_pad,
    wspace,
    hspace,
    rect,
    compress
});

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

/// matplotlib's `do_constrained_layout` on the figure: the grids laid
/// out, `None` when no axes is on a grid, the figure has no area or the
/// solver failed.
pub(super) fn execute(fig: &mut FigureState, params: &ConstrainedLayout) -> Option<LayoutTree> {
    let (width, height) = fig.size_inches;
    // matplotlib divides the pads by the figure's size with numpy, which
    // warns of a size of zero.
    let pads = Pads {
        w_pad: numpy_arith::divide(params.w_pad, width),
        h_pad: numpy_arith::divide(params.h_pad, height),
        wspace: params.wspace,
        hspace: params.hspace,
    };
    let solved = LayoutTree::new(fig, params.rect).and_then(|tree| {
        let Some(mut tree) = tree else {
            if !fig.all_axes().is_empty() {
                crate::warnings::warn(
                    "There are no gridspecs with layoutgrids. Possibly did not call parent \
                     GridSpec with the \"figure\" keyword",
                );
            }
            return Ok(None);
        };
        if width == 0.0 || height == 0.0 {
            // matplotlib measures the axes through the inverse of the
            // figure's transform, which a figure of no area has not: its
            // draw swallows that error, and the layout ends.
            return Ok(None);
        }
        lay_out(&mut tree, fig, params, &pads)?;
        Ok(Some(tree))
    });
    // kiwisolver's exceptions end matplotlib's layout, out of its draw.
    solved.unwrap_or_else(|error| {
        crate::warnings::warn(&format!("constrained_layout not applied: {error}"));
        None
    })
}

/// The two rounds of margins, solution and placement of
/// `do_constrained_layout`.
fn lay_out(
    tree: &mut LayoutTree,
    fig: &mut FigureState,
    params: &ConstrainedLayout,
    pads: &Pads,
) -> Result<(), SolverError> {
    for pass in 0..2 {
        if pass > 0 {
            fig.measure_painters_lent();
        }
        if tree.measures_axes_placed_nowhere(fig) {
            return Ok(());
        }
        tree.make_layout_margins(fig, FIGURE, pads)?;
        tree.make_margin_suptitles(fig, FIGURE, pads)?;
        if super::failures::failed() {
            // The program's code failed measuring the figure, which ends
            // matplotlib's pass there.
            return Ok(());
        }
        tree.match_submerged_margins(FIGURE)?;
        tree.grids.solve();
        if tree.no_collapsed_axes() {
            tree.reposition_axes(fig, FIGURE, false);
            if params.compress {
                // matplotlib raises when the axes do not share one grid
                // specification, and its draw swallows the error: the
                // layout ends with this first placement.
                if !tree.compress_fixed_aspect(fig)? {
                    return Ok(());
                }
                tree.grids.solve();
                if tree.no_collapsed_axes() {
                    tree.reposition_axes(fig, FIGURE, true);
                } else {
                    warn_collapsed();
                }
                tree.place_suptitle(fig, pads.h_pad);
            }
        } else {
            warn_collapsed();
        }
        tree.reset_margins()?;
    }
    Ok(())
}

/// matplotlib's warning when cells left no room for their axes.
fn warn_collapsed() {
    crate::warnings::warn(
        "constrained_layout not applied because axes sizes collapsed to zero.  Try making \
         figure larger or Axes decorations smaller.",
    );
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
        )))
        .unwrap();
        let axs = fig.subplots(2, 2).unwrap();
        for (i, ax) in axs.as_slice().iter().enumerate() {
            ax.set_xlim(0.0, 10.0)
                .and_then(|ax| ax.set_ylim(-1.0, 1.0))
                .expect("finite limits");
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
    fn texts_placed_by_hand_stay_and_take_no_room() {
        // matplotlib's test_constrained_layout22 and test_suplabels: a
        // suptitle at y = 0.5 leaves the layout alone, and so do labels
        // placed across their edges; placed along them, they still count.
        let layout = |place: Option<bool>| {
            let fig = Figure::new();
            fig.set_layout_engine(Some(super::super::LayoutEngine::Constrained(
                ConstrainedLayout::from_rc(&RcParams::matplotlib()),
            )))
            .unwrap();
            let ax = fig.subplot();
            let texts = place.map(|across| {
                let texts = [fig.suptitle("t"), fig.supxlabel("x"), fig.supylabel("y")];
                if across {
                    fig.set_suptitle_y(0.5)
                        .set_supxlabel_y(0.3)
                        .set_supylabel_x(0.4);
                } else {
                    texts[0].set_x(0.2);
                    texts[1].set_x(0.5);
                    texts[2].set_y(0.5);
                }
                texts
            });
            fig.layout();
            let placed = texts.map(|texts| texts.map(|text| text.position()));
            (ax.position(), placed)
        };
        let (bare, _) = layout(None);
        let (across, placed) = layout(Some(true));
        assert_eq!(across, bare);
        assert_eq!(
            placed,
            Some([
                pltrs_core::Point::new(0.5, 0.5),
                pltrs_core::Point::new(0.5, 0.3),
                pltrs_core::Point::new(0.4, 0.5),
            ])
        );
        let (along, _) = layout(Some(false));
        assert!(along.y0() > bare.y0() && along.y1() < bare.y1() && along.x0() > bare.x0());
    }

    #[test]
    fn empty_suptitles_take_no_room() {
        let positions = |suptitle: Option<&str>| {
            let fig = Figure::new();
            fig.set_layout_engine(Some(super::super::LayoutEngine::Constrained(
                ConstrainedLayout::from_rc(&RcParams::matplotlib()),
            )))
            .unwrap();
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
