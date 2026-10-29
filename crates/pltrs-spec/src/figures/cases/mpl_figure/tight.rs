//! `test_tightlayout.py`: tight layout of single and multiple subplots,
//! uneven grids, images, titles on both sides, the tight layout engine and
//! hidden axes.

use pltrs::axes::{ImshowOptions, TitleLocation};
use pltrs::gridspec::GridSpec;
use pltrs::image::Interpolation;
use pltrs::layout::{LayoutEngine, TightLayout};
use pltrs::numeric::Grid;
use pltrs::prelude::*;

use super::example_plot;
use crate::figures::{CaseResult, FigureCase};

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_figure/tight_layout1", |fig| {
        example_plot(&fig.subplot(), 24.0, false)?;
        fig.tight_layout(TightLayout::default())?;
        Ok(())
    }),
    FigureCase::new("mpl_figure/tight_layout2", |fig| {
        for ax in fig.subplots(2, 2)?.as_slice() {
            example_plot(ax, 12.0, false)?;
        }
        fig.tight_layout(TightLayout::default())?;
        Ok(())
    }),
    FigureCase::new("mpl_figure/tight_layout3", |fig| {
        for (nrows, ncols, index) in [(2, 2, 1), (2, 2, 3), (1, 2, 2)] {
            example_plot(&fig.add_subplot(nrows, ncols, index)?, 12.0, false)?;
        }
        fig.tight_layout(TightLayout::default())?;
        Ok(())
    }),
    FigureCase::new("mpl_figure/tight_layout4", |fig| {
        // `subplot2grid((3, 3), ...)` places every axes on one grid.
        let gs = fig.add_gridspec(3, 3)?;
        for (rows, cols) in [(0..1, 0..1), (0..1, 1..3), (1..3, 0..2), (1..3, 2..3)] {
            example_plot(&fig.add_subplot_spec(&gs.span(rows, cols)?)?, 12.0, false)?;
        }
        fig.tight_layout(TightLayout::default())?;
        Ok(())
    }),
    FigureCase::new("mpl_figure/tight_layout5", |fig| {
        let arr = Grid::from_fn(10, 10, |r, c| (r * 10 + c) as f64);
        let options = ImshowOptions {
            interpolation: Some(Interpolation::None),
            ..ImshowOptions::default()
        };
        fig.subplot().imshow(arr, options)?;
        fig.tight_layout(TightLayout::default())?;
        Ok(())
    }),
    FigureCase::new("mpl_figure/tight_layout6", tight_layout6),
    FigureCase::new("mpl_figure/tight_layout7", |fig| {
        let ax = fig.subplot();
        example_plot(&ax, 24.0, false)?;
        ax.set_title("");
        for (location, title) in [
            (TitleLocation::Left, "Left Title"),
            (TitleLocation::Right, "Right Title"),
        ] {
            ax.set_title_at(location, title).set_fontsize(24.0);
        }
        fig.tight_layout(TightLayout::default())?;
        Ok(())
    }),
    FigureCase::new("mpl_figure/tight_layout8", |fig| {
        fig.set_layout_engine(Some(LayoutEngine::Tight(TightLayout {
            pad: 0.1,
            ..TightLayout::default()
        })))?;
        example_plot(&fig.subplot(), 24.0, false)
    }),
    FigureCase::new("mpl_figure/tight_layout9", |fig| {
        let axs = fig.subplots(2, 2)?;
        axs[(1, 1)].set_visible(false);
        fig.tight_layout(TightLayout::default())?;
        Ok(())
    }),
];

/// Tight layout fitting `rect` with rows `h_pad` font sizes apart.
fn fitting(rect: [f64; 4], h_pad: Option<f64>) -> TightLayout {
    TightLayout {
        rect: Some(rect),
        h_pad,
        ..TightLayout::default()
    }
}

/// `test_tight_layout6`: two grids laid out side by side, then again with
/// their top and bottom edges lined up. A margin of the `rect` left
/// `None` is measured anew, as 0 (left, bottom) or 1 (right, top) has it
/// measured.
fn tight_layout6(fig: &Figure) -> CaseResult {
    let gs1 = GridSpec::new(2, 1)?;
    for row in 0..2 {
        example_plot(&fig.add_subplot_spec(&gs1.cell(row, 0)?)?, 12.0, false)?;
    }
    gs1.tight_layout(fig, fitting([0.0, 0.0, 0.5, 1.0], None))?;

    let gs2 = GridSpec::new(3, 1)?;
    let mut last = None;
    for row in 0..3 {
        let ax = fig.add_subplot_spec(&gs2.cell(row, 0)?)?;
        example_plot(&ax, 12.0, false)?;
        ax.set_title("");
        ax.set_xlabel("");
        last = Some(ax);
    }
    // Still in the 12 points `example_plot` gave the labels.
    last.ok_or("no axes")?.set_xlabel("x-label");

    gs2.tight_layout(fig, fitting([0.5, 0.0, 1.0, 1.0], Some(0.45)))?;

    let edges = |grid: &GridSpec| {
        let o = grid.overrides();
        o.top.zip(o.bottom).ok_or("a grid laid out tight has edges")
    };
    let ((top1, bottom1), (top2, bottom2)) = (edges(&gs1)?, edges(&gs2)?);
    let (top, bottom) = (top1.min(top2), bottom1.max(bottom2));
    gs1.tight_layout(
        fig,
        fitting([0.0, bottom - bottom1, 0.5, 1.0 - (top1 - top)], None),
    )?;
    gs2.tight_layout(
        fig,
        fitting([0.5, bottom - bottom2, 1.0, 1.0 - (top2 - top)], Some(0.45)),
    )?;
    Ok(())
}
