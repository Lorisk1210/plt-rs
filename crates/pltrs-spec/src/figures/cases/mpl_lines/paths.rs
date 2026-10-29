//! The cases from `test_path.py`.

use pltrs::figure::Figure;
use pltrs::lines::LineProps;
use pltrs::numeric::{arange, linspace};
use pltrs::patches::Polygon;
use pltrs::transforms::Point;

use super::super::mpl_tests::{CLASSIC_TEST_PATCH, classic, remove_ticks_and_titles};
use super::plot_y;
use crate::figures::cases::common::xkcd;
use crate::figures::{CaseResult, FigureCase};

/// `test_path_clipping`: polygons crossing the axes edges in every way.
fn path_clipping(fig: &Figure) -> CaseResult {
    let polygons: [[(f64, f64); 4]; 7] = [
        [
            (200.0, 200.0),
            (200.0, 350.0),
            (400.0, 350.0),
            (400.0, 200.0),
        ],
        [
            (200.0, 200.0),
            (200.0, 350.0),
            (400.0, 350.0),
            (400.0, 100.0),
        ],
        [
            (200.0, 100.0),
            (200.0, 350.0),
            (400.0, 350.0),
            (400.0, 100.0),
        ],
        [
            (200.0, 100.0),
            (200.0, 415.0),
            (400.0, 350.0),
            (400.0, 100.0),
        ],
        [
            (200.0, 100.0),
            (200.0, 415.0),
            (400.0, 415.0),
            (400.0, 100.0),
        ],
        [
            (200.0, 415.0),
            (400.0, 415.0),
            (400.0, 100.0),
            (200.0, 100.0),
        ],
        [
            (400.0, 415.0),
            (400.0, 100.0),
            (200.0, 100.0),
            (200.0, 415.0),
        ],
    ];
    for (i, corners) in polygons.into_iter().enumerate() {
        let ax = fig.add_subplot(4, 2, i + 1)?;
        ax.set_xlim(0.0, 640.0)?.set_ylim(140.0, 400.0)?;
        ax.add_patch(Polygon::new(corners.map(|(x, y)| Point::new(x, y)), true))
            .set_facecolor("none")
            .set_edgecolor("red");
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_log_transform_with_zero`: a curve touching zero on a log axis.
fn semi_log_with_zero(fig: &Figure) -> CaseResult {
    let x = arange(-10.0, 10.0, 1.0);
    let y: Vec<f64> = x
        .iter()
        .map(|x| (1.0 - 1.0 / (x * x + 1.0)).powi(20))
        .collect();
    let ax = fig.subplot();
    let props = LineProps::fmt("-o")?
        .linewidth(15.0)
        .markeredgecolor(pltrs::rc::ColorOrAuto::Color("k".into()));
    ax.plot_with(&x, y, props)?;
    ax.set_yscale("log")?;
    ax.set_ylim(1e-7, 1.0)?;
    ax.grid(true);
    Ok(())
}

/// `test_xkcd_marker`: markers drawn in the xkcd style.
fn xkcd_marker(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, 5.0, 8);
    let ax = fig.subplot();
    let lines: [(Vec<f64>, &str); 3] = [
        (x.clone(), "+"),
        (x.iter().map(|x| 5.0 - x).collect(), "o"),
        (vec![2.5; 8], "^"),
    ];
    for (y, fmt) in lines {
        ax.plot_with(&x, y, LineProps::fmt(fmt)?.markersize(10.0))?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_marker_paths_pdf`: error bars with their cap markers.
fn marker_paths(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = arange(0.0, 7.0, 1.0);
    ax.errorbar(
        &x,
        vec![5.0; 7],
        pltrs::axes::ErrorbarOptions {
            yerr: Some(vec![1.0; 7].into()),
            ..pltrs::axes::ErrorbarOptions::default()
        },
    )?;
    ax.set_xlim(-1.0, 7.0)?.set_ylim(-1.0, 7.0)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_nan_isolated_points`: points cut off by NaN on both sides.
fn nan_path(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let nan = f64::NAN;
    plot_y(
        &ax,
        [0.0, nan, 2.0, nan, 4.0, 5.0, 6.0],
        LineProps::fmt("-o")?,
    )?;
    plot_y(
        &ax,
        [nan, 7.0, nan, 9.0, 10.0, nan, 12.0],
        LineProps::fmt("-o")?,
    )?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_lines/path_clipping", path_clipping)
        .sized(6.0, 6.2, 100.0)
        .with_style("classic")
        .with_rc(CLASSIC_TEST_PATCH),
    FigureCase::new("mpl_lines/semi_log_with_zero", semi_log_with_zero),
    classic("mpl_lines/xkcd", |fig| {
        let x = linspace(0.0, std::f64::consts::TAU, 100);
        let y: Vec<f64> = x.iter().map(|x| x.sin()).collect();
        fig.subplot().plot(&x, y)?;
        remove_ticks_and_titles(fig);
        Ok(())
    })
    .with_rc_setup(xkcd),
    classic("mpl_lines/xkcd_marker", xkcd_marker).with_rc_setup(xkcd),
    classic("mpl_lines/marker_paths", marker_paths),
    FigureCase::new("mpl_lines/nan_path", nan_path),
];
