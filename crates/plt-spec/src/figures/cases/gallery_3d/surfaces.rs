//! Surfaces: `surface3d`, `surface3d_2`, `surface3d_3`,
//! `surface3d_radial`, `offset`, `custom_shaded_3d_surface`,
//! `mixed_subplots` and `subplot3d`.

use std::f64::consts::PI;

use plt::axes::Adjustable;
use plt::collections::CollectionProps;
use plt::colorbar::ColorbarOptions;
use plt::colors::{BlendMode, Color, LightSource, Relief, ShadeOptions, colormaps};
use plt::figure::Figure;
use plt::lines::LineProps;
use plt::mplot3d::{
    Aspect3D, Axes3DOptions, Sampling, SurfaceOptions, WireframeOptions, test_data,
};
use plt::numeric::{Grid, arange, linspace, meshgrid};
use plt::ticker::{LinearLocator, StrMethodFormatter};

use super::{EVERY_POINT, label_xyz, sinc_surface};
use crate::figures::cases::common::{aliased, face_cmap};
use crate::figures::{CaseResult, FigureCase};

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_3d/surface3d", surface3d),
    FigureCase::new("gallery_3d/surface3d_2", surface3d_2),
    FigureCase::new("gallery_3d/surface3d_3", surface3d_3),
    FigureCase::new("gallery_3d/surface3d_radial", surface3d_radial),
    FigureCase::new("gallery_3d/offset", offset),
    FigureCase::new(
        "gallery_3d/custom_shaded_3d_surface",
        custom_shaded_3d_surface,
    ),
    FigureCase::new("gallery_3d/mixed_subplots", mixed_subplots).sized(4.0, 8.0, 100.0),
    FigureCase::new("gallery_3d/subplot3d", subplot3d).sized(9.6, 4.8, 100.0),
];

fn surface3d(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let [x, y, z] = sinc_surface();
    let options = SurfaceOptions {
        colors: face_cmap("coolwarm")?,
        props: aliased(0.0),
        ..SurfaceOptions::default()
    };
    let surface = ax.plot_surface(&x, &y, &z, options)?;
    ax.set_zlim(-1.01, 1.01)?;
    ax.zaxis()
        .set_major_locator(LinearLocator::new(Some(10)))
        .set_major_formatter(StrMethodFormatter::new("{x:.02f}")?);
    let colorbar = ColorbarOptions {
        shrink: 0.5,
        aspect: 5.0,
        ..ColorbarOptions::default()
    };
    fig.colorbar(&surface, colorbar)?;
    Ok(())
}

fn surface3d_2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let u = linspace(0.0, 2.0 * PI, 100);
    let v = linspace(0.0, PI, 100);
    let outer = |f: &dyn Fn(f64, f64) -> f64| Grid::from_fn(100, 100, |i, j| 10.0 * f(u[i], v[j]));
    let x = outer(&|u, v| u.cos() * v.sin());
    let y = outer(&|u, v| u.sin() * v.sin());
    let z = outer(&|_, v| v.cos());
    ax.plot_surface(&x, &y, &z, SurfaceOptions::default())?;
    ax.set_aspect(Aspect3D::Equal, Adjustable::Box)?;
    Ok(())
}

fn surface3d_3(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let [x, y, z] = sinc_surface();
    let checker = Grid::from_fn(x.rows(), x.cols(), |row, col| {
        Color::from(if (row + col) % 2 == 0 { "y" } else { "b" })
    });
    let options = SurfaceOptions {
        cell_colors: Some(checker),
        props: CollectionProps::default().linewidth(0.0),
        ..SurfaceOptions::default()
    };
    ax.plot_surface(&x, &y, &z, options)?;
    ax.set_zlim(-1.0, 1.0)?;
    ax.zaxis().set_major_locator(LinearLocator::new(Some(6)));
    Ok(())
}

fn surface3d_radial(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let (r, p) = meshgrid(&linspace(0.0, 1.25, 50), &linspace(0.0, 2.0 * PI, 50));
    let z = r.map(|r| (r.powi(2) - 1.0).powi(2));
    let x = Grid::from_fn(50, 50, |i, j| r[(i, j)] * p[(i, j)].cos());
    let y = Grid::from_fn(50, 50, |i, j| r[(i, j)] * p[(i, j)].sin());
    let options = SurfaceOptions {
        colors: face_cmap("YlGnBu_r")?,
        ..SurfaceOptions::default()
    };
    ax.plot_surface(&x, &y, &z, options)?;
    ax.set_zlim(0.0, 1.0)?;
    label_xyz(
        &ax,
        [r"$\phi_\mathrm{real}$", r"$\phi_\mathrm{im}$", r"$V(\phi)$"],
    );
    Ok(())
}

fn offset(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    // np.mgrid[0:6 * np.pi:0.25, 0:4 * np.pi:0.25]
    let rows = arange(0.0, 6.0 * PI, 0.25);
    let cols = arange(0.0, 4.0 * PI, 0.25);
    let (n, m) = (rows.len(), cols.len());
    let x = Grid::from_fn(n, m, |i, _| rows[i] + 1e5);
    let y = Grid::from_fn(n, m, |_, j| cols[j] + 1e5);
    let z = Grid::from_fn(n, m, |i, j| (rows[i].cos() + cols[j].cos()).abs().sqrt());
    let options = SurfaceOptions {
        sampling: Sampling::Strides { rows: 2, cols: 2 },
        colors: face_cmap("autumn")?,
        ..SurfaceOptions::default()
    };
    ax.plot_surface(&x, &y, &z, options)?;
    label_xyz(&ax, ["X label", "Y label", "Z label"]);
    ax.set_zlim(0.0, 2.0)?;
    Ok(())
}

/// A stand-in for the part of `jacksboro_fault_dem.npz` the example
/// shows: the same 45 × 45 grid of longitudes and latitudes, and whole
/// elevations from about 370 to 730 m (a ridge cut by a valley).
fn terrain() -> [Grid<f64>; 3] {
    let x = linspace(-84.413_75, -84.077_916_666_666_67, 403);
    let y = linspace(36.732_916_666_666_67, 36.446_25, 344);
    let (x, y) = meshgrid(&x[5..50], &y[5..50]);
    let t = linspace(0.0, 1.0, 45);
    let z = Grid::from_fn(45, 45, |i, j| {
        let (u, v) = (t[j], t[i]);
        let ridge = 120.0 * (5.0 * u + 2.0 * v).sin() * (3.0 * v).cos();
        let valley = 100.0 * (-((u - v - 0.1) / 0.15).powi(2)).exp();
        (560.0 + ridge - valley).round_ties_even()
    });
    [x, y, z]
}

fn custom_shaded_3d_surface(fig: &Figure) -> CaseResult {
    let [x, y, z] = terrain();
    let ax = fig.subplot_3d();
    let shading = ShadeOptions {
        blend: BlendMode::SoftLight,
        relief: Relief {
            vert_exag: 0.1,
            ..Relief::default()
        },
        ..ShadeOptions::default()
    };
    let rgb = LightSource::new(270.0, 45.0).shade(&z, &colormaps::get("gist_earth")?, &shading)?;
    let options = SurfaceOptions {
        sampling: EVERY_POINT,
        cell_colors: Some(rgb.map(|&c| Color::from(c))),
        shade: Some(false),
        props: aliased(0.0),
        ..SurfaceOptions::default()
    };
    ax.plot_surface(&x, &y, &z, options)?;
    Ok(())
}

fn mixed_subplots(fig: &Figure) -> CaseResult {
    let f = |t: &f64| (2.0 * PI * t).cos() * (-t).exp();
    fig.suptitle("A tale of 2 subplots");
    let ax = fig.add_subplot(2, 1, 1)?;
    let t1 = arange(0.0, 5.0, 0.1);
    let t2 = arange(0.0, 5.0, 0.02);
    for (t, format) in [(&t1, "bo"), (&t2, "k--")] {
        let y: Vec<f64> = t.iter().map(f).collect();
        let props = LineProps::fmt(format)?.markerfacecolor("green");
        ax.plot_with(t.clone(), y, props)?;
    }
    ax.grid(true);
    ax.set_ylabel("Damped oscillation");
    let ax = fig.add_subplot_3d(2, 1, 2, Axes3DOptions::default())?;
    let [x, y, z] = sinc_surface();
    let options = SurfaceOptions {
        sampling: EVERY_POINT,
        props: aliased(0.0),
        ..SurfaceOptions::default()
    };
    ax.plot_surface(&x, &y, &z, options)?;
    ax.set_zlim(-1.0, 1.0)?;
    Ok(())
}

fn subplot3d(fig: &Figure) -> CaseResult {
    let ax = fig.add_subplot_3d(1, 2, 1, Axes3DOptions::default())?;
    let [x, y, z] = sinc_surface();
    let options = SurfaceOptions {
        sampling: EVERY_POINT,
        colors: face_cmap("coolwarm")?,
        props: aliased(0.0),
        ..SurfaceOptions::default()
    };
    let surface = ax.plot_surface(&x, &y, &z, options)?;
    ax.set_zlim(-1.01, 1.01)?;
    let colorbar = ColorbarOptions {
        shrink: 0.5,
        aspect: 10.0,
        ..ColorbarOptions::default()
    };
    fig.colorbar(&surface, colorbar)?;
    let ax = fig.add_subplot_3d(1, 2, 2, Axes3DOptions::default())?;
    let (x, y, z) = test_data(0.05);
    let options = WireframeOptions {
        sampling: Sampling::Strides { rows: 10, cols: 10 },
        ..WireframeOptions::default()
    };
    ax.plot_wireframe(&x, &y, &z, options)?;
    Ok(())
}
