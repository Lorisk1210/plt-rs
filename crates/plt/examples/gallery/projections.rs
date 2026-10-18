//! Other projections: polar, geographic and 3-D axes.

use plt::colors::colormaps;
use plt::geo::GeoProjection;
use plt::mplot3d::{FaceColormap, FaceColors, SurfaceOptions};
use plt::numeric::{Grid, linspace, meshgrid};
use plt::prelude::*;

use super::{Built, Entry};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry {
        name: "polar",
        caption: "A rose curve on polar axes",
        build: polar,
    },
    Entry {
        name: "mollweide",
        caption: "A Mollweide map grid with a great-circle track",
        build: mollweide,
    },
    Entry {
        name: "surface_3d",
        caption: "A 3-D surface coloured by height",
        build: surface_3d,
    },
];

/// `r = cos(4θ)` traced over a full turn.
fn polar() -> Built {
    let fig = Figure::with_size(5.0, 5.0, 100.0);
    let ax = fig.polar_subplot();
    let theta = linspace(0.0, 2.0 * std::f64::consts::PI, 600);
    let r: Vec<f64> = theta.iter().map(|t| (4.0 * t).cos().abs()).collect();
    ax.plot(theta, r)?;
    ax.set_title("r = |cos 4θ|");
    Ok(fig)
}

/// A sinusoidal track across a Mollweide grid.
fn mollweide() -> Built {
    let fig = Figure::with_size(7.0, 4.0, 100.0);
    let ax = fig.geo_subplot(GeoProjection::Mollweide);
    let lon = linspace(-std::f64::consts::PI, std::f64::consts::PI, 200);
    let lat: Vec<f64> = lon.iter().map(|l| 0.6 * (2.0 * l).sin()).collect();
    ax.plot(lon, lat)?;
    ax.grid(true);
    ax.set_title("Mollweide");
    Ok(fig)
}

/// `z = sin(r) / r` over a square, coloured by height.
fn surface_3d() -> Built {
    let fig = Figure::with_size(6.0, 5.0, 100.0);
    let ax = fig.subplot_3d();
    let x = linspace(-8.0, 8.0, 50);
    let (xx, yy) = meshgrid(&x, &x);
    let z = Grid::from_fn(50, 50, |i, j| {
        let r = xx[(i, j)].hypot(yy[(i, j)]);
        if r == 0.0 { 1.0 } else { r.sin() / r }
    });
    let options = SurfaceOptions {
        colors: FaceColors::Mapped(Box::new(FaceColormap::new(colormaps::get("viridis")?))),
        ..SurfaceOptions::default()
    };
    ax.plot_surface(&xx, &yy, &z, options)?;
    ax.set_title("sin(r) / r");
    Ok(fig)
}
