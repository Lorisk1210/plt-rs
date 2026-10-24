//! Scatter plots (`TestScatter`): colours, markers, 2D inputs, invalid
//! values, single points and empty scatters.

use std::f64::consts::{PI, TAU};

use plt::axes::{MappedColors, ScatterColors, ScatterOptions};
use plt::collections::EdgeColors;
use plt::colors::{Color, Colorize, colormaps};
use plt::figure::Figure;
use plt::lines::LineStyle;
use plt::markers::{FillStyle, Marker, MarkerStyle};
use plt::numeric::linspace;
use plt::prelude::ArtistHandle;
use plt_core::Point;

use super::{classic, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};

const X: [f64; 4] = [3.0, 4.0, 2.0, 6.0];
const Y: [f64; 4] = [2.0, 5.0, 2.0, 3.0];
const SIZES: [f64; 4] = [24.0, 15.0, 19.0, 29.0];

/// One colour per point.
fn per_point(colors: &[&str]) -> ScatterColors {
    ScatterColors::PerPoint(colors.iter().copied().map(Color::from).collect())
}

/// `c=['r', 'y', 'b', 'lime']` with `s=[24, 15, 19, 29]`.
fn colored_points() -> ScatterOptions {
    ScatterOptions {
        colors: per_point(&["r", "y", "b", "lime"]),
        sizes: Some(SIZES.to_vec()),
        ..ScatterOptions::default()
    }
}

/// `X` and `Y` shifted by `delta`.
fn shifted(delta: f64) -> (Vec<f64>, Vec<f64>) {
    (X.map(|v| v + delta).to_vec(), Y.map(|v| v + delta).to_vec())
}

/// `test_scatter_marker`: square, half-filled circle and ellipse markers.
fn scatter_marker(fig: &Figure) -> CaseResult {
    let (rx, ry) = (3.0, 1.0);
    let area = rx * ry * PI;
    let verts = linspace(0.0, TAU, 21)
        .into_iter()
        .map(|theta| Point::new(theta.cos() * rx / area, theta.sin() * ry / area))
        .collect();
    let markers = [
        MarkerStyle::new("s".parse::<Marker>()?, FillStyle::Full)?,
        MarkerStyle::new("o".parse::<Marker>()?, FillStyle::Top)?,
        MarkerStyle::new(Marker::Vertices(verts), FillStyle::Full)?,
    ];
    let axs = fig.subplots(1, 3)?;
    for (ax, marker) in axs.as_slice().iter().zip(markers) {
        let options = ScatterOptions {
            colors: ScatterColors::PerPoint(vec![
                Color::from((1.0, 0.0, 0.0)),
                "y".into(),
                "b".into(),
                "lime".into(),
            ]),
            sizes: Some(vec![60.0, 50.0, 40.0, 30.0]),
            edgecolors: Some(EdgeColors::Colors(
                ["k", "r", "g", "b"].map(Color::from).to_vec(),
            )),
            marker: Some(marker),
            ..ScatterOptions::default()
        };
        ax.scatter_with(X, Y, options)?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `c=[1, nan, 2, nan]` through a 16-colour viridis with black for bad
/// values.
fn invalid_colors(fig: &Figure, plot_nonfinite: bool) -> CaseResult {
    let cmap = colormaps::get("viridis")?
        .resampled(16)?
        .with_bad("black")?;
    let options = ScatterOptions {
        colors: MappedColors::new([1.0, f64::NAN, 2.0, f64::NAN])
            .cmap(cmap)
            .into(),
        sizes: Some(vec![1.0, 2.0, 3.0, 4.0]),
        plot_nonfinite,
        ..ScatterOptions::default()
    };
    let x = [0.0, 1.0, 2.0, 3.0];
    fig.subplot().scatter_with(x, x, options)?;
    Ok(())
}

/// `test_empty_scatter`: no points, explicit edge and face colours and
/// line style, unclipped.
fn empty_scatter(
    fig: &Figure,
    edgecolor: &str,
    facecolor: &str,
    linestyle: LineStyle,
) -> CaseResult {
    let ax = fig.subplot();
    let options = ScatterOptions {
        colors: ScatterColors::Single(facecolor.into()),
        edgecolors: Some(EdgeColors::Colors(vec![edgecolor.into()])),
        ..ScatterOptions::default()
    };
    ax.scatter_with(Vec::<f64>::new(), Vec::<f64>::new(), options)?
        .set_linestyles(vec![linestyle])
        .set_clip_on(false);
    ax.set_xlim(0.0, 1.0);
    ax.set_ylim(0.0, 1.0);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_axes_b/scatter_plot", |fig| {
        let ax = fig.subplot();
        let (x, y) = shifted(-1.0);
        ax.scatter_with(x, y, colored_points())?;
        let (x, y) = shifted(1.0);
        let grays = ScatterOptions {
            colors: per_point(&["0.5", "0.6", "0.7", "0.8"]),
            ..colored_points()
        };
        ax.scatter_with(x, y, grays)?;
        ax.scatter_with(X, Y, colored_points())?;
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    classic("mpl_axes_b/scatter_marker", scatter_marker),
    classic("mpl_axes_b/scatter_2D", |fig| {
        // `np.meshgrid(np.arange(3), np.arange(2))`, flattened row by row.
        let x = [0.0, 1.0, 2.0, 0.0, 1.0, 2.0];
        let y = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let z: Vec<f64> = x.iter().zip(y).map(|(x, y)| x + y).collect();
        let options = ScatterOptions {
            colors: MappedColors::new(z).into(),
            sizes: Some(vec![200.0]),
            edgecolors: Some(EdgeColors::Face),
            ..ScatterOptions::default()
        };
        fig.subplot().scatter_with(x, y, options)?;
        remove_ticks_and_titles(fig);
        Ok(())
    }),
    classic("mpl_axes_b/scatter_decimal", |fig| {
        let x = [1.5, 8.4, 5.3, 4.2];
        let y = [1.1, 2.2, 3.3, 4.4];
        fig.subplot().scatter_with(x, y, colored_points())?;
        Ok(())
    }),
    classic("mpl_axes_b/scatter_invalid_color", |fig| {
        invalid_colors(fig, true)
    }),
    classic("mpl_axes_b/scatter_no_invalid_color", |fig| {
        invalid_colors(fig, false)
    }),
    classic("mpl_axes_b/scatter_single_point", |fig| {
        let options = ScatterOptions {
            colors: MappedColors::new([1.0]).into(),
            ..ScatterOptions::default()
        };
        fig.subplot().scatter_with([1.0], [1.0], options)?;
        Ok(())
    }),
    classic("mpl_axes_b/scatter_different_shapes", |fig| {
        // x, y and c hold `0..10` in different shapes; only the flat order
        // matters.
        let x: Vec<f64> = (0..10).map(f64::from).collect();
        let options = ScatterOptions {
            colors: MappedColors::new(x.clone()).into(),
            ..ScatterOptions::default()
        };
        fig.subplot().scatter_with(x.clone(), x, options)?;
        Ok(())
    }),
    FigureCase::new("mpl_axes_b/scatter_single_color_c", |fig| {
        let ax = fig.subplot();
        let single = |color: Color| ScatterOptions {
            colors: ScatterColors::Single(color),
            ..ScatterOptions::default()
        };
        ax.scatter_with([1.0; 3], [0.0, 1.0, 2.0], single((1.0, 0.5, 0.05).into()))?;
        ax.scatter_with(
            [2.0; 4],
            [0.0, 1.0, 2.0, 3.0],
            single((1.0, 0.5, 0.05, 0.5).into()),
        )?;
        Ok(())
    }),
    classic("mpl_axes_b/empty_scatter_red_blue_solid", |fig| {
        empty_scatter(fig, "red", "blue", LineStyle::Solid)
    }),
    classic("mpl_axes_b/empty_scatter_red_blue_dashed", |fig| {
        empty_scatter(fig, "red", "blue", LineStyle::Dashed)
    }),
    classic("mpl_axes_b/empty_scatter_red_none_solid", |fig| {
        empty_scatter(fig, "red", "none", LineStyle::Solid)
    }),
    classic("mpl_axes_b/empty_scatter_none_blue_solid", |fig| {
        empty_scatter(fig, "none", "blue", LineStyle::Solid)
    }),
];
