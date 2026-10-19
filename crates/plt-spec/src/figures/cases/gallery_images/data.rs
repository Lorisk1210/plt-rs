//! The data the gallery examples share, and the synthetic stand-ins for
//! their sample data files.

use std::f64::consts::PI;

use plt::numeric::{Grid, arange, linspace, linspace_with, meshgrid};
use plt::tri::{TriError, Triangulation};

use crate::rng::SpecRng;

/// The seed of the examples' `np.random.seed(19680801)`.
pub(super) const SEED: u64 = 19_680_801;

/// The gallery's favourite field: a positive and a negative bump
/// (`_two_bumps`).
pub(super) fn two_bumps(x: f64, y: f64) -> f64 {
    let z1 = (-(x * x) - y * y).exp();
    let z2 = (-((x - 1.0) * (x - 1.0)) - (y - 1.0) * (y - 1.0)).exp();
    (z1 - z2) * 2.0
}

/// A wide bump with a narrow spike fifty times higher on top (`_spike`).
pub(super) fn spike(x: f64, y: f64) -> f64 {
    let z1 = (-(x * x) - y * y).exp();
    let (x10, y10) = (x * 10.0, y * 10.0);
    let z2 = (-(x10 * x10) - y10 * y10).exp();
    z1 + 50.0 * z2
}

/// `_func3`: `(1 - x/2 + x⁵ + y³) e^{-(x² + y²)}`.
pub(super) fn func3(x: f64, y: f64) -> f64 {
    (1.0 - x / 2.0 + x.powf(5.0) + y.powf(3.0)) * (-(x * x + y * y)).exp()
}

/// `f` evaluated cell by cell on two grids of the same shape.
pub(super) fn on_mesh(xx: &Grid<f64>, yy: &Grid<f64>, f: impl Fn(f64, f64) -> f64) -> Grid<f64> {
    Grid::from_fn(xx.rows(), xx.cols(), |r, c| f(xx[(r, c)], yy[(r, c)]))
}

/// The smallest and largest values of `values`.
pub(super) fn extremes(values: &Grid<f64>) -> (f64, f64) {
    values
        .as_slice()
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
            (lo.min(v), hi.max(v))
        })
}

/// `np.meshgrid(x, y)` and `f` evaluated on it.
pub(super) fn mesh_field(
    x: &[f64],
    y: &[f64],
    f: impl Fn(f64, f64) -> f64,
) -> (Grid<f64>, Grid<f64>, Grid<f64>) {
    let (xx, yy) = meshgrid(x, y);
    let z = on_mesh(&xx, &yy, f);
    (xx, yy, z)
}

/// `_contour_field()`: `two_bumps` over `[-3, 3) × [-2, 2)` in steps of
/// 0.025.
pub(super) fn contour_field() -> (Grid<f64>, Grid<f64>, Grid<f64>) {
    mesh_field(
        &arange(-3.0, 3.0, 0.025),
        &arange(-2.0, 2.0, 0.025),
        two_bumps,
    )
}

/// `np.mgrid[start:stop:step]` along one axis: `i * step + start`.
pub(super) fn mgrid_step(start: f64, stop: f64, step: f64) -> Vec<f64> {
    let count = ((stop - start) / step).ceil() as usize;
    (0..count).map(|i| i as f64 * step + start).collect()
}

/// `np.mgrid[start:stop:n*1j]` along one axis: like `linspace`, but the
/// last value is computed rather than pinned to `stop`.
pub(super) fn mgrid_count(start: f64, stop: f64, n: usize) -> Vec<f64> {
    let step = (stop - start) / (n - 1) as f64;
    (0..n).map(|i| i as f64 * step + start).collect()
}

/// `np.mgrid[rows, cols]`: the row coordinate down the rows, the column
/// coordinate along them.
pub(super) fn mgrid(rows: &[f64], cols: &[f64]) -> (Grid<f64>, Grid<f64>) {
    let r = Grid::from_fn(rows.len(), cols.len(), |i, _| rows[i]);
    let c = Grid::from_fn(rows.len(), cols.len(), |_, j| cols[j]);
    (r, c)
}

/// `n` uniform samples in `[0, 1)` from the examples' seed, as a grid.
pub(super) fn uniform_grid(seed: u64, rows: usize, cols: usize) -> Grid<f64> {
    let values = SpecRng::new(seed).uniform(rows * cols, 0.0, 1.0);
    Grid::from_fn(rows, cols, |r, c| values[r * cols + c])
}

/// Points on staggered rings between a minimum radius and 0.95
/// (`_annulus`), in cartesian and polar coordinates.
pub(super) struct Annulus {
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
    pub(super) fn new(n_angles: usize, n_radii: usize, min_radius: f64) -> Self {
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
    pub(super) fn waves(&self) -> Vec<f64> {
        self.polar
            .iter()
            .map(|&(r, a)| r.cos() * (3.0 * a).cos())
            .collect()
    }

    /// The Delaunay triangulation of the points without the triangles
    /// whose centroid is inside `min_radius` (`_masked_delaunay`).
    pub(super) fn masked_delaunay(&self, min_radius: f64) -> Result<Triangulation, TriError> {
        masked_delaunay(&self.x, &self.y, min_radius)
    }
}

/// The Delaunay triangulation of `x, y` without the triangles whose
/// centroid is inside `min_radius`.
pub(super) fn masked_delaunay(
    x: &[f64],
    y: &[f64],
    min_radius: f64,
) -> Result<Triangulation, TriError> {
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

/// The outline of Great Britain in radians (`BRITAIN`).
const BRITAIN: [[f64; 2]; 74] = [
    [-0.101, 0.872],
    [-0.080, 0.883],
    [-0.069, 0.888],
    [-0.054, 0.890],
    [-0.045, 0.897],
    [-0.057, 0.895],
    [-0.073, 0.900],
    [-0.087, 0.898],
    [-0.090, 0.904],
    [-0.069, 0.907],
    [-0.069, 0.921],
    [-0.080, 0.919],
    [-0.073, 0.928],
    [-0.052, 0.930],
    [-0.048, 0.942],
    [-0.062, 0.949],
    [-0.054, 0.958],
    [-0.069, 0.954],
    [-0.087, 0.952],
    [-0.087, 0.959],
    [-0.080, 0.966],
    [-0.085, 0.973],
    [-0.087, 0.965],
    [-0.097, 0.965],
    [-0.097, 0.975],
    [-0.092, 0.984],
    [-0.101, 0.980],
    [-0.108, 0.980],
    [-0.104, 0.987],
    [-0.102, 0.993],
    [-0.115, 1.001],
    [-0.099, 0.996],
    [-0.101, 1.007],
    [-0.090, 1.010],
    [-0.087, 1.021],
    [-0.069, 1.021],
    [-0.052, 1.022],
    [-0.052, 1.017],
    [-0.069, 1.010],
    [-0.064, 1.005],
    [-0.048, 1.005],
    [-0.031, 1.005],
    [-0.031, 0.996],
    [-0.040, 0.987],
    [-0.045, 0.980],
    [-0.052, 0.975],
    [-0.040, 0.973],
    [-0.026, 0.968],
    [-0.020, 0.954],
    [-0.006, 0.947],
    [0.003, 0.935],
    [0.006, 0.926],
    [0.005, 0.921],
    [0.022, 0.923],
    [0.033, 0.912],
    [0.029, 0.905],
    [0.017, 0.900],
    [0.012, 0.895],
    [0.027, 0.893],
    [0.019, 0.886],
    [0.001, 0.883],
    [-0.012, 0.884],
    [-0.029, 0.883],
    [-0.038, 0.879],
    [-0.057, 0.881],
    [-0.062, 0.876],
    [-0.078, 0.876],
    [-0.087, 0.872],
    [-0.030, 0.907],
    [-0.007, 0.905],
    [-0.057, 0.916],
    [-0.025, 0.933],
    [-0.077, 0.990],
    [-0.059, 0.993],
];

/// The user-specified triangles over the outline (`BRITAIN_TRIANGLES`).
pub(super) const BRITAIN_TRIANGLES: [[usize; 3]; 79] = [
    [67, 66, 1],
    [65, 2, 66],
    [1, 66, 2],
    [64, 2, 65],
    [63, 3, 64],
    [60, 59, 57],
    [2, 64, 3],
    [3, 63, 4],
    [0, 67, 1],
    [62, 4, 63],
    [57, 59, 56],
    [59, 58, 56],
    [61, 60, 69],
    [57, 69, 60],
    [4, 62, 68],
    [6, 5, 9],
    [61, 68, 62],
    [69, 68, 61],
    [9, 5, 70],
    [6, 8, 7],
    [4, 70, 5],
    [8, 6, 9],
    [56, 69, 57],
    [69, 56, 52],
    [70, 10, 9],
    [54, 53, 55],
    [56, 55, 53],
    [68, 70, 4],
    [52, 56, 53],
    [11, 10, 12],
    [69, 71, 68],
    [68, 13, 70],
    [10, 70, 13],
    [51, 50, 52],
    [13, 68, 71],
    [52, 71, 69],
    [12, 10, 13],
    [71, 52, 50],
    [71, 14, 13],
    [50, 49, 71],
    [49, 48, 71],
    [14, 16, 15],
    [14, 71, 48],
    [17, 19, 18],
    [17, 20, 19],
    [48, 16, 14],
    [48, 47, 16],
    [47, 46, 16],
    [16, 46, 45],
    [23, 22, 24],
    [21, 24, 22],
    [17, 16, 45],
    [20, 17, 45],
    [21, 25, 24],
    [27, 26, 28],
    [20, 72, 21],
    [25, 21, 72],
    [45, 72, 20],
    [25, 28, 26],
    [44, 73, 45],
    [72, 45, 73],
    [28, 25, 29],
    [29, 25, 31],
    [43, 73, 44],
    [73, 43, 40],
    [72, 73, 39],
    [72, 31, 25],
    [42, 40, 43],
    [31, 30, 29],
    [39, 73, 40],
    [42, 41, 40],
    [72, 33, 31],
    [32, 31, 33],
    [39, 38, 72],
    [33, 72, 38],
    [33, 38, 34],
    [37, 35, 38],
    [34, 38, 35],
    [35, 37, 36],
];

/// Longitudes and latitudes in degrees of the outline (`_britain()`).
pub(super) fn britain() -> (Vec<f64>, Vec<f64>) {
    BRITAIN
        .iter()
        .map(|&[lon, lat]| (lon.to_degrees(), lat.to_degrees()))
        .unzip()
}

/// The user-specified triangulation over the outline.
pub(super) fn britain_triangulation() -> Result<Triangulation, TriError> {
    let (x, y) = britain();
    Triangulation::new(&x, &y, BRITAIN_TRIANGLES.to_vec())
}

/// A bump over Wales (`_wales_bump`).
pub(super) fn wales_bump(x: f64, y: f64) -> f64 {
    let (dx, dy) = (x + 5.0, y - 52.0);
    (-0.01 * (dx * dx + dy * dy)).exp()
}

/// A 600 × 512 RGB picture in place of `grace_hopper.jpg` (`_photo`).
pub(super) fn photo() -> Grid<[u8; 3]> {
    Grid::from_fn(600, 512, |y, x| {
        let byte = |v: usize| u8::try_from(v % 256).expect("reduced modulo 256");
        [byte(x / 2), byte(y * 255 / 599), byte((x + 2 * y) / 5)]
    })
}

/// A 256 × 256 MRI-like slice in place of `s1045.ima.gz` (`_scan`): a
/// bright ellipse with darker rings inside.
pub(super) fn scan() -> Grid<f64> {
    let v = linspace(-1.0, 1.0, 256);
    Grid::from_fn(256, 256, |r, c| {
        let (x, y) = (v[c], v[r]);
        let (ex, ey) = (x / 0.75, y / 0.9);
        let head = (-(ex * ex + ey * ey).powf(4.0)).exp();
        let folds = 0.5 + 0.5 * (14.0 * (x - 0.1).hypot(y + 0.05)).cos();
        1000.0 * head * (0.6 + 0.4 * folds)
    })
}

/// A 130 × 542 RGBA image in place of `logo2.png` (`_logo`): coloured
/// stripes on a partly transparent background.
pub(super) fn logo() -> Grid<[f64; 4]> {
    Grid::from_fn(130, 542, |y, x| {
        let column = x / 60;
        let visible = (x / 30 + y / 26) % 3 != 0;
        [
            (column % 2) as f64,
            (column % 4) as f64 / 4.0,
            1.0 - (y / 26) as f64 / 4.0,
            f64::from(u8::from(visible)),
        ]
    })
}

/// A rough 344 × 403 elevation model in place of
/// `jacksboro_fault_dem.npz` (`_terrain`): ridges with noise on top.
pub(super) fn terrain() -> Grid<f64> {
    let noise = SpecRng::new(SEED).normal(344 * 403, 0.0, 12.0);
    Grid::from_fn(344, 403, |y, x| {
        let (xf, yf) = (x as f64, y as f64);
        let ridges = 600.0 + 250.0 * (xf / 37.0).sin() * (yf / 53.0).cos() + 0.5 * (xf - yf);
        ridges + noise[y * 403 + x]
    })
}

/// `image_antialiasing`'s chirp (`_chirp`): the radius from
/// `(offset, offset)` on an `n × n` grid of steps `1 / n`, and rings whose
/// frequency rises with it.
pub(super) fn chirp(n: usize, offset: f64) -> (Grid<f64>, Grid<f64>) {
    let x: Vec<f64> = (0..n).map(|i| i as f64 / n as f64 - offset).collect();
    let (xx, yy) = meshgrid(&x, &x);
    let r = on_mesh(&xx, &yy, |x, y| (x * x + y * y).sqrt());
    let a = r.map(|&r| (PI * 2.0 * (5.0 * r + 100.0 * (r * r) / 2.0)).sin());
    (r, a)
}
