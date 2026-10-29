//! 3-D axes: surfaces, wireframes, scatter plots, bars, contours on the
//! panes, voxels, curves and triangulated surfaces.

use std::f64::consts::PI;

use pltrs::axes::Adjustable;
use pltrs::collections::CollectionProps;
use pltrs::colors::ColorizerOptions;
use pltrs::contour::ContourOptions;
use pltrs::mplot3d::{
    Array3, Aspect3D, Axes3D, Axis3, Bar3DOptions, Boxes, Contour3DOptions, FaceColors, Sampling,
    Scatter3DOptions, SurfaceOptions, TrisurfOptions, VoxelColors, VoxelOptions, WireframeOptions,
    test_data,
};
use pltrs::prelude::*;
use pltrs::tri::Triangulation;

use super::data::Samples;
use super::{Built, Entry, background};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry::figure("surface_3d", "A 3-D surface with a colorbar", surface_3d),
    Entry::figure("wireframe_3d", "A wireframe", wireframe_3d),
    Entry::figure("scatter_3d", "A 3-D scatter coloured by height", scatter_3d),
    Entry::figure("bars_3d", "3-D bars (bar3d)", bars_3d),
    Entry::figure(
        "contours_3d",
        "A translucent surface with its contours on the panes",
        contours_3d,
    ),
    Entry::figure("voxels", "Voxels: a sphere carved from a cube", voxels),
    Entry::figure("curve_3d", "A parametric curve: a trefoil knot", curve_3d),
    Entry::figure(
        "trisurf_3d",
        "A triangulated surface (plot_trisurf)",
        trisurf_3d,
    ),
];

/// Labels the three axes.
fn label_xyz(ax: &Axes3D) {
    ax.set_xlabel("x").set_ylabel("y").set_zlabel("z");
}

/// `sin(r) / r` over a square, coloured by height, with a colorbar
/// shrunk to the height of the box.
fn surface_3d() -> Built {
    let fig = Figure::with_size(6.4, 5.0, 100.0);
    let ax = fig.subplot_3d();
    let x = linspace(-8.0, 8.0, 60);
    let (xx, yy) = meshgrid(&x, &x);
    let z = Grid::from_fn(60, 60, |i, j| {
        let r = xx[(i, j)].hypot(yy[(i, j)]);
        if r == 0.0 { 1.0 } else { r.sin() / r }
    });
    let options = SurfaceOptions {
        colors: FaceColors::from(ColorizerOptions::default().cmap(colormaps::get("viridis")?)),
        props: CollectionProps::default().linewidth(0.0),
        ..SurfaceOptions::default()
    };
    let surface = ax.plot_surface(&xx, &yy, &z, options)?;
    let colorbar = ColorbarOptions {
        shrink: 0.6,
        label: "height".into(),
        ..ColorbarOptions::default()
    };
    fig.colorbar(&surface, colorbar)?;
    ax.set_title("sin(r) / r");
    Ok(fig)
}

/// A surface drawn as lines only: every few rows and columns
/// of the grid, which shows the shape without hiding what is behind.
fn wireframe_3d() -> Built {
    let fig = Figure::with_size(6.0, 5.0, 100.0);
    let ax = fig.subplot_3d();
    let (x, y, z) = test_data(0.1);
    let options = WireframeOptions {
        sampling: Sampling::Strides { rows: 3, cols: 3 },
        props: CollectionProps::default().linewidth(0.8),
        ..WireframeOptions::default()
    };
    ax.plot_wireframe(&x, &y, &z, options)?;
    label_xyz(&ax);
    ax.set_title("plot_wireframe");
    Ok(fig)
}

/// Points on a noisy spiral, coloured by height; markers further from
/// the viewer fade (depth shading).
fn scatter_3d() -> Built {
    let fig = Figure::with_size(6.0, 5.0, 100.0);
    let ax = fig.subplot_3d();
    let mut samples = Samples::new(31);
    let t: Vec<f64> = (0..400).map(|_| 6.0 * PI * samples.uniform()).collect();
    let jitter = |samples: &mut Samples| samples.normal(0.0, 0.15);
    let x: Vec<f64> = t
        .iter()
        .map(|t| t.cos() * (1.0 + t / 10.0) + jitter(&mut samples))
        .collect();
    let y: Vec<f64> = t
        .iter()
        .map(|t| t.sin() * (1.0 + t / 10.0) + jitter(&mut samples))
        .collect();
    let z: Vec<f64> = t.iter().map(|t| t / PI).collect();
    let options = Scatter3DOptions {
        scatter: ScatterOptions {
            colors: MappedColors::new(&z).cmap(colormaps::get("plasma")?).into(),
            ..ScatterOptions::default()
        },
        ..Scatter3DOptions::default()
    };
    ax.scatter_with(&x, &y, &z, options)?;
    label_xyz(&ax);
    ax.set_title("A noisy spiral");
    Ok(fig)
}

/// Monthly figures of four years as a block of bars, one row per year.
fn bars_3d() -> Built {
    let fig = Figure::with_size(6.4, 5.0, 100.0);
    let ax = fig.subplot_3d();
    let (months, years) = (12, 4);
    let (x, y): (Vec<f64>, Vec<f64>) = (0..years)
        .flat_map(|year| (0..months).map(move |month| (f64::from(month), f64::from(year))))
        .unzip();
    let heights: Vec<f64> = x
        .iter()
        .zip(&y)
        .map(|(m, year)| 5.0 + 3.0 * (m * PI / 6.0).sin() + 1.5 * year)
        .collect();
    let colors = y
        .iter()
        .map(|year| Color::from(format!("C{year}")))
        .collect();
    let boxes = Boxes::new(&x, &y, [0.0], [0.7], [0.5], &heights);
    let options = Bar3DOptions {
        color: colors,
        ..Bar3DOptions::default()
    };
    ax.bar3d(boxes, options)?;
    ax.yaxis().set_ticks([0.25, 1.25, 2.25, 3.25]);
    ax.yaxis()
        .set_ticklabels(["2022", "2023", "2024", "2025"])?;
    ax.set_xlabel("month").set_zlabel("sales");
    ax.set_title("bar3d");
    Ok(fig)
}

/// A translucent surface with its contours projected onto the three
/// panes behind it: `offset` moves each set of contour lines into the
/// plane of a pane.
fn contours_3d() -> Built {
    let fig = Figure::with_size(6.0, 5.0, 100.0);
    let ax = fig.subplot_3d();
    let (x, y, z) = test_data(0.05);
    let surface = SurfaceOptions {
        sampling: Sampling::Strides { rows: 8, cols: 8 },
        props: CollectionProps::default()
            .edgecolor("royalblue")
            .linewidth(0.5)
            .alpha(0.3),
        ..SurfaceOptions::default()
    };
    ax.plot_surface(&x, &y, &z, surface)?;
    for (zdir, offset) in [(Axis3::Z, -100.0), (Axis3::X, -40.0), (Axis3::Y, 40.0)] {
        let options = Contour3DOptions {
            zdir,
            offset: Some(offset),
            contour: ContourOptions::default().cmap(colormaps::get("coolwarm")?),
            ..Contour3DOptions::default()
        };
        ax.contour(&x, &y, &z, options)?;
    }
    ax.set_xlim(-40.0, 40.0)?
        .set_ylim(-40.0, 40.0)?
        .set_zlim(-100.0, 100.0)?;
    label_xyz(&ax);
    ax.set_title("Contours on the panes");
    Ok(fig)
}

/// A 10 × 10 × 10 block of voxels with a sphere cut out of the front corner,
/// each voxel coloured by its height.
fn voxels() -> Built {
    let fig = Figure::with_size(6.0, 5.0, 100.0);
    let ax = fig.subplot_3d();
    let n = 10;
    // A sphere of radius 7 centred on the corner nearest the viewer.
    let inside_sphere = |[i, j, k]: [usize; 3]| {
        let (x, y, z) = (
            i as f64 + 0.5 - n as f64,
            j as f64 + 0.5,
            k as f64 + 0.5 - n as f64,
        );
        x * x + y * y + z * z < 49.0
    };
    let filled = Array3::from_fn([n; 3], |cell| !inside_sphere(cell));
    let viridis = colormaps::get("viridis")?;
    let colors = Array3::from_fn([n; 3], |[_, _, k]| {
        Color::from(viridis.sample(k as f64 / 9.0))
    });
    let options = VoxelOptions {
        facecolors: Some(VoxelColors::PerCell(colors)),
        props: CollectionProps::default()
            .edgecolor(background())
            .linewidth(0.3),
        ..VoxelOptions::default()
    };
    ax.voxels(&filled, options)?;
    ax.set_aspect(Aspect3D::Equal, Adjustable::Box)?;
    ax.set_title("voxels");
    Ok(fig)
}

/// A trefoil knot traced by `(sin t + 2 sin 2t, cos t − 2 cos 2t,
/// −sin 3t)`, with its shadow on the floor.
fn curve_3d() -> Built {
    let fig = Figure::with_size(6.0, 5.0, 100.0);
    let ax = fig.subplot_3d();
    let t = linspace(0.0, 2.0 * PI, 600);
    let x: Vec<f64> = t.iter().map(|t| t.sin() + 2.0 * (2.0 * t).sin()).collect();
    let y: Vec<f64> = t.iter().map(|t| t.cos() - 2.0 * (2.0 * t).cos()).collect();
    let z: Vec<f64> = t.iter().map(|t| -(3.0 * t).sin()).collect();
    ax.plot_with(
        &x,
        &y,
        vec![-1.5; t.len()],
        Axis3::Z,
        LineProps::default().color("0.8"),
    )?;
    ax.plot_with(&x, &y, &z, Axis3::Z, LineProps::default().linewidth(2.5))?;
    ax.set_zlim(-1.5, 1.5)?;
    label_xyz(&ax);
    ax.set_title("A trefoil knot");
    Ok(fig)
}

/// Scattered points on rings, triangulated in the plane and lifted to
/// `z = cos(3θ) r`: a surface without a regular grid.
fn trisurf_3d() -> Built {
    let fig = Figure::with_size(6.0, 5.0, 100.0);
    let ax = fig.subplot_3d();
    let (mut x, mut y) = (vec![0.0], vec![0.0]);
    for ring in 1..=8 {
        let r = f64::from(ring) / 8.0;
        let count = 6 * ring;
        for k in 0..count {
            let angle = 2.0 * PI * f64::from(k) / f64::from(count);
            x.push(r * angle.cos());
            y.push(r * angle.sin());
        }
    }
    let z: Vec<f64> = x
        .iter()
        .zip(&y)
        .map(|(x, y)| (3.0 * y.atan2(*x)).cos() * x.hypot(*y))
        .collect();
    let options = TrisurfOptions {
        colors: FaceColors::from(ColorizerOptions::default().cmap(colormaps::get("Spectral")?)),
        props: CollectionProps::default().edgecolor("0.3").linewidth(0.2),
        ..TrisurfOptions::default()
    };
    ax.plot_trisurf(&Triangulation::delaunay(&x, &y)?, &z, options)?;
    for axis in [ax.xaxis(), ax.yaxis(), ax.zaxis()] {
        axis.set_ticks([-1.0, -0.5, 0.0, 0.5, 1.0]);
    }
    label_xyz(&ax);
    ax.set_title("plot_trisurf");
    Ok(fig)
}
