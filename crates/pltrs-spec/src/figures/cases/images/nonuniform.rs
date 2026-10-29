//! The `NonUniformImage` cases.

use pltrs::axes::{Axes, NonUniformOptions};
use pltrs::colors::{Colorize, ColorizerOptions, colormaps};
use pltrs::figure::Figure;
use pltrs::image::{ImageData, NonUniformInterpolation};
use pltrs::numeric::{Grid, linspace};

use super::data::rgb_float;
use crate::figures::CaseResult;

/// `_cubic_grid()`: centres crowded near 0 along x, even along y, and
/// their radius.
fn cubic_grid() -> (Vec<f64>, Vec<f64>, Grid<f64>) {
    let x: Vec<f64> = linspace(-4.0, 4.0, 9)
        .into_iter()
        .map(|v| v.powi(3) / 16.0)
        .collect();
    let y = linspace(-4.0, 4.0, 7);
    let radius = Grid::from_fn(y.len(), x.len(), |r, c| (x[c] * x[c] + y[r] * y[r]).sqrt());
    (x, y, radius)
}

/// `_nonuniform(ax, interpolation, *_cubic_grid(), cmap=cmap)`.
fn cubic(ax: &Axes, interpolation: NonUniformInterpolation, cmap: &str) -> CaseResult {
    let (x, y, radius) = cubic_grid();
    let options = NonUniformOptions {
        colorizer: ColorizerOptions::default().cmap(colormaps::get(cmap)?),
        interpolation,
    };
    ax.nonuniform_image(&x, &y, radius, options)?;
    Ok(())
}

pub(super) fn nearest(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    cubic(&ax, NonUniformInterpolation::Nearest, "viridis")?;
    ax.set_xlim(-5.0, 5.0)?.set_ylim(-5.0, 5.0)?;
    Ok(())
}

pub(super) fn bilinear_relim(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    cubic(&ax, NonUniformInterpolation::Bilinear, "magma")?;
    ax.relim(false).autoscale_view(None);
    Ok(())
}

pub(super) fn rgb_log(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let options = NonUniformOptions {
        interpolation: NonUniformInterpolation::Bilinear,
        ..NonUniformOptions::default()
    };
    ax.nonuniform_image(
        &[1.0, 3.0, 10.0, 30.0, 100.0],
        &[0.0, 1.0, 3.0, 3.5, 6.0],
        ImageData::from(rgb_float(5)),
        options,
    )?;
    ax.set_xscale("log")?;
    ax.set_xlim(0.5, 200.0)?.set_ylim(-1.0, 7.0)?;
    Ok(())
}
