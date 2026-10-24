//! `figures/cases_mpl_tests_agg.py`: matplotlib's own tests of the Agg
//! canvas (`test_agg.py`) that draw a figure and only check that drawing
//! it works, one case per figure, in the test suite's default style.

use plt::artist::ArtistHandle;
use plt::axes::FillBetweenOptions;
use plt::collections::CollectionSpec;
use plt::figure::Figure;
use plt::patheffects::{GcUpdate, PathEffect, Stroke};
use plt_core::{Path, Point};

use super::mpl_tests::classic;
use crate::figures::{CaseResult, FigureCase};

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    vec![
        classic(
            "mpl_tests_agg/large_single_path_collection",
            large_single_path_collection,
        ),
        classic("mpl_tests_agg/marker_with_nan", marker_with_nan),
        classic("mpl_tests_agg/long_path", long_path),
        classic("mpl_tests_agg/chunksize", sine),
        classic("mpl_tests_agg/chunksize_105", sine)
            .with_rc_setup(|rc| Ok(rc.set("agg.path.chunksize", "105")?)),
        classic("mpl_tests_agg/non_tuple_rgbaface", non_tuple_rgbaface),
    ]
}

/// `test_large_single_path_collection`: one rectangle far larger than the
/// view, in a path collection added without autoscaling.
fn large_single_path_collection(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let corners = [
        (-10.0, 5.0),
        (10.0, 5.0),
        (10.0, -5.0),
        (-10.0, -5.0),
        (-10.0, 5.0),
    ];
    let path = Path::polyline(corners.iter().map(|&(x, y)| Point::new(x, y)).collect());
    ax.add_collection(CollectionSpec::paths(vec![path]).without_autolim());
    ax.set_xlim(1e-3, 1.0)?;
    Ok(())
}

/// `test_marker_with_nan`: a log-x line of 0 to 999, whose first point the
/// scale loses, and a band between 0.8 and 1.2 times it.
fn marker_with_nan(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let data: Vec<f64> = (0..1000).map(f64::from).collect();
    // `semilogx(data)` plots the data against its indices, the same values.
    ax.semilogx(data.clone(), data.clone())?;
    let low: Vec<f64> = data.iter().map(|v| v * 0.8).collect();
    let high: Vec<f64> = data.iter().map(|v| v * 1.2).collect();
    ax.fill_between(data, low, high, FillBetweenOptions::default())?;
    Ok(())
}

/// `test_long_path`: 100 000 points alternating between -1 and 1.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn long_path(fig: &Figure) -> CaseResult {
    let points: Vec<f64> = (0..100_000)
        .map(|i| if i % 2 == 0 { -1.0 } else { 1.0 })
        .collect();
    fig.subplot().plot_y(points);
    Ok(())
}

/// `test_chunksize`: the sine of 0 to 199.
fn sine(fig: &Figure) -> CaseResult {
    let x: Vec<f64> = (0..200).map(f64::from).collect();
    let y: Vec<f64> = x.iter().map(|x| x.sin()).collect();
    fig.subplot().plot(x, y)?;
    Ok(())
}

/// `test_non_tuple_rgbaface`: a 3-D scatter stroked by a path effect.
fn non_tuple_rgbaface(fig: &Figure) -> CaseResult {
    let points = fig
        .subplot_3d()
        .scatter(&[0.0, 1.0, 2.0], &[0.0, 1.0, 2.0], &[0.0])?;
    points.set_path_effects(vec![PathEffect::stroke(Stroke::new(
        GcUpdate::default().linewidth(4.0),
    ))]);
    Ok(())
}
