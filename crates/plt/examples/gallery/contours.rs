//! Contours: filled and labelled contours on grids, hatched bands, and
//! contours of scattered points through a triangulation.

use plt::colors::{ColorizerOptions, Extend};
use plt::contour::{ClabelOptions, ContourOptions, Levels, QuadCoords, QuadField};
use plt::layout::LayoutEngine;
use plt::prelude::*;
use plt::rc::Aspect;
use plt::tri::Triangulation;

use super::data::Samples;
use super::{Built, Entry};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry::figure(
        "contours",
        "Filled contours with labelled contour lines",
        contours,
    ),
    Entry::figure(
        "contourf_hatching",
        "Hatched filled contours with labels and extensions",
        contourf_hatching,
    ),
    Entry::figure(
        "tricontour",
        "Contours of scattered points (tricontourf, triplot)",
        tricontour,
    ),
];

/// Two Gaussian bumps of opposite signs over `[-3, 3] × [-2, 2]`.
fn bumps(nx: usize, ny: usize) -> QuadField {
    let (xx, yy) = meshgrid(&linspace(-3.0, 3.0, nx), &linspace(-2.0, 2.0, ny));
    let z = Grid::from_fn(ny, nx, |i, j| bump_pair(xx[(i, j)], yy[(i, j)]));
    QuadField::new(z).with_coords(QuadCoords::Mesh { x: xx, y: yy })
}

/// A positive bump at the origin and a negative one at `(1, 1)`.
fn bump_pair(x: f64, y: f64) -> f64 {
    let bump = |cx: f64, cy: f64| (-((x - cx).powi(2)) - (y - cy).powi(2)).exp();
    2.0 * (bump(0.0, 0.0) - bump(1.0, 1.0))
}

/// Filled contours in a diverging colormap, black contour lines on top,
/// labelled in place.
fn contours() -> Built {
    let fig = Figure::with_size(6.4, 4.5, 100.0);
    let ax = fig.subplot();
    let field = bumps(120, 80);
    let filled = ax.contourf(
        &field,
        ContourOptions {
            colorizer: ColorizerOptions {
                cmap: Some(colormaps::get("RdBu_r")?),
                ..ColorizerOptions::default()
            },
            levels: Levels::Count(14),
            ..ContourOptions::default()
        },
    )?;
    let lines = ax.contour(
        &field,
        ContourOptions {
            levels: Levels::Count(7),
            colors: Some(vec![Color::from("k")]),
            linewidths: Some(vec![0.8]),
            ..ContourOptions::default()
        },
    )?;
    lines.clabel(ClabelOptions::default())?;
    fig.colorbar(&filled, ColorbarOptions::default())?;
    ax.set_title("contourf + contour + clabel");
    Ok(fig)
}

/// Bands told apart by hatching as well as by colour, which survives
/// printing in grey. `Extend::Both` adds bands for the values beyond
/// the outer levels, and the colorbar shows them as pointed ends.
fn contourf_hatching() -> Built {
    let fig = Figure::with_size(6.4, 4.5, 100.0);
    let ax = fig.subplot();
    let (x, y) = (linspace(-3.0, 5.0, 150), linspace(-3.0, 5.0, 120));
    let z = Grid::from_fn(120, 150, |r, c| x[c].cos() + y[r].sin());
    let field = QuadField::new(z).with_coords(QuadCoords::Axes { x, y });
    let levels = Levels::Values(vec![-1.5, -0.75, 0.0, 0.75, 1.5]);
    let bands = ContourOptions {
        levels: levels.clone(),
        extend: Extend::Both,
        hatches: ["..", "/", "", "\\", "xx", "++"]
            .map(|h| Some(h.to_owned()))
            .to_vec(),
        alpha: Some(0.8),
        ..ContourOptions::default()
    }
    .cmap(colormaps::get("PuOr")?);
    let filled = ax.contourf(&field, bands)?;
    let lines = ContourOptions {
        levels,
        colors: Some(vec!["0.2".into()]),
        linewidths: Some(vec![0.6]),
        ..ContourOptions::default()
    };
    ax.contour(&field, lines)?
        .clabel(ClabelOptions::default())?;
    fig.colorbar(&filled, ColorbarOptions::default())?;
    ax.set_title("cos x + sin y, hatched");
    Ok(fig)
}

/// Two hundred scattered measurements: a Delaunay triangulation connects
/// them, and contours are drawn on its triangles without a regular grid.
fn tricontour() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    fig.set_layout_engine(LayoutEngine::compressed())?;
    let ax = fig.subplot();
    let mut samples = Samples::new(13);
    let (x, y): (Vec<f64>, Vec<f64>) = (0..200)
        .map(|_| (6.0 * samples.uniform() - 3.0, 4.0 * samples.uniform() - 2.0))
        .unzip();
    let z: Vec<f64> = x.iter().zip(&y).map(|(x, y)| bump_pair(*x, *y)).collect();
    let tri = Triangulation::delaunay(&x, &y)?;
    let options = ContourOptions {
        levels: Levels::Count(12),
        ..ContourOptions::default()
    }
    .cmap(colormaps::get("RdBu_r")?);
    let filled = ax.tricontourf(&tri, &z, options)?;
    ax.triplot(&tri, "-", LineProps::default().color("0.3").linewidth(0.3))?;
    ax.plot_with(&x, &y, LineProps::fmt("o")?.color("k").markersize(2.0))?;
    fig.colorbar(&filled, ColorbarOptions::default())?;
    ax.set_aspect(Aspect::Equal)?;
    ax.set_title("tricontourf over a Delaunay triangulation");
    Ok(fig)
}
