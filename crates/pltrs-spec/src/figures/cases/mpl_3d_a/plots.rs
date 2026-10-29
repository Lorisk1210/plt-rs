//! Bars, lines and `fill_between`.

use std::f64::consts::PI;

use pltrs::axes::{BarAlign, BarOptions};
use pltrs::collections::CollectionProps;
use pltrs::colors::Color;
use pltrs::figure::Figure;
use pltrs::lines::LineProps;
use pltrs::mplot3d::{
    Axes3D, Axes3DError, Axes3DOptions, Axis3, Bar3DOptions, Boxes, FillBetween3DOptions, FillMode,
    Sampling, SurfaceOptions,
};
use pltrs::numeric::{arange, linspace};
use pltrs::patches::PatchProps;

use super::{aliased, range, remove_ticks_and_titles, sinc_surface, subplots_3d};
use crate::figures::CaseResult;

/// The 4 × 5 grid of bars of the `bar3d` shading tests: their x and y
/// corners (`np.meshgrid(np.arange(4), np.arange(5))`, raveled) and their
/// heights `x + y + height_offset`.
fn bar_grid(height_offset: f64) -> [Vec<f64>; 3] {
    let (x, y): (Vec<f64>, Vec<f64>) = (0..5)
        .flat_map(|row| (0..4).map(move |col| (f64::from(col), f64::from(row))))
        .unzip();
    let heights = x
        .iter()
        .zip(&y)
        .map(|(x, y)| x + y + height_offset)
        .collect();
    [x, y, heights]
}

/// `ax.bar3d(x2d, y2d, x2d * 0, 1, 1, z, shade=shade)` on [`bar_grid`].
fn bar3d_grid(ax: &Axes3D, height_offset: f64, shade: bool) -> Result<(), Axes3DError> {
    let [x, y, heights] = bar_grid(height_offset);
    let boxes = Boxes::new(&x, &y, [0.0], [1.0], [1.0], &heights);
    let options = Bar3DOptions {
        shade,
        ..Bar3DOptions::default()
    };
    ax.bar3d(boxes, options)?;
    Ok(())
}

pub(super) fn bar3d(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let xs = range(20);
    for (c, z) in [("r", 30.0), ("g", 20.0), ("b", 10.0), ("y", 0.0)] {
        let mut color = vec![Color::from(c); xs.len()];
        color[0] = "c".into();
        let options = BarOptions {
            align: BarAlign::Edge,
            color,
            props: PatchProps::default().alpha(0.8),
            ..BarOptions::default()
        };
        ax.bar(&xs, &xs, [z], Axis3::Y, options)?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn bar3d_shaded(fig: &Figure) -> CaseResult {
    let views = [
        (30.0, -60.0, 0.0),
        (30.0, 30.0, 30.0),
        (-30.0, 30.0, -90.0),
        (300.0, -30.0, 0.0),
    ];
    for (ax, (elev, azim, roll)) in subplots_3d(fig, 1, 4)?.iter().zip(views) {
        bar3d_grid(ax, 1.0, true)?;
        ax.view_init(elev, azim, roll);
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn bar3d_notshaded(fig: &Figure) -> CaseResult {
    bar3d_grid(&fig.subplot_3d(), 0.0, false)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn lines3d(fig: &Figure) -> CaseResult {
    let theta = linspace(-4.0 * PI, 4.0 * PI, 100);
    let z = linspace(-2.0, 2.0, 100);
    let r: Vec<f64> = z.iter().map(|z| z.powi(2) + 1.0).collect();
    let x: Vec<f64> = r.iter().zip(&theta).map(|(r, t)| r * t.sin()).collect();
    let y: Vec<f64> = r.iter().zip(&theta).map(|(r, t)| r * t.cos()).collect();
    fig.subplot_3d().plot(&x, &y, &z)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn plot_scalar(fig: &Figure) -> CaseResult {
    fig.subplot_3d().plot_fmt([1.0], [1.0], [0.0], "o")?;
    Ok(())
}

pub(super) fn mixedsubplot(fig: &Figure) -> CaseResult {
    let f = |t: &f64| (2.0 * PI * t).cos() * (-t).exp();
    let t1 = arange(0.0, 5.0, 0.1);
    let t2 = arange(0.0, 5.0, 0.02);
    let ax = fig.add_subplot(2, 1, 1)?;
    for (t, format) in [(&t1, "bo"), (&t2, "k--")] {
        let y: Vec<f64> = t.iter().map(f).collect();
        let props = LineProps::fmt(format)?.markerfacecolor("green");
        ax.plot_with(t.clone(), y, props)?;
    }
    ax.grid(true);
    let ax = fig.add_subplot_3d(2, 1, 2, Axes3DOptions::default())?;
    let (x, y, z) = sinc_surface();
    let options = SurfaceOptions {
        sampling: Sampling::Counts { rows: 40, cols: 40 },
        props: aliased(0.0),
        ..SurfaceOptions::default()
    };
    ax.plot_surface(&x, &y, &z, options)?;
    ax.set_zlim(-1.0, 1.0)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn plot_3d_from_2d(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let xs = range(5);
    let ys: Vec<f64> = (5..10).map(f64::from).collect();
    for zdir in [Axis3::X, Axis3::Y] {
        ax.plot_with(&xs, &ys, [0.0], zdir, LineProps::default())?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `theta = np.linspace(0, 2 * np.pi, 50)` and the test's `where`:
/// `(theta < np.pi / 2) | (theta > 3 * np.pi / 2)`.
fn theta_and_where() -> (Vec<f64>, Vec<bool>) {
    let theta = linspace(0.0, 2.0 * PI, 50);
    let mask = theta
        .iter()
        .map(|&t| t < PI / 2.0 || t > 3.0 * PI / 2.0)
        .collect();
    (theta, mask)
}

pub(super) fn fill_between_quad(fig: &Figure) -> CaseResult {
    let (theta, mask) = theta_and_where();
    let x1: Vec<f64> = theta.iter().map(|t| t.cos()).collect();
    let y1: Vec<f64> = theta.iter().map(|t| t.sin()).collect();
    let z1: Vec<f64> = theta.iter().map(|t| 0.1 * (6.0 * t).sin()).collect();
    let x2: Vec<f64> = x1.iter().map(|x| 0.6 * x).collect();
    let y2: Vec<f64> = y1.iter().map(|y| 0.6 * y).collect();
    let options = FillBetween3DOptions {
        mask: Some(mask),
        mode: FillMode::Auto,
        // The alpha fades the faces only; matplotlib also fades the edges
        // here because `edgecolor` follows `alpha` among the keywords.
        props: CollectionProps::default()
            .alpha(0.5)
            .edgecolor((0.0, 0.0, 0.0, 0.5)),
        ..FillBetween3DOptions::default()
    };
    fig.subplot_3d()
        .fill_between((&x1, &y1, &z1), (&x2, &y2, &[2.0]), options)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn fill_between_polygon(fig: &Figure) -> CaseResult {
    let (theta, mask) = theta_and_where();
    let z1: Vec<f64> = theta.iter().map(|t| t.cos()).collect();
    let z2: Vec<f64> = z1.iter().map(|z| z + 1.0).collect();
    let options = FillBetween3DOptions {
        mask: Some(mask),
        mode: FillMode::Auto,
        props: CollectionProps::default().edgecolor("k"),
        ..FillBetween3DOptions::default()
    };
    fig.subplot_3d()
        .fill_between((&theta, &[0.0], &z1), (&theta, &[0.0], &z2), options)?;
    remove_ticks_and_titles(fig);
    Ok(())
}
