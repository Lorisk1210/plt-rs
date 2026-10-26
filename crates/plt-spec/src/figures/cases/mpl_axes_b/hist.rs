//! Histograms: log counts, empty data, stacked steps and densities.

use plt::axes::{AxisName, HistPlotOptions};
use plt::figure::Figure;
use plt::numeric::linspace;
use plt::patches::PatchProps;
use plt::stats::{Bins, HistType};

use super::{classic, from, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// Histogram options of the given type, otherwise default.
fn of_type(histtype: HistType) -> HistPlotOptions<'static> {
    HistPlotOptions {
        histtype,
        ..HistPlotOptions::default()
    }
}

/// `ax.hist([], histtype=...)` with the text removed.
fn empty(fig: &Figure, histtype: HistType) -> CaseResult {
    fig.subplot()
        .hist(&[Vec::<f64>::new()], of_type(histtype))?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_hist_step_filled`: three stacked samples under every `fill`
/// keyword for both step types.
fn hist_step_filled(fig: &Figure) -> CaseResult {
    let samples = SpecRng::new(0).normal(1000 * 3, 0.0, 1.0);
    let columns: Vec<Vec<f64>> = (0..3)
        .map(|j| samples.iter().skip(j).step_by(3).copied().collect())
        .collect();
    // `{'fill': True}, {'fill': False}, {'fill': None}, {}`; a `None`
    // fill reaches `Patch.set_fill` and turns filling off.
    let fills = [Some(true), Some(false), Some(false), None];
    let axs = fig.subplots(2, 4)?;
    let kinds = [HistType::Step, HistType::StepFilled];
    let styles = kinds
        .iter()
        .flat_map(|&kind| fills.iter().map(move |&fill| (kind, fill)));
    for (ax, (histtype, fill)) in axs.as_slice().iter().zip(styles) {
        let options = HistPlotOptions {
            histtype,
            stacked: true,
            props: PatchProps {
                fill,
                ..PatchProps::default()
            },
            ..HistPlotOptions::default()
        };
        ax.hist(&columns, options)?;
        ax.set_limits_with(AxisName::Y, from(-50.0))?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    classic("mpl_axes_b/hist_log", |fig| {
        let cubes: Vec<f64> = linspace(0.0, 1.0, 200)
            .into_iter()
            .map(|v| v.powf(3.0))
            .collect();
        let data: Vec<f64> = cubes
            .iter()
            .map(|c| 1.0 - c)
            .chain(cubes.iter().map(|c| 1.0 + c))
            .collect();
        let options = HistPlotOptions {
            log: true,
            props: PatchProps {
                fill: Some(false),
                ..PatchProps::default()
            },
            ..HistPlotOptions::default()
        };
        fig.subplot().hist(&[data], options)?;
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    classic("mpl_axes_b/hist_log_2", |fig| {
        let axs = fig.subplots(2, 3)?;
        for (i, histtype) in [HistType::Bar, HistType::Step, HistType::StepFilled]
            .into_iter()
            .enumerate()
        {
            let options = HistPlotOptions {
                bins: Some(Bins::Count(1)),
                histtype,
                ..HistPlotOptions::default()
            };
            axs[(0, i)].set_yscale("log")?;
            axs[(0, i)].hist(&[[1.0]], options.clone())?;
            axs[(1, i)].hist(&[[1.0]], options)?;
            axs[(1, i)].set_yscale("log")?;
        }
        Ok(())
    }),
    classic("mpl_axes_b/hist_bar_empty", |fig| empty(fig, HistType::Bar)),
    classic("mpl_axes_b/hist_step_empty", |fig| {
        empty(fig, HistType::Step)
    }),
    classic("mpl_axes_b/hist_step_filled", hist_step_filled),
    FigureCase::new("mpl_axes_b/hist_density", |fig| {
        let data = SpecRng::new(19_680_801).normal(2000, 0.0, 1.0);
        let options = HistPlotOptions {
            density: true,
            ..HistPlotOptions::default()
        };
        fig.subplot().hist(&[data], options)?;
        Ok(())
    }),
];
