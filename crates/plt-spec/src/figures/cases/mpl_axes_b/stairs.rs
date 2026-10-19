//! Stairs: baselines, orientations, fills, updates, options and dates.

use plt::axes::{AxisName, FillBetweenOptions, StairsOptions, TickLabelProps, TickLevel};
use plt::collections::CollectionProps;
use plt::dates::iso::Datetime64;
use plt::figure::Figure;
use plt::layout::{ConstrainedLayout, LayoutEngine};
use plt::legend::LegendOptions;
use plt::lines::{DrawStyle, LineProps, LineStyle, StepWhere};
use plt::numeric::linspace;
use plt::orientation::Orientation;
use plt::patches::{StepBaseline, StepPatch};
use plt::prelude::ArtistHandle;
use plt_render::{CapStyle, JoinStyle};

use super::{classic, from, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};

/// The histogram and bins of `test_stairs`.
const STAIRS_Y: [f64; 8] = [6.0, 14.0, 32.0, 37.0, 48.0, 32.0, 21.0, 4.0];
const STAIRS_X: [f64; 9] = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];

/// The heights and bins of `test_stairs_fill`.
const STAIRS_H: [f64; 5] = [1.0, 2.0, 3.0, 4.0, 2.0];
const STAIRS_BINS: [f64; 6] = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0];

/// Stairs between `edges` with the given baseline and orientation.
fn between(edges: &[f64], baseline: StepBaseline, orientation: Orientation) -> StairsOptions {
    StairsOptions {
        edges: Some(edges.to_vec()),
        baseline,
        orientation,
        ..StairsOptions::default()
    }
}

/// `test_stairs`: open, closed and log-scaled stairs both ways.
fn stairs(fig: &Figure) -> CaseResult {
    let (y, x) = (STAIRS_Y, STAIRS_X);
    let axs = fig.subplots(3, 2)?;
    let axs = axs.as_slice();
    let open = StepBaseline::None;
    let zero = StepBaseline::default;
    axs[0].stairs(y, between(&x, open.clone(), Orientation::Vertical))?;
    axs[1].stairs(y, between(&x, open, Orientation::Horizontal))?;
    axs[2].stairs(y, between(&x, zero(), Orientation::Vertical))?;
    axs[3].stairs(y, between(&x, zero(), Orientation::Horizontal))?;
    axs[4].stairs(y, between(&x, zero(), Orientation::Vertical))?;
    axs[4].set_yscale("log")?;
    axs[5].stairs(y, between(&x, zero(), Orientation::Horizontal))?;
    axs[5].set_xscale("log")?;
    Ok(())
}

/// The reference of `test_stairs`: post-step lines, closed by lines to
/// zero with `PathPatch`'s joins and caps.
fn stairs_ref(fig: &Figure) -> CaseResult {
    let (y, x) = (STAIRS_Y, STAIRS_X);
    let (first, last) = (y[0], y[y.len() - 1]);
    let (left, right) = (x[0], x[x.len() - 1]);
    let style = || {
        LineProps::default()
            .solid_joinstyle(JoinStyle::Miter)
            .solid_capstyle(CapStyle::Butt)
    };
    let steps = || style().drawstyle(DrawStyle::StepsPost);
    // `np.append(y, y[-1])` and `np.append(y[0], y)`.
    let held: Vec<f64> = y.iter().copied().chain([last]).collect();
    let led: Vec<f64> = [first].into_iter().chain(y).collect();
    let axs = fig.subplots(3, 2)?;
    let axs = axs.as_slice();
    for (i, ax) in axs.iter().enumerate() {
        let closed = i >= 2;
        if i % 2 == 0 {
            ax.plot_with(x, held.clone(), steps())?;
            if closed {
                ax.add_line(&[left, left], &[0.0, first], style())?;
                ax.add_line(&[right, right], &[0.0, last], style())?;
            }
        } else {
            ax.plot_with(led.clone(), x, steps())?;
            if closed {
                ax.add_line(&[0.0, first], &[left, left], style())?;
                ax.add_line(&[0.0, last], &[right, right], style())?;
            }
        }
    }
    axs[2].set_limits_with(AxisName::Y, from(0.0))?;
    axs[3].set_limits_with(AxisName::X, from(0.0))?;
    axs[4].set_yscale("log")?;
    axs[5].set_xscale("log")?;
    Ok(())
}

/// `test_stairs_fill`: filled stairs down to 0 and to -2, both ways.
fn stairs_fill(fig: &Figure) -> CaseResult {
    let (h, bins) = (STAIRS_H, STAIRS_BINS);
    let axs = fig.subplots(2, 2)?;
    let styles = [
        (StepBaseline::default(), Orientation::Vertical),
        (StepBaseline::default(), Orientation::Horizontal),
        (StepBaseline::Value(-2.0), Orientation::Vertical),
        (StepBaseline::Value(-2.0), Orientation::Horizontal),
    ];
    for (ax, (baseline, orientation)) in axs.as_slice().iter().zip(styles) {
        let options = StairsOptions {
            fill: true,
            ..between(&bins, baseline, orientation)
        };
        ax.stairs(h, options)?;
    }
    Ok(())
}

/// The reference of `test_stairs_fill`: post-step fills without edges.
fn stairs_fill_ref(fig: &Figure) -> CaseResult {
    // `np.append(h, h[-1])`.
    let top: Vec<f64> = STAIRS_H
        .iter()
        .copied()
        .chain([STAIRS_H[STAIRS_H.len() - 1]])
        .collect();
    let bottom = [-2.0; 6];
    let options = || FillBetweenOptions {
        step: Some(StepWhere::Post),
        props: CollectionProps {
            linewidths: Some(vec![0.0]),
            ..CollectionProps::default()
        },
        ..FillBetweenOptions::default()
    };
    let axs = fig.subplots(2, 2)?;
    let axs = axs.as_slice();
    axs[0].fill_between(STAIRS_BINS, &top, [0.0], options())?;
    axs[0].set_limits_with(AxisName::Y, from(0.0))?;
    axs[1].fill_betweenx(STAIRS_BINS, &top, [0.0], options())?;
    axs[1].set_limits_with(AxisName::X, from(0.0))?;
    axs[2].fill_between(STAIRS_BINS, &top, bottom, options())?;
    axs[2].set_limits_with(AxisName::Y, from(-2.0))?;
    axs[3].fill_betweenx(STAIRS_BINS, &top, bottom, options())?;
    axs[3].set_limits_with(AxisName::X, from(-2.0))?;
    Ok(())
}

/// `test_stairs_options`: colours, dashes, NaN gaps, raised, per-step and
/// open baselines, hatching and a legend.
fn stairs_options(fig: &Figure) -> CaseResult {
    let x = [1.0, 2.0, 3.0, 4.0, 5.0];
    let y = [1.0, 2.0, 3.0, 4.0];
    let yn = [1.0, f64::NAN, 3.0, 4.0];
    let scaled = |values: &[f64], factor: f64, offset: f64| -> Vec<f64> {
        values.iter().map(|v| v * factor + offset).collect()
    };
    let reversed = |values: &[f64]| -> Vec<f64> { values.iter().rev().copied().collect() };
    let ax = fig.subplot();
    let vertical = |edges: Vec<f64>, baseline| between(&edges, baseline, Orientation::Vertical);
    let colored = |options: StairsOptions, color: &str, fill: bool| StairsOptions {
        color: Some(color.into()),
        fill,
        ..options
    };
    let zero = StepBaseline::default;

    ax.stairs(
        scaled(&y, 3.0, 0.0),
        colored(vertical(x.to_vec(), zero()), "green", true),
    )?
    .set_label("A");
    let b_edges = scaled(&x, 3.0, -3.0);
    ax.stairs(
        y,
        colored(
            between(&b_edges, zero(), Orientation::Horizontal),
            "red",
            true,
        ),
    )?
    .set_label("B");
    ax.stairs(yn, colored(vertical(x.to_vec(), zero()), "orange", false))?
        .set_linestyle(LineStyle::Dashed)
        .set_linewidth(2.0)
        .set_label("C");
    let d_edges = scaled(&x, 3.0, -2.0);
    ax.stairs(
        yn.map(|v| v / 3.0),
        between(&d_edges, StepBaseline::Value(0.5), Orientation::Horizontal),
    )?
    .set_linestyle(LineStyle::Dashed)
    .set_linewidth(2.0)
    .set_label("D");
    ax.stairs(
        scaled(&reversed(&y), 3.0, 13.0),
        colored(
            vertical(scaled(&x, 1.0, -1.0), StepBaseline::None),
            "red",
            false,
        ),
    )?
    .set_linestyle(LineStyle::Dashed)
    .set_linewidth(2.0)
    .set_label("E");
    ax.stairs(
        scaled(&reversed(&y), 3.0, 14.0),
        colored(
            vertical(x.to_vec(), StepBaseline::Value(26.0)),
            "purple",
            false,
        ),
    )?
    .set_linestyle(LineStyle::Dashed)
    .set_linewidth(2.0)
    .set_label("F");
    let per_step = StepBaseline::PerStep(linspace(27.0, 25.0, y.len()));
    ax.stairs(
        scaled(&reversed(&yn), 3.0, 15.0),
        colored(vertical(scaled(&x, 1.0, 1.0), per_step), "blue", true),
    )?
    .set_linestyle(LineStyle::Dashed)
    .set_label("G");
    ax.stairs(
        scaled(&reversed(&y[..3]), 2.0, 11.0),
        colored(
            vertical(scaled(&x[..4], 1.0, 0.5), StepBaseline::Value(12.0)),
            "black",
            false,
        ),
    )?
    .set_linestyle(LineStyle::Dashed)
    .set_linewidth(2.0)
    .set_hatch("//")
    .set_label("H");
    ax.legend(LegendOptions::default())?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_stairs_datetime`: daily date edges under constrained layout with
/// rotated tick labels.
fn stairs_datetime(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(Some(
        LayoutEngine::Constrained(ConstrainedLayout::default()),
    ));
    let ax = fig.subplot();
    let first = Datetime64::parse("2001-12-27")?.ok_or("not a time")?;
    let days: Vec<Datetime64> = (0..37)
        .map(|day| Datetime64::new(first.seconds() + day * 86_400, 0))
        .collect();
    let edges = ax.xaxis().convert_units(days);
    let values: Vec<f64> = (0..36).map(f64::from).collect();
    let options = StairsOptions {
        edges: Some(edges),
        ..StairsOptions::default()
    };
    ax.stairs(values, options)?;
    ax.xaxis().set_ticklabel_props(
        TickLevel::Major,
        &TickLabelProps {
            rotation: Some(30.0),
            ..TickLabelProps::default()
        },
    );
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    classic("mpl_axes_b/stairs", stairs),
    classic("mpl_axes_b/stairs_ref", stairs_ref),
    classic("mpl_axes_b/stairs_fill", stairs_fill),
    classic("mpl_axes_b/stairs_fill_ref", stairs_fill_ref),
    classic("mpl_axes_b/stairs_update", |fig| {
        let ax = fig.subplot();
        let steps = ax.stairs([1.0, 2.0, 3.0], StairsOptions::default())?;
        ax.set_ylim(-3.0, 4.0);
        // The `set_data` calls of the test end at these values, edges
        // `0..=3` and baseline -2 (the data limits stay those of the
        // first steps).
        let updated = StepPatch::new(vec![1.0, 2.0, 3.0], vec![0.0, 1.0, 2.0, 3.0])?
            .with_baseline(StepBaseline::Value(-2.0))?;
        steps.set_shape(updated);
        Ok(())
    }),
    classic("mpl_axes_b/stairs_baseline_None", |fig| {
        let x = [0.0, 2.0, 3.0, 5.0, 10.0];
        let y = [1.148, 1.231, 1.248, 1.25];
        fig.subplot()
            .stairs(y, between(&x, StepBaseline::None, Orientation::Vertical))?;
        Ok(())
    }),
    FigureCase::new("mpl_axes_b/stairs_options", stairs_options),
    FigureCase::new("mpl_axes_b/stairs_datetime", stairs_datetime),
    classic("mpl_axes_b/stairs_edge_handling", |fig| {
        let options = StairsOptions {
            fill: true,
            color: Some("red".into()),
            ..StairsOptions::default()
        };
        fig.subplot().stairs([1.0, 2.0, 3.0], options)?;
        Ok(())
    }),
];
