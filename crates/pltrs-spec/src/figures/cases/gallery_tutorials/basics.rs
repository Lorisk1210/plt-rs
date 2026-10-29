//! The artist tutorial (a figure of axes made two ways, and lines added to
//! the figure itself) and the coding shortcuts tutorial (one plot written
//! ever more briefly: matplotlib's keyword shortcuts are plain setters
//! here, so its first five figures share one build).

use std::f64::consts::PI;

use pltrs::axes::{Axes, HistPlotOptions, PlotError};
use pltrs::gridspec::SubplotOverrides;
use pltrs::prelude::*;
use pltrs::stats::Bins;

use super::SEED;
use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

fn artists_1(fig: &Figure) -> CaseResult {
    fig.subplots_adjust(SubplotOverrides {
        top: Some(0.8),
        ..SubplotOverrides::default()
    })?;
    let ax1 = fig.add_subplot(2, 1, 1)?;
    ax1.set_ylabel("Voltage [V]");
    ax1.set_title("A sine wave");
    let t = arange(0.0, 1.0, 0.01);
    let blue = LineProps::default().color("blue").linewidth(2.0);
    ax1.plot_with(&t, map(&t, |t| (2.0 * PI * t).sin()), blue)?;
    let ax2 = fig.add_axes([0.15, 0.1, 0.7, 0.3])?;
    let yellow = HistPlotOptions {
        bins: Some(Bins::Count(50)),
        props: PatchProps::default()
            .facecolor("yellow")
            .edgecolor("yellow"),
        ..HistPlotOptions::default()
    };
    ax2.hist(&[SpecRng::new(SEED).normal(1000, 0.0, 1.0)], yellow)?;
    ax2.set_xlabel("Time [s]");
    Ok(())
}

fn artists_2(fig: &Figure) -> CaseResult {
    fig.add_line(&[0.0, 1.0], &[0.0, 1.0], LineProps::default())?;
    fig.add_line(&[0.0, 1.0], &[1.0, 0.0], LineProps::default())?;
    Ok(())
}

/// Daylight hours in London over the days of a year, labelled
/// (`_daylight`).
fn daylight(fig: &Figure) -> Result<Axes, PlotError> {
    let day: Vec<f64> = (0..365).map(f64::from).collect();
    let hours = map(&day, |d| {
        4.276 * (2.0 * PI * (d - 80.0) / 365.0).sin() + 12.203
    });
    let ax = fig.subplot();
    ax.plot_with(&day, hours, LineProps::default().color("orange"))?;
    ax.set_xlabel("day").set_ylabel("daylight hours");
    Ok(ax)
}

fn coding_shortcuts(fig: &Figure) -> CaseResult {
    daylight(fig)?.set_title("London");
    Ok(())
}

fn coding_shortcuts_6(fig: &Figure) -> CaseResult {
    daylight(fig)?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_tutorials/artists_1", artists_1),
    FigureCase::new("gallery_tutorials/artists_2", artists_2),
    FigureCase::new("gallery_tutorials/coding_shortcuts_1", coding_shortcuts),
    FigureCase::new("gallery_tutorials/coding_shortcuts_2", coding_shortcuts),
    FigureCase::new("gallery_tutorials/coding_shortcuts_3", coding_shortcuts),
    FigureCase::new("gallery_tutorials/coding_shortcuts_4", coding_shortcuts),
    FigureCase::new("gallery_tutorials/coding_shortcuts_5", coding_shortcuts),
    FigureCase::new("gallery_tutorials/coding_shortcuts_6", coding_shortcuts_6),
];
