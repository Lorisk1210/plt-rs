//! Surfaces, triangulated surfaces and wireframes.

use plt::collections::CollectionProps;
use plt::colorbar::ColorbarOptions;
use plt::colors::{Color, colormaps};
use plt::figure::Figure;
use plt::mplot3d::{
    FaceColors, Sampling, SurfaceOptions, TrisurfOptions, WireframeOptions, test_data,
};
use plt::numeric::{Grid, arange, meshgrid};
use plt::tri::Triangulation;
use plt_core::Rgba;

use super::{aliased, disc_points, remove_ticks_and_titles, sinc_surface};
use crate::figures::CaseResult;
use crate::figures::cases::common::face_cmap;

pub(super) fn surface3d(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let (x, y, z) = sinc_surface();
    let options = SurfaceOptions {
        sampling: Sampling::Counts { rows: 40, cols: 40 },
        colors: face_cmap("coolwarm")?,
        props: aliased(0.0),
        ..SurfaceOptions::default()
    };
    let surface = ax.plot_surface(&x, &y, &z, options)?;
    ax.set_zlim(-1.01, 1.01)?;
    let colorbar = ColorbarOptions {
        shrink: 0.5,
        aspect: 5.0,
        ..ColorbarOptions::default()
    };
    fig.colorbar(&surface, colorbar)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn label_offset_tick_position(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    // np.mgrid[0:6 * np.pi:0.25, 0:4 * np.pi:0.25]
    let rows = arange(0.0, 6.0 * std::f64::consts::PI, 0.25);
    let cols = arange(0.0, 4.0 * std::f64::consts::PI, 0.25);
    let (n, m) = (rows.len(), cols.len());
    let x = Grid::from_fn(n, m, |i, _| rows[i] * 1e5);
    let y = Grid::from_fn(n, m, |_, j| cols[j] * 1e6);
    let z = Grid::from_fn(n, m, |i, j| {
        (rows[i].cos() + cols[j].cos()).abs().sqrt() * 1e8
    });
    let options = SurfaceOptions {
        sampling: Sampling::Strides { rows: 2, cols: 2 },
        colors: face_cmap("autumn")?,
        ..SurfaceOptions::default()
    };
    ax.plot_surface(&x, &y, &z, options)?;
    ax.set_xlabel("X label")
        .set_ylabel("Y label")
        .set_zlabel("Z label");
    Ok(())
}

pub(super) fn surface3d_shaded(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let axis = arange(-5.0, 5.0, 0.25);
    let (x, y) = meshgrid(&axis, &axis);
    let z = Grid::from_fn(x.rows(), x.cols(), |i, j| {
        (x[(i, j)].powi(2) + y[(i, j)].powi(2)).sqrt().sin()
    });
    let options = SurfaceOptions {
        sampling: Sampling::Strides { rows: 5, cols: 5 },
        colors: FaceColors::Single(Color::from((0.25, 1.0, 0.25))),
        props: aliased(1.0),
        ..SurfaceOptions::default()
    };
    ax.plot_surface(&x, &y, &z, options)?;
    ax.set_zlim(-1.01, 1.01)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// The heights of `surface3d_masked`; the negative ones are masked.
const MASKED_MATRIX: [[f64; 11]; 8] = [
    [-1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [-1.0, 1.0, 2.0, 3.0, 4.0, 4.0, 4.0, 3.0, 2.0, 1.0, 1.0],
    [-1.0, -1.0, 4.0, 5.0, 6.0, 8.0, 6.0, 5.0, 4.0, 3.0, -1.0],
    [-1.0, -1.0, 7.0, 8.0, 11.0, 12.0, 11.0, 8.0, 7.0, -1.0, -1.0],
    [-1.0, -1.0, 8.0, 9.0, 10.0, 16.0, 10.0, 9.0, 10.0, 7.0, -1.0],
    [
        -1.0, -1.0, -1.0, 12.0, 16.0, 20.0, 16.0, 12.0, 11.0, -1.0, -1.0,
    ],
    [
        -1.0, -1.0, -1.0, -1.0, 22.0, 24.0, 22.0, 20.0, 18.0, -1.0, -1.0,
    ],
    [
        -1.0, -1.0, -1.0, -1.0, -1.0, 28.0, 26.0, 25.0, -1.0, -1.0, -1.0,
    ],
];

pub(super) fn surface3d_masked(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let xs: Vec<f64> = (1..=11).map(f64::from).collect();
    let ys: Vec<f64> = (1..=8).map(f64::from).collect();
    let (x, y) = meshgrid(&xs, &ys);
    // np.ma.masked_less(matrix, 0), the masked heights becoming NaN.
    let z = Grid::from_fn(8, 11, |i, j| {
        let v = MASKED_MATRIX[i][j];
        if v < 0.0 { f64::NAN } else { v }
    });
    // plasma(Normalize(z.min(), z.max())(z)): masked cells take the bad
    // colour.
    let plasma = colormaps::get("plasma")?;
    let (low, high) = (0.0, 28.0);
    let colors = z.map(|&v| {
        Color::from(if v.is_nan() {
            Rgba::new(0.0, 0.0, 0.0, 0.0)
        } else {
            plasma.sample((v - low) / (high - low))
        })
    });
    let options = SurfaceOptions {
        cell_colors: Some(colors),
        ..SurfaceOptions::default()
    };
    ax.plot_surface(&x, &y, &z, options)?;
    ax.view_init(30.0, -80.0, 0.0);
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn none_arg(fig: &Figure) -> CaseResult {
    let axis: Vec<f64> = (0..5).map(f64::from).collect();
    let (x, y) = meshgrid(&axis, &axis);
    let z = Grid::from_fn(5, 5, |i, j| x[(i, j)] + y[(i, j)]);
    fig.subplot_3d()
        .plot_surface(&x, &y, &z, SurfaceOptions::default())?;
    Ok(())
}

pub(super) fn masked_strides(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    // np.mgrid[-6:6.1:1, -6:6.1:1] and np.ma.masked_less(x * y, 2).
    let x = Grid::from_fn(13, 13, |i, _| i as f64 - 6.0);
    let y = Grid::from_fn(13, 13, |_, j| j as f64 - 6.0);
    let z = Grid::from_fn(13, 13, |i, j| {
        let v = x[(i, j)] * y[(i, j)];
        if v < 2.0 { f64::NAN } else { v }
    });
    let options = SurfaceOptions {
        sampling: Sampling::Strides { rows: 4, cols: 4 },
        ..SurfaceOptions::default()
    };
    ax.plot_surface(&x, &y, &z, options)?;
    ax.view_init(60.0, -45.0, 0.0);
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `plot_trisurf(x, y, z, linewidth=0.2)` over [`disc_points`] coloured
/// by `colors`.
fn trisurf(fig: &Figure, colors: FaceColors) -> CaseResult {
    let [x, y, z] = disc_points();
    let tri = Triangulation::delaunay(&x, &y)?;
    let options = TrisurfOptions {
        colors,
        props: CollectionProps::default().linewidth(0.2),
        ..TrisurfOptions::default()
    };
    fig.subplot_3d().plot_trisurf(&tri, &z, options)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn trisurf3d(fig: &Figure) -> CaseResult {
    trisurf(fig, face_cmap("jet")?)
}

pub(super) fn trisurf3d_shaded(fig: &Figure) -> CaseResult {
    trisurf(fig, FaceColors::Single(Color::from((1.0, 0.5, 0.0))))
}

/// `ax.plot_wireframe(X, Y, Z, ...)` of `get_test_data(0.05)`, without
/// its last `drop_rows` rows, sampled by `sampling`.
fn wireframe(fig: &Figure, drop_rows: usize, sampling: Sampling) -> CaseResult {
    let (x, y, z) = test_data(0.05);
    let rows = x.rows() - drop_rows;
    let head = |grid: &Grid<f64>| Grid::from_fn(rows, grid.cols(), |i, j| grid[(i, j)]);
    let options = WireframeOptions {
        sampling,
        ..WireframeOptions::default()
    };
    fig.subplot_3d()
        .plot_wireframe(&head(&x), &head(&y), &head(&z), options)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn wireframe3d(fig: &Figure) -> CaseResult {
    wireframe(fig, 0, Sampling::Counts { rows: 13, cols: 13 })
}

pub(super) fn wireframe3dasymmetric(fig: &Figure) -> CaseResult {
    wireframe(fig, 1, Sampling::Counts { rows: 3, cols: 13 })
}

pub(super) fn wireframe3dzerocstride(fig: &Figure) -> CaseResult {
    wireframe(fig, 0, Sampling::Counts { rows: 13, cols: 0 })
}

pub(super) fn wireframe3dzerorstride(fig: &Figure) -> CaseResult {
    wireframe(fig, 0, Sampling::Strides { rows: 0, cols: 10 })
}
