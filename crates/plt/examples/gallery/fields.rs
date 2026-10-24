//! Images and fields: colour-mapped images, filled and labelled contours,
//! streamlines and arrows.

use plt::axes::ImshowOptions;
use plt::colorbar::ColorbarOptions;
use plt::colors::{Color, ColorizerOptions, colormaps};
use plt::contour::{ClabelOptions, ContourOptions, Levels, QuadCoords, QuadField};
use plt::numeric::{Grid, linspace, meshgrid};
use plt::prelude::*;
use plt::quiver::{QuiverOptions, VectorField};
use plt::rc::Aspect;
use plt::streamplot::{StreamColorMap, StreamColors, StreamField, StreamplotOptions};

use super::{Built, Entry};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry {
        name: "image_colorbar",
        caption: "An image with a colorbar",
        build: image_colorbar,
    },
    Entry {
        name: "contours",
        caption: "Filled contours with labelled contour lines",
        build: contours,
    },
    Entry {
        name: "streamlines",
        caption: "Streamlines coloured by speed",
        build: streamlines,
    },
    Entry {
        name: "quiver",
        caption: "A vector field of arrows",
        build: quiver,
    },
];

/// Two Gaussian bumps of opposite signs over `[-3, 3] × [-2, 2]`.
fn bumps(nx: usize, ny: usize) -> (Grid<f64>, Grid<f64>, Grid<f64>) {
    let (xx, yy) = meshgrid(&linspace(-3.0, 3.0, nx), &linspace(-2.0, 2.0, ny));
    let z = Grid::from_fn(ny, nx, |i, j| {
        let (x, y) = (xx[(i, j)], yy[(i, j)]);
        let bump = |cx: f64, cy: f64| (-((x - cx).powi(2)) - (y - cy).powi(2)).exp();
        2.0 * (bump(0.0, 0.0) - bump(1.0, 1.0))
    });
    (xx, yy, z)
}

/// A wave interference pattern in the viridis colormap.
fn image_colorbar() -> Built {
    let fig = Figure::with_size(6.0, 4.5, 100.0);
    let ax = fig.subplot();
    let n = 200;
    let z = Grid::from_fn(n, n, |i, j| {
        let (x, y) = (j as f64 / 20.0 - 5.0, i as f64 / 20.0 - 5.0);
        (x.hypot(y) * 2.0).sin() + (x * 1.5).cos() * (y * 0.7).sin()
    });
    let options = ImshowOptions {
        colorizer: ColorizerOptions::default().cmap(colormaps::get("viridis")?),
        ..ImshowOptions::default()
    };
    let image = ax.imshow(z, options)?;
    fig.colorbar(&image, ColorbarOptions::default())?;
    ax.set_title("Interference");
    Ok(fig)
}

/// Filled contours in a diverging colormap, black contour lines on top,
/// labelled in place.
fn contours() -> Built {
    let fig = Figure::with_size(6.4, 4.5, 100.0);
    let ax = fig.subplot();
    let (xx, yy, z) = bumps(120, 80);
    let field = QuadField::new(z).with_coords(QuadCoords::Mesh { x: xx, y: yy });
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

/// Streamlines of `u = -1 - x² + y`, `v = 1 + x - y²`, coloured by speed.
fn streamlines() -> Built {
    let fig = Figure::with_size(5.0, 5.0, 100.0);
    let ax = fig.subplot();
    let n = 60;
    let x = linspace(-3.0, 3.0, n);
    let u = Grid::from_fn(n, n, |i, j| -1.0 - x[j] * x[j] + x[i]);
    let v = Grid::from_fn(n, n, |i, j| 1.0 + x[j] - x[i] * x[i]);
    let speed = Grid::from_fn(n, n, |i, j| u[(i, j)].hypot(v[(i, j)]));
    let field = StreamField::new(x.clone(), x, u, v);
    let options = StreamplotOptions {
        density: (1.2, 1.2),
        color: Some(StreamColors::Mapped(Box::new(StreamColorMap {
            colorizer: ColorizerOptions {
                cmap: Some(colormaps::get("plasma")?),
                norm: None,
                ..ColorizerOptions::default()
            },
            values: speed,
        }))),
        ..StreamplotOptions::default()
    };
    ax.streamplot(&field, options)?;
    ax.set_title("streamplot");
    Ok(fig)
}

/// A swirl with a sink, as arrows on a regular mesh.
fn quiver() -> Built {
    let fig = Figure::with_size(5.0, 5.0, 100.0);
    let ax = fig.subplot();
    let x = linspace(-2.0, 2.0, 15);
    let (xx, yy) = meshgrid(&x, &x);
    let u = Grid::from_fn(15, 15, |i, j| -yy[(i, j)] - 0.3 * xx[(i, j)]);
    let v = Grid::from_fn(15, 15, |i, j| xx[(i, j)] - 0.3 * yy[(i, j)]);
    ax.quiver(
        VectorField::on_mesh(&xx, &yy, &u, &v),
        QuiverOptions::default(),
    )?;
    ax.set_aspect(Aspect::Equal)?;
    ax.set_title("quiver");
    Ok(fig)
}
