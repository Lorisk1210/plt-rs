//! Image grids: aligned images, fixed-aspect cells, colorbars for each,
//! every row or column, or the whole grid, inner titles and RGB channel
//! axes (`simple_axesgrid`, `simple_axesgrid2`, `demo_imagegrid_aspect`,
//! `demo_axes_grid`, `demo_axes_grid2`, `demo_edge_colorbar` and
//! `demo_axes_rgb`).

use pltrs::axes::{ImshowOptions, TickDirection, TickParams};
use pltrs::axes_grid1::{
    CbarMode, GridDirection, GridRect, ImageGrid, ImageGridOptions, LabelMode, RgbAxes,
    SimpleAxisArtist, SizeSpec, make_rgb_axes,
};
use pltrs::colorbar::ColorbarOptions;
use pltrs::colors::{ColorizerOptions, colormaps};
use pltrs::fonts::FontProperties;
use pltrs::gridspec::GridSpec;
use pltrs::image::{Extent, ImageData, Origin};
use pltrs::numeric::Grid;
use pltrs::offsetbox::{AnchorLoc, AnchoredBox, AnchoredOptions};
use pltrs::patheffects::{GcUpdate, PathEffect, Stroke};
use pltrs::prelude::*;
use pltrs::rc::Aspect;
use pltrs::spines::SpineSide;
use pltrs::text::TextProps;

use super::EXTENT;
use crate::figures::cases::common::bivariate_normal;
use crate::figures::{CaseResult, FigureCase};

/// The cell `index` (1-based) of a `rows × cols` subplot grid (matplotlib's
/// three-digit subplot codes).
fn subplot(rows: usize, cols: usize, index: usize) -> Result<GridRect, Box<dyn std::error::Error>> {
    Ok(GridRect::Subplot(
        GridSpec::new(rows, cols)?.numbered(index)?,
    ))
}

/// Imshow options showing the data over `extent`, with row 0 at the
/// bottom when `lower`.
fn placed(extent: [f64; 4], lower: bool) -> ImshowOptions {
    let [left, right, bottom, top] = extent;
    ImshowOptions {
        extent: Some(Extent::new(left, right, bottom, top)),
        origin: lower.then_some(Origin::Lower),
        ..ImshowOptions::default()
    }
}

/// The options with the colour limits `vmin` and `vmax`.
fn limited(options: ImshowOptions, (vmin, vmax): (f64, f64)) -> ImshowOptions {
    options.colorizer(ColorizerOptions {
        vmin: Some(vmin),
        vmax: Some(vmax),
        ..ColorizerOptions::default()
    })
}

/// The smallest and largest values of `grids`.
fn value_range<'a>(grids: impl IntoIterator<Item = &'a Grid<f64>>) -> (f64, f64) {
    grids
        .into_iter()
        .flat_map(|grid| grid.as_slice().iter().copied())
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(v), hi.max(v))
        })
}

fn simple_axesgrid(fig: &Figure) -> CaseResult {
    let im1 = Grid::from_fn(10, 10, |r, c| (r * 10 + c) as f64);
    let im2 = im1.transposed();
    let im3 = Grid::from_fn(10, 10, |r, c| im1[(9 - r, c)]);
    let im4 = Grid::from_fn(10, 10, |r, c| im2[(r, 9 - c)]);
    let options = ImageGridOptions {
        axes_pad: (0.1, 0.1),
        ..ImageGridOptions::default()
    };
    let grid = ImageGrid::new(fig, &subplot(1, 1, 1)?, (2, 2), options)?;
    for (ax, image) in grid.axes().iter().zip([im1, im2, im3, im4]) {
        ax.imshow(image, ImshowOptions::default())?;
    }
    Ok(())
}

fn simple_axesgrid2(fig: &Figure) -> CaseResult {
    let options = ImageGridOptions {
        axes_pad: (0.1, 0.1),
        label_mode: LabelMode::L,
        ..ImageGridOptions::default()
    };
    let grid = ImageGrid::new(fig, &subplot(1, 1, 1)?, (1, 3), options)?;
    let z = bivariate_normal();
    let limits = value_range([&z]);
    let left = Grid::from_fn(15, 10, |r, c| z[(r, c)]);
    let right = Grid::from_fn(15, 5, |r, c| z[(r, c + 10)]);
    for (ax, image) in grid.axes().iter().zip([z.clone(), left, right]) {
        let options = ImshowOptions {
            origin: Some(Origin::Lower),
            ..ImshowOptions::default()
        };
        ax.imshow(image, limited(options, limits))?;
    }
    Ok(())
}

fn demo_imagegrid_aspect(fig: &Figure) -> CaseResult {
    let options = ImageGridOptions {
        axes_pad: (0.1, 0.1),
        aspect: true,
        share_all: true,
        ..ImageGridOptions::default()
    };
    for (position, stretched) in [(1, [0, 1]), (2, [1, 3])] {
        let grid = ImageGrid::new(fig, &subplot(1, 2, position)?, (2, 2), options.clone())?;
        for i in stretched {
            grid.axes()[i].set_aspect(Aspect::Ratio(2.0))?;
        }
    }
    Ok(())
}

/// Ticks at -2, 0 and 2 on the lower left axes (shared by its column and
/// row, or by all).
fn llc_ticks(grid: &ImageGrid) {
    let ticks = vec![-2.0, 0.0, 2.0];
    grid.axes_llc().set_xticks(ticks.clone()).set_yticks(ticks);
}

/// No tick labels at the top of `ax`.
fn no_top_labels(ax: &Axes) {
    ax.tick_params(TickParams {
        labeltop: Some(false),
        ..TickParams::default()
    });
}

fn demo_axes_grid(fig: &Figure) -> CaseResult {
    let z = bivariate_normal();
    let image = || placed(EXTENT, false);

    let options = ImageGridOptions {
        axes_pad: (0.05, 0.05),
        label_mode: LabelMode::One,
        ..ImageGridOptions::default()
    };
    let grid = ImageGrid::new(fig, &subplot(1, 4, 1)?, (2, 2), options)?;
    for ax in grid.axes() {
        ax.imshow(z.clone(), image())?;
    }
    llc_ticks(&grid);

    let options = ImageGridOptions {
        axes_pad: (0.0, 0.0),
        label_mode: LabelMode::L,
        share_all: true,
        cbar_location: SpineSide::Top,
        cbar_mode: Some(CbarMode::Single),
        ..ImageGridOptions::default()
    };
    let grid = ImageGrid::new(fig, &subplot(1, 4, 2)?, (2, 2), options)?;
    let mut last = None;
    for ax in grid.axes() {
        last = Some(ax.imshow(z.clone(), image())?);
    }
    grid.cbar_axes()[0].colorbar(
        &last.ok_or("the grid has axes")?,
        ColorbarOptions::default(),
    )?;
    for cax in grid.cbar_axes() {
        no_top_labels(cax.ax());
    }
    llc_ticks(&grid);

    let options = ImageGridOptions {
        axes_pad: (0.1, 0.1),
        label_mode: LabelMode::One,
        share_all: true,
        cbar_location: SpineSide::Top,
        cbar_mode: Some(CbarMode::Each),
        cbar_size: SizeSpec::Percent(7.0),
        cbar_pad: Some(SizeSpec::Percent(2.0)),
        ..ImageGridOptions::default()
    };
    let grid = ImageGrid::new(fig, &subplot(1, 4, 3)?, (2, 2), options)?;
    for (ax, cax) in grid.axes().iter().zip(grid.cbar_axes()) {
        let shown = ax.imshow(z.clone(), image())?;
        cax.colorbar(&shown, ColorbarOptions::default())?;
        no_top_labels(cax.ax());
    }
    llc_ticks(&grid);

    let options = ImageGridOptions {
        axes_pad: (0.45, 0.15),
        label_mode: LabelMode::One,
        share_all: true,
        cbar_location: SpineSide::Right,
        cbar_mode: Some(CbarMode::Each),
        cbar_size: SizeSpec::Percent(7.0),
        cbar_pad: Some(SizeSpec::Percent(2.0)),
        ..ImageGridOptions::default()
    };
    let grid = ImageGrid::new(fig, &subplot(1, 4, 4)?, (2, 2), options)?;
    let limits = [(0.0, 1.0), (-2.0, 2.0), (-1.7, 1.4), (-1.5, 1.0)];
    for ((ax, cax), (vmin, vmax)) in grid.axes().iter().zip(grid.cbar_axes()).zip(limits) {
        let shown = ax.imshow(z.clone(), limited(image(), (vmin, vmax)))?;
        cax.colorbar(&shown, ColorbarOptions::default())?
            .set_ticks(vec![vmin, vmax]);
    }
    llc_ticks(&grid);
    Ok(())
}

/// `add_inner_title`: frameless text with a white halo in a corner.
fn inner_title(ax: &Axes, title: &str, loc: AnchorLoc) -> CaseResult {
    let rc = pltrs::rc::params();
    let halo = Stroke::new(GcUpdate::default().linewidth(3.0).foreground("w"));
    let props = TextProps::default()
        .font(FontProperties::from_rc(&rc).with_size(rc.legend_fontsize, rc.font_size))
        .path_effects(vec![PathEffect::with_stroke(halo)]);
    let options = AnchoredOptions {
        pad: 0.0,
        borderpad: (0.5, 0.5),
        frameon: false,
        ..AnchoredOptions::new(loc)
    };
    ax.add_anchored_box(AnchoredBox::text(title, props, options)?);
    Ok(())
}

fn demo_axes_grid2(fig: &Figure) -> CaseResult {
    let z = bivariate_normal();
    // `Z[i::3, :]`: every third row from row `i`.
    let zs: Vec<Grid<f64>> = (0..3)
        .map(|i| Grid::from_fn(5, 15, |r, c| z[(i + 3 * r, c)]))
        .collect();
    let [left, right, bottom, top] = EXTENT;
    let extent = [left, right / 3.0, bottom, top];

    let options = ImageGridOptions {
        axes_pad: (0.05, 0.05),
        label_mode: LabelMode::One,
        share_all: true,
        cbar_location: SpineSide::Top,
        cbar_mode: Some(CbarMode::Each),
        cbar_size: SizeSpec::Percent(7.0),
        cbar_pad: Some(SizeSpec::Percent(1.0)),
        ..ImageGridOptions::default()
    };
    let grid = ImageGrid::new(fig, &subplot(2, 1, 1)?, (1, 3), options)?;
    grid.axes()[0]
        .set_xticks(vec![-2.0, 0.0])
        .set_yticks(vec![-2.0, 0.0, 2.0]);
    for (i, (ax, zi)) in grid.axes().iter().zip(&zs).enumerate() {
        let shown = ax.imshow(zi.clone(), placed(extent, true))?;
        let colorbar = grid
            .cax(i)
            .ok_or("every axes has a colorbar axes")?
            .colorbar(&shown, ColorbarOptions::default())?;
        if i > 0 {
            colorbar.set_ticks(vec![-1.0, 0.0, 1.0]);
        }
    }
    for (ax, title) in grid.axes().iter().zip(["Image 1", "Image 2", "Image 3"]) {
        inner_title(ax, title, AnchorLoc::LowerLeft)?;
    }

    let options = ImageGridOptions {
        axes_pad: (0.05, 0.05),
        label_mode: LabelMode::One,
        share_all: true,
        cbar_location: SpineSide::Right,
        cbar_mode: Some(CbarMode::Single),
        cbar_size: SizeSpec::Percent(10.0),
        cbar_pad: Some(SizeSpec::Inches(0.05)),
        ..ImageGridOptions::default()
    };
    let grid2 = ImageGrid::new(fig, &subplot(2, 1, 2)?, (1, 3), options)?;
    let first = &grid2.axes()[0];
    first.set_xlabel("X").set_ylabel("Y");
    first
        .set_xticks(vec![-2.0, 0.0])
        .set_yticks(vec![-2.0, 0.0, 2.0]);
    let clim = value_range(&zs);
    let mut last = None;
    for (ax, zi) in grid2.axes().iter().zip(&zs) {
        last = Some(ax.imshow(zi.clone(), limited(placed(extent, true), clim))?);
    }
    grid2
        .cax(2)
        .ok_or("the grid has a colorbar axes")?
        .colorbar(
            &last.ok_or("the grid has axes")?,
            ColorbarOptions::default(),
        )?;
    for (ax, title) in grid2.axes().iter().zip(["(a)", "(b)", "(c)"]) {
        inner_title(ax, title, AnchorLoc::UpperLeft)?;
    }
    Ok(())
}

/// `demo_bottom_cbar` / `demo_right_cbar`: a 2 × 2 grid with one colorbar
/// per column or row, labelled `label`.
fn edge_colorbars(
    fig: &Figure,
    position: usize,
    cmaps: [&str; 2],
    label: &str,
    options: &ImageGridOptions,
) -> CaseResult {
    let options = ImageGridOptions {
        axes_pad: (0.1, 0.1),
        share_all: true,
        label_mode: LabelMode::One,
        cbar_mode: Some(CbarMode::Edge),
        ..options.clone()
    };
    let grid = ImageGrid::new(fig, &subplot(1, 2, position)?, (2, 2), options)?;
    for (i, ax) in grid.axes().iter().enumerate() {
        let colors = ColorizerOptions::default().cmap(colormaps::get(cmaps[i / 2])?);
        let shown = ax.imshow(bivariate_normal(), placed(EXTENT, false).colorizer(colors))?;
        if i % 2 == 1 {
            grid.cbar_axes()[i / 2].colorbar(&shown, ColorbarOptions::default())?;
        }
    }
    for cax in grid.cbar_axes() {
        SimpleAxisArtist::new(cax.ax(), cax.location()).set_label(label);
    }
    llc_ticks(&grid);
    Ok(())
}

fn demo_edge_colorbar(fig: &Figure) -> CaseResult {
    let bottom = ImageGridOptions {
        cbar_location: SpineSide::Bottom,
        cbar_pad: Some(SizeSpec::Inches(0.25)),
        cbar_size: SizeSpec::Percent(15.0),
        direction: GridDirection::Column,
        ..ImageGridOptions::default()
    };
    edge_colorbars(fig, 1, ["autumn", "summer"], "Bar", &bottom)?;
    let right = ImageGridOptions {
        cbar_location: SpineSide::Right,
        cbar_size: SizeSpec::Percent(7.0),
        cbar_pad: Some(SizeSpec::Percent(2.0)),
        ..ImageGridOptions::default()
    };
    edge_colorbars(fig, 2, ["spring", "winter"], "Foo", &right)
}

/// `get_rgb`: three overlapping 13 × 13 crops of the clipped, normalised
/// sample data.
fn rgb_channels() -> [Grid<f64>; 3] {
    let z = bivariate_normal().map(|&v| v.max(0.0));
    let peak = z
        .as_slice()
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let crop = |row: usize, col: usize| Grid::from_fn(13, 13, |r, c| z[(r + row, c + col)] / peak);
    [crop(0, 0), crop(2, 2), crop(0, 2)]
}

fn demo_axes_rgb_1(fig: &Figure) -> CaseResult {
    let [r, g, b] = rgb_channels();
    RgbAxes::new(fig, [0.1, 0.1, 0.8, 0.8], 0.0)?.imshow_rgb(&r, &g, &b, ImshowOptions::default)?;
    Ok(())
}

fn demo_axes_rgb_2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let channel_axes = make_rgb_axes(&ax, 0.02);
    let channels = rgb_channels();
    let (rows, cols) = channels[0].shape();
    let cube = |pick: [bool; 3]| {
        let pixels = Grid::from_fn(rows, cols, |i, j| {
            let value = |k: usize| if pick[k] { channels[k][(i, j)] } else { 0.0 };
            [value(0), value(1), value(2)]
        });
        ImageData::Rgb(pixels)
    };
    ax.imshow(cube([true, true, true]), ImshowOptions::default())?;
    for (axes, pick) in channel_axes.iter().zip([
        [true, false, false],
        [false, true, false],
        [false, false, true],
    ]) {
        axes.imshow(cube(pick), ImshowOptions::default())?;
    }
    for axes in fig.axes() {
        axes.tick_params(TickParams {
            direction: Some(TickDirection::In),
            color: Some("w".into()),
            ..TickParams::default()
        });
        for spine in axes.spines().iter() {
            spine.set_color("w");
        }
    }
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_shapes/simple_axesgrid", simple_axesgrid).sized(4.0, 4.0, 100.0),
    FigureCase::new("gallery_shapes/simple_axesgrid2", simple_axesgrid2).sized(5.5, 3.5, 100.0),
    FigureCase::new(
        "gallery_shapes/demo_imagegrid_aspect",
        demo_imagegrid_aspect,
    ),
    FigureCase::new("gallery_shapes/demo_axes_grid", demo_axes_grid).sized(10.5, 2.5, 100.0),
    FigureCase::new("gallery_shapes/demo_axes_grid2", demo_axes_grid2).sized(6.0, 6.0, 100.0),
    FigureCase::new("gallery_shapes/demo_edge_colorbar", demo_edge_colorbar),
    FigureCase::new("gallery_shapes/demo_axes_rgb_1", demo_axes_rgb_1),
    FigureCase::new("gallery_shapes/demo_axes_rgb_2", demo_axes_rgb_2),
];
