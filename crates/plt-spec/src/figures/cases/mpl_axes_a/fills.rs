//! `fill_between` with interpolated crossings (hatched, with masked and
//! NaN data, decreasing along y) and symmetric log scales.

use std::f64::consts::PI;

use plt::axes::{AxisName, FillBetweenOptions};
use plt::collections::CollectionProps;
use plt::figure::Share;
use plt::numeric::arange;
use plt::prelude::*;
use plt::scale::ScaleOptions;

use super::remove_ticks_and_titles;
use crate::figures::CaseResult;

/// Interpolated fill where `selected` holds, with `props`.
fn interpolated(selected: Vec<bool>, props: CollectionProps) -> FillBetweenOptions {
    FillBetweenOptions {
        mask: Some(selected),
        interpolate: true,
        props,
        ..FillBetweenOptions::default()
    }
}

/// `a[i] >= b[i]` for every point (false where either is NaN).
fn at_least(a: &[f64], b: &[f64]) -> Vec<bool> {
    a.iter().zip(b).map(|(a, b)| a >= b).collect()
}

/// Both curves in black, then green (or white and hatched) fills where
/// `y2 >= y1` and red ones where `y2 <= y1`.
fn crossing_fills(
    ax: &Axes,
    x: &[f64],
    y1: &[f64],
    y2: &[f64],
    above: CollectionProps,
) -> CaseResult {
    ax.plot_with(x, y1, LineProps::default().color("black"))?;
    ax.plot_with(x, y2, LineProps::default().color("black"))?;
    ax.fill_between(x, y1, y2, interpolated(at_least(y2, y1), above))?;
    let below = CollectionProps::default().facecolor("red");
    ax.fill_between(x, y1, y2, interpolated(at_least(y1, y2), below))?;
    Ok(())
}

pub(super) fn fill_between_interpolate(fig: &Figure) -> CaseResult {
    let x = arange(0.0, 2.0, 0.02);
    let y1: Vec<f64> = x.iter().map(|x| (2.0 * PI * x).sin()).collect();
    let mut y2: Vec<f64> = x.iter().map(|x| 1.2 * (4.0 * PI * x).sin()).collect();
    let axs = fig.subplots_shared(2, 1, Share::All, Share::None)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(1, 0)]);
    let hatched = CollectionProps::default().facecolor("white").hatch("/");
    crossing_fills(ax1, &x, &y1, &y2, hatched)?;
    // The masked values of `np.ma.masked_greater(y2, 1.0)` and `y2[0]`.
    for (i, v) in y2.iter_mut().enumerate() {
        if i == 0 || *v > 1.0 {
            *v = f64::NAN;
        }
    }
    crossing_fills(
        ax2,
        &x,
        &y1,
        &y2,
        CollectionProps::default().facecolor("green"),
    )?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn fill_between_interpolate_decreasing(fig: &Figure) -> CaseResult {
    let p = [724.3, 700.0, 655.0];
    let t = [9.4, 7.0, 2.2];
    let prof = [7.9, 6.6, 3.8];
    let ax = fig.subplot();
    ax.plot_fmt(t, p, "tab:red")?;
    ax.plot_fmt(prof, p, "k")?;
    for (selected, color) in [(at_least(&t, &prof), "blue"), (at_least(&prof, &t), "red")] {
        // `prof < t` and `prof > t`: no point has prof == t.
        let props = CollectionProps::default().facecolor(color).alpha(0.4);
        ax.fill_betweenx(p, t, prof, interpolated(selected, props))?;
    }
    ax.set_xlim(0.0, 30.0)?.set_ylim(800.0, 600.0)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn fill_between_interpolate_nan(fig: &Figure) -> CaseResult {
    let nan = f64::NAN;
    let x = arange(0.0, 10.0, 1.0);
    let y1 = [8.0, 18.0, nan, 18.0, 8.0, 18.0, 24.0, 18.0, 8.0, 18.0];
    let y2 = [18.0, 11.0, 8.0, 11.0, 18.0, 26.0, 32.0, 30.0, nan, nan];
    let ax = fig.subplot();
    ax.plot_with(&x, y1, LineProps::default().color("k"))?;
    ax.plot_with(&x, y2, LineProps::default().color("b"))?;
    for (selected, color) in [(at_least(&y2, &y1), "green"), (at_least(&y1, &y2), "red")] {
        let props = CollectionProps::default().facecolor(color).alpha(0.5);
        ax.fill_between(&x, y1, y2, interpolated(selected, props))?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn symlog(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot(
        [0.0, 1.0, 2.0, 4.0, 6.0, 9.0, 12.0, 24.0],
        [1_000_000.0, 500_000.0, 100_000.0, 100.0, 5.0, 0.0, 0.0, 0.0],
    )?;
    ax.set_yscale("symlog")?.set_xscale("linear")?;
    ax.set_ylim(-1.0, 10_000_000.0)?;
    Ok(())
}

pub(super) fn symlog2(fig: &Figure) -> CaseResult {
    let x = arange(-50.0, 50.0, 0.001);
    let axs = fig.subplots(5, 1)?;
    for (ax, linthresh) in axs.as_slice().iter().zip([20.0, 2.0, 1.0, 0.1, 0.01]) {
        ax.plot(&x, &x)?;
        let options = ScaleOptions {
            linthresh: Some(linthresh),
            ..ScaleOptions::default()
        };
        ax.set_scale_with(AxisName::X, "symlog", options)?;
        ax.grid(true);
    }
    axs[(4, 0)].set_ylim(-0.1, 0.1)?;
    remove_ticks_and_titles(fig);
    Ok(())
}
