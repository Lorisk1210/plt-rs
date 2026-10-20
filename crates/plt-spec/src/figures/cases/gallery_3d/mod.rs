//! `figures/cases_gallery_3d.py`: the gallery's "3D plotting" section, one
//! case per figure of each example, by topic.

mod bars;
mod contours;
mod gamma;
mod lines;
mod planes;
mod plots;
mod surfaces;
mod triangulations;
mod views;
mod voxels;

use std::f64::consts::PI;

use plt::mplot3d::{Axes3D, Sampling};
use plt::numeric::{Grid, arange, linspace, linspace_with, meshgrid};

use crate::figures::FigureCase;

/// The seed of most examples (`np.random.seed(19680801)`).
const SEED: u64 = 19_680_801;

/// Every grid point a polygon corner (`rstride=1, cstride=1`).
const EVERY_POINT: Sampling = Sampling::Strides { rows: 1, cols: 1 };

/// Whether contours are drawn as lines or filled.
#[derive(Clone, Copy)]
enum Fill {
    /// Line contours (`contour`).
    Lines,
    /// Filled contours (`contourf`).
    Filled,
}

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        bars::CASES,
        contours::CASES,
        lines::CASES,
        planes::CASES,
        plots::CASES,
        surfaces::CASES,
        triangulations::CASES,
        views::CASES,
        voxels::CASES,
    ]
    .concat()
}

/// `sin(sqrt(X**2 + Y**2))` on `np.arange(-5, 5, 0.25)` squared, the
/// surface of several examples.
fn sinc_surface() -> [Grid<f64>; 3] {
    let axis = arange(-5.0, 5.0, 0.25);
    let (x, y) = meshgrid(&axis, &axis);
    let z = Grid::from_fn(x.rows(), x.cols(), |i, j| {
        (x[(i, j)].powi(2) + y[(i, j)].powi(2)).sqrt().sin()
    });
    [x, y, z]
}

/// Labels the three axes.
fn label_xyz(ax: &Axes3D, [x, y, z]: [&str; 3]) {
    ax.set_xlabel(x).set_ylabel(y).set_zlabel(z);
}

/// The rings of the triangulation examples: 8 radii from 0.25 to 0.95,
/// `n_angles` points on each, every other ring turned by half a step,
/// and `cos(r) cos(3 angle)` over them (flattened angle by angle).
fn ring_points(n_angles: u32) -> [Vec<f64>; 3] {
    let radii = linspace(0.25, 0.95, 8);
    let angles = linspace_with(0.0, 2.0 * PI, n_angles as usize, false);
    let half_step = PI / f64::from(n_angles);
    let mut points = [Vec::new(), Vec::new(), Vec::new()];
    for &angle in &angles {
        for (ring, &radius) in radii.iter().enumerate() {
            let angle = if ring % 2 == 1 {
                angle + half_step
            } else {
                angle
            };
            points[0].push(radius * angle.cos());
            points[1].push(radius * angle.sin());
            points[2].push(radius.cos() * (3.0 * angle).cos());
        }
    }
    points
}
