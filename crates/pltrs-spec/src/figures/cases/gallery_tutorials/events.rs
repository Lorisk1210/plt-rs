//! The event handling examples' figures before any interaction: plain
//! plots waiting for clicks, keys or the mouse, cross-hair cursors where
//! `cursor_demo`'s simulated mouse move leaves them, point browsers, a
//! legend to pick, a looking glass, the pickable artists of the picking
//! demos and the zoom windows.

use std::f64::consts::PI;

use pltrs::artist::ClipPathSpec;
use pltrs::axes::{FillBetweenOptions, ImshowOptions, MappedColors, PlotError};
use pltrs::collections::CollectionProps;
use pltrs::image::Extent;
use pltrs::legend::LegendOptions;
use pltrs::lines::StepWhere;
use pltrs::numeric::Grid;
use pltrs::patches::Circle;
use pltrs::prelude::*;
use pltrs::text::{TextBoxProps, VAlign};

use super::{SEED, uniform_rows};
use crate::figures::cases::common::{map, mgrid_count};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn close_event(fig: &Figure) -> CaseResult {
    fig.subplot()
        .text(0.35, 0.5, "Close Me!")
        .set_fontsize(30.0);
    Ok(())
}

fn coords_demo(fig: &Figure) -> CaseResult {
    let t = arange(0.0, 1.0, 0.01);
    fig.subplot().plot(&t, map(&t, |t| (2.0 * PI * t).sin()))?;
    Ok(())
}

/// `cursor_demo`'s points with the cross hair where its simulated mouse
/// move to the data point (0.5, 0.5) puts it: there, or on the point with
/// the next x value if it `snap`s (`_cursor`).
fn cursor(fig: &Figure, title: &str, snap: bool) -> CaseResult {
    let x = arange(0.0, 1.0, 0.01);
    let y = map(&x, |x| (2.0 * 2.0 * PI * x).sin());
    let ax = fig.subplot();
    ax.set_title(title);
    ax.plot_fmt(&x, &y, "o")?;
    let (mut cx, mut cy) = (0.5, 0.5);
    if snap {
        let index = x.partition_point(|&v| v < cx).min(x.len() - 1);
        (cx, cy) = (x[index], y[index]);
    }
    let cross_hair = || {
        LineProps::default()
            .color("k")
            .linewidth(0.8)
            .linestyle(LineStyle::Dashed)
    };
    ax.axhline(cy, cross_hair());
    ax.axvline(cx, cross_hair());
    ax.text(0.72, 0.9, format!("x={cx:.2}, y={cy:.2}"))
        .set_transform(ax.trans_axes());
    Ok(())
}

fn cursor_demo_1(fig: &Figure) -> CaseResult {
    cursor(fig, "Simple cursor", false)
}

fn cursor_demo_2(fig: &Figure) -> CaseResult {
    cursor(fig, "Blitted cursor", false)
}

fn cursor_demo_3(fig: &Figure) -> CaseResult {
    cursor(fig, "Snapping cursor", true)
}

/// The mean and the standard deviation of each row.
fn means_and_deviations(rows: &Grid<f64>) -> (Vec<f64>, Vec<f64>) {
    rows.iter_rows()
        .map(|row| {
            let n = row.len() as f64;
            let mean = row.iter().sum::<f64>() / n;
            let variance = row.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / n;
            (mean, variance.sqrt())
        })
        .unzip()
}

/// The mean and standard deviation of 100 series of `samples` uniform
/// draws, as points on `ax` (`_means_and_deviations`).
fn browsed_points(ax: &Axes, samples: usize) -> Result<(Vec<f64>, Vec<f64>), PlotError> {
    let (xs, ys) = means_and_deviations(&uniform_rows(&mut SpecRng::new(SEED), 100, samples));
    ax.set_title("click on point to plot time series");
    ax.plot_with(&xs, &ys, LineProps::fmt("o")?.pickradius(5.0))?;
    Ok((xs, ys))
}

fn data_browser(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 1)?;
    let ax = &axs[(0, 0)];
    let (xs, ys) = browsed_points(ax, 200)?;
    ax.text(0.05, 0.95, "selected: none")
        .set_valign(VAlign::Top)
        .set_transform(ax.trans_axes());
    let selected = LineProps::fmt("o")?
        .markersize(12.0)
        .alpha(0.4)
        .color("yellow")
        .visible(false);
    ax.plot_with([xs[0]], [ys[0]], selected)?;
    Ok(())
}

fn figure_axes_enter_leave(fig: &Figure) -> CaseResult {
    fig.subplots(2, 1)?;
    fig.suptitle("mouse hover over figure or Axes to trigger events");
    Ok(())
}

fn image_slices_viewer(fig: &Figure) -> CaseResult {
    // The first slice of sin(xyz) / (xyz), at z = 1.
    let x = mgrid_count(-10.0, 10.0, 100);
    let slice = Grid::from_fn(x.len(), x.len(), |i, j| {
        let xy = x[i] * x[j];
        xy.sin() / xy
    });
    let ax = fig.subplot();
    ax.imshow(slice, ImshowOptions::default())?;
    ax.set_title("Use scroll wheel to navigate\nindex 0");
    Ok(())
}

fn keypress_demo(fig: &Figure) -> CaseResult {
    let mut g = SpecRng::new(SEED);
    let ax = fig.subplot();
    ax.plot_fmt(g.uniform(12, 0.0, 1.0), g.uniform(12, 0.0, 1.0), "go")?;
    ax.set_xlabel("easy come, easy go");
    ax.set_title("Press a key");
    Ok(())
}

fn legend_picking(fig: &Figure) -> CaseResult {
    let t = linspace(0.0, 1.0, 50);
    let ax = fig.subplot();
    ax.set_title("Click on legend line to toggle line on/off");
    for (amplitude, frequency, label) in [(2.0, 1.0, "1 Hz"), (4.0, 2.0, "2 Hz")] {
        ax.plot_with(
            &t,
            map(&t, |t| amplitude * (2.0 * PI * frequency * t).sin()),
            LineProps::default().linewidth(2.0).label(label),
        )?;
    }
    ax.legend(LegendOptions {
        fancybox: Some(true),
        shadow: Some(true.into()),
        ..LegendOptions::default()
    })?;
    Ok(())
}

fn looking_glass(fig: &Figure) -> CaseResult {
    let points = uniform_rows(&mut SpecRng::new(SEED), 2, 200);
    let (x, y) = (points.row(0), points.row(1));
    let ax = fig.subplot();
    let circ = ax.add_patch_with(
        Circle::new((0.5, 0.5), 0.25),
        PatchProps::default().alpha(0.8).facecolor("yellow"),
    );
    ax.plot_with(x, y, LineProps::default().alpha(0.2))?;
    let magnified = LineProps::default()
        .alpha(1.0)
        .clip_path(ClipPathSpec::Artist(circ.id()));
    ax.plot_with(x, y, magnified)?;
    ax.set_title("Left click and drag to move looking glass");
    Ok(())
}

fn pick_event_demo_1(fig: &Figure) -> CaseResult {
    let mut g = SpecRng::new(SEED);
    let axs = fig.subplots(2, 1)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(1, 0)]);
    ax1.set_title("click on points, rectangles or text");
    ax1.set_ylabel("ylabel");
    ax1.yaxis().set_label_bbox(Some(TextBoxProps {
        patch: PatchProps::default().facecolor("red"),
        ..TextBoxProps::default()
    }));
    let y = g.uniform(100, 0.0, 1.0);
    let x: Vec<f64> = (0..y.len()).map(|i| i as f64).collect();
    ax1.plot_with(&x, &y, LineProps::fmt("o")?.pickradius(5.0))?;
    let positions: Vec<f64> = (0..10).map(f64::from).collect();
    ax2.bar(positions, g.uniform(10, 0.0, 1.0), BarOptions::default())?;
    Ok(())
}

fn pick_event_demo_2(fig: &Figure) -> CaseResult {
    let mut g = SpecRng::new(SEED);
    let ax = fig.subplot();
    ax.set_title("custom picker for line data");
    ax.plot_fmt(g.uniform(100, 0.0, 1.0), g.uniform(100, 0.0, 1.0), "o")?;
    Ok(())
}

fn pick_event_demo_3(fig: &Figure) -> CaseResult {
    let values = uniform_rows(&mut SpecRng::new(SEED), 4, 100);
    let options = ScatterOptions {
        sizes: Some(map(values.row(3), |s| 100.0 * s)),
        colors: MappedColors::new(values.row(2).to_vec()).into(),
        ..ScatterOptions::default()
    };
    fig.subplot()
        .scatter_with(values.row(0).to_vec(), values.row(1).to_vec(), options)?;
    Ok(())
}

fn pick_event_demo_4(fig: &Figure) -> CaseResult {
    let mut g = SpecRng::new(SEED);
    let ax = fig.subplot();
    for (rows, cols, extent) in [
        (10, 5, Extent::new(1.0, 2.0, 1.0, 2.0)),
        (5, 10, Extent::new(3.0, 4.0, 1.0, 2.0)),
        (20, 25, Extent::new(1.0, 2.0, 3.0, 4.0)),
        (30, 12, Extent::new(3.0, 4.0, 3.0, 4.0)),
    ] {
        let placed = ImshowOptions {
            extent: Some(extent),
            ..ImshowOptions::default()
        };
        ax.imshow(uniform_rows(&mut g, rows, cols), placed)?;
    }
    ax.set_xlim(0.0, 5.0)?.set_ylim(0.0, 5.0)?;
    Ok(())
}

fn pick_event_demo2(fig: &Figure) -> CaseResult {
    browsed_points(&fig.subplot(), 1000)?;
    Ok(())
}

fn resample(fig: &Figure) -> CaseResult {
    let xdata = linspace(16.0, 365.0, (365 - 16) * 4);
    let y1data = map(&xdata, |x| {
        (2.0 * PI * x / 153.0).sin() + (2.0 * PI * x / 127.0).cos()
    });
    // The whole range is visible: every 27th of the 1396 points is shown.
    let x: Vec<f64> = xdata.iter().copied().step_by(27).collect();
    let y1: Vec<f64> = y1data.iter().copied().step_by(27).collect();
    let y2 = map(&y1, |y| y + 0.2);
    let ax = fig.subplot();
    ax.plot_fmt(&x, &y1, "o-")?;
    let red_steps = FillBetweenOptions {
        step: Some(StepWhere::Pre),
        props: CollectionProps::default().color("r"),
        ..FillBetweenOptions::default()
    };
    ax.fill_between(&x, &y1, &y2, red_steps)?;
    ax.set_autoscale_on(false);
    ax.set_xlim(16.0, 365.0)?;
    Ok(())
}

fn timers(fig: &Figure) -> CaseResult {
    let x = linspace(-3.0, 3.0, 50);
    fig.subplot().plot(&x, map(&x, |x| x * x))?;
    Ok(())
}

/// `zoom_window`'s scatter seen over `xlim × ylim` (`_zoom_window`).
fn zoom_window(fig: &Figure, title: &str, xlim: (f64, f64), ylim: (f64, f64)) -> CaseResult {
    let values = uniform_rows(&mut SpecRng::new(SEED), 4, 200);
    let ax = fig.subplot();
    ax.set_xlim(xlim.0, xlim.1)?.set_ylim(ylim.0, ylim.1)?;
    ax.set_autoscale_on(false);
    ax.set_title(title);
    let options = ScatterOptions {
        sizes: Some(map(values.row(2), |s| s * 200.0)),
        colors: MappedColors::new(values.row(3).to_vec()).into(),
        ..ScatterOptions::default()
    };
    ax.scatter_with(values.row(0).to_vec(), values.row(1).to_vec(), options)?;
    Ok(())
}

fn zoom_window_1(fig: &Figure) -> CaseResult {
    zoom_window(fig, "Click to zoom", (0.0, 1.0), (0.0, 1.0))
}

fn zoom_window_2(fig: &Figure) -> CaseResult {
    zoom_window(fig, "Zoom window", (0.45, 0.55), (0.4, 0.6))
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_tutorials/close_event", close_event),
    FigureCase::new("gallery_tutorials/coords_demo", coords_demo),
    FigureCase::new("gallery_tutorials/cursor_demo_1", cursor_demo_1),
    FigureCase::new("gallery_tutorials/cursor_demo_2", cursor_demo_2),
    FigureCase::new("gallery_tutorials/cursor_demo_3", cursor_demo_3),
    FigureCase::new("gallery_tutorials/data_browser", data_browser),
    FigureCase::new(
        "gallery_tutorials/figure_axes_enter_leave",
        figure_axes_enter_leave,
    ),
    FigureCase::new("gallery_tutorials/image_slices_viewer", image_slices_viewer),
    FigureCase::new("gallery_tutorials/keypress_demo", keypress_demo),
    FigureCase::new("gallery_tutorials/legend_picking", legend_picking),
    FigureCase::new("gallery_tutorials/looking_glass", looking_glass),
    FigureCase::new("gallery_tutorials/pick_event_demo_1", pick_event_demo_1),
    FigureCase::new("gallery_tutorials/pick_event_demo_2", pick_event_demo_2),
    FigureCase::new("gallery_tutorials/pick_event_demo_3", pick_event_demo_3),
    FigureCase::new("gallery_tutorials/pick_event_demo_4", pick_event_demo_4),
    FigureCase::new("gallery_tutorials/pick_event_demo2", pick_event_demo2),
    FigureCase::new("gallery_tutorials/resample", resample),
    FigureCase::new("gallery_tutorials/timers", timers),
    FigureCase::new("gallery_tutorials/zoom_window_1", zoom_window_1).sized(3.7, 3.7, 100.0),
    FigureCase::new("gallery_tutorials/zoom_window_2", zoom_window_2).sized(3.7, 3.7, 100.0),
];
