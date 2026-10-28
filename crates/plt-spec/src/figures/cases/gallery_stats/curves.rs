//! Confidence ellipses of correlated data and error bands around a
//! parametrised curve.

use plt::axes::{Axes, ScatterColors};
use plt::figure::Share;
use plt::gridspec::SubplotOverrides;
use plt::layout::LayoutEngine;
use plt::legend::LegendOptions;
use plt::patches::{Ellipse, PathShape};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::stats::{mean, var};
use plt::transforms::{Affine2D, Point, Transform};
use plt_core::Path;
use plt_core::path::PathCode;

use crate::figures::cases::common::{map, tickless};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The sample covariance of `x` and `y` (`np.cov(x, y)[0, 1]`).
fn covariance(x: &[f64], y: &[f64]) -> f64 {
    let (mx, my) = (mean(x), mean(y));
    let sum: f64 = x.iter().zip(y).map(|(x, y)| (x - mx) * (y - my)).sum();
    sum / (x.len() - 1) as f64
}

/// Draws the covariance ellipse of `x` and `y`, `n_std` standard
/// deviations wide, styled by `props` (`confidence_ellipse`; the face is
/// empty unless `props` fills it).
fn confidence_ellipse(ax: &Axes, x: &[f64], y: &[f64], n_std: f64, props: PatchProps) {
    let (var_x, var_y) = (var(x, 1), var(y, 1));
    let pearson = covariance(x, y) / (var_x * var_y).sqrt();
    let ellipse = Ellipse::new(
        (0.0, 0.0),
        (1.0 + pearson).sqrt() * 2.0,
        (1.0 - pearson).sqrt() * 2.0,
    );
    let transf = Affine2D::identity()
        .rotate_deg(45.0)
        .scale(var_x.sqrt() * n_std, var_y.sqrt() * n_std)
        .translate(mean(x), mean(y));
    let props = PatchProps::default()
        .facecolor("none")
        .overlay(props)
        .transform(Transform::Affine(transf).then(ax.trans_data()));
    ax.add_patch_with(ellipse, props);
}

/// `n` points of two normal variables mixed by `dependency`, scaled and
/// offset (`correlated_dataset`).
fn correlated_dataset(
    g: &mut SpecRng,
    n: usize,
    dependency: [[f64; 2]; 2],
    mu: (f64, f64),
    scale: (f64, f64),
) -> (Vec<f64>, Vec<f64>) {
    let latent = g.normal(2 * n, 0.0, 1.0);
    latent
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&[a, b]| {
            let x = a * dependency[0][0] + b * dependency[1][0];
            let y = a * dependency[0][1] + b * dependency[1][1];
            (x * scale.0 + mu.0, y * scale.1 + mu.1)
        })
        .unzip()
}

/// The grey lines through the origin (`cross_axes`).
fn cross_axes(ax: &Axes) {
    let grey = || LineProps::default().color("grey").linewidth(1.0);
    ax.axvline(0.0, grey());
    ax.axhline(0.0, grey());
}

/// Tiny dots of the data (`ax.scatter(x, y, s=0.5)`).
fn dots(ax: &Axes, x: &[f64], y: &[f64]) -> CaseResult {
    let options = ScatterOptions {
        sizes: Some(vec![0.5]),
        ..ScatterOptions::default()
    };
    ax.scatter_with(x, y, options)?;
    Ok(())
}

/// The red dot at the mean the data was drawn around.
fn red_mean(ax: &Axes, (x, y): (f64, f64)) -> CaseResult {
    let options = ScatterOptions {
        colors: ScatterColors::Single("red".into()),
        sizes: Some(vec![3.0]),
        ..ScatterOptions::default()
    };
    ax.scatter_with([x], [y], options)?;
    Ok(())
}

fn confidence_ellipse_1(fig: &Figure) -> CaseResult {
    let parameters = [
        ("Positive correlation", [[0.85, 0.35], [0.15, -0.65]]),
        ("Negative correlation", [[0.9, -0.4], [0.1, -0.6]]),
        ("Weak correlation", [[1.0, 0.0], [0.0, 1.0]]),
    ];
    let mu = (2.0, 4.0);
    let mut g = SpecRng::new(0);
    let axs = fig.subplots(1, 3)?;
    for (ax, (title, dependency)) in axs.as_slice().iter().zip(parameters) {
        let (x, y) = correlated_dataset(&mut g, 800, dependency, mu, (3.0, 5.0));
        dots(ax, &x, &y)?;
        cross_axes(ax);
        confidence_ellipse(ax, &x, &y, 3.0, PatchProps::default().edgecolor("red"));
        red_mean(ax, mu)?;
        ax.set_title(title);
    }
    Ok(())
}

fn confidence_ellipse_2(fig: &Figure) -> CaseResult {
    let ax_nstd = fig.subplot();
    cross_axes(&ax_nstd);
    let dependency = [[0.8, 0.75], [-0.2, 0.35]];
    let (x, y) = correlated_dataset(
        &mut SpecRng::new(0),
        500,
        dependency,
        (0.0, 0.0),
        (8.0, 5.0),
    );
    dots(&ax_nstd, &x, &y)?;
    let sigmas = [
        (1.0, "firebrick", LineStyle::Solid),
        (2.0, "fuchsia", LineStyle::Dashed),
        (3.0, "blue", LineStyle::Dotted),
    ];
    for (n_std, color, linestyle) in sigmas {
        let props = PatchProps::default()
            .label(format!("${n_std}\\sigma$"))
            .edgecolor(color)
            .linestyle(linestyle);
        confidence_ellipse(&ax_nstd, &x, &y, n_std, props);
    }
    red_mean(&ax_nstd, (0.0, 0.0))?;
    ax_nstd.set_title("Different standard deviations");
    ax_nstd.legend(LegendOptions::default())?;
    Ok(())
}

fn confidence_ellipse_3(fig: &Figure) -> CaseResult {
    let mu = (2.0, -3.0);
    let ax_kwargs = fig.subplot();
    cross_axes(&ax_kwargs);
    let dependency = [[-0.8, 0.5], [-0.2, 0.5]];
    let (x, y) = correlated_dataset(&mut SpecRng::new(0), 500, dependency, mu, (6.0, 5.0));
    let props = PatchProps::default()
        .alpha(0.5)
        .facecolor("pink")
        .edgecolor("purple")
        .zorder(0.0);
    confidence_ellipse(&ax_kwargs, &x, &y, 3.0, props);
    dots(&ax_kwargs, &x, &y)?;
    red_mean(&ax_kwargs, mu)?;
    ax_kwargs.set_title("Using keyword arguments");
    fig.subplots_adjust(SubplotOverrides {
        hspace: Some(0.25),
        ..SubplotOverrides::default()
    })?;
    Ok(())
}

/// `curve_error_band`'s curve: the parameter and the points of a limaçon
/// (`cardioid`).
fn cardioid() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let t = linspace(0.0, 2.0 * std::f64::consts::PI, 400);
    let x = map(&t, |t| (0.5 + t.cos()) * t.cos());
    let y = map(&t, |t| (0.5 + t.cos()) * t.sin());
    (t, x, y)
}

fn curve_error_band_1(fig: &Figure) -> CaseResult {
    let (_, x, y) = cardioid();
    let ax = fig.subplot();
    ax.plot_with(&x, &y, LineProps::fmt("k")?)?;
    ax.set_aspect(Aspect::Ratio(1.0))?;
    Ok(())
}

/// The finite differences along a curve: one-sided at the ends, central
/// inside.
fn differences(values: &[f64]) -> Vec<f64> {
    let n = values.len();
    let mut out = vec![values[1] - values[0]];
    out.extend(values.windows(3).map(|w| w[2] - w[0]));
    out.push(values[n - 1] - values[n - 2]);
    out
}

/// A band `err` wide on both sides of the curve, along its normals
/// (`draw_error_band`).
fn draw_error_band(ax: &Axes, x: &[f64], y: &[f64], err: &[f64], props: PatchProps) -> CaseResult {
    let (dx, dy) = (differences(x), differences(y));
    let offset = |i: usize| {
        let length = dx[i].hypot(dy[i]);
        let err = err[i % err.len()];
        (dy[i] / length * err, -dx[i] / length * err)
    };
    let outer = (0..x.len()).map(|i| {
        let (nx, ny) = offset(i);
        Point::new(x[i] + nx, y[i] + ny)
    });
    let inner = (0..x.len()).rev().map(|i| {
        let (nx, ny) = offset(i);
        Point::new(x[i] - nx, y[i] - ny)
    });
    let vertices: Vec<Point> = outer.chain(inner).collect();
    let mut codes = vec![PathCode::LineTo; vertices.len()];
    codes[0] = PathCode::MoveTo;
    codes[x.len()] = PathCode::MoveTo;
    ax.add_patch_with(PathShape(Path::new(vertices, codes)?), props);
    Ok(())
}

fn curve_error_band_2(fig: &Figure) -> CaseResult {
    let (t, x, y) = cardioid();
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots_shared(1, 2, Share::All, Share::All)?;
    let errs = [
        ("constant error", vec![0.05]),
        (
            "variable error",
            map(&t, |t| 0.05 * (2.0 * t).sin().powi(2) + 0.04),
        ),
    ];
    for (i, (ax, (title, err))) in axs.as_slice().iter().zip(errs).enumerate() {
        ax.set_title(title);
        ax.set_aspect(Aspect::Ratio(1.0))?;
        tickless(ax);
        ax.plot_with(&x, &y, LineProps::fmt("k")?)?;
        let props = PatchProps::default()
            .facecolor(format!("C{i}"))
            .edgecolor("none")
            .alpha(0.3);
        draw_error_band(ax, &x, &y, &err, props)?;
    }
    Ok(())
}

/// Every case of the module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_stats/confidence_ellipse_1", confidence_ellipse_1)
        .sized(9.0, 3.0, 100.0),
    FigureCase::new("gallery_stats/confidence_ellipse_2", confidence_ellipse_2)
        .sized(6.0, 6.0, 100.0),
    FigureCase::new("gallery_stats/confidence_ellipse_3", confidence_ellipse_3)
        .sized(6.0, 6.0, 100.0),
    FigureCase::new("gallery_stats/curve_error_band_1", curve_error_band_1),
    FigureCase::new("gallery_stats/curve_error_band_2", curve_error_band_2),
];
