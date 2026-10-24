//! `constrainedlayout_guide`, second part: the rc switch, grid
//! specifications, nested grids and subfigures, manual positions,
//! fixed-aspect axes, compressed layout and `subplot` grids.

use plt::axes::ImshowOptions;
use plt::colorbar::ColorbarOptions;
use plt::colors::colormaps;
use plt::figure::{Share, SubfigureOptions};
use plt::fonts::RelativeSize;
use plt::gridspec::{GridSpec, SubplotOverrides};
use plt::layout::LayoutEngine;
use plt::numeric::Grid;
use plt::prelude::*;

use super::constrained::{arr_mesh, constrained, shared_colorbar};
use super::{SEED, arange_grid, subplot2grid};
use crate::figures::cases::common::example_plot;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

fn constrained_18(fig: &Figure) -> CaseResult {
    for ax in fig.subplots(2, 2)?.as_slice() {
        example_plot(ax, 12.0, false)?;
    }
    Ok(())
}

fn constrained_19(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let gs1 = fig.add_gridspec(2, 1)?;
    for row in 0..2 {
        example_plot(&fig.add_subplot_spec(&gs1.cell(row, 0)?)?, 12.0, false)?;
    }
    Ok(())
}

fn constrained_20(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let gs0 = fig.add_gridspec(1, 2)?;
    let gs1 = GridSpec::nested(&gs0.cell(0, 0)?, 2, 1, None, None, (None, None))?;
    for row in 0..2 {
        example_plot(&fig.add_subplot_spec(&gs1.cell(row, 0)?)?, 12.0, false)?;
    }
    let gs2 = GridSpec::nested(&gs0.cell(0, 1)?, 3, 1, None, None, (None, None))?;
    let mut last = None;
    for row in 0..3 {
        let ax = fig.add_subplot_spec(&gs2.cell(row, 0)?)?;
        example_plot(&ax, 12.0, false)?;
        ax.set_title("");
        ax.set_xlabel("");
        last = Some(ax);
    }
    let ax = last.ok_or("three axes")?;
    ax.set_xlabel("x-label");
    ax.xaxis()
        .set_label_fontsize(plt::fonts::FontSize::Points(12.0));
    Ok(())
}

fn constrained_21(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let gs0 = fig.add_gridspec(6, 2)?;
    for rows in [0..3, 3..6] {
        example_plot(&fig.add_subplot_spec(&gs0.span(rows, 0..1)?)?, 12.0, false)?;
    }
    for rows in [0..2, 2..4, 4..6] {
        example_plot(&fig.add_subplot_spec(&gs0.span(rows, 1..2)?)?, 12.0, true)?;
    }
    fig.suptitle("Overlapping Gridspecs");
    Ok(())
}

/// `arr` meshes on `axs`, each labelled and titled; returns the last.
fn labelled_meshes(
    axs: &[Axes],
) -> Result<plt::collections::Collection, Box<dyn std::error::Error>> {
    let mut last = None;
    for ax in axs {
        last = Some(arr_mesh(ax)?);
        ax.set_xlabel("x-label").set_ylabel("y-label");
        ax.set_title("title");
    }
    Ok(last.ok_or("at least one mesh")?)
}

fn constrained_22(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let gs0 = GridSpec::with_ratios(
        1,
        2,
        Some(vec![1.0, 2.0]),
        None,
        SubplotOverrides::default(),
    )?;
    let gs_left = GridSpec::nested(&gs0.cell(0, 0)?, 2, 1, None, None, (None, None))?;
    let gs_right = GridSpec::nested(&gs0.cell(0, 1)?, 2, 2, None, None, (None, None))?;
    for row in 0..2 {
        example_plot(&fig.add_subplot_spec(&gs_left.cell(row, 0)?)?, 12.0, false)?;
    }
    let axs = (0..4)
        .map(|i| fig.add_subplot_spec(&gs_right.cell(i / 2, i % 2)?))
        .collect::<Result<Vec<_>, _>>()?;
    let pcm = labelled_meshes(&axs)?;
    fig.suptitle("Nested plots using subgridspec");
    shared_colorbar(fig, &pcm, axs, 1.0)
}

fn constrained_23(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let sfigs = fig.subfigures(
        1,
        2,
        SubfigureOptions {
            width_ratios: Some(vec![1.0, 2.0]),
            ..SubfigureOptions::default()
        },
    )?;
    for ax in sfigs[(0, 0)].subplots(2, 1)?.as_slice() {
        example_plot(ax, 12.0, false)?;
    }
    let right = &sfigs[(0, 1)];
    let axs_right = right.subplots(2, 2)?.as_slice().to_vec();
    let pcm = labelled_meshes(&axs_right)?;
    shared_colorbar(right, &pcm, axs_right, 1.0)?;
    fig.suptitle("Nested plots using subfigures");
    Ok(())
}

fn constrained_24(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(1, 2)?;
    example_plot(&axs[(0, 0)], 12.0, false)?;
    axs[(0, 1)].set_position([0.2, 0.2, 0.4, 0.4]);
    Ok(())
}

/// Four shared `arr` images in a 2 × 2 grid laid out by `layout`, named
/// `name` in the title.
fn fixed_aspect_images(fig: &Figure, layout: LayoutEngine, name: &str) -> CaseResult {
    fig.set_layout_engine(layout)?;
    for ax in fig
        .subplots_shared(2, 2, Share::All, Share::All)?
        .as_slice()
    {
        ax.imshow(arange_grid(), ImshowOptions::default())?;
    }
    fig.suptitle(format!("fixed-aspect plots, layout='{name}'"));
    Ok(())
}

fn constrained_25(fig: &Figure) -> CaseResult {
    fixed_aspect_images(fig, LayoutEngine::constrained(), "constrained")
}

fn constrained_26(fig: &Figure) -> CaseResult {
    fixed_aspect_images(fig, LayoutEngine::compressed(), "compressed")
}

/// A random 10 × 10 image with a colorbar, titled `title`; `zoom`
/// limits it to rows 4 to 8.
fn colorbar_title(fig: &Figure, layout: LayoutEngine, title: &str, zoom: bool) -> CaseResult {
    fig.set_layout_engine(layout)?;
    let ax = fig.subplot();
    let noise = Grid::from_vec(10, 10, SpecRng::new(SEED).normal(100, 0.0, 1.0))?;
    let pcm = ax.imshow(
        noise,
        ImshowOptions::default().cmap(colormaps::get("viridis")?),
    )?;
    if zoom {
        ax.set_ylim(4.0, 8.0)?;
    }
    ax.set_title(title).set_fontsize(RelativeSize::Medium);
    fig.colorbar(&pcm, ColorbarOptions::beside(&ax))?;
    Ok(())
}

fn constrained_27(fig: &Figure) -> CaseResult {
    colorbar_title(
        fig,
        LayoutEngine::constrained(),
        "Colorbar with layout='constrained'",
        false,
    )
}

fn constrained_28(fig: &Figure) -> CaseResult {
    colorbar_title(
        fig,
        LayoutEngine::compressed(),
        "Colorbar with layout='compressed'",
        false,
    )
}

fn constrained_29(fig: &Figure) -> CaseResult {
    colorbar_title(
        fig,
        LayoutEngine::compressed(),
        "Layout='compressed' with zoom",
        true,
    )
}

fn constrained_30(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax1 = fig.add_subplot(2, 2, 1)?;
    let ax2 = fig.add_subplot(2, 2, 3)?;
    let grid = ax1.subplot_spec().ok_or("a subplot")?.gridspec().clone();
    let ax3 = fig.add_subplot_spec(&grid.numbered_span(2, 4)?)?;
    for ax in [ax1, ax2, ax3] {
        example_plot(&ax, 12.0, false)?;
    }
    fig.suptitle("Homogeneous nrows, ncols");
    Ok(())
}

fn constrained_31(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axes = [
        fig.add_subplot(2, 2, 1)?,
        fig.add_subplot(2, 2, 3)?,
        fig.add_subplot(1, 2, 2)?,
    ];
    for ax in &axes {
        example_plot(ax, 12.0, false)?;
    }
    fig.suptitle("Mixed nrows, ncols");
    Ok(())
}

fn constrained_32(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    subplot2grid(fig)?;
    fig.suptitle("subplot2grid");
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_18",
        constrained_18,
    )
    .sized(3.0, 3.0, 100.0)
    .with_rc(&[
        ("savefig.facecolor", "0.8"),
        ("figure.constrained_layout.use", "True"),
    ]),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_19",
        constrained_19,
    ),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_20",
        constrained_20,
    ),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_21",
        constrained_21,
    )
    .sized(4.0, 6.0, 100.0),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_22",
        constrained_22,
    ),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_23",
        constrained_23,
    ),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_24",
        constrained_24,
    ),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_25",
        constrained_25,
    )
    .sized(5.0, 3.0, 100.0),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_26",
        constrained_26,
    )
    .sized(5.0, 3.0, 100.0),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_27",
        constrained_27,
    )
    .sized(3.0, 3.0, 100.0),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_28",
        constrained_28,
    )
    .sized(3.0, 3.0, 100.0),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_29",
        constrained_29,
    )
    .sized(3.0, 3.0, 100.0),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_30",
        constrained_30,
    ),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_31",
        constrained_31,
    ),
    constrained(
        "gallery_guides_axes/constrainedlayout_guide_32",
        constrained_32,
    ),
];
