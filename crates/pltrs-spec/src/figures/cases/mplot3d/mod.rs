//! `figures/cases_mplot3d.py` and `figures/cases_mplot3d_api.py`, split by
//! topic.

mod api;
mod api_plots;
mod axlim;
mod contours;
mod objects;
mod scatter;
mod surfaces;
mod view;

pub(crate) use api::CASES as API;
pub(crate) use api_plots::CASES as API_PLOTS;
pub(crate) use axlim::CASES as AXLIM;
pub(crate) use contours::CASES as CONTOURS;
pub(crate) use objects::CASES as OBJECTS;
pub(crate) use scatter::CASES as SCATTER;
pub(crate) use surfaces::CASES as SURFACES;
pub(crate) use view::CASES as VIEW;

use std::f64::consts::PI;

use pltrs::mplot3d::{Axes3D, Axes3DError};
use pltrs::numeric::{Grid, linspace, meshgrid};

/// `_surface(n)`: `sin(r) / (r + 1)` on an `n × n` grid over `[-3, 3]²`.
fn surface(n: usize) -> (Grid<f64>, Grid<f64>, Grid<f64>) {
    let x = linspace(-3.0, 3.0, n);
    let (xx, yy) = meshgrid(&x, &x);
    let zz = Grid::from_fn(n, n, |i, j| {
        let r = xx[(i, j)].hypot(yy[(i, j)]);
        r.sin() / (r + 1.0)
    });
    (xx, yy, zz)
}

/// `_helix()`: two turns of a helix rising from 0 to 1.
pub(super) fn helix() -> [Vec<f64>; 3] {
    let t = linspace(0.0, 4.0 * PI, 200);
    [
        t.iter().map(|t| t.cos()).collect(),
        t.iter().map(|t| t.sin()).collect(),
        t.iter().map(|t| t / (4.0 * PI)).collect(),
    ]
}

/// `ax.plot(*_helix())`.
fn plot_helix(ax: &Axes3D) -> Result<(), Axes3DError> {
    let [x, y, z] = helix();
    ax.plot(&x, &y, &z).map(drop)
}
