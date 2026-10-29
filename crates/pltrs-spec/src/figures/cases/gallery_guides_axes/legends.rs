//! `legend_guide`: proxy artists, legends placed by `bbox_to_anchor`,
//! figure legends outside the axes, several legends on one axes, tuple
//! handles, and handlers of the user's own (in `handlers`).

use pltrs::colors::to_rgba;
use pltrs::figure::MosaicOptions;
use pltrs::layout::LayoutEngine;
use pltrs::legend::{HandlerTuple, LegendHandle, LegendLoc, LegendMode, LegendOptions};
use pltrs::markers::Marker;
use pltrs::offsetbox::AnchorLoc;
use pltrs::prelude::*;

use super::{SEED, handlers};
use crate::figures::cases::common::mosaic;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// A rising and a falling line labelled `labels`, in the colours of
/// `fmts` (empty for the property cycle).
fn rising_falling(ax: &Axes, labels: [&str; 2], fmts: [&str; 2]) -> CaseResult {
    for ((y, label), fmt) in [[1.0, 2.0, 3.0], [3.0, 2.0, 1.0]]
        .into_iter()
        .zip(labels)
        .zip(fmts)
    {
        ax.plot_with([0.0, 1.0, 2.0], y, LineProps::fmt(fmt)?.label(label))?;
    }
    Ok(())
}

fn legend_guide_1(fig: &Figure) -> CaseResult {
    let red_patch = LegendHandle::patch(PatchProps::default().color("red").label("The red data"));
    fig.subplot()
        .legend(LegendOptions::default().handles([red_patch]))?;
    Ok(())
}

fn legend_guide_2(fig: &Figure) -> CaseResult {
    let blue_line = LegendHandle::line(
        LineProps::default()
            .color("blue")
            .marker("*".parse::<Marker>()?)
            .markersize(15.0)
            .label("Blue stars"),
    );
    fig.subplot()
        .legend(LegendOptions::default().handles([blue_line]))?;
    Ok(())
}

fn legend_guide_3(fig: &Figure) -> CaseResult {
    // `empty_sentinel="BLANK"`: the BLANK cell is empty.
    let layout = mosaic(&[&["top", "top"], &["bottom", "."]])?;
    let axd = fig.subplot_mosaic(&layout, MosaicOptions::default())?;
    let top = axd.get("top").ok_or("a top axes")?;
    rising_falling(top, ["test1", "test2"], ["", ""])?;
    top.legend(LegendOptions {
        ncols: 2,
        mode: LegendMode::Expand,
        ..LegendOptions::default()
            .bbox_to_anchor((0.0, 1.02, 1.0, 0.102))
            .loc(AnchorLoc::LowerLeft)
            .borderaxespad(0.0)
    })?;
    let bottom = axd.get("bottom").ok_or("a bottom axes")?;
    rising_falling(bottom, ["test1", "test2"], ["", ""])?;
    bottom.legend(
        LegendOptions::default()
            .bbox_to_anchor((1.05, 1.0))
            .loc(AnchorLoc::UpperLeft)
            .borderaxespad(0.0),
    )?;
    Ok(())
}

fn legend_guide_4(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplot_mosaic(&mosaic(&[&["left", "right"]])?, MosaicOptions::default())?;
    let (left, right) = (
        axs.get("left").ok_or("a left axes")?,
        axs.get("right").ok_or("a right axes")?,
    );
    rising_falling(left, ["test1", "test2"], ["", ""])?;
    rising_falling(right, ["test3", "test4"], ["C2", "C3"])?;
    fig.legend(LegendOptions::default().loc("outside upper right".parse::<LegendLoc>()?))?;
    Ok(())
}

/// A light grey constrained figure with a figure legend titled by its
/// location at each of `locations`.
fn outside_legends(fig: &Figure, label: &str, locations: &[&str]) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    fig.set_facecolor(to_rgba("0.95", None)?);
    let ax = fig.subplot();
    ax.plot_with([1.0, 2.0], [1.0, 2.0], LineProps::default().label(label))?;
    for &loc in locations {
        fig.legend(
            LegendOptions::default()
                .loc(loc.parse::<LegendLoc>()?)
                .title(loc),
        )?;
    }
    Ok(())
}

fn legend_guide_5(fig: &Figure) -> CaseResult {
    outside_legends(
        fig,
        "TEST",
        &[
            "outside upper left",
            "outside upper center",
            "outside upper right",
            "outside lower left",
            "outside lower center",
            "outside lower right",
        ],
    )
}

fn legend_guide_6(fig: &Figure) -> CaseResult {
    outside_legends(
        fig,
        "test",
        &[
            "outside left upper",
            "outside right upper",
            "outside left center",
            "outside right center",
            "outside left lower",
            "outside right lower",
        ],
    )
}

fn legend_guide_7(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = [0.0, 1.0, 2.0];
    let line1 = ax.plot_with(x, [1.0, 2.0, 3.0], LineProps::fmt("--")?.label("Line 1"))?;
    let line2 = ax.plot_with(
        x,
        [3.0, 2.0, 1.0],
        LineProps::default().label("Line 2").linewidth(4.0),
    )?;
    let first_legend = ax.legend(
        LegendOptions::default()
            .handles([&line1])
            .loc(AnchorLoc::UpperRight),
    )?;
    ax.keep_legend(&first_legend);
    ax.legend(
        LegendOptions::default()
            .handles([&line2])
            .loc(AnchorLoc::LowerRight),
    )?;
    Ok(())
}

fn legend_guide_8(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = [0.0, 1.0, 2.0];
    let dotted = |label: &str| LineProps::fmt("o-").map(|props| props.label(label));
    let line1 = ax.plot_with(x, [3.0, 2.0, 1.0], dotted("Line 1")?)?;
    let line2 = ax.plot_with(x, [1.0, 2.0, 3.0], dotted("Line 2")?)?;
    ax.legend(
        LegendOptions::default()
            .handles([
                LegendHandle::from(&line1).with_numpoints(4),
                LegendHandle::from(&line2),
            ])
            .handlelength(4.0),
    )?;
    Ok(())
}

fn legend_guide_9(fig: &Figure) -> CaseResult {
    let z = SpecRng::new(SEED).normal(10, 0.0, 1.0);
    let ax = fig.subplot();
    let red_dot = ax.plot_with(
        arange(0.0, 10.0, 1.0),
        &z,
        LineProps::fmt("ro")?.markersize(15.0),
    )?;
    let white_cross = ax.plot_with(
        arange(0.0, 5.0, 1.0),
        &z[..5],
        LineProps::fmt("w+")?.markeredgewidth(3.0).markersize(15.0),
    )?;
    let both = LegendHandle::tuple(
        [
            LegendHandle::from(&red_dot),
            LegendHandle::from(&white_cross),
        ],
        HandlerTuple::default(),
    );
    ax.legend(
        LegendOptions::default()
            .handles([LegendHandle::from(&red_dot), both])
            .labels(["Attr A", "Attr A+B"]),
    )?;
    Ok(())
}

fn legend_guide_10(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = [0.0, 1.0, 2.0];
    let p1 = ax.plot_with(x, [1.0, 2.5, 3.0], LineProps::fmt("r-d")?)?;
    let p2 = ax.plot_with(x, [3.0, 2.0, 1.0], LineProps::fmt("k-o")?)?;
    let keys = LegendHandle::tuple(
        [LegendHandle::from(&p1), LegendHandle::from(&p2)],
        HandlerTuple {
            ndivide: None,
            ..HandlerTuple::default()
        },
    );
    ax.legend(
        LegendOptions::default()
            .handles([keys])
            .labels(["Two keys"])
            .numpoints(1_usize),
    )?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_guides_axes/legend_guide_1", legend_guide_1),
    FigureCase::new("gallery_guides_axes/legend_guide_2", legend_guide_2),
    FigureCase::new("gallery_guides_axes/legend_guide_3", legend_guide_3),
    FigureCase::new("gallery_guides_axes/legend_guide_4", legend_guide_4),
    FigureCase::new("gallery_guides_axes/legend_guide_5", legend_guide_5).sized(6.0, 4.0, 100.0),
    FigureCase::new("gallery_guides_axes/legend_guide_6", legend_guide_6).sized(6.0, 4.0, 100.0),
    FigureCase::new("gallery_guides_axes/legend_guide_7", legend_guide_7),
    FigureCase::new("gallery_guides_axes/legend_guide_8", legend_guide_8),
    FigureCase::new("gallery_guides_axes/legend_guide_9", legend_guide_9),
    FigureCase::new("gallery_guides_axes/legend_guide_10", legend_guide_10),
    FigureCase::new(
        "gallery_guides_axes/legend_guide_11",
        handlers::legend_guide_11,
    ),
    FigureCase::new(
        "gallery_guides_axes/legend_guide_12",
        handlers::legend_guide_12,
    ),
];
