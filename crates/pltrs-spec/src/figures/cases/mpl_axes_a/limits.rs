//! Autoscaling: tiny data ranges, sticky edges shared between axes and
//! within floating-point tolerance, non-finite data, empty data before
//! dates, lone points with `lines.marker` set, 2-D data, the canonical
//! line plot and a single labelled bar.

use pltrs::axes::{BarAlign, BarOptions, FillBetweenOptions, Mesh, PcolorOptions, ScatterColors};
use pltrs::collections::CollectionProps;
use pltrs::contour::{ContourOptions, QuadCoords, QuadField};
use pltrs::dates::{DateTime, Datetime64};
use pltrs::numeric::{Grid, arange};
use pltrs::prelude::*;

use super::remove_ticks_and_titles;
use crate::figures::CaseResult;

pub(super) fn autoscale_tiny_range(fig: &Figure) -> CaseResult {
    for (i, ax) in fig.subplots(2, 2)?.as_slice().iter().enumerate() {
        let y1 = 10f64.powi(-11 - i32::try_from(i)?);
        ax.plot([0.0, 1.0], [1.0, 1.0 + y1])?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `_sticky_shared`: a mesh in one of two x-shared axes, the leader drawn
/// at the bottom (mesh in the leader) or at the top (mesh in the
/// follower).
fn sticky_shared(fig: &Figure, mesh_on_leader: bool) -> CaseResult {
    let (leader_row, follower_row) = if mesh_on_leader { (2, 1) } else { (1, 2) };
    let leader = fig.add_subplot(2, 1, leader_row)?;
    let follower = fig.add_subplot(2, 1, follower_row)?;
    follower.share_x(&leader)?;
    let z = Grid::from_fn(3, 5, |r, c| (r * 5 + c) as f64);
    let target = if mesh_on_leader { &leader } else { &follower };
    target.pcolormesh(Mesh::new(z), PcolorOptions::default())?;
    Ok(())
}

pub(super) fn sticky_shared_axes(fig: &Figure) -> CaseResult {
    sticky_shared(fig, false)
}

pub(super) fn sticky_shared_axes_ref(fig: &Figure) -> CaseResult {
    sticky_shared(fig, true)
}

/// One bar of `height` (or width) 0.1 starting at `base`.
fn thin_bar(base: f64) -> BarOptions {
    BarOptions {
        base: vec![base].into(),
        ..BarOptions::default()
    }
}

pub(super) fn sticky_tolerance(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 2)?;
    let width = 0.1;
    for (ax, sign) in [(&axs[(0, 0)], 1.0), (&axs[(0, 1)], -1.0)] {
        ax.bar([0.0], [sign * width], thin_bar(20000.6))?;
        ax.bar([1.0], [sign * width], thin_bar(20000.1))?;
    }
    for (ax, sign) in [(&axs[(1, 0)], -1.0), (&axs[(1, 1)], 1.0)] {
        ax.barh([0.0], [sign * width], thin_bar(-20000.6))?;
        ax.barh([1.0], [sign * width], thin_bar(-20000.1))?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn sticky_tolerance_contourf(fig: &Figure) -> CaseResult {
    let coords = vec![14496.71, 14496.75];
    let field = QuadField::new(Grid::from_vec(2, 2, vec![0.0, 1.0, 2.0, 3.0])?).with_coords(
        QuadCoords::Axes {
            x: coords.clone(),
            y: coords,
        },
    );
    fig.subplot().contourf(&field, ContourOptions::default())?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn nonfinite_limits(fig: &Figure) -> CaseResult {
    let mut x = arange(0.0, std::f64::consts::E, 0.01);
    let y: Vec<f64> = x.iter().map(|v| v.ln()).collect();
    let middle = x.len() / 2;
    x[middle] = f64::NAN;
    fig.subplot().plot(x, y)?;
    Ok(())
}

/// The days of 2010 (`np.arange("2010-01-01", "2011-01-01",
/// dtype="datetime64[D]")`) and their indices.
fn days_of_2010() -> Result<(Vec<Datetime64>, Vec<f64>), Box<dyn std::error::Error>> {
    const DAY: i64 = 86_400;
    let start = DateTime::new(2010, 1, 1, 0, 0, 0, 0)?.timestamp_micros() / 1_000_000;
    let days = (0..365)
        .map(|d| Datetime64::new(start + d * DAY, 0))
        .collect();
    Ok((days, (0..365).map(f64::from).collect()))
}

pub(super) fn limits_empty_data_scatter(fig: &Figure) -> CaseResult {
    let (x, y) = days_of_2010()?;
    let ax = fig.subplot();
    ax.scatter(Vec::<f64>::new(), Vec::<f64>::new())?;
    let options = ScatterOptions {
        colors: ScatterColors::Single("C0".into()),
        ..ScatterOptions::default()
    };
    ax.scatter_with(x, y, options)?;
    Ok(())
}

pub(super) fn limits_empty_data_plot(fig: &Figure) -> CaseResult {
    let (x, y) = days_of_2010()?;
    let ax = fig.subplot();
    ax.plot(Vec::<f64>::new(), Vec::<f64>::new())?;
    ax.plot_with(x, y, LineProps::default().color("C0"))?;
    Ok(())
}

pub(super) fn limits_empty_data_fill_between(fig: &Figure) -> CaseResult {
    let (x, y) = days_of_2010()?;
    let ax = fig.subplot();
    let none = Vec::<f64>::new();
    ax.fill_between(&none, &none, [0.0], FillBetweenOptions::default())?;
    // fill_between converts the dates as matplotlib's does.
    let x = ax.xaxis().convert_units(x);
    let filled = FillBetweenOptions {
        props: CollectionProps::default().color("C0"),
        ..FillBetweenOptions::default()
    };
    ax.fill_between(x, y, [0.0], filled)?;
    Ok(())
}

pub(super) fn single_point(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 1)?;
    axs[(0, 0)].plot_fmt([0.0], [0.0], "o")?;
    axs[(1, 0)].plot_fmt([1.0], [1.0], "o")?;
    Ok(())
}

pub(super) fn shaped_data(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 1)?;
    // A 1×10 array is ten columns of one point each.
    for value in 0..10 {
        axs[(0, 0)].plot([0.0], [f64::from(value)])?;
    }
    axs[(1, 0)].plot_y(arange(0.0, 100.0, 10.0));
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
pub(super) fn canonical(fig: &Figure) -> CaseResult {
    fig.subplot().plot_y([1.0, 2.0, 3.0]);
    Ok(())
}

pub(super) fn bar_tick_label_single(fig: &Figure) -> CaseResult {
    let options = BarOptions {
        align: BarAlign::Edge,
        tick_labels: vec!["0".to_owned()],
        ..BarOptions::default()
    };
    fig.gca().bar([0.0], [1.0], options)?;
    Ok(())
}
