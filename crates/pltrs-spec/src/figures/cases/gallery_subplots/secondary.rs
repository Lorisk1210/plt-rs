//! `secondary_axis`: extra axes showing the host's range converted, at
//! its sides, at a data value and at a fraction beyond it.

use std::f64::consts::PI;
use std::sync::Arc;

use pltrs::axes::{AxisName, SecondaryLocation, TickParams};
use pltrs::dates::{DateTime, get_epoch};
use pltrs::layout::{LayoutEngine, LayoutEngineError};
use pltrs::legend::LegendOptions;
use pltrs::numeric::interp;
use pltrs::prelude::*;
use pltrs::scale::FuncTransform;
use pltrs::spines::SpineSide;
use pltrs::stats::mean;

use super::SEED;
use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// A constrained figure's only axes, titled and labelled.
fn host(fig: &Figure, title: &str, xlabel: &str, ylabel: &str) -> Result<Axes, LayoutEngineError> {
    fig.set_layout_engine(Some(LayoutEngine::constrained()))?;
    let ax = fig.subplot();
    ax.set_xlabel(xlabel).set_ylabel(ylabel);
    ax.set_title(title);
    Ok(ax)
}

/// The top of the host.
const TOP: SecondaryLocation = SecondaryLocation::Side(SpineSide::Top);

fn secondary_axis_1(fig: &Figure) -> CaseResult {
    let ax = host(fig, "Sine wave", "angle [degrees]", "signal")?;
    let x = arange(0.0, 360.0, 1.0);
    ax.plot(&x, map(&x, |x| (2.0 * x * PI / 180.0).sin()))?;
    let radians = FuncTransform::new(|x| x * PI / 180.0, |x| x * 180.0 / PI);
    ax.secondary_xaxis(TOP, Some(radians))?
        .set_xlabel("angle [rad]");
    Ok(())
}

fn secondary_axis_2(fig: &Figure) -> CaseResult {
    let ax = host(fig, "Random data", "X", "Y")?;
    let x = arange(0.0, 10.0, 1.0);
    ax.plot(&x, SpecRng::new(SEED).normal(x.len(), 0.0, 1.0))?;
    ax.secondary_xaxis(SecondaryLocation::Data(0.0), None)?
        .set_xlabel("Axis at Y = 0");
    Ok(())
}

/// `_one_over`: `1/x`, infinite where `x` is close to 0.
fn one_over(x: f64) -> f64 {
    if x.abs() <= 1e-8 {
        f64::INFINITY
    } else {
        1.0 / x
    }
}

fn secondary_axis_3(fig: &Figure) -> CaseResult {
    let ax = host(fig, "Random spectrum", "f [Hz]", "PSD")?;
    let x = arange(0.02, 1.0, 0.02);
    let y = map(&SpecRng::new(SEED).normal(x.len(), 0.0, 1.0), |v| v.powi(2));
    ax.loglog(&x, &y)?;
    ax.secondary_xaxis(TOP, Some(FuncTransform::new(one_over, one_over)))?
        .set_xlabel("period [s]");
    Ok(())
}

fn secondary_axis_4(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(Some(LayoutEngine::constrained()))?;
    let ax = fig.subplot();
    let x1_vals = arange(2.0, 11.0, 0.4);
    let x2_vals = map(&x1_vals, |x| x.powi(2));
    let ydata = map(&SpecRng::new(SEED).normal(x1_vals.len(), 0.0, 1.0), |v| {
        50.0 + 20.0 * v
    });
    ax.plot_with(&x1_vals, &ydata, LineProps::default().label("Plotted data"))?;
    ax.plot_with(
        &x1_vals,
        &x2_vals,
        LineProps::default().label(r"$x_2 = x_1^2$"),
    )?;
    ax.set_xlabel(r"$x_1$");
    ax.legend(LegendOptions::default())?;
    let x1n = Arc::new(linspace(0.0, 20.0, 201));
    let x2n = Arc::new(map(&x1n, |x| x.powi(2)));
    let guide = || {
        LineProps::default()
            .color("grey")
            .linestyle(LineStyle::Dashed)
    };
    ax.axvline(40.0_f64.sqrt(), guide());
    ax.axvline(10.0, guide());
    let (forward_x, forward_y) = (Arc::clone(&x1n), Arc::clone(&x2n));
    let squares = FuncTransform::new(
        move |x| interp(x, &forward_x, &forward_y, None),
        move |x| interp(x, &x2n, &x1n, None),
    );
    let secax = ax.secondary_xaxis(TOP, Some(squares))?;
    secax.set_xticks([10.0, 20.0, 40.0, 60.0, 80.0, 100.0]);
    secax.set_xlabel(r"$x_2$");
    Ok(())
}

fn secondary_axis_5(fig: &Figure) -> CaseResult {
    let new_year = DateTime::new(2018, 1, 1, 0, 0, 0, 0)?;
    let six_hours = 6 * 3_600_000_000;
    let dates = (0..240)
        .map(|k| DateTime::from_timestamp_micros(new_year.timestamp_micros() + k * six_hours))
        .collect::<Result<Vec<_>, _>>()?;
    let temperature = map(&SpecRng::new(SEED).normal(dates.len(), 0.0, 1.0), |v| {
        v * 4.0 + 6.7
    });
    fig.set_layout_engine(Some(LayoutEngine::constrained()))?;
    let ax = fig.subplot();
    ax.plot(dates, &temperature)?;
    ax.set_ylabel(r"$T\ [^oC]$");
    ax.tick_params(TickParams {
        axis: Some(AxisName::X),
        labelrotation: Some(70.0),
        ..TickParams::default()
    });
    let day_zero = get_epoch().date2num(new_year);
    let yday = FuncTransform::new(move |x| x - day_zero, move |x| x + day_zero);
    ax.secondary_xaxis(TOP, Some(yday))?
        .set_xlabel("yday [2018]");
    let fahrenheit = FuncTransform::new(|x| x * 1.8 + 32.0, |x| (x - 32.0) / 1.8);
    ax.secondary_yaxis(SecondaryLocation::Side(SpineSide::Right), Some(fahrenheit))?
        .set_ylabel(r"$T\ [^oF]$");
    let average = mean(&temperature);
    let anomaly = FuncTransform::new(move |x| x - average, move |x| x + average);
    ax.secondary_yaxis(SecondaryLocation::Fraction(1.2), Some(anomaly))?
        .set_ylabel(r"$T - \overline{T}\ [^oC]$");
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_subplots/secondary_axis_1", secondary_axis_1),
    FigureCase::new("gallery_subplots/secondary_axis_2", secondary_axis_2),
    FigureCase::new("gallery_subplots/secondary_axis_3", secondary_axis_3),
    FigureCase::new("gallery_subplots/secondary_axis_4", secondary_axis_4),
    FigureCase::new("gallery_subplots/secondary_axis_5", secondary_axis_5),
];
