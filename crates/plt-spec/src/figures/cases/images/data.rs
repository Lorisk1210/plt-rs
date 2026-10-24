//! The deterministic arrays of `figures/cases_images.py`.

use plt::numeric::{Grid, linspace, meshgrid};

/// `_small()`: an 8 × 8 test pattern — a checkerboard quadrant, an
/// isolated spike and a ramp.
pub(super) fn small() -> Grid<f64> {
    let ramp = linspace(0.0, 1.0, 8);
    Grid::from_fn(8, 8, |r, c| {
        if c >= 5 {
            ramp[r]
        } else if r < 4 && c < 4 {
            ((r + c) % 2) as f64
        } else if (r, c) == (6, 2) {
            1.0
        } else {
            0.0
        }
    })
}

/// `np.meshgrid(x, x)` evaluated through `f(xx, yy)`.
fn on_mesh(x: &[f64], f: impl Fn(f64, f64) -> f64) -> Grid<f64> {
    Grid::from_fn(x.len(), x.len(), |r, c| f(x[c], x[r]))
}

/// `_rings(n)`: concentric rings whose frequency rises towards the corners.
pub(super) fn rings(n: usize) -> Grid<f64> {
    on_mesh(&linspace(-1.0, 1.0, n), |x, y| {
        (120.0 * (x * x + y * y)).sin()
    })
}

/// `_bump(n)`: a smooth field in `[-1, 1]`.
pub(super) fn bump(n: usize) -> Grid<f64> {
    on_mesh(&linspace(-3.0, 3.0, n), |x, y| {
        x.sin() * (y * 0.8).cos() * (-(x * x + y * y) / 12.0).exp()
    })
}

/// `_rgb_float(n)`: red along x, green along y, blue `1 - x·y`.
pub(super) fn rgb_float(n: usize) -> Grid<[f64; 3]> {
    let x = linspace(0.0, 1.0, n);
    Grid::from_fn(n, n, |r, c| [x[c], x[r], 1.0 - x[c] * x[r]])
}

/// numpy's `(a * 255).astype(np.uint8)` for unit floats.
pub(super) fn bytes<const N: usize>(grid: &Grid<[f64; N]>) -> Grid<[u8; N]> {
    grid.map(|pixel| pixel.map(|v| (v * 255.0) as u8))
}

/// `np.linspace(lo, hi, n)[None, :].repeat(rows, axis=0)`.
pub(super) fn column_ramp(rows: usize, lo: f64, hi: f64, n: usize) -> Grid<f64> {
    let ramp = linspace(lo, hi, n);
    Grid::from_fn(rows, n, |_, c| ramp[c])
}

/// `np.meshgrid(np.linspace(-3, 3, nx), np.linspace(-2, 2, ny))`.
pub(super) fn grid(nx: usize, ny: usize) -> (Grid<f64>, Grid<f64>) {
    meshgrid(&linspace(-3.0, 3.0, nx), &linspace(-2.0, 2.0, ny))
}

/// Applies `f` cell by cell to two grids of the same shape.
pub(super) fn zip_with(a: &Grid<f64>, b: &Grid<f64>, f: impl Fn(f64, f64) -> f64) -> Grid<f64> {
    Grid::from_fn(a.rows(), a.cols(), |r, c| f(a[(r, c)], b[(r, c)]))
}
