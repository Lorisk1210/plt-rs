//! Images on planes: `imshow3d` and `intersecting_planes`.

use plt::colors::{Color, Colormap, colormaps};
use plt::figure::Figure;
use plt::mplot3d::{Axes3D, Axes3DError, Axis3, SurfaceOptions};
use plt::numeric::Grid;

use super::{EVERY_POINT, SEED, label_xyz};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_3d/imshow3d", imshow3d),
    FigureCase::new("gallery_3d/intersecting_planes", intersecting_planes),
];

/// `values` through `cmap`, normalised from `low` to `high`.
fn colored(values: &Grid<f64>, cmap: &Colormap, (low, high): (f64, f64)) -> Grid<Color> {
    values.map(|v| Color::from(cmap.sample((v - low) / (high - low))))
}

/// A surface of unshaded faces of one colour each through the grid
/// points `(x, y, z)`.
fn colored_plane(
    ax: &Axes3D,
    [x, y, z]: [Grid<f64>; 3],
    colors: Grid<Color>,
) -> Result<(), Axes3DError> {
    let options = SurfaceOptions {
        sampling: EVERY_POINT,
        cell_colors: Some(colors),
        shade: Some(false),
        ..SurfaceOptions::default()
    };
    ax.plot_surface(&x, &y, &z, options).map(drop)
}

/// The smallest and largest of `values`.
fn extent(values: impl IntoIterator<Item = f64>) -> (f64, f64) {
    values
        .into_iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), v| {
            (low.min(v), high.max(v))
        })
}

/// `imshow3d`: `array` as a plane normal to `direction` at `pos`, one
/// face per value coloured through `cmap` over its range; the rows and
/// columns of `array` run along z and y for x, x and z for y, and y and x
/// for z.
fn image_plane(
    ax: &Axes3D,
    array: &Grid<f64>,
    direction: Axis3,
    pos: f64,
    cmap: &str,
) -> CaseResult {
    let colors = colored(
        array,
        &colormaps::get(cmap)?,
        extent(array.as_slice().iter().copied()),
    );
    // np.mgrid[0:rows + 1, 0:cols + 1] and np.full_like(..., pos).
    let (rows, cols) = (array.rows() + 1, array.cols() + 1);
    let down = Grid::from_fn(rows, cols, |i, _| i as f64);
    let across = Grid::from_fn(rows, cols, |_, j| j as f64);
    let plane = Grid::filled(rows, cols, pos);
    let xyz = match direction {
        Axis3::X => [plane, across, down],
        Axis3::Y => [down, plane, across],
        Axis3::Z => [across, down, plane],
    };
    colored_plane(ax, xyz, colors)?;
    Ok(())
}

fn imshow3d(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    label_xyz(&ax, ["x", "y", "z"]);
    let (nx, ny, nz) = (8, 10, 5);
    let mut rng = SpecRng::new(SEED);
    // np.arange(rows * cols).reshape(rows, cols) + scale * random values.
    let mut data = |rows: usize, cols: usize, scale: f64| {
        let noise = rng.uniform(rows * cols, 0.0, 1.0);
        Grid::from_fn(rows, cols, |i, j| {
            (i * cols + j) as f64 + scale * noise[i * cols + j]
        })
    };
    let data_xy = data(ny, nx, 15.0);
    let data_yz = data(nz, ny, 10.0);
    let data_zx = data(nx, nz, 8.0);
    image_plane(&ax, &data_xy, Axis3::Z, 0.0, "viridis")?;
    image_plane(&ax, &data_yz, Axis3::X, 0.0, "magma")?;
    image_plane(&ax, &data_zx, Axis3::Y, ny as f64, "plasma")?;
    Ok(())
}

/// The shape of `intersecting_planes`' array.
const SHAPE: [usize; 3] = [70, 100, 50];

/// The coordinate of index `i` along an axis of `n` points from -1 to 1
/// (`np.mgrid[-1:1:n * 1j]`, which unlike `linspace` does not pin the
/// last point to 1).
fn mgrid_coordinate(i: usize, n: usize) -> f64 {
    i as f64 * (2.0 / (n - 1) as f64) - 1.0
}

/// `intersecting_planes`' array at `index`: the squared distance from
/// the centre of the cube `[-1, 1]³`.
fn r_square(index: [usize; 3]) -> f64 {
    let [x, y, z] = [0, 1, 2].map(|k| mgrid_coordinate(index[k], SHAPE[k]).powi(2));
    x + y + z
}

/// `plot_quadrants`: the plane through the centre of the array normal to
/// `fixed`, drawn as four quadrants so that the planes cutting it sort
/// correctly.
fn plot_quadrants(ax: &Axes3D, fixed: Axis3, cmap: &Colormap, range: (f64, f64)) -> CaseResult {
    let f = fixed.index();
    let (a, b) = match fixed {
        Axis3::X => (1, 2),
        Axis3::Y => (0, 2),
        Axis3::Z => (0, 1),
    };
    let (half_a, half_b) = (SHAPE[a] / 2, SHAPE[b] / 2);
    let center = SHAPE[f] / 2;
    for quadrant in 0..4 {
        let (offset_a, offset_b) = ((quadrant / 2) * half_a, (quadrant % 2) * half_b);
        // The array index (and position) of point (p, q) of the quadrant.
        let index = |p: usize, q: usize| {
            let mut index = [0; 3];
            index[f] = center;
            index[a] = offset_a + p;
            index[b] = offset_b + q;
            index
        };
        let values = Grid::from_fn(half_a, half_b, |p, q| r_square(index(p, q)));
        let xyz = [0, 1, 2].map(|k| Grid::from_fn(half_a, half_b, |p, q| index(p, q)[k] as f64));
        colored_plane(ax, xyz, colored(&values, cmap, range))?;
    }
    Ok(())
}

fn intersecting_planes(fig: &Figure) -> CaseResult {
    let [nx, ny, nz] = SHAPE;
    let range = extent(
        (0..nx).flat_map(|i| (0..ny).flat_map(move |j| (0..nz).map(move |k| r_square([i, j, k])))),
    );
    let ax = fig.subplot_3d();
    ax.set_box_aspect(Some(SHAPE.map(|n| n as f64)), 1.0)?;
    let cmap = colormaps::get("viridis_r")?;
    for fixed in Axis3::ALL {
        plot_quadrants(&ax, fixed, &cmap, range)?;
    }
    Ok(())
}
