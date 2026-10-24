//! `arranging_axes`: axes added by hand, grids from `subplots`,
//! `subplot_mosaic` and `GridSpec`, fixed-aspect grids, spans, ratios,
//! subfigures and nested grids.

use plt::colors::to_rgba;
use plt::figure::{Mosaic, MosaicCell, MosaicOptions, SubfigureOptions};
use plt::gridspec::{GridSpec, SubplotOverrides, SubplotSpec};
use plt::layout::LayoutEngine;
use plt::prelude::*;
use plt::rc::Aspect;
use plt::text::AnnotateOptions;

use super::{annotate_axes, centered};
use crate::figures::cases::common::mosaic;
use crate::figures::{CaseResult, FigureCase};

fn arranging_axes_1(fig: &Figure) -> CaseResult {
    let (w, h) = (4.0, 3.0);
    let margin = 0.5;
    fig.set_facecolor(to_rgba("lightblue", None)?);
    fig.add_axes([
        margin / w,
        margin / h,
        (w - 2.0 * margin) / w,
        (h - 2.0 * margin) / h,
    ])?;
    Ok(())
}

fn arranging_axes_2(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(2, 2)?;
    for row in 0..2 {
        for col in 0..2 {
            let label = axs[(row, col)].annotate(
                format!("axs[{row}, {col}]"),
                (0.5, 0.5),
                AnnotateOptions::default(),
            )?;
            centered(&label, 18.0);
        }
    }
    fig.suptitle("plt.subplots()");
    Ok(())
}

/// A constrained mosaic with every axes named by its key, under the
/// `plt.subplot_mosaic()` title.
fn annotated_mosaic(fig: &Figure, layout: &Mosaic, options: &MosaicOptions) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    for (label, ax) in fig.subplot_mosaic(layout, options.clone())?.iter() {
        annotate_axes(ax, &format!("axd['{label}']"), 14.0);
    }
    fig.suptitle("plt.subplot_mosaic()");
    Ok(())
}

/// The mosaic of the spanning examples: two axes on the left, one on the
/// right.
fn left_pair_right() -> Result<Mosaic, plt::figure::MosaicError> {
    mosaic(&[&["upper left", "right"], &["lower left", "right"]])
}

fn arranging_axes_3(fig: &Figure) -> CaseResult {
    let layout = mosaic(&[
        &["upper left", "upper right"],
        &["lower left", "lower right"],
    ])?;
    annotated_mosaic(fig, &layout, &MosaicOptions::default())
}

fn arranging_axes_5(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::compressed())?;
    fig.set_facecolor(to_rgba("lightblue", None)?);
    for ax in fig.subplots(2, 2)?.as_slice() {
        ax.set_aspect(Aspect::Ratio(1.0))?;
    }
    fig.suptitle("Fixed aspect Axes: compressed");
    Ok(())
}

fn arranging_axes_6(fig: &Figure) -> CaseResult {
    annotated_mosaic(fig, &left_pair_right()?, &MosaicOptions::default())
}

fn arranging_axes_7(fig: &Figure) -> CaseResult {
    let options = MosaicOptions {
        width_ratios: Some(vec![1.4, 1.0]),
        height_ratios: Some(vec![1.0, 2.0]),
        ..MosaicOptions::default()
    };
    annotated_mosaic(fig, &left_pair_right()?, &options)
}

fn arranging_axes_8(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let subfigs = fig.subfigures(
        1,
        2,
        SubfigureOptions {
            wspace: Some(0.07),
            width_ratios: Some(vec![1.5, 1.0]),
            ..SubfigureOptions::default()
        },
    )?;
    let (left, right) = (&subfigs[(0, 0)], &subfigs[(0, 1)]);
    left.subplots(2, 2)?;
    left.set_facecolor(to_rgba("lightblue", None)?);
    left.suptitle("subfigs[0]\nLeft side");
    left.supxlabel("xlabel for subfigs[0]");
    right.subplots(3, 1)?;
    right.suptitle("subfigs[1]");
    right.supylabel("ylabel for subfigs[1]");
    Ok(())
}

fn arranging_axes_9(fig: &Figure) -> CaseResult {
    let label = |name: &str| MosaicCell::Label(name.to_owned());
    let inner = mosaic(&[&["innerA"], &["innerB"]])?;
    let outer = Mosaic::new(vec![
        vec![label("upper left"), MosaicCell::Nested(inner)],
        vec![label("lower left"), label("lower right")],
    ])?;
    fig.set_layout_engine(LayoutEngine::constrained())?;
    for (name, ax) in fig.subplot_mosaic(&outer, MosaicOptions::default())?.iter() {
        annotate_axes(ax, &format!("axd['{name}']"), 18.0);
    }
    Ok(())
}

/// Constrained axes on the `cells` of a grid, each named in its middle,
/// under `title`.
fn named_cells(fig: &Figure, cells: &[(&str, SubplotSpec)], title: &str) -> CaseResult {
    for (name, cell) in cells {
        annotate_axes(&fig.add_subplot_spec(cell)?, name, 18.0);
    }
    fig.suptitle(title);
    Ok(())
}

fn arranging_axes_10(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let spec = fig.add_gridspec(2, 2)?;
    let cells = [
        ("ax0", spec.cell(0, 0)?),
        ("ax1", spec.cell(0, 1)?),
        ("ax2", spec.cell(1, 0)?),
        ("ax3", spec.cell(1, 1)?),
    ];
    named_cells(fig, &cells, "Manually added subplots using add_gridspec")
}

fn arranging_axes_11(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let spec = fig.add_gridspec(2, 2)?;
    let cells = [
        ("ax0", spec.span(0..1, 0..2)?),
        ("ax10", spec.cell(1, 0)?),
        ("ax11", spec.cell(1, 1)?),
    ];
    named_cells(fig, &cells, "Manually added subplots, spanning a column")
}

fn arranging_axes_12(fig: &Figure) -> CaseResult {
    fig.set_facecolor(to_rgba("lightblue", None)?);
    let gs = GridSpec::with_ratios(
        3,
        3,
        None,
        None,
        SubplotOverrides {
            left: Some(0.05),
            right: Some(0.75),
            hspace: Some(0.1),
            wspace: Some(0.05),
            ..SubplotOverrides::default()
        },
    )?;
    let cells = [
        ("ax0", gs.span(0..2, 0..3)?),
        ("ax1", gs.span(2..3, 0..2)?),
        ("ax2", gs.cell(2, 2)?),
    ];
    named_cells(fig, &cells, "Manual gridspec with right=0.75")
}

fn arranging_axes_13(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let gs0 = fig.add_gridspec(1, 2)?;
    let gs00 = GridSpec::nested(&gs0.cell(0, 0)?, 2, 2, None, None, (None, None))?;
    let gs01 = GridSpec::nested(&gs0.cell(0, 1)?, 3, 1, None, None, (None, None))?;
    for a in 0..2 {
        for b in 0..2 {
            let ax = fig.add_subplot_spec(&gs00.cell(a, b)?)?;
            annotate_axes(&ax, &format!("axLeft[{a}, {b}]"), 10.0);
            if a == 1 && b == 1 {
                ax.set_xlabel("xlabel");
            }
        }
    }
    // The example labels the right column with the left loop's last `b`.
    for a in 0..3 {
        let ax = fig.add_subplot_spec(&gs01.cell(a, 0)?)?;
        annotate_axes(&ax, &format!("axRight[{a}, 1]"), 18.0);
        if a == 2 {
            ax.set_ylabel("ylabel");
        }
    }
    fig.suptitle("nested gridspecs");
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_guides_axes/arranging_axes_1", arranging_axes_1)
        .sized(4.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_axes/arranging_axes_2", arranging_axes_2)
        .sized(5.5, 3.5, 100.0),
    FigureCase::new("gallery_guides_axes/arranging_axes_3", arranging_axes_3)
        .sized(5.5, 3.5, 100.0),
    FigureCase::new("gallery_guides_axes/arranging_axes_5", arranging_axes_5)
        .sized(5.5, 3.5, 100.0),
    FigureCase::new("gallery_guides_axes/arranging_axes_6", arranging_axes_6)
        .sized(5.5, 3.5, 100.0),
    FigureCase::new("gallery_guides_axes/arranging_axes_7", arranging_axes_7)
        .sized(5.5, 3.5, 100.0),
    FigureCase::new("gallery_guides_axes/arranging_axes_8", arranging_axes_8),
    FigureCase::new("gallery_guides_axes/arranging_axes_9", arranging_axes_9),
    FigureCase::new("gallery_guides_axes/arranging_axes_10", arranging_axes_10)
        .sized(5.5, 3.5, 100.0),
    FigureCase::new("gallery_guides_axes/arranging_axes_11", arranging_axes_11)
        .sized(5.5, 3.5, 100.0),
    FigureCase::new("gallery_guides_axes/arranging_axes_12", arranging_axes_12),
    FigureCase::new("gallery_guides_axes/arranging_axes_13", arranging_axes_13),
];
