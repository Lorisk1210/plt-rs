//! Images and meshes: colour-mapped images, interpolation, masked data
//! and pseudocolour meshes on irregular grids.

use plt::axes::{Mesh, PcolorOptions, Shading};
use plt::collections::CollectionProps;
use plt::colors::ColorizerOptions;
use plt::image::{Interpolation, ScalarField};
use plt::layout::LayoutEngine;
use plt::prelude::*;
use plt::rc::Aspect;

use super::data::Samples;
use super::{Built, Entry, background};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry::figure("image_colorbar", "An image with a colorbar", image_colorbar),
    Entry::figure(
        "imshow_interpolation",
        "One small image, six interpolations",
        imshow_interpolation,
    ),
    Entry::figure("imshow_mask", "An image with masked cells", imshow_mask),
    Entry::figure(
        "pcolormesh_irregular",
        "pcolormesh on a curvilinear grid",
        pcolormesh_irregular,
    ),
];

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

/// An 8 × 8 grid enlarged to fill each panel: the interpolation decides
/// how the pixels between the data points are coloured.
fn imshow_interpolation() -> Built {
    let fig = Figure::with_size(7.5, 5.0, 100.0);
    let axs = fig.subplots(2, 3)?;
    let mut samples = Samples::new(41);
    let z = Grid::from_fn(8, 8, |i, j| {
        let (x, y) = (i as f64 / 7.0 - 0.5, j as f64 / 7.0 - 0.5);
        (-(x * x + y * y) * 6.0).exp() + 0.25 * samples.uniform()
    });
    let methods = [
        Interpolation::Nearest,
        Interpolation::Bilinear,
        Interpolation::Bicubic,
        Interpolation::Spline36,
        Interpolation::Gaussian,
        Interpolation::Lanczos,
    ];
    for (ax, method) in axs.as_slice().iter().zip(methods) {
        let options = ImshowOptions {
            interpolation: Some(method),
            colorizer: ColorizerOptions::default().cmap(colormaps::get("cividis")?),
            ..ImshowOptions::default()
        };
        ax.imshow(z.clone(), options)?;
        ax.set_title(method.name());
        ax.set_axis_off();
    }
    Ok(fig)
}

/// A height map with its sea masked: masked cells take the colormap's
/// "bad" colour (here a light grey) instead of a colour of the data.
fn imshow_mask() -> Built {
    let fig = Figure::with_size(6.0, 4.0, 100.0);
    // The compressed layout fits the colorbar to an axes of fixed aspect.
    fig.set_layout_engine(LayoutEngine::compressed())?;
    let ax = fig.subplot();
    let (rows, cols) = (150, 200);
    let height = Grid::from_fn(rows, cols, |i, j| {
        let (x, y) = (j as f64 / 40.0 - 2.5, i as f64 / 40.0 - 1.9);
        1.5 * (-(x * x + y * y) / 2.0).exp() + 0.4 * (3.0 * x).sin() * (2.0 * y).cos() - 0.6
    });
    let sea = height.map(|h| *h < 0.0);
    let options = ImshowOptions {
        colorizer: ColorizerOptions::default()
            .cmap(colormaps::get("terrain")?.with_bad("0.85")?)
            .limits(Some(-0.5), Some(1.5)),
        ..ImshowOptions::default()
    };
    let image = ax.imshow(ScalarField::masked(height, &sea)?, options)?;
    fig.colorbar(
        &image,
        ColorbarOptions {
            label: "height (km)".into(),
            ..ColorbarOptions::default()
        },
    )?;
    ax.set_title("Land, with the sea masked");
    Ok(fig)
}

/// A pseudocolour mesh whose cells are not rectangles: the grid points
/// are given as full x and y grids, here a ring sector bent like a fan.
fn pcolormesh_irregular() -> Built {
    let fig = Figure::with_size(6.0, 4.0, 100.0);
    fig.set_layout_engine(LayoutEngine::compressed())?;
    let ax = fig.subplot();
    let (radii, angles) = (16, 40);
    let r = |i: usize| 1.0 + i as f64 / 8.0;
    let t = |j: usize| 0.15 + 2.8 * j as f64 / (angles - 1) as f64;
    let x = Grid::from_fn(radii + 1, angles + 1, |i, j| r(i) * t(j).cos());
    let y = Grid::from_fn(radii + 1, angles + 1, |i, j| {
        r(i) * t(j).sin() * (1.0 + 0.1 * r(i))
    });
    let c = Grid::from_fn(radii, angles, |i, j| {
        (j as f64 / 5.0).sin() * (i as f64 / 4.0).cos()
    });
    let options = PcolorOptions {
        shading: Some(Shading::Flat),
        colorizer: ColorizerOptions::default().cmap(colormaps::get("RdYlBu_r")?),
        props: CollectionProps::default()
            .edgecolor(background())
            .linewidth(0.3),
        ..PcolorOptions::default()
    };
    let mesh = ax.pcolormesh(Mesh::from_coords(x, y, c), options)?;
    fig.colorbar(&mesh, ColorbarOptions::default())?;
    ax.set_aspect(Aspect::Equal)?;
    ax.set_title("A curvilinear mesh");
    Ok(fig)
}
