//! `mosaic`: `subplot_mosaic` from nested lists and strings, empty cells,
//! ratios, grid margins, subfigures, polar and styled axes, nesting and
//! sharing.

use plt::axes::{HistPlotOptions, ImshowOptions, TickParams};
use plt::figure::{Mosaic, MosaicAxes, MosaicCell, MosaicError, MosaicOptions};
use plt::gridspec::SubplotOverrides;
use plt::layout::LayoutEngine;
use plt::numeric::Grid;
use plt::prelude::*;
use plt::stats::{BinEstimator, Bins, HistOptions};

use super::{SEED, annotate_axes};
use crate::figures::cases::common::mosaic;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `identify_axes`: every axes' label, big, in its middle.
fn identify_axes<'a>(axes: impl IntoIterator<Item = (&'a str, &'a Axes)>, points: f64) {
    for (label, ax) in axes {
        annotate_axes(ax, label, points);
    }
}

/// A constrained mosaic of `layout` with every axes identified at 48
/// points.
fn identified(
    fig: &Figure,
    layout: &Mosaic,
    options: &MosaicOptions,
) -> Result<MosaicAxes, MosaicError> {
    fig.set_layout_engine(LayoutEngine::constrained());
    let axd = fig.subplot_mosaic(layout, options)?;
    identify_axes(axd.iter(), 48.0);
    Ok(axd)
}

/// [`identified`] with the default options.
fn identified_default(fig: &Figure, layout: &Mosaic) -> CaseResult {
    identified(fig, layout, &MosaicOptions::default())?;
    Ok(())
}

/// The page's bar, line, histogram and image axes; the histogram has
/// `bins` bins.
fn plot_kinds(axes: [&Axes; 4], bins: Bins) -> CaseResult {
    let [bar, plot, hist, image] = axes;
    bar.bar(vec!["a", "b", "c"], [5.0, 7.0, 9.0], BarOptions::default())?;
    plot.plot_y([1.0, 2.0, 3.0]);
    let options = HistPlotOptions {
        layout: HistOptions {
            bins: Some(bins),
            ..HistOptions::default()
        },
        ..HistPlotOptions::default()
    };
    hist.hist(&[SpecRng::new(SEED).normal(1_500, 0.0, 1.0)], &options)?;
    image.imshow(
        Grid::from_vec(2, 2, vec![1.0, 2.0, 2.0, 1.0])?,
        ImshowOptions::default(),
    )?;
    Ok(())
}

fn mosaic_1(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained());
    let axs = fig.subplots(2, 2)?;
    plot_kinds(
        [&axs[(0, 0)], &axs[(0, 1)], &axs[(1, 0)], &axs[(1, 1)]],
        Bins::Estimator(BinEstimator::Auto),
    )?;
    for row in 0..2 {
        for col in 0..2 {
            annotate_axes(&axs[(row, col)], &format!("({row}, {col})"), 48.0);
        }
    }
    Ok(())
}

fn mosaic_2(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained());
    let axd = fig.subplot_mosaic(
        &mosaic(&[&["bar", "plot"], &["hist", "image"]])?,
        &MosaicOptions::default(),
    )?;
    let ax = |label: &str| axd.get(label).ok_or("a mosaic axes");
    plot_kinds(
        [ax("bar")?, ax("plot")?, ax("hist")?, ax("image")?],
        Bins::default(),
    )?;
    identify_axes(axd.iter(), 48.0);
    Ok(())
}

fn mosaic_3(fig: &Figure) -> CaseResult {
    identified_default(fig, &Mosaic::parse("\n    AB\n    CD\n    ")?)
}

fn mosaic_5(fig: &Figure) -> CaseResult {
    identified_default(fig, &Mosaic::parse("\n    ABD\n    CCD\n    ")?)
}

fn mosaic_6(fig: &Figure) -> CaseResult {
    identified_default(fig, &Mosaic::parse("\n    A.C\n    BBB\n    .D.\n    ")?)
}

fn mosaic_7(fig: &Figure) -> CaseResult {
    // `empty_sentinel="X"`: the X cells are empty.
    identified_default(fig, &mosaic(&[&["a", "."], &[".", "b"]])?)
}

fn mosaic_8(fig: &Figure) -> CaseResult {
    identified_default(fig, &Mosaic::parse("αб\n       ℝ☢")?)
}

fn mosaic_9(fig: &Figure) -> CaseResult {
    let options = MosaicOptions {
        height_ratios: Some(vec![1.0, 3.5, 1.0]),
        width_ratios: Some(vec![1.0, 3.5, 1.0]),
        ..MosaicOptions::default()
    };
    identified(
        fig,
        &Mosaic::parse("\n    .a.\n    bAc\n    .d.\n    ")?,
        &options,
    )?;
    Ok(())
}

/// The mosaic the grid and subfigure figures repeat.
fn wide_over_two() -> Result<Mosaic, MosaicError> {
    Mosaic::parse("AA\n            BC")
}

fn mosaic_10(fig: &Figure) -> CaseResult {
    for (bottom, top, left, right) in [(0.25, 0.95, 0.1, 0.5), (0.05, 0.75, 0.6, 0.95)] {
        let options = MosaicOptions {
            gridspec: SubplotOverrides {
                bottom: Some(bottom),
                top: Some(top),
                left: Some(left),
                right: Some(right),
                wspace: Some(0.5),
                hspace: Some(0.5),
            },
            ..MosaicOptions::default()
        };
        identify_axes(
            fig.subplot_mosaic(&wide_over_two()?, &options)?.iter(),
            48.0,
        );
    }
    Ok(())
}

fn mosaic_11(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained());
    let halves = fig.subfigures(1, 2, &plt::figure::SubfigureOptions::default())?;
    for subfig in halves.as_slice() {
        let axd = subfig.subplot_mosaic(&wide_over_two()?, &MosaicOptions::default())?;
        identify_axes(axd.iter(), 48.0);
    }
    Ok(())
}

fn mosaic_12(fig: &Figure) -> CaseResult {
    let options = MosaicOptions {
        polar: true,
        ..MosaicOptions::default()
    };
    identified(fig, &Mosaic::parse("AB")?, &options)?;
    Ok(())
}

/// The `AB;CD` mosaic with the axes of `polar` polar, identified, on a
/// figure without layout.
fn polar_mosaic(fig: &Figure, polar: &[&str]) -> Result<MosaicAxes, Box<dyn std::error::Error>> {
    let options = MosaicOptions {
        polar_labels: polar.iter().map(|&label| label.to_owned()).collect(),
        ..MosaicOptions::default()
    };
    let axd = fig.subplot_mosaic(&Mosaic::parse("AB;CD")?, &options)?;
    identify_axes(axd.iter(), 48.0);
    Ok(axd)
}

fn mosaic_13(fig: &Figure) -> CaseResult {
    let axd = polar_mosaic(fig, &["A"])?;
    for label in ["C", "D"] {
        axd.get(label).ok_or("a mosaic axes")?.set_xscale("log")?;
    }
    Ok(())
}

fn mosaic_14(fig: &Figure) -> CaseResult {
    let axd = polar_mosaic(fig, &["A", "D"])?;
    for label in ["B", "C"] {
        axd.get(label).ok_or("a mosaic axes")?.set_facecolor(".9");
    }
    Ok(())
}

fn mosaic_15(fig: &Figure) -> CaseResult {
    let options = MosaicOptions {
        polar_labels: vec!["D".to_owned()],
        ..MosaicOptions::default()
    };
    fig.set_layout_engine(LayoutEngine::constrained());
    let axd = fig.subplot_mosaic(&Mosaic::parse("AB;CD")?, &options)?;
    for (label, ax) in axd.iter() {
        ax.set_facecolor(match label {
            "B" => "xkcd:water blue",
            "D" => "w",
            _ => "xkcd:tangerine",
        });
    }
    identify_axes(axd.iter(), 48.0);
    Ok(())
}

fn mosaic_16(fig: &Figure) -> CaseResult {
    let options = MosaicOptions {
        width_ratios: Some(vec![2.0, 1.0]),
        ..MosaicOptions::default()
    };
    identified(
        fig,
        &mosaic(&[&["main", "zoom"], &["main", "."]])?,
        &options,
    )?;
    Ok(())
}

fn mosaic_17(fig: &Figure) -> CaseResult {
    let label = |name: &str| MosaicCell::Label(name.to_owned());
    let inner = mosaic(&[&["inner A"], &["inner B"]])?;
    let outer = Mosaic::new(vec![
        vec![label("main"), MosaicCell::Nested(inner)],
        vec![label("bottom"), label("bottom")],
    ])?;
    fig.set_layout_engine(LayoutEngine::constrained());
    let axd = fig.subplot_mosaic(&outer, &MosaicOptions::default())?;
    identify_axes(axd.iter(), 36.0);
    Ok(())
}

fn mosaic_18(fig: &Figure) -> CaseResult {
    let labels: Vec<String> = (1..=4).map(|j: i32| j.to_string()).collect();
    let rows = (0..4)
        .map(|row| {
            (0..4)
                .map(|col| {
                    if row == col {
                        MosaicCell::Label(labels[row].clone())
                    } else {
                        MosaicCell::Empty
                    }
                })
                .collect()
        })
        .collect();
    identified_default(fig, &Mosaic::new(rows)?)
}

/// The zigzag every axes of the sharing figures plots.
fn zigzags(axd: &MosaicAxes) -> CaseResult {
    for (_, ax) in axd.iter() {
        ax.plot([1.0, 2.0, 3.0, 4.0], [1.0, 4.0, 2.0, 3.0])?;
    }
    Ok(())
}

fn mosaic_19(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained());
    let options = MosaicOptions {
        sharex: true,
        ..MosaicOptions::default()
    };
    let axd = fig.subplot_mosaic(&Mosaic::parse("AC;BC")?, &options)?;
    for label in ["A", "B", "C"] {
        axd.get(label)
            .ok_or("a mosaic axes")?
            .set_title(format!("Axes {label}"));
    }
    zigzags(&axd)?;
    identify_axes(axd.iter(), 48.0);
    Ok(())
}

/// The `.A;BC` mosaic sharing y, with zigzags.
fn shared_y(fig: &Figure) -> Result<MosaicAxes, Box<dyn std::error::Error>> {
    fig.set_layout_engine(LayoutEngine::constrained());
    let options = MosaicOptions {
        sharey: true,
        ..MosaicOptions::default()
    };
    let axd = fig.subplot_mosaic(&Mosaic::parse(".A;BC")?, &options)?;
    zigzags(&axd)?;
    Ok(axd)
}

fn mosaic_20(fig: &Figure) -> CaseResult {
    identify_axes(shared_y(fig)?.iter(), 48.0);
    Ok(())
}

fn mosaic_21(fig: &Figure) -> CaseResult {
    let axd = shared_y(fig)?;
    axd.get("A")
        .ok_or("a mosaic axes")?
        .tick_params(&TickParams {
            labelleft: Some(true),
            ..TickParams::default()
        });
    identify_axes(axd.iter(), 48.0);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_guides_axes/mosaic_1", mosaic_1),
    FigureCase::new("gallery_guides_axes/mosaic_2", mosaic_2),
    FigureCase::new("gallery_guides_axes/mosaic_3", mosaic_3),
    FigureCase::new("gallery_guides_axes/mosaic_5", mosaic_5),
    FigureCase::new("gallery_guides_axes/mosaic_6", mosaic_6),
    FigureCase::new("gallery_guides_axes/mosaic_7", mosaic_7),
    FigureCase::new("gallery_guides_axes/mosaic_8", mosaic_8),
    FigureCase::new("gallery_guides_axes/mosaic_9", mosaic_9),
    FigureCase::new("gallery_guides_axes/mosaic_10", mosaic_10),
    FigureCase::new("gallery_guides_axes/mosaic_11", mosaic_11),
    FigureCase::new("gallery_guides_axes/mosaic_12", mosaic_12),
    FigureCase::new("gallery_guides_axes/mosaic_13", mosaic_13),
    FigureCase::new("gallery_guides_axes/mosaic_14", mosaic_14),
    FigureCase::new("gallery_guides_axes/mosaic_15", mosaic_15),
    FigureCase::new("gallery_guides_axes/mosaic_16", mosaic_16),
    FigureCase::new("gallery_guides_axes/mosaic_17", mosaic_17),
    FigureCase::new("gallery_guides_axes/mosaic_18", mosaic_18),
    FigureCase::new("gallery_guides_axes/mosaic_19", mosaic_19),
    FigureCase::new("gallery_guides_axes/mosaic_20", mosaic_20),
    FigureCase::new("gallery_guides_axes/mosaic_21", mosaic_21),
];
