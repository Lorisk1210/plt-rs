//! The deterministic fields of `figures/cases_colorbar.py`.

use plt::axes::ImshowOptions;
use plt::colors::colormaps;
use plt::contour::{QuadCoords, QuadField};
use plt::image::ColorizerOptions;
use plt::numeric::{Grid, linspace, meshgrid};

/// The field's formula.
fn wave(x: f64, y: f64) -> f64 {
    x.sin() * (y * 0.8).cos() * (-(x * x + y * y) / 12.0).exp()
}

/// `_field(n)`: the wave on an `n × n` grid over `[-3, 3]²`.
pub(super) fn field(n: usize) -> Grid<f64> {
    let x = linspace(-3.0, 3.0, n);
    Grid::from_fn(n, n, |r, c| wave(x[c], x[r]))
}

/// `_xyz()`: the 40 × 40 wave with its coordinates.
pub(super) fn xyz() -> QuadField {
    let x = linspace(-3.0, 3.0, 40);
    let (xx, yy) = meshgrid(&x, &x);
    let z = Grid::from_fn(40, 40, |r, c| wave(xx[(r, c)], yy[(r, c)]));
    QuadField::new(z).with_coords(QuadCoords::Mesh { x: xx, y: yy })
}

/// `imshow` options with only the colouring set.
pub(super) fn colored(colors: ColorizerOptions) -> ImshowOptions {
    ImshowOptions {
        colors,
        ..ImshowOptions::default()
    }
}

/// `imshow` options with a named colormap.
pub(super) fn cmap(name: &str) -> Result<ImshowOptions, plt::colors::ColormapError> {
    Ok(colored(
        ColorizerOptions::default().cmap(colormaps::get(name)?),
    ))
}
