//! The 3-D plot types: bars, a filled double helix, a line, arrows,
//! scatter, stems, surfaces, voxels and a wireframe, each without tick
//! labels.

use std::f64::consts::PI;

use plt::collections::CollectionProps;
use plt::colors::{ColorizerOptions, colormaps};
use plt::mplot3d::{
    Array3, Axes3D, Axis3, Bar3DOptions, Boxes, FaceColors, FillBetween3DOptions, Quiver3DOptions,
    Sampling, Stem3DOptions, SurfaceOptions, TrisurfOptions, VoxelOptions, WireframeOptions,
    test_data,
};
use plt::numeric::{Grid, arange, linspace, linspace_with, meshgrid};
use plt::prelude::*;
use plt::tri::Triangulation;

use super::gridded;
use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// Blanks the tick labels of all three axes, as every 3-D example does
/// (`ax.set(xticklabels=[], yticklabels=[], zticklabels=[])`).
fn no_tick_labels(ax: &Axes3D) -> CaseResult {
    for axis in [ax.xaxis(), ax.yaxis(), ax.zaxis()] {
        axis.set_ticklabels(Vec::<String>::new())?;
    }
    Ok(())
}

/// `Blues` mapped from `vmin` up to the largest face value.
fn blues_from(vmin: f64) -> Result<FaceColors, Box<dyn std::error::Error>> {
    Ok(ColorizerOptions::default()
        .cmap(colormaps::get("Blues")?)
        .limits(Some(vmin), None)
        .into())
}

/// The smallest of `values`.
fn minimum(values: &[f64]) -> f64 {
    values.iter().copied().fold(f64::INFINITY, f64::min)
}

fn bar3d_simple(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let boxes = Boxes::new(
        [1.0, 1.0, 2.0, 2.0],
        [1.0, 2.0, 1.0, 2.0],
        [0.0, 0.0, 0.0, 0.0],
        [0.5; 4],
        [0.5; 4],
        [2.0, 3.0, 1.0, 4.0],
    );
    ax.bar3d(boxes, Bar3DOptions::default())?;
    no_tick_labels(&ax)
}

fn fill_between3d_simple(fig: &Figure) -> CaseResult {
    let theta = linspace(0.0, 2.0 * PI, 50);
    let x1 = map(&theta, f64::cos);
    let y1 = map(&theta, f64::sin);
    let z = linspace(0.0, 1.0, theta.len());
    let x2 = map(&theta, |t| (t + PI).cos());
    let y2 = map(&theta, |t| (t + PI).sin());
    let ax = fig.subplot_3d();
    let translucent = FillBetween3DOptions {
        props: CollectionProps::default().alpha(0.5),
        ..FillBetween3DOptions::default()
    };
    ax.fill_between((&x1, &y1, &z), (&x2, &y2, &z), translucent)?;
    for (x, y) in [(&x1, &y1), (&x2, &y2)] {
        ax.plot_with(
            x,
            y,
            &z,
            Axis3::Z,
            LineProps::default().linewidth(2.0).color("C0"),
        )?;
    }
    no_tick_labels(&ax)
}

fn plot3d_simple(fig: &Figure) -> CaseResult {
    let xs = linspace(0.0, 1.0, 100);
    let ax = fig.subplot_3d();
    ax.plot(
        &xs,
        map(&xs, |x| (x * 6.0 * PI).sin()),
        map(&xs, |x| (x * 6.0 * PI).cos()),
    )?;
    no_tick_labels(&ax)
}

fn quiver3d_simple(fig: &Figure) -> CaseResult {
    let t = linspace(-1.0, 1.0, 4);
    // `np.meshgrid(x, y, z)` raveled: y varies slowest, then x, then z.
    let mut positions = Vec::new();
    let mut vectors = Vec::new();
    for &y in &t {
        for &x in &t {
            for &z in &t {
                positions.push([x, y, z]);
                vectors.push([(x + y) / 5.0, (y - x) / 5.0, z * 0.0]);
            }
        }
    }
    let ax = fig.subplot_3d();
    ax.quiver(&positions, &vectors, Quiver3DOptions::default())?;
    no_tick_labels(&ax)
}

fn scatter3d_simple(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(19_680_801);
    let xs = rng.uniform(100, 23.0, 32.0);
    let ys = rng.uniform(100, 0.0, 100.0);
    let zs = rng.uniform(100, -50.0, -25.0);
    let ax = fig.subplot_3d();
    ax.scatter(&xs, &ys, &zs)?;
    no_tick_labels(&ax)
}

fn stem3d(fig: &Figure) -> CaseResult {
    let theta = linspace(0.0, 2.0 * PI, 20);
    let ax = fig.subplot_3d();
    ax.stem(
        map(&theta, f64::sin),
        map(&theta, f64::cos),
        linspace(0.0, 1.0, theta.len()),
        Stem3DOptions::default(),
    )?;
    no_tick_labels(&ax)
}

fn surface3d_simple(fig: &Figure) -> CaseResult {
    let t = arange(-5.0, 5.0, 0.25);
    let (x, y) = meshgrid(&t, &t);
    let z = Grid::from_fn(t.len(), t.len(), |i, j| {
        (t[j] * t[j] + t[i] * t[i]).sqrt().sin()
    });
    let options = SurfaceOptions {
        colors: blues_from(minimum(z.as_slice()) * 2.0)?,
        ..SurfaceOptions::default()
    };
    let ax = fig.subplot_3d();
    ax.plot_surface(&x, &y, &z, options)?;
    no_tick_labels(&ax)
}

fn trisurf3d_simple(fig: &Figure) -> CaseResult {
    let radii = linspace(0.125, 1.0, 8);
    let angles = linspace_with(0.0, 2.0 * PI, 36, false);
    let (mut x, mut y) = (vec![0.0], vec![0.0]);
    for angle in &angles {
        for radius in &radii {
            x.push(radius * angle.cos());
            y.push(radius * angle.sin());
        }
    }
    let z: Vec<f64> = x.iter().zip(&y).map(|(x, y)| (-x * y).sin()).collect();
    let options = TrisurfOptions {
        colors: blues_from(minimum(&z) * 2.0)?,
        ..TrisurfOptions::default()
    };
    let ax = fig.subplot_3d();
    ax.plot_trisurf(&Triangulation::delaunay(&x, &y)?, &z, options)?;
    no_tick_labels(&ax)
}

fn voxels_simple(fig: &Figure) -> CaseResult {
    let voxelarray = Array3::from_fn([8, 8, 8], |[x, y, z]| {
        (x < 3 && y < 3 && z < 3) || (x >= 5 && y >= 5 && z >= 5)
    });
    let options = VoxelOptions {
        props: CollectionProps::default().edgecolor("k"),
        ..VoxelOptions::default()
    };
    let ax = fig.subplot_3d();
    ax.voxels(&voxelarray, options)?;
    no_tick_labels(&ax)
}

fn wire3d_simple(fig: &Figure) -> CaseResult {
    let (x, y, z) = test_data(0.05);
    let options = WireframeOptions {
        sampling: Sampling::Strides { rows: 10, cols: 10 },
        ..WireframeOptions::default()
    };
    let ax = fig.subplot_3d();
    ax.plot_wireframe(&x, &y, &z, options)?;
    no_tick_labels(&ax)
}

pub(super) const CASES: &[FigureCase] = &[
    gridded("gallery_plot_types/bar3d_simple", bar3d_simple),
    gridded(
        "gallery_plot_types/fill_between3d_simple",
        fill_between3d_simple,
    ),
    gridded("gallery_plot_types/plot3d_simple", plot3d_simple),
    gridded("gallery_plot_types/quiver3d_simple", quiver3d_simple),
    gridded("gallery_plot_types/scatter3d_simple", scatter3d_simple),
    gridded("gallery_plot_types/stem3d", stem3d),
    gridded("gallery_plot_types/surface3d_simple", surface3d_simple),
    gridded("gallery_plot_types/trisurf3d_simple", trisurf3d_simple),
    gridded("gallery_plot_types/voxels_simple", voxels_simple),
    gridded("gallery_plot_types/wire3d_simple", wire3d_simple),
];
