//! `colorbar_placement`: colorbars beside one axes, a column or a block
//! of axes, on every side, padded, inset, and under constrained and
//! compressed layouts.

use plt::axes::{ImshowOptions, Mesh, PcolorOptions};
use plt::collections::Collection;
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::colors::{ColorizerOptions, colormaps};
use plt::figure::Share;
use plt::image::AxesImage;
use plt::layout::LayoutEngine;
use plt::numeric::Grid;
use plt::orientation::Orientation;
use plt::prelude::*;
use plt::spines::SpineSide;

use super::SEED;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// A `rows × cols` grid of the next uniform draws (`np.random.random`).
fn uniform(
    rng: &mut SpecRng,
    rows: usize,
    cols: usize,
) -> Result<Grid<f64>, plt::numeric::GridShapeError> {
    Grid::from_vec(rows, cols, rng.uniform(rows * cols, 0.0, 1.0))
}

/// A `rows × cols` grid of the next normal draws (`np.random.randn`).
fn normal(
    rng: &mut SpecRng,
    rows: usize,
    cols: usize,
) -> Result<Grid<f64>, plt::numeric::GridShapeError> {
    Grid::from_vec(rows, cols, rng.normal(rows * cols, 0.0, 1.0))
}

/// A mesh of `values` coloured by the colormap `cmap`.
fn mesh(
    ax: &Axes,
    values: Grid<f64>,
    cmap: &str,
) -> Result<Collection, Box<dyn std::error::Error>> {
    let options = PcolorOptions {
        colorizer: ColorizerOptions::default().cmap(colormaps::get(cmap)?),
        ..PcolorOptions::default()
    };
    Ok(ax.pcolormesh(Mesh::new(values), options)?)
}

/// The page's first figures: random meshes in a 2 × 2 grid, blue-red in
/// the left column and viridis (with twice the values) in the right one,
/// with a colorbar per axes or, when `shared`, per column.
fn cmap_columns(fig: &Figure, shared: bool) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    let axs = fig.subplots(2, 2)?;
    for (col, cmap) in ["RdBu_r", "viridis"].into_iter().enumerate() {
        let mut last = None;
        for row in 0..2 {
            let ax = &axs[(row, col)];
            let scale = (col + 1) as f64;
            let pcm = mesh(ax, uniform(&mut rng, 20, 20)?.map(|v| v * scale), cmap)?;
            if !shared {
                fig.colorbar(&pcm, ColorbarOptions::beside(ax))?;
            }
            last = Some(pcm);
        }
        if shared {
            let column = vec![axs[(0, col)].clone(), axs[(1, col)].clone()];
            fig.colorbar(
                &last.ok_or("a mesh per column")?,
                ColorbarOptions {
                    axes: ColorbarAxes::Parents(column),
                    shrink: 0.6,
                    ..ColorbarOptions::default()
                },
            )?;
        }
    }
    Ok(())
}

fn colorbar_placement_1(fig: &Figure) -> CaseResult {
    cmap_columns(fig, false)
}

fn colorbar_placement_2(fig: &Figure) -> CaseResult {
    cmap_columns(fig, true)
}

/// A random mesh with a colorbar under the line of its column sums, laid
/// out by `layout`.
fn summed_mesh(fig: &Figure, layout: Option<LayoutEngine>) -> CaseResult {
    let x = normal(&mut SpecRng::new(SEED), 20, 20)?;
    fig.set_layout_engine(layout)?;
    let axs = fig.subplots_shared(2, 1, Share::All, Share::None)?;
    let sums: Vec<f64> = (0..20)
        .map(|col| (0..20).map(|row| x[(row, col)]).sum())
        .collect();
    axs[(0, 0)].plot_y(sums);
    let pcm = axs[(1, 0)].pcolormesh(Mesh::new(x), PcolorOptions::default())?;
    fig.colorbar(
        &pcm,
        ColorbarOptions {
            shrink: 0.6,
            ..ColorbarOptions::beside(&axs[(1, 0)])
        },
    )?;
    Ok(())
}

fn colorbar_placement_3(fig: &Figure) -> CaseResult {
    summed_mesh(fig, None)
}

fn colorbar_placement_4(fig: &Figure) -> CaseResult {
    summed_mesh(fig, Some(LayoutEngine::constrained()))
}

fn colorbar_placement_5(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(3, 3)?;
    let mut last = None;
    for ax in axs.as_slice() {
        last = Some(ax.pcolormesh(
            Mesh::new(uniform(&mut rng, 20, 20)?),
            PcolorOptions::default(),
        )?);
    }
    let pcm = last.ok_or("nine meshes")?;
    let block = |rows: std::ops::Range<usize>, cols: std::ops::Range<usize>| {
        rows.flat_map(|row| cols.clone().map(move |col| (row, col)))
            .map(|cell| axs[cell].clone())
            .collect::<Vec<_>>()
    };
    for (parents, location, shrink) in [
        (block(0..1, 0..2), SpineSide::Bottom, 0.6),
        (block(0..1, 2..3), SpineSide::Bottom, 1.0),
        (block(1..3, 0..3), SpineSide::Right, 0.6),
        (block(2..3, 1..2), SpineSide::Left, 1.0),
    ] {
        fig.colorbar(
            &pcm,
            ColorbarOptions {
                axes: ColorbarAxes::Parents(parents),
                location: Some(location),
                shrink,
                ..ColorbarOptions::default()
            },
        )?;
    }
    Ok(())
}

/// Three random meshes with colorbars at increasing pads, laid out by
/// `layout`.
fn padded_colorbars(fig: &Figure, layout: Option<LayoutEngine>, title: &str) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    fig.set_layout_engine(layout)?;
    let axs = fig.subplots(3, 1)?;
    for (ax, pad) in axs.as_slice().iter().zip([0.025, 0.05, 0.1]) {
        let pcm = mesh(ax, normal(&mut rng, 20, 20)?, "viridis")?;
        fig.colorbar(
            &pcm,
            ColorbarOptions {
                pad: Some(pad),
                label: format!("pad: {pad}"),
                ..ColorbarOptions::beside(ax)
            },
        )?;
    }
    fig.suptitle(title);
    Ok(())
}

fn colorbar_placement_6(fig: &Figure) -> CaseResult {
    padded_colorbars(
        fig,
        Some(LayoutEngine::constrained()),
        "layout='constrained'",
    )
}

fn colorbar_placement_7(fig: &Figure) -> CaseResult {
    padded_colorbars(fig, None, "No layout manager")
}

/// A random mesh with a horizontal colorbar inset at `bounds`, in axes or
/// (`in_data`) data coordinates.
fn inset_colorbar(fig: &Figure, bounds: [f64; 4], in_data: bool) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    let pcm = mesh(&ax, normal(&mut SpecRng::new(SEED), 20, 20)?, "viridis")?;
    ax.set_ylim(-4.0, 20.0)?;
    let cax = if in_data {
        ax.inset_axes_in(bounds, ax.trans_data())
    } else {
        ax.inset_axes(bounds)
    };
    fig.colorbar(
        &pcm,
        ColorbarOptions {
            axes: ColorbarAxes::Cax(cax),
            orientation: Some(Orientation::Horizontal),
            ..ColorbarOptions::default()
        },
    )?;
    Ok(())
}

fn colorbar_placement_8(fig: &Figure) -> CaseResult {
    inset_colorbar(fig, [0.3, 0.07, 0.4, 0.04], false)
}

fn colorbar_placement_9(fig: &Figure) -> CaseResult {
    inset_colorbar(fig, [7.5, -1.7, 5.0, 1.2], true)
}

/// A random 10 × 10 image on one axes laid out by `layout`.
fn random_image(
    fig: &Figure,
    layout: LayoutEngine,
) -> Result<(Axes, AxesImage), Box<dyn std::error::Error>> {
    fig.set_layout_engine(layout)?;
    let ax = fig.subplot();
    let image = ax.imshow(
        normal(&mut SpecRng::new(SEED), 10, 10)?,
        ImshowOptions::default().cmap(colormaps::get("viridis")?),
    )?;
    Ok((ax, image))
}

fn colorbar_placement_10(fig: &Figure) -> CaseResult {
    let (ax, pcm) = random_image(fig, LayoutEngine::constrained())?;
    fig.colorbar(&pcm, ColorbarOptions::beside(&ax))?;
    Ok(())
}

fn colorbar_placement_11(fig: &Figure) -> CaseResult {
    let (ax, pcm) = random_image(fig, LayoutEngine::compressed())?;
    ax.set_title("Colorbar with layout='compressed'")
        .set_fontsize(plt::fonts::RelativeSize::Medium);
    fig.colorbar(&pcm, ColorbarOptions::beside(&ax))?;
    Ok(())
}

fn colorbar_placement_12(fig: &Figure) -> CaseResult {
    let (ax, pcm) = random_image(fig, LayoutEngine::constrained())?;
    let cax = ax.inset_axes([1.04, 0.0, 0.05, 1.0]);
    ax.set_title("Colorbar with inset_axes")
        .set_fontsize(plt::fonts::RelativeSize::Medium);
    fig.colorbar(
        &pcm,
        ColorbarOptions {
            axes: ColorbarAxes::Cax(cax),
            ..ColorbarOptions::default()
        },
    )?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new(
        "gallery_guides_axes/colorbar_placement_1",
        colorbar_placement_1,
    ),
    FigureCase::new(
        "gallery_guides_axes/colorbar_placement_2",
        colorbar_placement_2,
    ),
    FigureCase::new(
        "gallery_guides_axes/colorbar_placement_3",
        colorbar_placement_3,
    )
    .sized(4.0, 5.0, 100.0),
    FigureCase::new(
        "gallery_guides_axes/colorbar_placement_4",
        colorbar_placement_4,
    )
    .sized(4.0, 5.0, 100.0),
    FigureCase::new(
        "gallery_guides_axes/colorbar_placement_5",
        colorbar_placement_5,
    ),
    FigureCase::new(
        "gallery_guides_axes/colorbar_placement_6",
        colorbar_placement_6,
    )
    .sized(5.0, 5.0, 100.0),
    FigureCase::new(
        "gallery_guides_axes/colorbar_placement_7",
        colorbar_placement_7,
    )
    .sized(5.0, 5.0, 100.0),
    FigureCase::new(
        "gallery_guides_axes/colorbar_placement_8",
        colorbar_placement_8,
    )
    .sized(4.0, 4.0, 100.0),
    FigureCase::new(
        "gallery_guides_axes/colorbar_placement_9",
        colorbar_placement_9,
    )
    .sized(4.0, 4.0, 100.0),
    FigureCase::new(
        "gallery_guides_axes/colorbar_placement_10",
        colorbar_placement_10,
    )
    .sized(4.0, 4.0, 100.0),
    FigureCase::new(
        "gallery_guides_axes/colorbar_placement_11",
        colorbar_placement_11,
    )
    .sized(4.0, 4.0, 100.0),
    FigureCase::new(
        "gallery_guides_axes/colorbar_placement_12",
        colorbar_placement_12,
    )
    .sized(4.0, 4.0, 100.0),
];
