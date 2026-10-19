//! The quiver tests: pivots, no arrows, masked vectors, colour-coded
//! arrows and a smoke test.

use std::f64::consts::PI;

use plt::collections::{CollectionProps, EdgeColors};
use plt::colors::{Color, colormaps};
use plt::mplot3d::{Quiver3DOptions, Scatter3DOptions, Vec3};
use plt::prelude::*;
use plt::quiver::Pivot;

use super::{product3, remove_ticks_and_titles};
use crate::figures::CaseResult;

/// `np.meshgrid(xs, ys, zs)` raveled: y varies slowest, then x, then z.
fn meshgrid3(xs: &[f64], ys: &[f64], zs: &[f64]) -> Vec<Vec3> {
    (product3(ys, xs, zs).into_iter())
        .map(|[y, x, z]| [x, y, z])
        .collect()
}

/// `np.mgrid[start:stop:num * 1j]` along one axis: `start + i·step`
/// with the step of `num` values, the last one possibly off `stop` by
/// rounding.
fn mgrid_axis(start: f64, stop: f64, num: usize) -> Vec<f64> {
    let step = (stop - start) / (num - 1) as f64;
    (0..num).map(|i| i as f64 * step + start).collect()
}

/// The coordinates of `points` along axis `k`.
fn column(points: &[Vec3], k: usize) -> Vec<f64> {
    points.iter().map(|p| p[k]).collect()
}

pub(super) fn quiver3d(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let pivots = [Pivot::Tip, Pivot::Middle, Pivot::Tail];
    let colors = ["tab:blue", "tab:orange", "tab:green"];
    let corners = [-0.5, 0.5];
    for (i, (pivot, color)) in pivots.into_iter().zip(colors).enumerate() {
        let grid = meshgrid3(&corners, &corners, &corners);
        let vectors: Vec<Vec3> = grid.iter().map(|p| p.map(|c| -c)).collect();
        let lift = 2.0 * i as f64;
        let positions: Vec<Vec3> = grid.iter().map(|&[x, y, z]| [x, y, z + lift]).collect();
        let options = Quiver3DOptions {
            pivot,
            props: CollectionProps::default().color(color),
            ..Quiver3DOptions::default()
        };
        ax.quiver(&positions, &vectors, options)?;
        let options = Scatter3DOptions {
            scatter: ScatterOptions {
                colors: ScatterColors::Single(color.into()),
                ..ScatterOptions::default()
            },
            ..Scatter3DOptions::default()
        };
        let [x, y, z] = [0, 1, 2].map(|k| column(&positions, k));
        ax.scatter_with(&x, &y, &z, options)?;
    }
    ax.set_xlim(-3.0, 3.0)?;
    ax.set_ylim(-3.0, 3.0)?;
    ax.set_zlim(-1.0, 5.0)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn quiver3d_empty(fig: &Figure) -> CaseResult {
    let options = Quiver3DOptions {
        length: 0.1,
        pivot: Pivot::Tip,
        normalize: true,
        ..Quiver3DOptions::default()
    };
    fig.subplot_3d().quiver(&[], &[], options)?;
    Ok(())
}

pub(super) fn quiver3d_masked(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let xy = mgrid_axis(-1.0, 0.8, 10);
    let zs = mgrid_axis(-1.0, 0.6, 3);
    // `np.mgrid` varies x slowest; `quiver` drops the masked arrows.
    let (positions, vectors): (Vec<Vec3>, Vec<Vec3>) = (product3(&xy, &xy, &zs).into_iter())
        .filter(|&[x, y, _]| !((-0.4 < x && x < 0.1) || (0.1 < y && y < 0.7)))
        .map(|[x, y, z]| {
            let (px, py, pz) = (PI * x, PI * y, PI * z);
            let u = px.sin() * py.cos() * pz.cos();
            let v = -px.cos() * py.sin() * pz.cos();
            let w = (2.0_f64 / 3.0).sqrt() * px.cos() * py.cos() * pz.sin();
            ([x, y, z], [u, v, w])
        })
        .unzip();
    let options = Quiver3DOptions {
        length: 0.1,
        pivot: Pivot::Tip,
        normalize: true,
        ..Quiver3DOptions::default()
    };
    ax.quiver(&positions, &vectors, options)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn quiver3d_colorcoded(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let positions: Vec<Vec3> = (0..10).map(|i| [0.0, 0.0, f64::from(i)]).collect();
    let vectors: Vec<Vec3> = (0..10).map(|i| [0.0, f64::from(i), 0.0]).collect();
    let reds = colormaps::get("Reds")?;
    let colors: Vec<Color> = (0..10)
        .map(|i| reds.sample(f64::from(i) / 9.0).into())
        .collect();
    let props = CollectionProps {
        edgecolors: Some(EdgeColors::Colors(colors)),
        ..CollectionProps::default()
    };
    let options = Quiver3DOptions {
        props,
        ..Quiver3DOptions::default()
    };
    ax.quiver(&positions, &vectors, options)?;
    ax.set_ylim(0.0, 10.0)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn quiver3d_smoke(fig: &Figure) -> CaseResult {
    let xy = arange(-0.8, 1.0, 0.2);
    let positions = meshgrid3(&xy, &xy, &arange(-0.8, 1.0, 0.8));
    let vectors = vec![[1.0; 3]; positions.len()];
    let options = Quiver3DOptions {
        pivot: Pivot::Middle,
        ..Quiver3DOptions::default()
    };
    fig.subplot_3d().quiver(&positions, &vectors, options)?;
    Ok(())
}
