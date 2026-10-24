//! `tight_layout_guide`: tight layout of single and gridded axes,
//! padding, spanning subplots, images, grid specifications laid out into
//! a rectangle, legends in and out of the layout, an `axes_grid1` grid and
//! colorbars.

use plt::axes::ImshowOptions;
use plt::axes::TitleLocation;
use plt::axes_grid1::{
    GridRect, ImageGrid, ImageGridOptions, LabelMode, SizeSpec, make_axes_locatable,
};
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::gridspec::GridSpec;
use plt::image::Interpolation;
use plt::layout::TightLayout;
use plt::legend::LegendOptions;
use plt::prelude::*;
use plt::spines::SpineSide;

use super::{LAYOUT_RC, arange_grid, subplot2grid};
use crate::figures::cases::common::example_plot;
use crate::figures::{CaseResult, FigureCase};

/// A case of the guide, on the default canvas.
const fn tight(name: &'static str, build: fn(&Figure) -> CaseResult) -> FigureCase {
    FigureCase::new(name, build).with_rc(LAYOUT_RC)
}

/// `example_plot` on each of `axes`.
fn examples(axes: &[Axes]) -> CaseResult {
    for ax in axes {
        example_plot(ax, 12.0, false)?;
    }
    Ok(())
}

/// [`examples`] on `axes`, then `tight_layout` with `params`.
fn tight_examples(fig: &Figure, axes: &[Axes], params: &TightLayout) -> CaseResult {
    examples(axes)?;
    fig.tight_layout(params)?;
    Ok(())
}

fn tight_layout_guide_1(fig: &Figure) -> CaseResult {
    example_plot(&fig.subplot(), 24.0, false)
}

fn tight_layout_guide_2(fig: &Figure) -> CaseResult {
    example_plot(&fig.subplot(), 24.0, false)?;
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

fn tight_layout_guide_3(fig: &Figure) -> CaseResult {
    examples(fig.subplots(2, 2)?.as_slice())
}

fn tight_layout_guide_4(fig: &Figure) -> CaseResult {
    tight_examples(fig, fig.subplots(2, 2)?.as_slice(), &TightLayout::default())
}

fn tight_layout_guide_5(fig: &Figure) -> CaseResult {
    let params = TightLayout {
        pad: 0.4,
        w_pad: Some(0.5),
        h_pad: Some(1.0),
        ..TightLayout::default()
    };
    tight_examples(fig, fig.subplots(2, 2)?.as_slice(), &params)
}

fn tight_layout_guide_6(fig: &Figure) -> CaseResult {
    let axes = [
        fig.add_subplot(2, 2, 1)?,
        fig.add_subplot(2, 2, 3)?,
        fig.add_subplot(1, 2, 2)?,
    ];
    tight_examples(fig, &axes, &TightLayout::default())
}

fn tight_layout_guide_7(fig: &Figure) -> CaseResult {
    subplot2grid(fig)?;
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

fn tight_layout_guide_8(fig: &Figure) -> CaseResult {
    fig.subplot().imshow(
        arange_grid(),
        ImshowOptions::default().interpolation(Interpolation::None),
    )?;
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

/// Two example plots on a 2 × 1 grid of their own, tightly laid out into
/// `rect` by the grid.
fn gridspec_column(fig: &Figure, rect: Option<[f64; 4]>) -> CaseResult {
    let gs1 = GridSpec::new(2, 1)?;
    for row in 0..2 {
        example_plot(&fig.add_subplot_spec(&gs1.cell(row, 0)?)?, 12.0, false)?;
    }
    let params = TightLayout {
        rect,
        ..TightLayout::default()
    };
    gs1.tight_layout(fig, &params)?;
    Ok(())
}

fn tight_layout_guide_9(fig: &Figure) -> CaseResult {
    gridspec_column(fig, None)
}

fn tight_layout_guide_10(fig: &Figure) -> CaseResult {
    gridspec_column(fig, Some([0.0, 0.0, 0.5, 1.0]))
}

/// A labelled line with a legend leaning out of the axes, tightly laid
/// out with or without the legend.
fn legend_line(fig: &Figure, label: &str, in_layout: bool) -> CaseResult {
    let ax = fig.subplot();
    let x = arange(0.0, 10.0, 1.0);
    ax.plot_with(&x, &x, LineProps::default().label(label))?;
    let leg = ax.legend(
        LegendOptions::default()
            .bbox_to_anchor((0.7, 0.5))
            .loc("center left"),
    )?;
    leg.set_in_layout(in_layout);
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

fn tight_layout_guide_11(fig: &Figure) -> CaseResult {
    legend_line(fig, "A simple plot", true)
}

fn tight_layout_guide_12(fig: &Figure) -> CaseResult {
    legend_line(fig, "B simple plot", false)
}

fn tight_layout_guide_13(fig: &Figure) -> CaseResult {
    let options = ImageGridOptions {
        axes_pad: (0.25, 0.25),
        aspect: false,
        label_mode: LabelMode::L,
        ..ImageGridOptions::default()
    };
    let rect = GridRect::Subplot(GridSpec::new(1, 1)?.cell(0, 0)?);
    let grid = ImageGrid::plain(fig, &rect, (2, 2), options)?;
    examples(grid.axes())?;
    grid.axes()
        .last()
        .ok_or("four axes")?
        .title_text(TitleLocation::Center)
        .set_visible(false);
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

/// An image of `arr` with a colorbar, tightly laid out; the colorbar
/// axes is appended by `make_axes_locatable` when `axes_locatable`.
fn imshow_colorbar(fig: &Figure, axes_locatable: bool) -> CaseResult {
    let ax = fig.subplot();
    let im = ax.imshow(
        arange_grid(),
        ImshowOptions::default().interpolation(Interpolation::None),
    )?;
    let options = if axes_locatable {
        let cax = make_axes_locatable(&ax).append_axes(
            SpineSide::Right,
            SizeSpec::Percent(5.0),
            Some(SizeSpec::Percent(3.0)),
        );
        ColorbarOptions {
            axes: ColorbarAxes::Cax(cax),
            ..ColorbarOptions::default()
        }
    } else {
        ColorbarOptions::default()
    };
    fig.colorbar(&im, options)?;
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

fn tight_layout_guide_14(fig: &Figure) -> CaseResult {
    imshow_colorbar(fig, false)
}

fn tight_layout_guide_15(fig: &Figure) -> CaseResult {
    imshow_colorbar(fig, true)
}

pub(super) const CASES: &[FigureCase] = &[
    tight(
        "gallery_guides_axes/tight_layout_guide_1",
        tight_layout_guide_1,
    ),
    tight(
        "gallery_guides_axes/tight_layout_guide_2",
        tight_layout_guide_2,
    ),
    tight(
        "gallery_guides_axes/tight_layout_guide_3",
        tight_layout_guide_3,
    ),
    tight(
        "gallery_guides_axes/tight_layout_guide_4",
        tight_layout_guide_4,
    ),
    tight(
        "gallery_guides_axes/tight_layout_guide_5",
        tight_layout_guide_5,
    ),
    tight(
        "gallery_guides_axes/tight_layout_guide_6",
        tight_layout_guide_6,
    ),
    tight(
        "gallery_guides_axes/tight_layout_guide_7",
        tight_layout_guide_7,
    ),
    tight(
        "gallery_guides_axes/tight_layout_guide_8",
        tight_layout_guide_8,
    )
    .sized(5.0, 4.0, 100.0),
    tight(
        "gallery_guides_axes/tight_layout_guide_9",
        tight_layout_guide_9,
    ),
    tight(
        "gallery_guides_axes/tight_layout_guide_10",
        tight_layout_guide_10,
    ),
    tight(
        "gallery_guides_axes/tight_layout_guide_11",
        tight_layout_guide_11,
    )
    .sized(4.0, 3.0, 100.0),
    tight(
        "gallery_guides_axes/tight_layout_guide_12",
        tight_layout_guide_12,
    )
    .sized(4.0, 3.0, 100.0),
    tight(
        "gallery_guides_axes/tight_layout_guide_13",
        tight_layout_guide_13,
    ),
    tight(
        "gallery_guides_axes/tight_layout_guide_14",
        tight_layout_guide_14,
    )
    .sized(4.0, 4.0, 100.0),
    tight(
        "gallery_guides_axes/tight_layout_guide_15",
        tight_layout_guide_15,
    )
    .sized(4.0, 4.0, 100.0),
];
