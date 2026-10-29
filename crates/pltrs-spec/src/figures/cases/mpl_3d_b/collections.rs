//! The collection tests: open and closed polygons, alpha, 2-D line
//! collections lifted into 3-D, and computed z-order.

use std::f64::consts::PI;

use pltrs::collections::{Collection, CollectionProps, CollectionSpec, ColorMapping};
use pltrs::colors::{Normalize, ScalarMappable, colormaps};
use pltrs::mplot3d::{
    Axes3D, Axes3DError, Axes3DOptions, Axis3, FaceColors, Poly3DOptions, Scatter3DOptions,
    SurfaceOptions, Vec3,
};
use pltrs::numeric::Grid;
use pltrs::numeric::remainder;
use pltrs::prelude::*;
use pltrs::transforms::Point;
use pltrs_core::Rgba;

use super::remove_ticks_and_titles;
use crate::figures::CaseResult;

/// The two triangles of the `poly3dcollection` tests.
const TRIANGLES: [[Vec3; 3]; 2] = [
    [[0.0, 0.0, 1.0], [0.0, 1.0, 1.0], [0.0, 0.0, 0.0]],
    [[0.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, 0.0]],
];

/// Adds one of [`TRIANGLES`] without autoscaling.
fn triangle(
    ax: &Axes3D,
    index: usize,
    closed: bool,
    props: CollectionProps,
) -> Result<Collection, Axes3DError> {
    let options = Poly3DOptions {
        props: props.linewidth(3.0),
        closed,
        autolim: false,
        ..Poly3DOptions::default()
    };
    ax.add_poly_collection3d_with(vec![TRIANGLES[index].to_vec()], options)
}

pub(super) fn poly3dcollection_closed(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let props = |face: (f64, f64, f64)| {
        CollectionProps::default()
            .edgecolor("k")
            .facecolor(Rgba::new(face.0, face.1, face.2, 0.5))
    };
    triangle(&ax, 0, true, props((0.5, 0.5, 1.0)))?;
    triangle(&ax, 1, false, props((1.0, 0.5, 0.5)))?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn poly3dcollection_alpha(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let props = |face: (f64, f64, f64)| {
        CollectionProps::default()
            .edgecolor("k")
            .facecolor(Rgba::opaque(face.0, face.1, face.2))
            .alpha(0.5)
    };
    triangle(&ax, 0, true, props((0.5, 0.5, 1.0)))?;
    triangle(&ax, 1, false, props((1.0, 0.5, 0.5)))?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `values` through `twilight` normalised over `[0, 2π]`, the colours of
/// the `add_collection3d_zs` tests.
fn twilight(values: &[f64]) -> Result<ColorMapping, Box<dyn std::error::Error>> {
    Ok(ColorMapping {
        values: values.iter().map(|&v| Some(v)).collect(),
        mappable: ScalarMappable::with_norm(
            colormaps::get("twilight")?,
            Box::new(Normalize::new(0.0, 2.0 * PI)),
        ),
    })
}

/// Consecutive pairs of `points` as segments.
fn segments<T: Copy>(points: &[T]) -> Vec<Vec<T>> {
    points.windows(2).map(<[T]>::to_vec).collect()
}

/// The limits the `add_collection3d_zs` tests end with.
fn set_limits(ax: &Axes3D, z: (f64, f64)) -> Result<(), Axes3DError> {
    ax.set_xlim(-5.0, 5.0)?;
    ax.set_ylim(-4.0, 6.0)?;
    ax.set_zlim(z.0, z.1)?;
    Ok(())
}

pub(super) fn add_collection3d_zs_array(fig: &Figure) -> CaseResult {
    let theta = linspace(-4.0 * PI, 4.0 * PI, 100);
    let points: Vec<Vec3> = (theta.iter().zip(linspace(-2.0, 2.0, 100)))
        .map(|(&t, z)| {
            let r = z * z + 1.0;
            [r * t.sin(), r * t.cos(), z]
        })
        .collect();
    let ax = fig.subplot_3d();
    // A 2-D line collection placed at one z per vertex is the line
    // collection through the 3-D points.
    let lines = ax.add_line_collection3d(segments(&points), CollectionProps::default(), true)?;
    let angles: Vec<f64> = theta.iter().map(|&t| remainder(t, 2.0 * PI)).collect();
    lines.set_mapping(twilight(&angles[..99])?);
    set_limits(&ax, (-2.0, 2.0))?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn add_collection3d_zs_scalar(fig: &Figure) -> CaseResult {
    let theta = linspace(0.0, 2.0 * PI, 100);
    let r = 2.0;
    let points: Vec<Point> = theta
        .iter()
        .map(|t| Point::new(r * t.sin(), r * t.cos()))
        .collect();
    let ax = fig.subplot_3d();
    let spec = CollectionSpec::lines(segments(&points)).with_mapping(twilight(&theta[..99])?);
    ax.add_collection3d(spec, &[1.0], Axis3::Z, true)?;
    set_limits(&ax, (0.0, 2.0))?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// The first row of `computed_zorder`: a plane, a vector and two points.
fn plane_vector_points(ax: &Axes3D) -> CaseResult {
    let corners = vec![
        [0.0, 0.0, 0.0],
        [0.0, 5.0, 0.0],
        [5.0, 5.0, 0.0],
        [5.0, 0.0, 0.0],
    ];
    let props = CollectionProps::default()
        .facecolor("white")
        .edgecolor("black")
        .zorder(1.0);
    ax.add_poly_collection3d(vec![corners], props, true)?;
    let red = LineProps::default().color("red").zorder(2.0);
    ax.plot_with([2.0, 2.0], [2.0, 2.0], [0.0, 4.0], Axis3::Z, red)?;
    let options = Scatter3DOptions {
        scatter: ScatterOptions {
            colors: ScatterColors::Single("red".into()),
            ..ScatterOptions::default()
        },
        ..Scatter3DOptions::default()
    };
    ax.scatter_with([3.0, 3.0], [1.0, 3.0], [1.0, 3.0], options)?
        .set_zorder(10.0);
    ax.set_xlim(0.0, 5.0)?;
    ax.set_ylim(0.0, 5.0)?;
    ax.set_zlim(0.0, 2.5)?;
    Ok(())
}

/// A translucent surface of one colour without edges, at `zorder`.
fn flat_surface(
    ax: &Axes3D,
    [x, y, z]: [&Grid<f64>; 3],
    color: &str,
    zorder: f64,
) -> Result<(), Axes3DError> {
    let options = SurfaceOptions {
        colors: FaceColors::Single(color.into()),
        props: CollectionProps::default()
            .alpha(0.5)
            .linewidth(0.0)
            .zorder(zorder),
        ..SurfaceOptions::default()
    };
    ax.plot_surface(x, y, z, options).map(drop)
}

/// The second row of `computed_zorder`: planes crossed by a circle and
/// the parts of a tilted circle.
fn planes_and_circles(ax: &Axes3D) -> CaseResult {
    let dim = 10.0;
    let (x, y) = meshgrid(&[-dim, dim], &[-dim, dim]);
    let z = Grid::filled(2, 2, 0.0);
    let angle = 0.5;
    let (x2, y2) = meshgrid(&[-dim, dim], &[0.0, dim]);
    let z2 = y2.map(|y| y * angle);
    let (_, y3) = meshgrid(&[-dim, dim], &[-dim, 0.0]);
    let z3 = y3.map(|y| y * angle);
    let r = 7.0;
    let th = linspace(0.0, 2.0 * PI, 1000);
    let circle: Vec<Vec3> = th
        .iter()
        .map(|t| [r * t.cos(), r * t.sin(), angle * r * t.sin()])
        .collect();
    let half = |front: bool| -> [Vec<f64>; 3] {
        let kept: Vec<&Vec3> = circle
            .iter()
            .filter(|p| if front { p[1] > 0.0 } else { p[1] < 0.0 })
            .collect();
        [0, 1, 2].map(|k| kept.iter().map(|p| p[k]).collect())
    };
    let dashed = |color: &str, zorder: f64| {
        LineProps::default()
            .linewidth(5.0)
            .linestyle(LineStyle::Dashed)
            .color(color)
            .zorder(zorder)
    };
    flat_surface(ax, [&x2, &y3, &z3], "blue", -1.0)?;
    let [bx, by, bz] = half(false);
    ax.plot_with(&bx, &by, &bz, Axis3::Z, dashed("green", 0.0))?;
    flat_surface(ax, [&x, &y, &z], "red", 1.0)?;
    let (cx, cy): (Vec<f64>, Vec<f64>) = th.iter().map(|t| (r * t.sin(), r * t.cos())).unzip();
    ax.plot_with(&cx, &cy, [0.0; 1000], Axis3::Z, dashed("black", 2.0))?;
    flat_surface(ax, [&x2, &y2, &z2], "blue", 3.0)?;
    let [fx, fy, fz] = half(true);
    ax.plot_with(&fx, &fy, &fz, Axis3::Z, dashed("green", 4.0))?;
    ax.view_init(20.0, -20.0, 0.0);
    ax.set_axis_off();
    Ok(())
}

pub(super) fn computed_zorder(fig: &Figure) -> CaseResult {
    for (index, computed) in [(1, true), (2, false)] {
        let ax = fig.add_subplot_3d(2, 2, index, Axes3DOptions::default())?;
        ax.set_computed_zorder(computed);
        plane_vector_points(&ax)?;
    }
    for (index, computed) in [(3, true), (4, false)] {
        let ax = fig.add_subplot_3d(2, 2, index, Axes3DOptions::default())?;
        ax.set_computed_zorder(computed);
        planes_and_circles(&ax)?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}
