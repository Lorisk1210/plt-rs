//! Contours.

use std::num::NonZeroUsize;

use pltrs::colors::{ColorizerOptions, Extend, colormaps};
use pltrs::contour::{ContourOptions, Levels};
use pltrs::figure::Figure;
use pltrs::mplot3d::{Axes3D, Axes3DError, Axes3DOptions, Axis3, Contour3DOptions, test_data};
use pltrs::numeric::{Grid, arange, meshgrid};
use pltrs::tri::Triangulation;

use super::remove_ticks_and_titles;
use crate::figures::CaseResult;
use crate::rng::SpecRng;

/// `cmap="coolwarm"` with `zdir` and `offset`.
fn coolwarm(zdir: Axis3, offset: f64) -> Result<Contour3DOptions, pltrs::colors::ColormapError> {
    Ok(Contour3DOptions {
        zdir,
        offset: Some(offset),
        contour: ContourOptions {
            colorizer: ColorizerOptions {
                cmap: Some(colormaps::get("coolwarm")?),
                ..ColorizerOptions::default()
            },
            ..ContourOptions::default()
        },
        ..Contour3DOptions::default()
    })
}

/// The three projections of `contour3d` and `contourf3d`: onto the
/// planes `z = -100`, `x = -40` and `y = 40`.
const PROJECTIONS: [(Axis3, f64); 3] = [(Axis3::Z, -100.0), (Axis3::X, -40.0), (Axis3::Y, 40.0)];

/// Limits along x, y and z.
fn set_limits(ax: &Axes3D, [x, y, z]: [(f64, f64); 3]) -> Result<(), Axes3DError> {
    ax.set_xlim(x.0, x.1)?
        .set_ylim(y.0, y.1)?
        .set_zlim(z.0, z.1)?;
    Ok(())
}

pub(super) fn contour3d(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let (x, y, z) = test_data(0.05);
    for (zdir, offset) in PROJECTIONS {
        ax.contour(&x, &y, &z, coolwarm(zdir, offset)?)?;
    }
    set_limits(&ax, [(-40.0, 40.0), (-40.0, 40.0), (-100.0, 100.0)])?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn contour3d_extend3d(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let (x, y, z) = test_data(0.05);
    let options = Contour3DOptions {
        extend3d: NonZeroUsize::new(5),
        ..coolwarm(Axis3::Z, -100.0)?
    };
    ax.contour(&x, &y, &z, options)?;
    set_limits(&ax, [(-30.0, 30.0), (-20.0, 40.0), (-80.0, 80.0)])?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn contourf3d(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let (x, y, z) = test_data(0.05);
    for (zdir, offset) in PROJECTIONS {
        ax.contourf(&x, &y, &z, coolwarm(zdir, offset)?)?;
    }
    set_limits(&ax, [(-40.0, 40.0), (-40.0, 40.0), (-100.0, 100.0)])?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `np.meshgrid(np.arange(-2, 2, 0.25), np.arange(-2, 2, 0.25))`.
fn quarter_grid() -> (Grid<f64>, Grid<f64>) {
    let axis = arange(-2.0, 2.0, 0.25);
    meshgrid(&axis, &axis)
}

pub(super) fn contourf3d_fill(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let (x, y) = quarter_grid();
    // Zero, but 0.1 at every fifth row and column: holes in the z = 0
    // surface.
    let z = Grid::from_fn(x.rows(), x.cols(), |i, j| {
        if i % 5 == 0 && j % 5 == 0 { 0.1 } else { 0.0 }
    });
    let mut options = coolwarm(Axis3::Z, 0.0)?;
    options.contour.levels = Levels::Values(vec![-0.1, 0.0]);
    ax.contourf(&x, &y, &z, options)?;
    set_limits(&ax, [(-2.0, 2.0), (-2.0, 2.0), (-1.0, 1.0)])?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_contourf3d_extend`: filled contours of a paraboloid at `levels`
/// with `extend` and a norm narrower than the levels.
fn contourf3d_extend(fig: &Figure, extend: Extend, levels: &[f64]) -> CaseResult {
    let (x, y) = quarter_grid();
    let z = Grid::from_fn(x.rows(), x.cols(), |i, j| {
        x[(i, j)].powi(2) + y[(i, j)].powi(2)
    });
    let ax = fig.subplot_3d();
    let options = Contour3DOptions {
        contour: ContourOptions {
            colorizer: ColorizerOptions {
                vmin: Some(1.0),
                vmax: Some(7.0),
                ..ColorizerOptions::default()
            },
            levels: Levels::Values(levels.to_vec()),
            extend,
            ..ContourOptions::default()
        },
        ..Contour3DOptions::default()
    };
    ax.contourf(&x, &y, &z, options)?;
    set_limits(&ax, [(-2.0, 2.0), (-2.0, 2.0), (-10.0, 10.0)])?;
    Ok(())
}

pub(super) fn contourf3d_extend_both(fig: &Figure) -> CaseResult {
    contourf3d_extend(fig, Extend::Both, &[2.0, 4.0, 6.0])
}

pub(super) fn contourf3d_extend_min(fig: &Figure) -> CaseResult {
    contourf3d_extend(fig, Extend::Min, &[2.0, 4.0, 6.0, 8.0])
}

pub(super) fn contourf3d_extend_max(fig: &Figure) -> CaseResult {
    contourf3d_extend(fig, Extend::Max, &[0.0, 2.0, 4.0, 6.0])
}

pub(super) fn tricontour(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(19_680_801);
    let x: Vec<f64> = rng
        .uniform(1000, 0.0, 1.0)
        .iter()
        .map(|v| v - 0.5)
        .collect();
    let y: Vec<f64> = rng
        .uniform(1000, 0.0, 1.0)
        .iter()
        .map(|v| v - 0.5)
        .collect();
    let z: Vec<f64> = x.iter().zip(&y).map(|(x, y)| -(x * x + y * y)).collect();
    let tri = Triangulation::delaunay(&x, &y)?;
    fig.add_subplot_3d(1, 2, 1, Axes3DOptions::default())?
        .tricontour(&tri, &z, Contour3DOptions::default())?;
    fig.add_subplot_3d(1, 2, 2, Axes3DOptions::default())?
        .tricontourf(&tri, &z, Contour3DOptions::default())?;
    remove_ticks_and_titles(fig);
    Ok(())
}
