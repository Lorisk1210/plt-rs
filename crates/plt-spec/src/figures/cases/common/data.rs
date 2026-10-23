//! The numbers several case groups draw: formulas, synthetic fields and
//! sample-data stand-ins, smoothing, and grids.

use std::f64::consts::PI;

use plt::numeric::{Grid, arange, linspace, linspace_with, meshgrid};
use plt::transforms::Point;
use plt::tri::{TriError, Triangulation};
use plt_core::{Path, PathCode, PathError};

use crate::rng::SpecRng;

/// `f` applied to each value.
pub(crate) fn map(values: &[f64], f: impl Fn(f64) -> f64) -> Vec<f64> {
    values.iter().copied().map(f).collect()
}

/// The surface many gallery examples sample (`_func3` of the image
/// demos): `(1 - x/2 + x⁵ + y³) e^{-(x² + y²)}`.
pub(crate) fn func3(x: f64, y: f64) -> f64 {
    (1.0 - x / 2.0 + x.powf(5.0) + y.powf(3.0)) * (-(x * x + y * y)).exp()
}

/// Running sums of `values` (`np.cumsum`).
pub(crate) fn cumsum(values: &[f64]) -> Vec<f64> {
    values
        .iter()
        .scan(0.0, |sum, v| {
            *sum += v;
            Some(*sum)
        })
        .collect()
}

/// The raised sine of matplotlib's `markevery` tests and demo: 200 points
/// of `sin(x) + 1.11` for `x` in `[0.11, 9.89]` (`_markevery_sine`).
pub(crate) fn markevery_sine() -> (Vec<f64>, Vec<f64>) {
    let delta = 0.11;
    let x = map(&linspace(0.0, 10.0 - 2.0 * delta, 200), |x| x + delta);
    let y = map(&x, |x| x.sin() + 1.0 + delta);
    (x, y)
}

/// The curve of the gallery's spine examples: `2 sin x` at 100 points of
/// `[0, 2π]`.
pub(crate) fn double_sine() -> (Vec<f64>, Vec<f64>) {
    let x = linspace(0.0, 2.0 * PI, 100);
    let y = map(&x, |x| 2.0 * x.sin());
    (x, y)
}

/// The seed of the gallery examples' `np.random.seed(19680801)`.
pub(crate) const GALLERY_SEED: u64 = 19_680_801;

/// The gallery's favourite field: a positive and a negative bump
/// (`spec_support.gallery_data.two_bumps`).
pub(crate) fn two_bumps(x: f64, y: f64) -> f64 {
    let z1 = (-(x * x) - y * y).exp();
    let z2 = (-((x - 1.0) * (x - 1.0)) - (y - 1.0) * (y - 1.0)).exp();
    (z1 - z2) * 2.0
}

/// `two_bumps` on the grid of `x` and `y`, with the grid (`np.meshgrid`).
fn bumps_on(x: &[f64], y: &[f64]) -> (Grid<f64>, Grid<f64>, Grid<f64>) {
    let (xx, yy) = meshgrid(x, y);
    let z = Grid::from_fn(xx.rows(), xx.cols(), |r, c| {
        two_bumps(xx[(r, c)], yy[(r, c)])
    });
    (xx, yy, z)
}

/// `contour_demo`'s field: `two_bumps` over `[-3, 3) × [-2, 2)` in steps
/// of 0.025, with its grid (`contour_field`).
pub(crate) fn contour_field() -> (Grid<f64>, Grid<f64>, Grid<f64>) {
    bumps_on(&arange(-3.0, 3.0, 0.025), &arange(-2.0, 2.0, 0.025))
}

/// `image_demo`'s field: `two_bumps` over `[-3, 3)²` in steps of 0.025
/// (`image_field`).
pub(crate) fn image_field() -> Grid<f64> {
    let x = arange(-3.0, 3.0, 0.025);
    bumps_on(&x, &x).2
}

/// A 256 × 256 MRI-like slice in place of `s1045.ima.gz` (`scan`): a
/// bright ellipse with darker rings inside, values up to 1000.
pub(crate) fn scan() -> Grid<f64> {
    let v = linspace(-1.0, 1.0, 256);
    Grid::from_fn(256, 256, |r, c| {
        let (x, y) = (v[c], v[r]);
        let (ex, ey) = (x / 0.75, y / 0.9);
        let head = (-(ex * ex + ey * ey).powf(4.0)).exp();
        let folds = 0.5 + 0.5 * (14.0 * (x - 0.1).hypot(y + 0.05)).cos();
        1000.0 * head * (0.6 + 0.4 * folds)
    })
}

/// A rough 344 × 403 elevation model in place of
/// `jacksboro_fault_dem.npz` (`terrain`): ridges with noise on top.
pub(crate) fn terrain() -> Grid<f64> {
    let noise = SpecRng::new(GALLERY_SEED).normal(344 * 403, 0.0, 12.0);
    Grid::from_fn(344, 403, |y, x| {
        let (xf, yf) = (x as f64, y as f64);
        let ridges = 600.0 + 250.0 * (xf / 37.0).sin() * (yf / 53.0).cos() + 0.5 * (xf - yf);
        ridges + noise[y * 403 + x]
    })
}

/// `np.mgrid[start:stop:n*1j]` along one axis: like `linspace`, but the
/// last value is computed rather than pinned to `stop`.
pub(crate) fn mgrid_count(start: f64, stop: f64, n: usize) -> Vec<f64> {
    let step = (stop - start) / (n - 1) as f64;
    (0..n).map(|i| i as f64 * step + start).collect()
}

/// numpy's `hanning(len)`, normalised to sum to 1.
fn hanning_weights(len: usize) -> Vec<f64> {
    let span = (len - 1) as f64;
    let window: Vec<f64> = (0..len)
        .map(|k| 0.5 + 0.5 * (PI * (2.0 * k as f64 - span) / span).cos())
        .collect();
    let total: f64 = window.iter().sum();
    window.iter().map(|w| w / total).collect()
}

/// `smooth1d` of matplotlib's agg filter test and example: a moving
/// average weighted by a Hanning window of `weights.len()` samples, the
/// ends extended by point reflection.
fn smooth1d(x: &[f64], weights: &[f64]) -> Vec<f64> {
    let len = weights.len();
    let (first, last) = (x[0], x[x.len() - 1]);
    let reflected: Vec<f64> = (2..=len)
        .rev()
        .map(|i| 2.0 * first - x[i])
        .chain(x.iter().copied())
        .chain((0..len - 1).map(|i| 2.0 * last - x[x.len() - 1 - i]))
        .collect();
    let start = len - 1 - (len - 1) / 2;
    (0..x.len())
        .map(|i| {
            let window = &reflected[start + i..start + i + len];
            window.iter().zip(weights).map(|(s, w)| s * w).sum()
        })
        .collect()
}

/// `smooth2d` of matplotlib's agg filter test and example: [`smooth1d`]
/// down every column of the `width`-wide row-major `values`, then along
/// every row.
pub(crate) fn smooth2d(values: &[f64], width: usize, sigma: f64) -> Vec<f64> {
    let weights = hanning_weights((sigma as usize).max(3) * 2 + 1);
    let height = values.len() / width;
    let mut smoothed = values.to_vec();
    for col in 0..width {
        let column: Vec<f64> = (0..height).map(|row| values[row * width + col]).collect();
        for (row, value) in smooth1d(&column, &weights).into_iter().enumerate() {
            smoothed[row * width + col] = value;
        }
    }
    smoothed
        .chunks(width)
        .flat_map(|row| smooth1d(row, &weights))
        .collect()
}

/// `cbook.get_sample_data("axes_grid/bivariate_normal.npy")`: ten times
/// the difference of two bivariate normal densities, the standard one and
/// one centred at `(1, 1)` with deviations 1.5 and 0.5, on the 15 × 15
/// grid `x = -3, -2.5, ..., 4`, `y = -4, -3.5, ..., 3`.
pub(crate) fn bivariate_normal() -> Grid<f64> {
    let density = |x: f64, y: f64, (sigma_x, sigma_y): (f64, f64), (mu_x, mu_y): (f64, f64)| {
        let z = (x - mu_x).powi(2) / sigma_x.powi(2) + (y - mu_y).powi(2) / sigma_y.powi(2);
        (-z / 2.0).exp() / (2.0 * std::f64::consts::PI * sigma_x * sigma_y)
    };
    Grid::from_fn(15, 15, |r, c| {
        let (x, y) = (-3.0 + 0.5 * c as f64, -4.0 + 0.5 * r as f64);
        (density(x, y, (1.0, 1.0), (0.0, 0.0)) - density(x, y, (1.5, 0.5), (1.0, 1.0))) * 10.0
    })
}

/// [`bivariate_normal`] framed by zeros, as the zoom examples show it: a
/// 150 × 150 grid holding the sample data from row and column 30 on
/// (`Z2 = np.zeros((150, 150)); Z2[30:30+ny, 30:30+nx] = Z`).
pub(crate) fn framed_bivariate_normal() -> Grid<f64> {
    let z = bivariate_normal();
    Grid::from_fn(150, 150, |r, c| {
        r.checked_sub(30)
            .zip(c.checked_sub(30))
            .and_then(|(r, c)| z.get(r, c).copied())
            .unwrap_or(0.0)
    })
}

/// A wide bump with a narrow spike fifty times higher on top (`_spike`).
pub(crate) fn spike(x: f64, y: f64) -> f64 {
    let z1 = (-(x * x) - y * y).exp();
    let (x10, y10) = (x * 10.0, y * 10.0);
    let z2 = (-(x10 * x10) - y10 * y10).exp();
    z1 + 50.0 * z2
}

/// `f` evaluated cell by cell on two grids of the same shape.
pub(crate) fn on_mesh(xx: &Grid<f64>, yy: &Grid<f64>, f: impl Fn(f64, f64) -> f64) -> Grid<f64> {
    Grid::from_fn(xx.rows(), xx.cols(), |r, c| f(xx[(r, c)], yy[(r, c)]))
}

/// The smallest and largest values of `values`.
pub(crate) fn extremes(values: &Grid<f64>) -> (f64, f64) {
    values
        .as_slice()
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
            (lo.min(v), hi.max(v))
        })
}

/// `np.meshgrid(x, y)` and `f` evaluated on it.
pub(crate) fn mesh_field(
    x: &[f64],
    y: &[f64],
    f: impl Fn(f64, f64) -> f64,
) -> (Grid<f64>, Grid<f64>, Grid<f64>) {
    let (xx, yy) = meshgrid(x, y);
    let z = on_mesh(&xx, &yy, f);
    (xx, yy, z)
}

/// `np.mgrid[rows, cols]`: the row coordinate down the rows, the column
/// coordinate along them.
pub(crate) fn mgrid(rows: &[f64], cols: &[f64]) -> (Grid<f64>, Grid<f64>) {
    let r = Grid::from_fn(rows.len(), cols.len(), |i, _| rows[i]);
    let c = Grid::from_fn(rows.len(), cols.len(), |_, j| cols[j]);
    (r, c)
}

/// `np.mgrid[x0:x1:100j, y0:y1:100j]`: x down the rows, y along them.
pub(crate) fn mgrid_100(x0: f64, x1: f64, y0: f64, y1: f64) -> (Grid<f64>, Grid<f64>) {
    mgrid(&mgrid_count(x0, x1, 100), &mgrid_count(y0, y1, 100))
}

/// The Mercator projection of latitude `a` in degrees, in degrees (the
/// forward function of matplotlib's function scale examples).
pub(crate) fn mercator(a: f64) -> f64 {
    let a = a.to_radians();
    (a.tan() + 1.0 / a.cos()).abs().ln().to_degrees()
}

/// The latitude in degrees whose Mercator projection is `a` (the inverse
/// of [`mercator`]).
pub(crate) fn inverse_mercator(a: f64) -> f64 {
    a.to_radians().sinh().atan().to_degrees()
}

/// Points on staggered rings between a minimum radius and 0.95
/// (`_annulus`), in cartesian and polar coordinates.
pub(crate) struct Annulus {
    /// x of every point, ring by ring within each angle.
    pub x: Vec<f64>,
    /// y of every point.
    pub y: Vec<f64>,
    /// `(radius, angle)` of every point.
    pub polar: Vec<(f64, f64)>,
}

impl Annulus {
    /// `n_radii` rings from `min_radius` out, each of `n_angles` points,
    /// every other ring turned by half a step.
    pub(crate) fn new(n_angles: usize, n_radii: usize, min_radius: f64) -> Self {
        let radii = linspace(min_radius, 0.95, n_radii);
        let angles = linspace_with(0.0, 2.0 * PI, n_angles, false);
        let polar: Vec<(f64, f64)> = angles
            .iter()
            .flat_map(|&angle| {
                radii.iter().enumerate().map(move |(j, &radius)| {
                    let staggered = if j % 2 == 1 {
                        angle + PI / n_angles as f64
                    } else {
                        angle
                    };
                    (radius, staggered)
                })
            })
            .collect();
        Self {
            x: polar.iter().map(|&(r, a)| r * a.cos()).collect(),
            y: polar.iter().map(|&(r, a)| r * a.sin()).collect(),
            polar,
        }
    }

    /// `(np.cos(radii) * np.cos(3 * angles)).flatten()`.
    pub(crate) fn waves(&self) -> Vec<f64> {
        self.polar
            .iter()
            .map(|&(r, a)| r.cos() * (3.0 * a).cos())
            .collect()
    }

    /// The Delaunay triangulation of the points without the triangles
    /// whose centroid is inside `min_radius` (`_masked_delaunay`).
    pub(crate) fn masked_delaunay(&self, min_radius: f64) -> Result<Triangulation, TriError> {
        masked_delaunay(&self.x, &self.y, min_radius)
    }
}

/// The Delaunay triangulation of `x, y` without the triangles whose
/// centroid is inside `min_radius`.
fn masked_delaunay(x: &[f64], y: &[f64], min_radius: f64) -> Result<Triangulation, TriError> {
    let tri = Triangulation::delaunay(x, y)?;
    let mask = tri
        .triangles()
        .iter()
        .map(|t| {
            let mean = |v: &[f64]| (v[t[0]] + v[t[1]] + v[t[2]]) / 3.0;
            mean(x).hypot(mean(y)) < min_radius
        })
        .collect();
    tri.with_mask(Some(mask))
}

/// The closed curve of the gallery's `path_patch` and `path_editor`
/// examples: a cubic Bézier arc, a line and a second arc back to the
/// start.
pub(crate) fn path_patch_curve() -> Result<Path, PathError> {
    use PathCode::{ClosePoly, Curve4, LineTo, MoveTo};
    let vertices = [
        (1.58, -2.57),
        (0.35, -1.1),
        (-1.75, 2.0),
        (0.375, 2.0),
        (0.85, 1.15),
        (2.2, 3.2),
        (3.0, 0.05),
        (2.0, -0.5),
        (1.58, -2.57),
    ];
    let codes = vec![
        MoveTo, Curve4, Curve4, Curve4, LineTo, Curve4, Curve4, Curve4, ClosePoly,
    ];
    Path::new(vertices.into_iter().map(Point::from).collect(), codes)
}
