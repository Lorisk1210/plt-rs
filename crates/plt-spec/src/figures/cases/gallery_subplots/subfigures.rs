//! `subfigures`: figures within a figure, side by side, mixed with
//! subplots, of different widths and nested.

use plt::axes::{Mesh, PcolorOptions};
use plt::collections::Collection;
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::colors::to_rgba;
use plt::figure::{Share, SubfigureOptions};
use plt::fonts::{FontSize, RelativeSize};
use plt::image::ColorizerOptions;
use plt::layout::LayoutEngine;
use plt::numeric::Grid;
use plt::prelude::*;
use plt::spines::SpineSide;

use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The seed of the example's `np.random.seed(19680808)`.
const SEED: u64 = 19_680_808;

/// `_random_mesh`: a 30 × 30 mesh of normal noise between -2.5 and 2.5,
/// labelled and titled in 12 points unless `hide_labels`.
fn random_mesh(
    ax: &Axes,
    rng: &mut SpecRng,
    hide_labels: bool,
) -> Result<Collection, Box<dyn std::error::Error>> {
    let noise = Grid::from_vec(30, 30, rng.normal(30 * 30, 0.0, 1.0))?;
    let options = PcolorOptions {
        colors: ColorizerOptions::default().limits(Some(-2.5), Some(2.5)),
        ..PcolorOptions::default()
    };
    let mesh = ax.pcolormesh(Mesh::new(noise), options)?;
    if !hide_labels {
        ax.set_xlabel("x-label").set_ylabel("y-label");
        ax.xaxis().set_label_fontsize(FontSize::Points(12.0));
        ax.yaxis().set_label_fontsize(FontSize::Points(12.0));
        ax.set_title("Title").set_fontsize(12.0);
    }
    Ok(mesh)
}

/// A mesh in each of `axes`, returning the last.
fn meshes(
    axes: &[Axes],
    rng: &mut SpecRng,
    hide_labels: bool,
) -> Result<Collection, Box<dyn std::error::Error>> {
    let mut last = None;
    for ax in axes {
        last = Some(random_mesh(ax, rng, hide_labels)?);
    }
    Ok(last.ok_or("at least one axes")?)
}

/// A colorbar for `mesh` taking room from `axes` on `location` (the
/// default side when `None`), shrunk to `shrink`.
fn colorbar_beside(
    subfig: &Figure,
    mesh: &Collection,
    axes: &[Axes],
    location: Option<SpineSide>,
    shrink: f64,
) -> CaseResult {
    subfig.colorbar(
        mesh,
        ColorbarOptions {
            shrink,
            axes: ColorbarAxes::Parents(axes.to_vec()),
            location,
            ..ColorbarOptions::default()
        },
    )?;
    Ok(())
}

/// `_left_plots`: two labelled meshes sharing y, a colorbar below them,
/// on grey.
fn left_plots(subfig: &Figure, rng: &mut SpecRng) -> CaseResult {
    let axs = subfig.subplots_shared(1, 2, Share::None, Share::All)?;
    subfig.set_facecolor(to_rgba("0.75", None)?);
    let mesh = meshes(axs.as_slice(), rng, false)?;
    subfig
        .suptitle("Left plots")
        .set_fontsize(RelativeSize::XLarge);
    colorbar_beside(subfig, &mesh, axs.as_slice(), Some(SpineSide::Bottom), 0.6)
}

/// `_right_plots`: three unlabelled meshes sharing x, a colorbar beside
/// them, on a lighter grey.
fn right_plots(subfig: &Figure, rng: &mut SpecRng) -> CaseResult {
    let axs = subfig.subplots_shared(3, 1, Share::All, Share::None)?;
    let mesh = meshes(axs.as_slice(), rng, true)?;
    axs[(2, 0)].set_xlabel("xlabel");
    axs[(1, 0)].set_ylabel("ylabel");
    subfig.set_facecolor(to_rgba("0.85", None)?);
    colorbar_beside(subfig, &mesh, axs.as_slice(), None, 0.6)?;
    subfig
        .suptitle("Right plots")
        .set_fontsize(RelativeSize::XLarge);
    Ok(())
}

/// The figure's own title.
fn figure_suptitle(fig: &Figure) {
    fig.suptitle("Figure suptitle")
        .set_fontsize(RelativeSize::XxLarge);
}

/// `_left_and_right`: the left and right plots in two subfigures with
/// the given relative widths.
fn left_and_right(fig: &Figure, width_ratios: Option<Vec<f64>>) -> CaseResult {
    fig.set_layout_engine(Some(LayoutEngine::constrained()));
    let mut rng = SpecRng::new(SEED);
    let subfigs = fig.subfigures(
        1,
        2,
        SubfigureOptions {
            wspace: Some(0.07),
            width_ratios,
            ..SubfigureOptions::default()
        },
    )?;
    left_plots(&subfigs[(0, 0)], &mut rng)?;
    right_plots(&subfigs[(0, 1)], &mut rng)?;
    figure_suptitle(fig);
    Ok(())
}

fn subfigures_1(fig: &Figure) -> CaseResult {
    left_and_right(fig, None)
}

fn subfigures_2(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(Some(LayoutEngine::constrained()));
    let axs = fig.subplots(2, 3)?;
    let grid = axs[(0, 0)]
        .subplot_spec()
        .ok_or("subplots lie on a grid")?
        .gridspec()
        .clone();
    for row in 0..2 {
        axs[(row, 0)].remove();
    }
    for row in 0..2 {
        for col in 1..3 {
            axs[(row, col)].plot_y(arange(0.0, 10.0, 1.0));
        }
    }
    let subfig = fig.add_subfigure(&grid.span(0..2, 0..1)?);
    left_plots(&subfig, &mut SpecRng::new(SEED))?;
    figure_suptitle(fig);
    Ok(())
}

fn subfigures_3(fig: &Figure) -> CaseResult {
    left_and_right(fig, Some(vec![2.0, 1.0]))
}

fn subfigures_4(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(Some(LayoutEngine::constrained()));
    let mut rng = SpecRng::new(SEED);
    fig.suptitle("fig");
    let subfigs = fig.subfigures(
        1,
        2,
        SubfigureOptions {
            wspace: Some(0.07),
            ..SubfigureOptions::default()
        },
    )?;
    for (i, subfig) in subfigs.as_slice().iter().enumerate() {
        subfig.set_facecolor(to_rgba("coral", None)?);
        subfig.suptitle(format!("subfigs[{i}]"));
    }
    let nested = subfigs[(0, 0)].subfigures(
        2,
        1,
        SubfigureOptions {
            height_ratios: Some(vec![1.0, 1.4]),
            ..SubfigureOptions::default()
        },
    )?;
    let (top, bottom) = (&nested[(0, 0)], &nested[(1, 0)]);
    top.suptitle("subfigsnest[0]");
    top.set_facecolor(to_rgba("r", None)?);
    let axs = top.subplots_shared(1, 2, Share::None, Share::All)?;
    let mesh = meshes(axs.as_slice(), &mut rng, true)?;
    colorbar_beside(top, &mesh, axs.as_slice(), None, 1.0)?;
    bottom.suptitle("subfigsnest[1]");
    bottom.set_facecolor(to_rgba("g", None)?);
    bottom.subplots_shared(3, 1, Share::All, Share::None)?;
    subfigs[(0, 1)].subplots(2, 2)?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_subplots/subfigures_1", subfigures_1).sized(10.0, 4.0, 100.0),
    FigureCase::new("gallery_subplots/subfigures_2", subfigures_2).sized(10.0, 4.0, 100.0),
    FigureCase::new("gallery_subplots/subfigures_3", subfigures_3).sized(10.0, 4.0, 100.0),
    FigureCase::new("gallery_subplots/subfigures_4", subfigures_4).sized(10.0, 8.0, 100.0),
];
