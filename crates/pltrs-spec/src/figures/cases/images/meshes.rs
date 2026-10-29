//! The `pcolor`, `pcolormesh`, `pcolorfast` and `hexbin` cases.

use std::f64::consts::FRAC_PI_2;

use pltrs::axes::{
    HexBins, HexGridSize, HexReduce, HexbinOptions, Mesh, MeshCoords, PcolorOptions,
    PcolorfastOptions, Shading,
};
use pltrs::collections::{CollectionProps, EdgeColors};
use pltrs::colors::{Colorize, ColorizerOptions, colormaps};
use pltrs::figure::Figure;
use pltrs::image::{Extent, ScalarField};
use pltrs::numeric::{Grid, linspace, logspace, meshgrid};
use pltrs::rc::Aspect;

use super::data::{bump, grid, zip_with};
use crate::figures::CaseResult;
use crate::figures::cases::common::{arange_grid, trimmed};
use crate::rng::SpecRng;

/// Options with only the colormap set.
fn cmap(name: &str) -> Result<ColorizerOptions, Box<dyn std::error::Error>> {
    Ok(ColorizerOptions::default().cmap(colormaps::get(name)?))
}

pub(super) fn pcolor_default(fig: &Figure) -> CaseResult {
    let values = SpecRng::new(3).uniform(48, 0.0, 1.0);
    fig.subplot().pcolor(
        Mesh::new(Grid::from_vec(6, 8, values)?),
        PcolorOptions::default(),
    )?;
    Ok(())
}

pub(super) fn pcolor_edges_masked(fig: &Figure) -> CaseResult {
    let (xx, yy) = grid(9, 7);
    let c = zip_with(&trimmed(&xx), &trimmed(&yy), |x, y| x.sin() * y.cos());
    let mask = c.map(|&v| v > 0.7);
    let options = PcolorOptions {
        colorizer: cmap("PuOr")?,
        props: CollectionProps {
            edgecolors: Some(EdgeColors::Colors(vec!["k".into()])),
            linewidths: Some(vec![1.5]),
            ..CollectionProps::default()
        },
        ..PcolorOptions::default()
    };
    fig.subplot().pcolor(
        Mesh::from_coords(xx, yy, ScalarField::masked(c, &mask)?),
        options,
    )?;
    Ok(())
}

/// `pcolormesh(xx, yy, values, shading=shading)` on `_grid(nx, ny)`.
fn mesh_on_grid(
    fig: &Figure,
    (nx, ny): (usize, usize),
    shading: Shading,
    values: impl Fn(&Grid<f64>, &Grid<f64>) -> Grid<f64>,
) -> CaseResult {
    let (xx, yy) = grid(nx, ny);
    let c = values(&xx, &yy);
    let options = PcolorOptions {
        shading: Some(shading),
        ..PcolorOptions::default()
    };
    fig.subplot()
        .pcolormesh(Mesh::from_coords(xx, yy, c), options)?;
    Ok(())
}

pub(super) fn pcolormesh_flat(fig: &Figure) -> CaseResult {
    mesh_on_grid(fig, (13, 9), Shading::Flat, |xx, yy| {
        trimmed(&zip_with(xx, yy, |x, y| x * y))
    })
}

pub(super) fn pcolormesh_nearest(fig: &Figure) -> CaseResult {
    mesh_on_grid(fig, (12, 8), Shading::Nearest, |xx, yy| {
        zip_with(xx, yy, |x, y| x * y)
    })
}

pub(super) fn pcolormesh_gouraud(fig: &Figure) -> CaseResult {
    mesh_on_grid(fig, (6, 5), Shading::Gouraud, |xx, yy| {
        zip_with(xx, yy, |x, y| x.sin() + y.cos())
    })
}

pub(super) fn pcolormesh_auto_same_shape(fig: &Figure) -> CaseResult {
    let mesh = Mesh::from_coords([1.0, 2.0, 4.0, 8.0], [0.0, 1.0, 3.0], arange_grid(3, 4));
    fig.subplot().pcolormesh(mesh, PcolorOptions::default())?;
    Ok(())
}

pub(super) fn pcolormesh_nonuniform(fig: &Figure) -> CaseResult {
    let mesh = Mesh::from_coords(
        [0.0, 0.5, 1.0, 2.0, 4.0, 7.0, 11.0],
        [0.0, 1.0, 1.5, 3.5, 4.0],
        arange_grid(4, 6),
    );
    let options = PcolorOptions {
        colorizer: cmap("cividis")?,
        ..PcolorOptions::default()
    };
    fig.subplot().pcolormesh(mesh, options)?;
    Ok(())
}

pub(super) fn pcolormesh_curvilinear_edges(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let (r, t) = meshgrid(&linspace(1.0, 2.0, 6), &linspace(0.0, FRAC_PI_2, 10));
    let xx = zip_with(&r, &t, |r, t| r * t.cos());
    let yy = zip_with(&r, &t, |r, t| r * t.sin());
    let c = trimmed(&zip_with(&r, &t, |r, t| r * t));
    let options = PcolorOptions {
        props: CollectionProps {
            edgecolors: Some(EdgeColors::Colors(vec!["white".into()])),
            linewidths: Some(vec![0.8]),
            ..CollectionProps::default()
        },
        ..PcolorOptions::default()
    };
    ax.pcolormesh(Mesh::from_coords(xx, yy, c), options)?;
    ax.set_aspect(Aspect::Equal)?;
    Ok(())
}

pub(super) fn pcolormesh_alpha_log_axes(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    // np.geomspace: logspace with the endpoints pinned to the arguments.
    let mut x = logspace(0.0, 3.0, 8, 10.0);
    x[0] = 1.0;
    x[7] = 1000.0;
    let options = PcolorOptions {
        props: CollectionProps::default().alpha(0.6),
        ..PcolorOptions::default()
    };
    ax.pcolormesh(
        Mesh::from_coords(x, linspace(0.0, 1.0, 5), arange_grid(4, 7)),
        options,
    )?;
    ax.set_xscale("log")?;
    Ok(())
}

pub(super) fn pcolorfast_uniform(fig: &Figure) -> CaseResult {
    let xy = (MeshCoords::from([0.0, 10.0]), MeshCoords::from([0.0, 5.0]));
    fig.subplot()
        .pcolorfast(Some(xy), bump(12), PcolorfastOptions::default())?;
    Ok(())
}

pub(super) fn pcolorfast_nonuniform(fig: &Figure) -> CaseResult {
    let xy = (
        MeshCoords::from([0.0, 1.0, 3.0, 6.0, 10.0]),
        MeshCoords::from([0.0, 2.0, 3.0, 5.0]),
    );
    let options = PcolorfastOptions {
        colorizer: cmap("plasma")?,
        ..PcolorfastOptions::default()
    };
    fig.subplot()
        .pcolorfast(Some(xy), arange_grid(3, 4), options)?;
    Ok(())
}

pub(super) fn pcolorfast_quadmesh(fig: &Figure) -> CaseResult {
    let (xx, yy) = grid(7, 6);
    let xx = zip_with(&xx, &yy, |x, y| x + 0.3 * y);
    let c = trimmed(&zip_with(&xx, &yy, |x, y| x * y));
    fig.subplot().pcolorfast(
        Some((xx.into(), yy.into())),
        c,
        PcolorfastOptions::default(),
    )?;
    Ok(())
}

/// `_cloud(n, seed)`: correlated normal samples.
fn cloud(n: usize, seed: u64) -> (Vec<f64>, Vec<f64>) {
    let mut rng = SpecRng::new(seed);
    let x = rng.normal(n, 0.0, 1.0);
    let noise = rng.normal(n, 0.0, 0.8);
    let y = x.iter().zip(&noise).map(|(x, e)| 0.6 * x + e).collect();
    (x, y)
}

pub(super) fn hexbin_default(fig: &Figure) -> CaseResult {
    let (x, y) = cloud(3000, 4);
    fig.subplot().hexbin(&x, &y, HexbinOptions::default())?;
    Ok(())
}

pub(super) fn hexbin_gridsize_tuple(fig: &Figure) -> CaseResult {
    let (x, y) = cloud(2000, 4);
    let options = HexbinOptions {
        gridsize: HexGridSize::Shape(15, 6),
        colorizer: cmap("Blues")?,
        props: CollectionProps {
            edgecolors: Some(EdgeColors::Colors(vec!["k".into()])),
            linewidths: Some(vec![0.3]),
            ..CollectionProps::default()
        },
        ..HexbinOptions::default()
    };
    fig.subplot().hexbin(&x, &y, options)?;
    Ok(())
}

pub(super) fn hexbin_log(fig: &Figure) -> CaseResult {
    let (x, y) = cloud(3000, 4);
    let options = HexbinOptions {
        gridsize: HexGridSize::Count(20),
        bins: Some(HexBins::Log),
        colorizer: cmap("inferno")?,
        ..HexbinOptions::default()
    };
    fig.subplot().hexbin(&x, &y, options)?;
    Ok(())
}

pub(super) fn hexbin_mincnt_extent(fig: &Figure) -> CaseResult {
    let (x, y) = cloud(2000, 4);
    let options = HexbinOptions {
        gridsize: HexGridSize::Count(18),
        mincnt: Some(3),
        extent: Some(Extent::new(-4.0, 4.0, -4.0, 4.0)),
        ..HexbinOptions::default()
    };
    fig.subplot().hexbin(&x, &y, options)?;
    Ok(())
}

pub(super) fn hexbin_reduce_mean(fig: &Figure) -> CaseResult {
    let (x, y) = cloud(2000, 5);
    let c = x.iter().zip(&y).map(|(x, y)| x * y).collect();
    let options = HexbinOptions {
        values: Some(c),
        reduce: HexReduce::Mean,
        gridsize: HexGridSize::Count(15),
        colorizer: cmap("RdBu")?,
        ..HexbinOptions::default()
    };
    fig.subplot().hexbin(&x, &y, options)?;
    Ok(())
}
