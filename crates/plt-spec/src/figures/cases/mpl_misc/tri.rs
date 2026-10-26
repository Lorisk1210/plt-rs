//! The cases from `test_triangulation.py`.

use std::f64::consts::{PI, TAU};

use plt::axes::{TriValues, TripcolorOptions};
use plt::collections::CollectionProps;
use plt::colors::{Color, ColorizerOptions, colormaps};
use plt::contour::{ContourOptions, Levels};
use plt::numeric::{arange, linspace, linspace_with};
use plt::prelude::*;
use plt::quiver::{QuiverArrowShape, QuiverOptions, QuiverUnits, VectorField};
use plt::rc::Aspect;
use plt::tri::{
    CubicKind, CubicTriInterpolator, TriInterpolator, Triangulation, UniformTriRefiner,
};

use super::{classic, remove_text};
use crate::figures::{CaseResult, FigureCase};

/// The mean of the values of `values` at the corners of every triangle of
/// `tri` (matplotlib's `values[tri.triangles].mean(axis=1)`).
fn corner_means(tri: &Triangulation, values: &[f64]) -> Vec<f64> {
    tri.triangles()
        .iter()
        .map(|corners| corners.iter().map(|&i| values[i]).sum::<f64>() / 3.0)
        .collect()
}

/// `test_tripcolor`: colours given per point and per triangle.
fn tripcolor(fig: &Figure) -> CaseResult {
    let x = [0.0, 0.5, 1.0, 0.0, 0.5, 1.0, 0.0, 0.5, 1.0, 0.75];
    let y = [0.0, 0.0, 0.0, 0.5, 0.5, 0.5, 1.0, 1.0, 1.0, 0.75];
    let triangles = vec![
        [0, 1, 3],
        [1, 4, 3],
        [1, 2, 4],
        [2, 5, 4],
        [3, 4, 6],
        [4, 7, 6],
        [4, 5, 9],
        [7, 4, 9],
        [8, 7, 9],
        [5, 8, 9],
    ];
    let tri = Triangulation::new(&x, &y, triangles)?;
    let c_points: Vec<f64> = x.iter().zip(&y).map(|(x, y)| x + 0.5 * y).collect();
    let c_faces: Vec<f64> = corner_means(&tri, &x)
        .iter()
        .zip(corner_means(&tri, &y))
        .map(|(xmid, ymid)| 0.5 * xmid + ymid)
        .collect();
    let black_edges = || TripcolorOptions {
        props: CollectionProps::default().edgecolor("k"),
        ..TripcolorOptions::default()
    };
    let ax = fig.add_subplot(1, 2, 1)?;
    ax.tripcolor(&tri, TriValues::Auto(c_points), black_edges())?;
    ax.set_title("point colors");
    let ax = fig.add_subplot(1, 2, 2)?;
    ax.tripcolor(&tri, TriValues::Faces(c_faces), black_edges())?;
    ax.set_title("facecolors");
    Ok(())
}

/// Points on `n_radii` rings from `min_radius` to 0.95, `n_angles` per
/// ring starting at `first_angle`, every other ring turned by half a step;
/// Delaunay-triangulated without the triangles centred inside
/// `min_radius`.
fn ring_triangulation(
    n_angles: usize,
    n_radii: usize,
    min_radius: f64,
    first_angle: f64,
) -> Result<Triangulation, plt::tri::TriError> {
    let radii = linspace(min_radius, 0.95, n_radii);
    let angles = linspace_with(first_angle, TAU + first_angle, n_angles, false);
    let half_step = PI / n_angles as f64;
    let (x, y): (Vec<f64>, Vec<f64>) = angles
        .iter()
        .flat_map(|&angle| {
            radii.iter().enumerate().map(move |(ring, &radius)| {
                let turned = if ring % 2 == 1 {
                    angle + half_step
                } else {
                    angle
                };
                (radius * turned.cos(), radius * turned.sin())
            })
        })
        .unzip();
    let tri = Triangulation::delaunay(&x, &y)?;
    let mask = corner_means(&tri, &x)
        .iter()
        .zip(corner_means(&tri, &y))
        .map(|(xmid, ymid)| xmid.hypot(ymid) < min_radius)
        .collect();
    tri.with_mask(Some(mask))
}

/// `values` rescaled so that the largest is 0 and the smallest 1.
fn flipped_unit(values: &[f64]) -> Vec<f64> {
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    values.iter().map(|z| (max - z) / (max - min)).collect()
}

/// Refined values, NaN where the refinement found none.
fn refined_values(z: &[Option<f64>]) -> Vec<f64> {
    z.iter().map(|z| z.unwrap_or(f64::NAN)).collect()
}

/// `test_tri_smooth_contouring`: contours of a field refined on a ring
/// triangulation.
fn tri_smooth_contouring(fig: &Figure) -> CaseResult {
    let n_angles = 20;
    let tri = ring_triangulation(n_angles, 10, 0.15, n_angles as f64)?;
    let z: Vec<f64> = tri
        .x()
        .iter()
        .zip(tri.y())
        .map(|(&x, &y)| {
            let r1 = (0.5 - x).hypot(0.5 - y);
            let theta1 = (0.5 - x).atan2(0.5 - y);
            let r2 = (-x - 0.2).hypot(-y - 0.2);
            let theta2 = (-x - 0.2).atan2(-y - 0.2);
            -(2.0 * ((r1 / 10.0).powi(2).exp() - 1.0) * 30.0 * (7.0 * theta1).cos()
                + ((r2 / 10.0).powi(2).exp() - 1.0) * 30.0 * (11.0 * theta2).cos()
                + 0.7 * (x * x + y * y))
        })
        .collect();
    let refined = UniformTriRefiner::new(&tri).refine_field(&flipped_unit(&z), None, 4)?;
    let ax = fig.add_subplot(1, 1, 1)?;
    ax.triplot(&tri, "", LineProps::default().linewidth(0.5).color("0.5"))?;
    ax.tricontour(
        &refined.triangulation,
        refined_values(&refined.z),
        ContourOptions {
            levels: Levels::Values(arange(0.0, 1.0, 0.025)),
            colors: Some(vec![Color::from("black")]),
            ..ContourOptions::default()
        },
    )?;
    remove_text(fig);
    Ok(())
}

/// `test_tri_smooth_gradient`: the potential of a dipole refined on a ring
/// triangulation, with the field's direction at every point.
fn tri_smooth_gradient(fig: &Figure) -> CaseResult {
    let tri = ring_triangulation(30, 10, 0.2, 0.0)?;
    let dipole: Vec<f64> = tri
        .x()
        .iter()
        .zip(tri.y())
        .map(|(&x, &y)| y.atan2(x).cos() / (x * x + y * y))
        .collect();
    let potential = flipped_unit(&dipole);
    let refined = UniformTriRefiner::new(&tri).refine_field(&potential, None, 3)?;
    let negated: Vec<f64> = potential.iter().map(|v| -v).collect();
    let interpolator = CubicTriInterpolator::new(&tri, &negated, &CubicKind::MinE)?;
    let (u, v): (Vec<f64>, Vec<f64>) = tri
        .x()
        .iter()
        .zip(tri.y())
        .map(|(&x, &y)| {
            let [ex, ey] = interpolator.gradient(x, y).unwrap_or([f64::NAN; 2]);
            let norm = ex.hypot(ey);
            (ex / norm, ey / norm)
        })
        .unzip();
    let ax = fig.add_subplot(1, 1, 1)?;
    ax.set_aspect(Aspect::Equal)?;
    ax.triplot(&tri, "", LineProps::default().color("0.8"))?;
    ax.tricontour(
        &refined.triangulation,
        refined_values(&refined.z),
        ContourOptions {
            colorizer: ColorizerOptions {
                cmap: Some(colormaps::get("hot")?),
                ..ColorizerOptions::default()
            },
            levels: Levels::Values(arange(0.0, 1.0, 0.01)),
            linewidths: Some(vec![2.0, 1.0, 1.0, 1.0]),
            ..ContourOptions::default()
        },
    )?;
    ax.quiver(
        VectorField::new(tri.x(), tri.y(), u, v),
        QuiverOptions {
            units: QuiverUnits::Xy,
            scale: Some(10.0),
            props: CollectionProps::default().zorder(3.0),
            color: Color::from("blue"),
            width: Some(0.007),
            shape: QuiverArrowShape {
                head_width: 3.0,
                head_length: 4.0,
                ..QuiverArrowShape::default()
            },
            ..QuiverOptions::default()
        },
    )?;
    remove_text(fig);
    Ok(())
}

/// `test_triplot_with_ls`: a dashed triangle.
fn triplot_with_ls(fig: &Figure) -> CaseResult {
    let tri = Triangulation::new(&[0.0, 2.0, 1.0], &[0.0, 0.0, 1.0], vec![[0, 1, 2]])?;
    fig.subplot().triplot(
        &tri,
        "",
        LineProps::default().linestyle("--".parse::<LineStyle>()?),
    )?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_misc/tripcolor", tripcolor),
    classic("mpl_misc/tri_smooth_contouring", tri_smooth_contouring),
    classic("mpl_misc/tri_smooth_gradient", tri_smooth_gradient),
    classic("mpl_misc/triplot_with_ls", triplot_with_ls),
];
