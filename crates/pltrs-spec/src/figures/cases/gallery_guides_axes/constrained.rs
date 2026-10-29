//! `constrainedlayout_guide`, first part: constrained against no layout,
//! colorbars, a suptitle, legends in and out of the layout, and padding
//! and spacing.

use pltrs::axes::{Mesh, PcolorOptions};
use pltrs::collections::Collection;
use pltrs::colorbar::{ColorbarAxes, ColorbarOptions};
use pltrs::colors::{ColorizerOptions, colormaps};
use pltrs::gridspec::SubplotOverrides;
use pltrs::layout::{ConstrainedLayout, LayoutEngine};
use pltrs::legend::LegendOptions;
use pltrs::numeric::Grid;
use pltrs::offsetbox::AnchorLoc;
use pltrs::prelude::*;

use super::LAYOUT_RC;
use crate::figures::cases::common::{arange_grid, example_plot};
use crate::figures::{CaseResult, FigureCase};

/// A case of the guide, on its `figure.figsize` of 4.5 × 4 in.
pub(super) const fn constrained(
    name: &'static str,
    build: fn(&Figure) -> CaseResult,
) -> FigureCase {
    FigureCase::new(name, build)
        .sized(4.5, 4.0, 100.0)
        .with_rc(LAYOUT_RC)
}

/// `example_plot` in `fontsize` points on every axes of an `nrows ×
/// ncols` grid laid out by `layout`.
fn example_grid(
    fig: &Figure,
    nrows: usize,
    ncols: usize,
    layout: Option<LayoutEngine>,
    fontsize: f64,
) -> CaseResult {
    fig.set_layout_engine(layout)?;
    for ax in fig.subplots(nrows, ncols)?.as_slice() {
        example_plot(ax, fontsize, false)?;
    }
    Ok(())
}

/// The guide's `pc_kwargs`: a rasterized viridis mesh of `arr` with
/// values normalised over `[0, 100]`.
pub(super) fn arr_mesh(ax: &Axes) -> Result<Collection, Box<dyn std::error::Error>> {
    let options = PcolorOptions {
        colorizer: ColorizerOptions::default()
            .cmap(colormaps::get("viridis")?)
            .limits(Some(0.0), Some(100.0)),
        ..PcolorOptions::default()
    };
    let mesh = ax.pcolormesh(Mesh::new(arange_grid(10, 10)), options)?;
    mesh.set_rasterized(true);
    Ok(mesh)
}

/// A constrained grid of `arr` meshes; returns its axes and the last mesh.
fn meshes(
    fig: &Figure,
    nrows: usize,
    ncols: usize,
) -> Result<(Grid<Axes>, Collection), Box<dyn std::error::Error>> {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(nrows, ncols)?;
    let mut last = None;
    for ax in axs.as_slice() {
        last = Some(arr_mesh(ax)?);
    }
    Ok((axs, last.ok_or("at least one mesh")?))
}

/// A colorbar for `mappable` taking space from `parents`, shrunk to
/// `shrink`.
pub(super) fn shared_colorbar(
    fig: &Figure,
    mappable: &Collection,
    parents: Vec<Axes>,
    shrink: f64,
) -> CaseResult {
    fig.colorbar(
        mappable,
        ColorbarOptions {
            axes: ColorbarAxes::Parents(parents),
            shrink,
            ..ColorbarOptions::default()
        },
    )?;
    Ok(())
}

fn constrained_1(fig: &Figure) -> CaseResult {
    example_grid(fig, 1, 1, None, 24.0)
}

fn constrained_2(fig: &Figure) -> CaseResult {
    example_grid(fig, 1, 1, Some(LayoutEngine::constrained()), 24.0)
}

fn constrained_3(fig: &Figure) -> CaseResult {
    example_grid(fig, 2, 2, None, 12.0)
}

fn constrained_4(fig: &Figure) -> CaseResult {
    example_grid(fig, 2, 2, Some(LayoutEngine::constrained()), 12.0)
}

fn constrained_5(fig: &Figure) -> CaseResult {
    let (axs, im) = meshes(fig, 1, 1)?;
    shared_colorbar(fig, &im, axs.as_slice().to_vec(), 0.6)
}

fn constrained_6(fig: &Figure) -> CaseResult {
    let (axs, im) = meshes(fig, 2, 2)?;
    shared_colorbar(fig, &im, axs.as_slice().to_vec(), 0.6)
}

fn constrained_7(fig: &Figure) -> CaseResult {
    let (axs, im) = meshes(fig, 3, 3)?;
    let middle_lower = vec![axs[(1, 1)].clone(), axs[(2, 1)].clone()];
    shared_colorbar(fig, &im, middle_lower, 0.8)?;
    let right = (0..3).map(|row| axs[(row, 2)].clone()).collect();
    shared_colorbar(fig, &im, right, 0.6)
}

fn constrained_8(fig: &Figure) -> CaseResult {
    let (axs, im) = meshes(fig, 2, 2)?;
    shared_colorbar(fig, &im, axs.as_slice().to_vec(), 0.6)?;
    fig.suptitle("Big Suptitle");
    Ok(())
}

/// A legend at the left of `(0.8, 0.5)` in the axes.
fn leaning_legend() -> LegendOptions {
    LegendOptions::default()
        .loc(AnchorLoc::CenterLeft)
        .bbox_to_anchor((0.8, 0.5))
}

fn constrained_9(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    ax.plot_with(
        arange(0.0, 10.0, 1.0),
        arange(0.0, 10.0, 1.0),
        LineProps::default().label("This is a plot"),
    )?;
    ax.legend(leaning_legend())?;
    Ok(())
}

/// Two constrained axes, the right one with a labelled line; returns the
/// right axes and its line.
fn legend_pair(fig: &Figure) -> Result<(Axes, Line2D), Box<dyn std::error::Error>> {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(1, 2)?;
    let x = arange(0.0, 10.0, 1.0);
    axs[(0, 0)].plot(&x, &x)?;
    let line = axs[(0, 1)].plot_with(&x, &x, LineProps::default().label("This is a plot"))?;
    Ok((axs[(0, 1)].clone(), line))
}

fn constrained_10(fig: &Figure) -> CaseResult {
    legend_pair(fig)?.0.legend(leaning_legend())?;
    Ok(())
}

fn constrained_11(fig: &Figure) -> CaseResult {
    let leg = legend_pair(fig)?.0.legend(leaning_legend())?;
    leg.set_in_layout(false);
    fig.layout();
    leg.set_in_layout(true);
    fig.set_layout_engine(None)?;
    Ok(())
}

fn constrained_12(fig: &Figure) -> CaseResult {
    let (ax, line) = legend_pair(fig)?;
    let label = line.label();
    fig.legend(
        leaning_legend()
            .handles([&line])
            .labels([label])
            .bbox_transform(ax.trans_axes()),
    )?;
    Ok(())
}

/// The guide's `set(w_pad=4 / 72, h_pad=4 / 72, hspace=..., wspace=...)`
/// on a constrained layout.
fn padded(hspace: f64, wspace: f64) -> LayoutEngine {
    LayoutEngine::Constrained(ConstrainedLayout {
        w_pad: 4.0 / 72.0,
        h_pad: 4.0 / 72.0,
        hspace,
        wspace,
        ..ConstrainedLayout::default()
    })
}

/// Label-less example plots in a constrained `nrows × ncols` grid with
/// `spacing` on the grid, then the padding and spaces of `engine`.
fn spaced_grid(
    fig: &Figure,
    nrows: usize,
    ncols: usize,
    spacing: SubplotOverrides,
    engine: LayoutEngine,
) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let grid = pltrs::gridspec::GridSpec::with_ratios(nrows, ncols, None, None, spacing)?;
    for ax in fig
        .subplots_on(
            &grid,
            pltrs::figure::Share::None,
            pltrs::figure::Share::None,
        )?
        .as_slice()
    {
        example_plot(ax, 12.0, true)?;
    }
    fig.set_layout_engine(engine)?;
    Ok(())
}

fn constrained_13(fig: &Figure) -> CaseResult {
    spaced_grid(fig, 2, 2, SubplotOverrides::default(), padded(0.0, 0.0))
}

fn constrained_14(fig: &Figure) -> CaseResult {
    spaced_grid(fig, 2, 2, SubplotOverrides::default(), padded(0.2, 0.2))
}

fn constrained_15(fig: &Figure) -> CaseResult {
    spaced_grid(fig, 2, 3, SubplotOverrides::default(), padded(0.2, 0.2))
}

fn constrained_16(fig: &Figure) -> CaseResult {
    let spacing = SubplotOverrides {
        wspace: Some(0.3),
        hspace: Some(0.2),
        ..SubplotOverrides::default()
    };
    spaced_grid(fig, 2, 2, spacing, padded(0.0, 0.0))
}

fn constrained_17(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(2, 2)?;
    for (ax, pad) in axs.as_slice().iter().zip([0.0, 0.05, 0.1, 0.2]) {
        let pc = arr_mesh(ax)?;
        fig.colorbar(
            &pc,
            ColorbarOptions {
                shrink: 0.6,
                pad: Some(pad),
                ..ColorbarOptions::beside(ax)
            },
        )?;
        ax.set_xticklabels(Vec::<String>::new())?;
        ax.set_yticklabels(Vec::<String>::new())?;
        ax.set_title(format!("pad: {pad}"));
    }
    fig.set_layout_engine(LayoutEngine::Constrained(ConstrainedLayout {
        w_pad: 2.0 / 72.0,
        h_pad: 2.0 / 72.0,
        hspace: 0.2,
        wspace: 0.2,
        ..ConstrainedLayout::default()
    }))?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_1",
        constrained_1,
    ),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_2",
        constrained_2,
    ),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_3",
        constrained_3,
    ),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_4",
        constrained_4,
    ),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_5",
        constrained_5,
    )
    .sized(4.0, 4.0, 100.0),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_6",
        constrained_6,
    )
    .sized(4.0, 4.0, 100.0),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_7",
        constrained_7,
    )
    .sized(4.0, 4.0, 100.0),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_8",
        constrained_8,
    )
    .sized(4.0, 4.0, 100.0),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_9",
        constrained_9,
    ),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_10",
        constrained_10,
    )
    .sized(4.0, 2.0, 100.0),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_11",
        constrained_11,
    )
    .sized(4.0, 2.0, 100.0),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_12",
        constrained_12,
    )
    .sized(4.0, 2.0, 100.0),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_13",
        constrained_13,
    ),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_14",
        constrained_14,
    ),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_15",
        constrained_15,
    ),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_16",
        constrained_16,
    ),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_17",
        constrained_17,
    ),
];
