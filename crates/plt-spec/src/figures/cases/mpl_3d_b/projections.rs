//! The projection matrix tests, drawn on 2-D axes: a cube seen through a
//! perspective and an orthographic projection.

use plt::axes::Axes;
use plt::collections::CollectionSpec;
use plt::mplot3d::Vec3;
use plt::mplot3d::math::{
    Matrix4, ortho_transformation, persp_transformation, view_axes, view_transformation,
};
use plt::prelude::*;
use plt::transforms::Point;

use super::remove_ticks_and_titles;
use crate::figures::CaseResult;

/// The corners of the cube walked by the `proj_axes_cube` tests, scaled.
fn cube_corners(scale: f64) -> Vec<Vec3> {
    let xs = [0, 1, 1, 0, 0, 0, 1, 1, 0, 0];
    let ys = [0, 0, 1, 1, 0, 0, 0, 1, 1, 0];
    let zs = [0, 0, 0, 0, 0, 1, 1, 1, 1, 1];
    (0..10)
        .map(|i| [xs[i], ys[i], zs[i]].map(|c| f64::from(c) * scale))
        .collect()
}

/// `_view_transformation_uvw` looking from `eye` at `target`, z up.
fn view_matrix(eye: Vec3, target: Vec3) -> Matrix4 {
    let [u, v, w] = view_axes(eye, target, [0.0, 0.0, 1.0], 0.0);
    view_transformation(u, v, w, eye)
}

/// `_test_proj_draw_axes`: 2-D axes with the projected unit axes of
/// length `s`, labelled.
fn draw_axes(fig: &Figure, m: &Matrix4, s: f64) -> Axes {
    let ends = [[0.0; 3], [s, 0.0, 0.0], [0.0, s, 0.0], [0.0, 0.0, s]];
    let projected: Vec<Point> = (m.project_all(&ends).into_iter())
        .map(|p| Point::new(p[0], p[1]))
        .collect();
    let ax = fig.subplot();
    let lines = projected[1..]
        .iter()
        .map(|&end| vec![projected[0], end])
        .collect();
    ax.add_collection(CollectionSpec::lines(lines));
    for (p, label) in projected.iter().zip(["o", "x", "y", "z"]) {
        ax.text(p.x, p.y, label);
    }
    ax
}

/// The projected cube corners joined by a red line and labelled.
fn label_corners(ax: &Axes, x: &[f64], y: &[f64]) -> CaseResult {
    ax.plot_with(x, y, LineProps::default().color("r"))?;
    let labels = "0 1 2 3 0 4 5 6 7 4".split(' ');
    for ((&x, &y), label) in x.iter().zip(y).zip(labels) {
        ax.text(x, y, label);
    }
    Ok(())
}

/// The cube corners projected by `m`, as x, y and depth columns.
fn project_corners(m: &Matrix4, scale: f64) -> [Vec<f64>; 3] {
    let projected = m.project_all(&cube_corners(scale));
    [0, 1, 2].map(|k| projected.iter().map(|p| p[k]).collect())
}

pub(super) fn proj_axes_cube(fig: &Figure) -> CaseResult {
    let m = persp_transformation(100.0, -100.0, 1.0)
        .dot(&view_matrix([1000.0, -1000.0, 2000.0], [100.0; 3]));
    let [tx, ty, tz] = project_corners(&m, 300.0);
    let ax = draw_axes(fig, &m, 400.0);
    let options = ScatterOptions {
        colors: MappedColors::new(tz).into(),
        ..ScatterOptions::default()
    };
    ax.scatter_with(tx.clone(), ty.clone(), options)?;
    label_corners(&ax, &tx, &ty)?;
    ax.set_xlim(-0.2, 0.2)?.set_ylim(-0.2, 0.2)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn proj_axes_cube_ortho(fig: &Figure) -> CaseResult {
    let m = ortho_transformation(-1.0, 1.0).dot(&view_matrix([200.0, 100.0, 100.0], [0.0; 3]));
    let [tx, ty, tz] = project_corners(&m, 100.0);
    let ax = draw_axes(fig, &m, 150.0);
    let options = ScatterOptions {
        sizes: Some(tz.iter().map(|z| 300.0 - z).collect()),
        ..ScatterOptions::default()
    };
    ax.scatter_with(tx.clone(), ty.clone(), options)?;
    label_corners(&ax, &tx, &ty)?;
    ax.set_xlim(-200.0, 200.0)?.set_ylim(-200.0, 200.0)?;
    remove_ticks_and_titles(fig);
    Ok(())
}
