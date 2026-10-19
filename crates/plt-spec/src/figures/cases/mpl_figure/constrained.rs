//! `test_constrainedlayout.py`: constrained and compressed layouts of
//! subplots, nested grids, colorbars, legends and super titles.

use std::rc::Rc;

use plt::axes::{ImshowOptions, TickDirection, TickParams};
use plt::collections::Collection;
use plt::colorbar::{ColorbarAxes, ColorbarOptions, ColorbarTicks};
use plt::contour::{ContourOptions, QuadField};
use plt::figure::{Mosaic, MosaicOptions, Share};
use plt::gridspec::{GridSpec, SubplotOverrides, SubplotSpec};
use plt::layout::{ConstrainedLayout, LayoutEngine};
use plt::legend::LegendOptions;
use plt::numeric::Grid;
use plt::orientation::Orientation;
use plt::prelude::*;
use plt::rc::Aspect;
use plt::spines::SpineSide;
use plt::ticker::{MaxNLocator, Nbins, NullFormatter};

use super::{
    Fallible, TIGHT_BBOX, constrained, example_pcolor, example_plot, remove_ticks_and_titles,
};
use crate::figures::{CaseResult, FigureCase};

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_figure/constrained_layout1", |fig| {
        constrained(fig, false);
        example_plot(&fig.subplot(), 24.0, false)
    }),
    FigureCase::new("mpl_figure/constrained_layout2", |fig| {
        constrained(fig, false);
        for ax in fig.subplots(2, 2)?.as_slice() {
            example_plot(ax, 24.0, false)?;
        }
        Ok(())
    }),
    FigureCase::new("mpl_figure/constrained_layout3", |fig| {
        constrained(fig, false);
        for (nn, ax) in fig.subplots(2, 2)?.as_slice().iter().enumerate() {
            let pcm = example_pcolor(ax, 24.0)?;
            let pad = if nn == 3 { 0.08 } else { 0.02 };
            fig.colorbar(
                &pcm,
                ColorbarOptions {
                    pad: Some(pad),
                    ..ColorbarOptions::beside(ax)
                },
            )?;
        }
        Ok(())
    }),
    FigureCase::new("mpl_figure/constrained_layout4", |fig| {
        shared_colorbar(fig, None)
    }),
    FigureCase::new("mpl_figure/constrained_layout5", |fig| {
        shared_colorbar(fig, Some(SpineSide::Bottom))
    }),
    FigureCase::new("mpl_figure/constrained_layout6", constrained_layout6),
    FigureCase::new("mpl_figure/constrained_layout8", constrained_layout8).sized(10.0, 5.0, 100.0),
    FigureCase::new("mpl_figure/constrained_layout9", |fig| {
        constrained(fig, false);
        let axs = fig.subplots(2, 2)?;
        let mut pcm = None;
        for ax in axs.as_slice() {
            pcm = Some(example_pcolor(ax, 24.0)?);
            ax.set_xlabel("").set_ylabel("");
        }
        axs[(1, 1)].set_aspect(Aspect::Ratio(2.0))?;
        fig.colorbar(
            &pcm.ok_or("no mesh")?,
            ColorbarOptions {
                pad: Some(0.01),
                shrink: 0.6,
                ..around(axs.as_slice())
            },
        )?;
        fig.suptitle("Test Suptitle").set_fontsize(28.0);
        Ok(())
    }),
    FigureCase::new("mpl_figure/constrained_layout10", |fig| {
        constrained(fig, false);
        let axs = fig.subplots(2, 2)?;
        for ax in axs.as_slice() {
            ax.plot_with(
                arange(0.0, 12.0, 1.0),
                arange(0.0, 12.0, 1.0),
                LineProps::default().label("This is a label"),
            )?;
        }
        axs[(1, 1)].legend(
            LegendOptions::default()
                .loc("center left")
                .bbox_to_anchor((0.8, 0.5)),
        )?;
        Ok(())
    }),
    FigureCase::new("mpl_figure/constrained_layout11", |fig| {
        constrained_layout11(fig, None, None)
    })
    .sized(13.0, 3.0, 100.0),
    FigureCase::new("mpl_figure/constrained_layout11rat", |fig| {
        constrained_layout11(fig, Some(vec![6.0, 1.0]), Some(vec![2.0, 1.0]))
    })
    .sized(10.0, 3.0, 100.0),
    FigureCase::new("mpl_figure/constrained_layout12", constrained_layout12).sized(6.0, 8.0, 100.0),
    FigureCase::new("mpl_figure/constrained_layout13", |fig| {
        padded_colorbars(fig, |layout| ConstrainedLayout {
            w_pad: 24.0 / 72.0,
            h_pad: 24.0 / 72.0,
            ..layout
        })
    }),
    FigureCase::new("mpl_figure/constrained_layout14", |fig| {
        padded_colorbars(fig, |layout| ConstrainedLayout {
            w_pad: 3.0 / 72.0,
            h_pad: 3.0 / 72.0,
            hspace: 0.2,
            wspace: 0.2,
            ..layout
        })
    }),
    FigureCase::new("mpl_figure/constrained_layout15", |fig| {
        for ax in fig.subplots(2, 2)?.as_slice() {
            example_plot(ax, 12.0, false)?;
        }
        Ok(())
    })
    .with_rc(&[("figure.constrained_layout.use", "True")]),
    FigureCase::new("mpl_figure/constrained_layout16", |fig| {
        constrained(fig, false);
        example_plot(&fig.subplot(), 12.0, false)?;
        fig.add_axes([0.2, 0.2, 0.4, 0.4]);
        Ok(())
    }),
    FigureCase::new("mpl_figure/constrained_layout17", |fig| {
        constrained(fig, false);
        let gs = GridSpec::new(3, 3)?;
        for spec in [
            gs.cell(0, 0)?,
            gs.span(0..1, 1..3)?,
            gs.span(1..3, 0..2)?,
            gs.span(1..3, 2..3)?,
        ] {
            example_plot(&fig.add_subplot_spec(&spec)?, 12.0, false)?;
        }
        Ok(())
    }),
    FigureCase::new("mpl_figure/colorbar_location", colorbar_location),
    FigureCase::new("mpl_figure/colorbars_no_overlapV", |fig| {
        colorbars_no_overlap(fig, Orientation::Vertical)
    })
    .sized(2.0, 4.0, 100.0),
    FigureCase::new("mpl_figure/colorbars_no_overlapH", |fig| {
        colorbars_no_overlap(fig, Orientation::Horizontal)
    })
    .sized(4.0, 2.0, 100.0),
    FigureCase::new("mpl_figure/bboxtight", |fig| {
        constrained(fig, false);
        fig.subplot().set_aspect(Aspect::Ratio(1.0))?;
        remove_ticks_and_titles(fig);
        Ok(())
    })
    .with_rc(&[TIGHT_BBOX]),
    FigureCase::new("mpl_figure/compressed_suptitle_colorbar", |fig| {
        compressed_colorbars(fig, 1, 2, Orientation::Vertical)?;
        fig.suptitle("Title");
        Ok(())
    })
    .sized(4.0, 2.0, 100.0),
    FigureCase::new("mpl_figure/compressed_supylabel_colorbar", |fig| {
        compressed_colorbars(fig, 2, 1, Orientation::Horizontal)?;
        fig.supylabel("Title");
        Ok(())
    })
    .sized(3.0, 4.0, 100.0),
    FigureCase::new("mpl_figure/submerged_with_colorbar", |fig| {
        constrained(fig, false);
        let axes =
            fig.subplot_mosaic(&Mosaic::parse("AABBCC;DDDEEE")?, &MosaicOptions::default())?;
        let ax = axes.get("A").ok_or("no axes A")?;
        let z = Grid::from_rows([[0.0, 1.0], [2.0, 3.0]])?;
        let filled = ax.contourf(&QuadField::new(z), ContourOptions::default())?;
        fig.colorbar(&filled, ColorbarOptions::default())?;
        Ok(())
    }),
];

/// Colorbar options stealing space from all of `axs`.
fn around(axs: &[Axes]) -> ColorbarOptions {
    ColorbarOptions {
        axes: ColorbarAxes::Parents(axs.to_vec()),
        ..ColorbarOptions::default()
    }
}

/// `test_constrained_layout4` and `5`: four meshes sharing one colorbar
/// at `location` (by default on the right).
fn shared_colorbar(fig: &Figure, location: Option<SpineSide>) -> CaseResult {
    constrained(fig, false);
    let axs = fig.subplots(2, 2)?;
    let mut pcm = None;
    for ax in axs.as_slice() {
        pcm = Some(example_pcolor(ax, 24.0)?);
    }
    fig.colorbar(
        &pcm.ok_or("no mesh")?,
        ColorbarOptions {
            use_gridspec: location.is_none(),
            location,
            pad: Some(0.01),
            shrink: 0.6,
            ..around(axs.as_slice())
        },
    )?;
    Ok(())
}

fn constrained_layout6(fig: &Figure) -> CaseResult {
    constrained(fig, false);
    let gs = GridSpec::new(1, 2)?;
    let gsl = GridSpec::nested(&gs.cell(0, 0)?, 2, 2, None, None, (None, None))?;
    let gsr = GridSpec::nested(&gs.cell(0, 1)?, 1, 2, None, None, (None, None))?;
    let mut last = None;
    for spec in cells(&gsl)? {
        let ax = fig.add_subplot_spec(&spec)?;
        example_plot(&ax, 12.0, false)?;
        last = Some(ax);
    }
    last.ok_or("no axes")?.set_xlabel("x-label\nMultiLine");
    let mut axsr = Vec::new();
    let mut pcm = None;
    for spec in cells(&gsr)? {
        let ax = fig.add_subplot_spec(&spec)?;
        pcm = Some(example_pcolor(&ax, 12.0)?);
        axsr.push(ax);
    }
    fig.colorbar(
        &pcm.ok_or("no mesh")?,
        ColorbarOptions {
            pad: Some(0.01),
            shrink: 0.99,
            location: Some(SpineSide::Bottom),
            ticks: Some(ColorbarTicks::Locator(Rc::new(MaxNLocator::new(
                Nbins::Fixed(5),
            )))),
            ..around(&axsr)
        },
    )?;
    Ok(())
}

/// The cells of `grid` in row-major order (iterating a `GridSpec`).
fn cells(grid: &GridSpec) -> Fallible<Vec<SubplotSpec>> {
    let (rows, cols) = grid.shape();
    let mut specs = Vec::with_capacity(rows * cols);
    for row in 0..rows {
        for col in 0..cols {
            specs.push(grid.cell(row, col)?);
        }
    }
    Ok(specs)
}

fn constrained_layout8(fig: &Figure) -> CaseResult {
    constrained(fig, false);
    let gs = GridSpec::new(3, 5)?;
    let mut axs = Vec::new();
    for (j, columns) in [(0, &[1][..]), (1, &[0, 4][..])] {
        for &i in columns {
            let ax = fig.add_subplot_spec(&gs.cell(j, i)?)?;
            example_pcolor(&ax, 9.0)?;
            if i > 0 {
                ax.set_ylabel("");
            }
            if j < 1 {
                ax.set_xlabel("");
            }
            ax.set_title("");
            axs.push(ax);
        }
    }
    let ax = fig.add_subplot_spec(&gs.span(2..3, 0..5)?)?;
    let pcm = example_pcolor(&ax, 9.0)?;
    axs.push(ax);
    fig.colorbar(
        &pcm,
        ColorbarOptions {
            pad: Some(0.01),
            shrink: 0.6,
            ..around(&axs)
        },
    )?;
    Ok(())
}

/// `test_constrained_layout11` and `11rat`: a plot beside a grid nested
/// two levels deep holding four meshes and their colorbar.
fn constrained_layout11(
    fig: &Figure,
    width_ratios: Option<Vec<f64>>,
    height_ratios: Option<Vec<f64>>,
) -> CaseResult {
    constrained(fig, false);
    let gs0 = GridSpec::with_ratios(1, 2, width_ratios, None, SubplotOverrides::default())?;
    let gsl = GridSpec::nested(&gs0.cell(0, 0)?, 1, 2, None, None, (None, None))?;
    let gsl0 = GridSpec::nested(&gsl.cell(0, 1)?, 2, 2, None, height_ratios, (None, None))?;
    example_plot(&fig.add_subplot_spec(&gs0.cell(0, 1)?)?, 9.0, false)?;
    let mut axs = Vec::new();
    let mut pcm = None;
    for spec in cells(&gsl0)? {
        let ax = fig.add_subplot_spec(&spec)?;
        pcm = Some(example_pcolor(&ax, 9.0)?);
        axs.push(ax);
    }
    fig.colorbar(
        &pcm.ok_or("no mesh")?,
        ColorbarOptions {
            shrink: 0.6,
            aspect: 70.0,
            ..around(&axs)
        },
    )?;
    example_plot(&fig.add_subplot_spec(&gsl.cell(0, 0)?)?, 9.0, false)
}

fn constrained_layout12(fig: &Figure) -> CaseResult {
    constrained(fig, false);
    let gs0 = GridSpec::new(6, 2)?;
    let ax1 = fig.add_subplot_spec(&gs0.span(0..3, 1..2)?)?;
    let ax2 = fig.add_subplot_spec(&gs0.span(3..6, 1..2)?)?;
    example_plot(&ax1, 18.0, false)?;
    example_plot(&ax2, 18.0, false)?;
    let mut last = None;
    for rows in [0..2, 2..4, 4..6] {
        let ax = fig.add_subplot_spec(&gs0.span(rows, 0..1)?)?;
        example_plot(&ax, 12.0, true)?;
        last = Some(ax);
    }
    last.ok_or("no axes")?.set_xlabel("x-label");
    Ok(())
}

/// `test_constrained_layout13` and `14`: a colorbar per mesh, then the
/// engine's padding and spacing changed by `adjust`.
fn padded_colorbars(
    fig: &Figure,
    adjust: fn(ConstrainedLayout) -> ConstrainedLayout,
) -> CaseResult {
    constrained(fig, false);
    for ax in fig.subplots(2, 2)?.as_slice() {
        let pcm = example_pcolor(ax, 12.0)?;
        fig.colorbar(
            &pcm,
            ColorbarOptions {
                shrink: 0.6,
                aspect: 20.0,
                pad: Some(0.02),
                ..ColorbarOptions::beside(ax)
            },
        )?;
    }
    let layout = ConstrainedLayout::from_rc(&plt::rc::params());
    fig.set_layout_engine(Some(LayoutEngine::Constrained(adjust(layout))));
    Ok(())
}

fn colorbar_location(fig: &Figure) -> CaseResult {
    constrained(fig, false);
    let axs = fig.subplots(4, 5)?;
    let mut pcm: Option<Collection> = None;
    for ax in axs.as_slice() {
        pcm = Some(example_pcolor(ax, 12.0)?);
        ax.set_xlabel("").set_ylabel("");
    }
    let pcm = pcm.ok_or("no mesh")?;
    let block = |rows: std::ops::Range<usize>, cols: std::ops::Range<usize>| -> Vec<Axes> {
        rows.flat_map(|r| cols.clone().map(move |c| (r, c)))
            .map(|cell| axs[cell].clone())
            .collect()
    };
    let colorbars = [
        (around(&block(0..4, 1..2)), 0.4, None, None),
        (
            around(&block(3..4, 0..2)),
            0.5,
            Some(SpineSide::Bottom),
            None,
        ),
        (
            around(&block(0..1, 2..5)),
            0.5,
            Some(SpineSide::Bottom),
            Some(0.05),
        ),
        (around(&block(2..3, 3..5)), 0.5, Some(SpineSide::Top), None),
        (
            ColorbarOptions::beside(&axs[(0, 0)]),
            0.5,
            Some(SpineSide::Left),
            None,
        ),
        (
            around(&block(1..3, 2..3)),
            0.5,
            Some(SpineSide::Right),
            None,
        ),
    ];
    for (parents, shrink, location, pad) in colorbars {
        fig.colorbar(
            &pcm,
            ColorbarOptions {
                shrink,
                location,
                pad,
                ..parents
            },
        )?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_colorbars_no_overlapV` and `H`: two shared images, each with a
/// colorbar in `orientation`, under a suptitle.
fn colorbars_no_overlap(fig: &Figure, orientation: Orientation) -> CaseResult {
    constrained(fig, false);
    let (nrows, ncols) = match orientation {
        Orientation::Vertical => (2, 1),
        Orientation::Horizontal => (1, 2),
    };
    let axs = fig.subplots_shared(nrows, ncols, Share::All, Share::All)?;
    for ax in axs.as_slice() {
        ax.yaxis().set_major_formatter(NullFormatter);
        ax.tick_params(&TickParams {
            direction: Some(TickDirection::In),
            ..TickParams::default()
        });
        let image = ax.imshow(
            Grid::from_rows([[1.0, 2.0], [3.0, 4.0]])?,
            ImshowOptions::default(),
        )?;
        fig.colorbar(
            &image,
            ColorbarOptions {
                orientation: Some(orientation),
                ..ColorbarOptions::beside(ax)
            },
        )?;
    }
    fig.suptitle("foo");
    Ok(())
}

/// `test_compressed_suptitle_colorbar` and `supylabel`: images under the
/// compressed layout, each with a colorbar in `orientation`.
fn compressed_colorbars(
    fig: &Figure,
    nrows: usize,
    ncols: usize,
    orientation: Orientation,
) -> CaseResult {
    constrained(fig, true);
    let arr = Grid::from_fn(10, 10, |r, c| (r * 10 + c) as f64);
    for ax in fig.subplots(nrows, ncols)?.as_slice() {
        let image = ax.imshow(arr.clone(), ImshowOptions::default())?;
        fig.colorbar(
            &image,
            ColorbarOptions {
                orientation: Some(orientation),
                ..ColorbarOptions::beside(ax)
            },
        )?;
    }
    Ok(())
}
