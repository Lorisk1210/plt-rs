//! The deterministic fields of `figures/cases_colorbar.py`.

use pltrs::contour::{QuadCoords, QuadField};
use pltrs::numeric::{Grid, linspace, meshgrid};

/// The field's formula.
fn wave(x: f64, y: f64) -> f64 {
    x.sin() * (y * 0.8).cos() * (-(x * x + y * y) / 12.0).exp()
}

/// `_field(n)`: the wave on an `n × n` grid over `[-3, 3]²`.
pub(in crate::figures::cases) fn field(n: usize) -> Grid<f64> {
    let x = linspace(-3.0, 3.0, n);
    Grid::from_fn(n, n, |r, c| wave(x[c], x[r]))
}

/// `_xyz()`: the 40 × 40 wave with its coordinates.
pub(in crate::figures::cases) fn xyz() -> QuadField {
    let x = linspace(-3.0, 3.0, 40);
    let (xx, yy) = meshgrid(&x, &x);
    let z = Grid::from_fn(40, 40, |r, c| wave(xx[(r, c)], yy[(r, c)]));
    QuadField::new(z).with_coords(QuadCoords::Mesh { x: xx, y: yy })
}
