//! `figures/cases_mpl_tests_constrainedlayout.py`: matplotlib's non-image
//! tests from `test_constrainedlayout.py` that draw and then check where
//! the axes went, in the test suite's default style: nested grids, twin
//! axes, hidden and hand-placed axes, colorbars on every side, the
//! engine's padding and rectangle, compressed layouts and subfigures of
//! axes spanning several cells.

use plt::axes::{ImshowOptions, Mesh, PcolorOptions, TickDirection, TickParams};
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::figure::{Share, SubfigureOptions};
use plt::gridspec::GridSpec;
use plt::layout::{ConstrainedLayout, LayoutEngine, LayoutEngineError};
use plt::numeric::Grid;
use plt::prelude::*;
use plt::spines::SpineSide;

use super::common::example_plot;
use super::mpl_figure::example_pcolor;
use super::mpl_tests::classic;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [PLACEMENT, ENGINE].concat()
}

/// Grids, twins, hidden and hand-placed axes and subfigures.
const PLACEMENT: &[FigureCase] = &[
    classic("mpl_tests_constrainedlayout/identical_subgridspec", |fig| {
        fig.set_layout_engine(LayoutEngine::constrained())?;
        let gs = fig.add_gridspec(2, 1)?;
        let gsa = GridSpec::nested(&gs.cell(0, 0)?, 1, 3, None, None, (None, None))?;
        let gsb = GridSpec::nested(&gs.cell(1, 0)?, 1, 3, None, None, (None, None))?;
        for i in 0..3 {
            fig.add_subplot_spec(&gsa.cell(0, i)?)?;
            fig.add_subplot_spec(&gsb.cell(0, i)?)?;
        }
        Ok(())
    }),
    classic("mpl_tests_constrainedlayout/constrained_layout18", |fig| {
        fig.set_layout_engine(LayoutEngine::constrained())?;
        let ax = fig.subplot();
        let ax2 = ax.twinx()?;
        example_plot(&ax, 12.0, false)?;
        example_plot(&ax2, 24.0, false)
    }),
    classic("mpl_tests_constrainedlayout/constrained_layout19", |fig| {
        fig.set_layout_engine(LayoutEngine::constrained())?;
        let ax = fig.subplot();
        let ax2 = ax.twiny()?;
        example_plot(&ax, 12.0, false)?;
        example_plot(&ax2, 24.0, false)?;
        ax2.set_title("");
        ax.set_title("");
        Ok(())
    }),
    classic("mpl_tests_constrainedlayout/constrained_layout20", |fig| {
        let gx = linspace(-5.0, 5.0, 4);
        let img = Grid::from_fn(3, 3, |row, col| gx[col].hypot(gx[row]));
        let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0])?;
        let mesh = ax.pcolormesh(
            Mesh::from_coords(gx.clone(), gx, img),
            PcolorOptions::default(),
        )?;
        fig.colorbar(&mesh, ColorbarOptions::default())?;
        Ok(())
    }),
    classic("mpl_tests_constrainedlayout/hidden_axes", |fig| {
        fig.set_layout_engine(LayoutEngine::constrained())?;
        fig.subplots(2, 2)?[(0, 1)].set_visible(false);
        Ok(())
    }),
    classic(
        "mpl_tests_constrainedlayout/manually_set_position_1",
        |fig| {
            fig.set_layout_engine(LayoutEngine::constrained())?;
            fig.subplots(1, 2)?[(0, 0)].set_position([0.2, 0.2, 0.3, 0.3]);
            Ok(())
        },
    ),
    classic(
        "mpl_tests_constrainedlayout/manually_set_position_2",
        |fig| {
            fig.set_layout_engine(LayoutEngine::constrained())?;
            let axs = fig.subplots(1, 2)?;
            let ax = &axs[(0, 0)];
            ax.set_position([0.2, 0.2, 0.3, 0.3]);
            let values = Grid::from_vec(20, 20, SpecRng::new(3).uniform(400, 0.0, 1.0))?;
            let pc = ax.pcolormesh(Mesh::new(values), PcolorOptions::default())?;
            fig.colorbar(&pc, ColorbarOptions::beside(ax))?;
            Ok(())
        },
    ),
    classic("mpl_tests_constrainedlayout/gridspec_addressing", |fig| {
        let gs = fig.add_gridspec(3, 3)?;
        fig.add_subplot_spec(&gs.span(0..3, 1..3)?)?;
        Ok(())
    }),
    classic(
        "mpl_tests_constrainedlayout/submerged_subfig",
        submerged_subfig,
    )
    .sized(4.0, 5.0, 100.0),
];

/// Colorbars, the engine's parameters and compressed layouts.
const ENGINE: &[FigureCase] = &[
    classic("mpl_tests_constrainedlayout/colorbar_align_right", |fig| {
        colorbar_align(fig, SpineSide::Right)
    }),
    classic("mpl_tests_constrainedlayout/colorbar_align_left", |fig| {
        colorbar_align(fig, SpineSide::Left)
    }),
    classic("mpl_tests_constrainedlayout/colorbar_align_top", |fig| {
        colorbar_align(fig, SpineSide::Top)
    }),
    classic("mpl_tests_constrainedlayout/colorbar_align_bottom", |fig| {
        colorbar_align(fig, SpineSide::Bottom)
    }),
    classic("mpl_tests_constrainedlayout/discouraged_api", |fig| {
        constrained(fig, |layout| ConstrainedLayout {
            w_pad: 0.02,
            h_pad: 0.02,
            ..layout
        })?;
        fig.subplot();
        Ok(())
    }),
    classic("mpl_tests_constrainedlayout/kwargs", |fig| {
        constrained(fig, |layout| ConstrainedLayout {
            h_pad: 0.02,
            ..layout
        })?;
        fig.subplot();
        Ok(())
    }),
    classic("mpl_tests_constrainedlayout/rect_1", |fig| {
        fig.subplot();
        constrained(fig, |layout| ConstrainedLayout {
            rect: [0.0, 0.0, 0.5, 0.5],
            ..layout
        })?;
        Ok(())
    }),
    classic("mpl_tests_constrainedlayout/rect_2", |fig| {
        fig.subplot();
        constrained(fig, |layout| ConstrainedLayout {
            rect: [0.2, 0.2, 0.3, 0.3],
            ..layout
        })?;
        Ok(())
    }),
    classic("mpl_tests_constrainedlayout/compressed1_1", |fig| {
        compressed1(fig, 3, 2, 10)
    }),
    classic("mpl_tests_constrainedlayout/compressed1_2", |fig| {
        compressed1(fig, 2, 3, 20)
    })
    .sized(5.0, 4.0, 100.0),
];

/// Lays `fig` out by the constrained engine with its defaults changed by
/// `adjust` (matplotlib's `get_layout_engine().set(...)`).
fn constrained(
    fig: &Figure,
    adjust: fn(ConstrainedLayout) -> ConstrainedLayout,
) -> Result<(), LayoutEngineError> {
    let layout = ConstrainedLayout::from_rc(&plt::rc::params());
    fig.set_layout_engine(LayoutEngine::Constrained(adjust(layout)))?;
    Ok(())
}

/// `test_colorbar_align`: a colorbar at `location` beside each of four
/// meshes, ticks inward, only the second axes and its colorbar labelled.
fn colorbar_align(fig: &Figure, location: SpineSide) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let inward = TickParams {
        direction: Some(TickDirection::In),
        ..TickParams::default()
    };
    for (nn, ax) in fig.subplots(2, 2)?.as_slice().iter().enumerate() {
        ax.tick_params(inward.clone());
        let pc = example_pcolor(ax, 12.0)?;
        let cb = fig.colorbar(
            &pc,
            ColorbarOptions {
                location: Some(location),
                shrink: 0.6,
                pad: Some(0.04),
                ..ColorbarOptions::beside(ax)
            },
        )?;
        cb.ax().tick_params(inward.clone());
        if nn != 1 {
            cb.ax().xaxis().set_ticks(Vec::<f64>::new());
            cb.ax().yaxis().set_ticks(Vec::<f64>::new());
            ax.set_xticklabels(Vec::<String>::new())?;
            ax.set_yticklabels(Vec::<String>::new())?;
        }
    }
    constrained(fig, |layout| ConstrainedLayout {
        w_pad: 4.0 / 72.0,
        h_pad: 4.0 / 72.0,
        hspace: 0.1,
        wspace: 0.1,
        ..layout
    })?;
    Ok(())
}

/// `test_compressed1`: a compressed grid of shared images of standard
/// normal noise (stream `seed + n` for the `n`-th) and their colorbar.
fn compressed1(fig: &Figure, nrows: usize, ncols: usize, seed: u64) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::compressed())?;
    let axs = fig.subplots_shared(nrows, ncols, Share::All, Share::All)?;
    let mut image = None;
    for (stream, ax) in (seed..).zip(axs.as_slice()) {
        let noise = Grid::from_vec(20, 20, SpecRng::new(stream).normal(400, 0.0, 1.0))?;
        image = Some(ax.imshow(noise, ImshowOptions::default())?);
    }
    fig.colorbar(
        &image.ok_or("no image")?,
        ColorbarOptions {
            axes: ColorbarAxes::Parents(axs.as_slice().to_vec()),
            ..ColorbarOptions::default()
        },
    )?;
    Ok(())
}

/// `test_submerged_subfig`: three subfigures, each with two axes stacked
/// beside one spanning both rows.
fn submerged_subfig(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    for sub in fig
        .subfigures(3, 1, SubfigureOptions::default())?
        .as_slice()
    {
        let gs = sub.add_gridspec(2, 2)?;
        let cells = [gs.cell(0, 0)?, gs.cell(1, 0)?, gs.span(0..2, 1..2)?];
        for cell in &cells {
            // `plot()` draws nothing but asks for autoscaling, as
            // `autoscale()` does: the empty view becomes ±0.05, rounded
            // out to ticks.
            sub.add_subplot_spec(cell)?.autoscale(None, None);
        }
    }
    Ok(())
}
