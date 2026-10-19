//! Infinite lines, `vlines`/`hlines`, steps and a patch collection.

use plt::axes::Axes;
use plt::collections::{Collection, CollectionPatch, CollectionSpec, EdgeColors};
use plt::colors::ColorError;
use plt::figure::Figure;
use plt::lines::{AxLineThrough, Line2D, LineProps, LineStyle, StepWhere};
use plt::orientation::Orientation;
use plt::patches::{Circle, PatchProps};
use plt::prelude::{ArtistHandle, Color, arange};
use plt::transforms::{Point, Transform};

use super::super::mpl_tests::{classic, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};

/// `ax.set(xlim=(-1, 1), ylim=(-1, 1))`.
fn unit_view(fig: &Figure) -> Axes {
    let ax = fig.subplot();
    ax.set_xlim(-1.0, 1.0).set_ylim(-1.0, 1.0);
    ax
}

/// An infinite line through `xy1`, in `lines.color` unless `color` is
/// given.
fn axline(ax: &Axes, xy1: (f64, f64), through: AxLineThrough, color: Option<&str>) -> Line2D {
    let props = color.map_or_else(LineProps::default, |c| LineProps::default().color(c));
    ax.axline(Point::new(xy1.0, xy1.1), through, props)
}

/// The second point of an infinite line.
fn to(x: f64, y: f64) -> AxLineThrough {
    AxLineThrough::Point(Point::new(x, y))
}

/// An infinite line given in the axes' fractions.
fn axline_in_axes(ax: &Axes, xy1: (f64, f64), through: AxLineThrough, color: Option<&str>) {
    axline(ax, xy1, through, color).set_transform(ax.trans_axes());
}

/// `hlines` for horizontal segments, `vlines` for vertical ones.
fn segments(
    ax: &Axes,
    orientation: Orientation,
    at: &[f64],
    low: &[f64],
    high: &[f64],
) -> Result<Collection, plt::axes::PlotError> {
    match orientation {
        Orientation::Horizontal => ax.hlines(at, low, high),
        Orientation::Vertical => ax.vlines(at, low, high),
    }
}

/// Limits the axis along which the segments are placed.
fn limit_positions(ax: &Axes, orientation: Orientation, low: f64, high: f64) {
    match orientation {
        Orientation::Horizontal => ax.set_ylim(low, high),
        Orientation::Vertical => ax.set_xlim(low, high),
    };
}

/// Segments of one colour, width and style.
fn styled(segments: &Collection, color: &str, width: f64, style: LineStyle) {
    segments
        .set_edgecolor(color)
        .set_linewidths(vec![width])
        .set_linestyles(vec![style]);
}

/// `vlines_basic.png` / `hlines_basic.png`.
fn lines_basic(fig: &Figure, orientation: Orientation) -> CaseResult {
    let ax = fig.subplot();
    let lines = segments(
        &ax,
        orientation,
        &[2.0, 3.0, 4.0, 5.0, 7.0],
        &[0.0],
        &[2.0, -6.0, 3.0, 8.0, 2.0],
    )?;
    styled(&lines, "g", 5.0, LineStyle::Solid);
    limit_positions(&ax, orientation, 0.0, 10.0);
    Ok(())
}

/// `vlines_with_nan.png` / `hlines_with_nan.png`.
fn lines_with_nan(fig: &Figure, orientation: Orientation) -> CaseResult {
    let at = [2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
    let rows = [
        (
            [2.0, -6.0, 3.0, 8.0, f64::NAN, 2.0],
            "g",
            5.0,
            LineStyle::Solid,
        ),
        (
            [f64::NAN, 2.0, -6.0, 3.0, 8.0, 2.0],
            "r",
            3.0,
            LineStyle::Dashed,
        ),
        (
            [f64::NAN, 2.0, -6.0, 3.0, 8.0, f64::NAN],
            "k",
            2.0,
            LineStyle::Solid,
        ),
    ];
    let axs = fig.subplots(3, 1)?;
    for (ax, (lengths, color, width, style)) in axs.as_slice().iter().zip(rows) {
        let lines = segments(ax, orientation, &at, &[0.0], &lengths)?;
        styled(&lines, color, width, style);
        limit_positions(ax, orientation, 0.0, 10.0);
    }
    Ok(())
}

/// `vlines_masked.png` / `hlines_masked.png`: the masked values are NaN.
fn lines_masked(fig: &Figure, orientation: Orientation) -> CaseResult {
    let ax = fig.subplot();
    let lines = segments(
        &ax,
        orientation,
        &[2.0, 4.0, 6.0, f64::NAN, 10.0, 12.0],
        &[0.0, 1.0, -1.0, 0.0, f64::NAN, 1.0],
        &[13.0, 14.0, 15.0, 16.0, 17.0, f64::NAN],
    )?;
    styled(&lines, "k", 2.0, LineStyle::Solid);
    limit_positions(&ax, orientation, 0.0, 15.0);
    Ok(())
}

/// `test_lines_with_colors`: five coloured segments of each kind, the
/// fourth missing (a NaN, or a masked value, which is NaN here too).
fn lines_with_colors(fig: &Figure) -> CaseResult {
    let data = [1.0, 2.0, 3.0, f64::NAN, 5.0];
    let colors: Vec<Color> = ["red", "green", "blue", "purple", "orange"]
        .map(Color::from)
        .to_vec();
    let top = fig.add_subplot(2, 1, 1)?.vlines(data, [0.0], [1.0])?;
    let bottom = fig.add_subplot(2, 1, 2)?.hlines(data, [0.0], [1.0])?;
    for lines in [top, bottom] {
        lines
            .set_edgecolors(EdgeColors::Colors(colors.clone()))
            .set_linewidths(vec![5.0]);
    }
    Ok(())
}

/// `test_step_linestyle`: steps of every kind in the four line styles.
fn step_linestyle(fig: &Figure) -> CaseResult {
    let x = arange(0.0, 10.0, 1.0);
    let styles = [
        LineStyle::Solid,
        LineStyle::Dashed,
        LineStyle::DashDot,
        LineStyle::Dotted,
    ];
    let axs = fig.subplots(2, 2)?;
    for (ax, style) in axs.as_slice().iter().zip(styles) {
        for (shift, at) in [
            (0.0, StepWhere::Pre),
            (1.0, StepWhere::Mid),
            (2.0, StepWhere::Post),
        ] {
            let y: Vec<f64> = x.iter().map(|v| v + shift).collect();
            let props = LineProps::default().linewidth(5.0).linestyle(style.clone());
            ax.step(&x, y, at, props)?;
        }
        ax.set_xlim(-1.0, 5.0).set_ylim(-1.0, 7.0);
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_mixed_collection`: a circle repeated at pixel offsets with
/// different widths and edge alphas.
fn mixed_collection(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let circle = CollectionPatch::new(
        &Circle::new((8.0, 8.0), 4.0),
        PatchProps::default().facecolor("none").edgecolor("green"),
    );
    // A collection without an offset transform places its offsets in
    // display pixels.
    let at_pixels = |offsets: [(f64, f64); 2]| -> Result<CollectionSpec, ColorError> {
        let pixels = offsets.map(|(x, y)| Point::new(x, y)).to_vec();
        Ok(
            CollectionSpec::patches(std::slice::from_ref(&circle), true)?
                .with_offsets(pixels, Transform::Display),
        )
    };
    ax.set_facecolor("0.5");
    ax.add_collection(at_pixels([(0.0, 0.0), (24.0, 24.0)])?)
        .set_linewidths(vec![1.0, 5.0]);
    ax.add_collection(at_pixels([(48.0, 0.0), (-32.0, -16.0)])?)
        .set_linewidths(vec![1.0, 5.0])
        .set_edgecolors(EdgeColors::Colors(vec![
            Color::from([0.0, 0.0, 0.1, 1.0]),
            Color::from([0.0, 0.0, 0.1, 0.5]),
        ]));
    ax.set_xlim(0.0, 16.0).set_ylim(0.0, 16.0);
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    classic("mpl_axes_d/axline_loglog", |fig| {
        let ax = fig.subplot();
        ax.set_xlim(0.1, 10.0).set_ylim(1e-3, 1.0);
        ax.set_xscale("log")?.set_yscale("log")?;
        ax.plot_fmt([0.3, 0.6], [0.3, 0.6], ".-")?;
        axline(&ax, (1.0, 1e-3), to(10.0, 1e-2), Some("k"));
        Ok(())
    }),
    classic("mpl_axes_d/axline", |fig| {
        let ax = unit_view(fig);
        axline(&ax, (0.0, 0.0), to(1.0, 1.0), None);
        axline(&ax, (0.0, 0.0), to(1.0, 0.0), Some("C1"));
        axline(&ax, (0.0, 0.5), to(1.0, 0.5), Some("C2"));
        axline(&ax, (-0.7, -0.5), AxLineThrough::Slope(0.0), Some("C3"));
        axline(&ax, (1.0, -0.5), AxLineThrough::Slope(-0.5), Some("C4"));
        axline(
            &ax,
            (-0.5, 1.0),
            AxLineThrough::Slope(f64::INFINITY),
            Some("C5"),
        );
        Ok(())
    }),
    classic("mpl_axes_d/axline_transaxes", |fig| {
        let ax = unit_view(fig);
        axline_in_axes(&ax, (0.0, 0.0), AxLineThrough::Slope(1.0), None);
        axline_in_axes(&ax, (1.0, 0.5), AxLineThrough::Slope(1.0), Some("C1"));
        axline_in_axes(&ax, (0.5, 0.5), AxLineThrough::Slope(0.0), Some("C2"));
        axline_in_axes(&ax, (0.5, 0.0), to(0.5, 1.0), Some("C3"));
        Ok(())
    }),
    classic("mpl_axes_d/axline_transaxes_panzoom", |fig| {
        let ax = unit_view(fig);
        axline_in_axes(&ax, (0.0, 0.0), AxLineThrough::Slope(1.0), None);
        axline_in_axes(&ax, (0.5, 0.5), AxLineThrough::Slope(2.0), Some("C1"));
        axline_in_axes(&ax, (0.5, 0.5), AxLineThrough::Slope(0.0), Some("C2"));
        ax.set_xlim(0.0, 5.0).set_ylim(0.0, 10.0);
        fig.set_size_inches(3.0, 3.0);
        Ok(())
    }),
    FigureCase::new("mpl_axes_d/vlines_1", |fig| {
        lines_basic(fig, Orientation::Vertical)
    }),
    FigureCase::new("mpl_axes_d/vlines_2", |fig| {
        lines_with_nan(fig, Orientation::Vertical)
    })
    .sized(4.0, 8.0, 100.0),
    FigureCase::new("mpl_axes_d/vlines_3", |fig| {
        lines_masked(fig, Orientation::Vertical)
    }),
    FigureCase::new("mpl_axes_d/hlines_1", |fig| {
        lines_basic(fig, Orientation::Horizontal)
    }),
    FigureCase::new("mpl_axes_d/hlines_2", |fig| {
        lines_with_nan(fig, Orientation::Horizontal)
    })
    .sized(4.0, 8.0, 100.0),
    FigureCase::new("mpl_axes_d/hlines_3", |fig| {
        lines_masked(fig, Orientation::Horizontal)
    }),
    classic("mpl_axes_d/lines_with_colors_nan", lines_with_colors),
    classic("mpl_axes_d/lines_with_colors_masked", lines_with_colors),
    FigureCase::new("mpl_axes_d/vlines_hlines_blended_transform", |fig| {
        let t = arange(5.0, 10.0, 0.1);
        let s: Vec<f64> = t
            .iter()
            .map(|t| (-t).exp() + (2.0 * std::f64::consts::PI * t).sin() + 10.0)
            .collect();
        let axs = fig.subplots(2, 1)?;
        let (hax, vax) = (&axs[(0, 0)], &axs[(1, 0)]);
        hax.plot_fmt(&t, &s, "^")?;
        hax.hlines_in([10.0, 9.0], [0.0], [0.5], hax.trans_yaxis())?
            .set_edgecolor("r");
        vax.plot_fmt(&t, &s, "^")?;
        vax.vlines_in([6.0, 7.0], [0.0], [0.15], vax.trans_xaxis())?
            .set_edgecolor("r");
        Ok(())
    })
    .sized(6.0, 6.0, 100.0),
    classic("mpl_axes_d/step_linestyle", step_linestyle),
    classic("mpl_axes_d/mixed_collection", mixed_collection),
];
