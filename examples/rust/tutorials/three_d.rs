//! matplotlib's "3D plotting" tutorial (`mpl_toolkits.mplot3d`) in pltrs
//! (Rust): lines, scatter plots, wireframes, surfaces, triangulated
//! surfaces, contours projected onto the walls, bars, arrows and text,
//! and the view angle.
//!
//! ```text
//! cargo run -p pltrs-examples --example three_d   # writes three_d.png here
//! ```

use std::f64::consts::PI;

use pltrs::collections::CollectionProps;
use pltrs::colors::ColorizerOptions;
use pltrs::contour::{ContourOptions, Levels};
use pltrs::layout::LayoutEngine;
use pltrs::mplot3d::{
    Axes3D, Axes3DOptions, Axis3, Bar3DOptions, Boxes, Contour3DOptions, FaceColors,
    Quiver3DOptions, Sampling, SurfaceOptions, TrisurfOptions, WireframeOptions,
};
use pltrs::prelude::*;
use pltrs::tri::Triangulation;

type Outcome = Result<(), Box<dyn std::error::Error>>;

fn main() -> Outcome {
    // `fig = plt.figure(); ax = fig.add_subplot(2, 3, i, projection="3d")`
    let fig = Figure::with_size(12.0, 8.0, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let panel = |index| fig.add_subplot_3d(2, 3, index, Axes3DOptions::default());
    lines_and_points(&panel(1)?)?;
    wireframe(&panel(2)?)?;
    surface_with_contours(&fig, &panel(3)?)?;
    trisurface(&panel(4)?)?;
    bars(&panel(5)?)?;
    arrows_and_text(&panel(6)?)?;
    fig.savefig("three_d.png")?;
    Ok(())
}

/// `ax.plot(x, y, z, label=...)` and `ax.scatter(xs, ys, zs, c=...)`.
fn lines_and_points(ax: &Axes3D) -> Outcome {
    let theta = linspace(-4.0 * PI, 4.0 * PI, 200);
    let z = linspace(-2.0, 2.0, 200);
    let r: Vec<f64> = z.iter().map(|z| z * z + 1.0).collect();
    let x: Vec<f64> = r.iter().zip(&theta).map(|(r, t)| r * t.sin()).collect();
    let y: Vec<f64> = r.iter().zip(&theta).map(|(r, t)| r * t.cos()).collect();
    ax.plot_with(
        &x,
        &y,
        &z,
        Axis3::Z,
        LineProps::default().label("parametric curve"),
    )?;
    let every: Vec<usize> = (0..200).step_by(10).collect();
    let pick = |values: &[f64]| every.iter().map(|&i| values[i]).collect::<Vec<f64>>();
    ax.scatter(pick(&x), pick(&y), pick(&z))?;
    ax.legend(LegendOptions::default())?;
    ax.set_title("plot and scatter");
    Ok(())
}

/// A sampled surface as a wire mesh (`ax.plot_wireframe(X, Y, Z,
/// rstride=10, cstride=10)`).
fn wireframe(ax: &Axes3D) -> Outcome {
    let (x, y, z) = bumps(60);
    let options = WireframeOptions {
        sampling: Sampling::Strides { rows: 5, cols: 5 },
        ..WireframeOptions::default()
    };
    ax.plot_wireframe(&x, &y, &z, options)?;
    ax.set_title("plot_wireframe");
    Ok(())
}

/// `ax.plot_surface(X, Y, Z, cmap="coolwarm")`, `fig.colorbar(surf)` and
/// filled contours on the floor (`ax.contourf(X, Y, Z, zdir="z",
/// offset=-1.5)`).
fn surface_with_contours(fig: &Figure, ax: &Axes3D) -> Outcome {
    let (x, y, z) = bumps(40);
    let options = SurfaceOptions {
        colors: FaceColors::from(ColorizerOptions::default().cmap(colormaps::get("coolwarm")?)),
        props: CollectionProps::default().linewidth(0.0),
        ..SurfaceOptions::default()
    };
    let surface = ax.plot_surface(&x, &y, &z, options)?;
    fig.colorbar(
        &surface,
        ColorbarOptions {
            shrink: 0.6,
            ..ColorbarOptions::default()
        },
    )?;
    let floor = Contour3DOptions {
        zdir: Axis3::Z,
        offset: Some(-1.5),
        contour: ContourOptions {
            levels: Levels::Count(8),
            ..ContourOptions::default()
        }
        .cmap(colormaps::get("coolwarm")?),
        ..Contour3DOptions::default()
    };
    ax.contourf(&x, &y, &z, floor)?;
    ax.set_zlim(-1.5, 1.5)?;
    ax.set_title("plot_surface + contourf");
    Ok(())
}

/// `ax.plot_trisurf(x, y, z, cmap=...)` over a Delaunay triangulation of
/// scattered points.
fn trisurface(ax: &Axes3D) -> Outcome {
    let mut radii = Vec::new();
    let mut angles = Vec::new();
    for ring in 1..=8 {
        let r = f64::from(ring) / 8.0;
        let count = 6 * ring;
        for k in 0..count {
            radii.push(r);
            angles.push(2.0 * PI * f64::from(k) / f64::from(count) + f64::from(ring) * 0.3);
        }
    }
    let x: Vec<f64> = radii
        .iter()
        .zip(&angles)
        .map(|(r, a)| r * a.cos())
        .collect();
    let y: Vec<f64> = radii
        .iter()
        .zip(&angles)
        .map(|(r, a)| r * a.sin())
        .collect();
    let z: Vec<f64> = x.iter().zip(&y).map(|(x, y)| (-(x * y)).sin()).collect();
    let tri = Triangulation::delaunay(&x, &y)?;
    let options = TrisurfOptions {
        colors: FaceColors::from(ColorizerOptions::default().cmap(colormaps::get("viridis")?)),
        ..TrisurfOptions::default()
    };
    ax.plot_trisurf(&tri, &z, options)?;
    ax.set_title("plot_trisurf");
    Ok(())
}

/// `ax.bar3d(x, y, bottom, width, depth, top, shade=True)`.
fn bars(ax: &Axes3D) -> Outcome {
    let (mut xs, mut ys, mut heights) = (Vec::new(), Vec::new(), Vec::new());
    for i in 0..4 {
        for j in 0..5 {
            xs.push(f64::from(i));
            ys.push(f64::from(j));
            heights.push(f64::from(1 + (i * 3 + j * 2) % 7));
        }
    }
    let n = xs.len();
    let boxes = Boxes::new(xs, ys, vec![0.0; n], vec![0.8; n], vec![0.8; n], heights);
    let options = Bar3DOptions {
        shade: true,
        ..Bar3DOptions::default()
    };
    ax.bar3d(boxes, options)?;
    ax.set_title("bar3d");
    Ok(())
}

/// `ax.quiver(x, y, z, u, v, w, length=0.3)`, `ax.text(x, y, z, s,
/// zdir)`, `ax.set_zlim` and `ax.view_init(elev=20, azim=-35)`.
fn arrows_and_text(ax: &Axes3D) -> Outcome {
    let mut positions = Vec::new();
    let mut vectors = Vec::new();
    for i in 0..4 {
        for j in 0..4 {
            let (x, y) = (f64::from(i) / 3.0 - 0.5, f64::from(j) / 3.0 - 0.5);
            positions.push([x, y, 0.0]);
            vectors.push([-y, x, 0.5]);
        }
    }
    let options = Quiver3DOptions {
        length: 0.3,
        ..Quiver3DOptions::default()
    };
    ax.quiver(&positions, &vectors, options)?;
    // As in matplotlib, the limits follow the tails of the arrows only.
    ax.set_zlim(-0.2, 0.8)?;
    ax.text(-0.5, -0.5, 0.6, "along x", Some(Axis3::X));
    ax.text(0.5, 0.5, 0.6, "facing the viewer", None);
    ax.view_init(20.0, -35.0, 0.0);
    ax.set_xlabel("x").set_ylabel("y").set_zlabel("z");
    ax.set_title("quiver, text, view_init");
    Ok(())
}

/// Two Gaussian bumps sampled on an `n × n` grid over `[-3, 3]²`.
fn bumps(n: usize) -> (Grid<f64>, Grid<f64>, Grid<f64>) {
    let axis = linspace(-3.0, 3.0, n);
    let (x, y) = meshgrid(&axis, &axis);
    let z = Grid::from_fn(n, n, |r, c| {
        let (x, y) = (x[(r, c)], y[(r, c)]);
        (-(x - 1.0).powi(2) - y * y).exp() - (-(x + 1.0).powi(2) - (y - 1.0).powi(2)).exp()
    });
    (x, y, z)
}
