//! Contours, vector fields and triangulations, and 3-D axes.

use plt::collections::CollectionProps;
use plt::colorbar::ColorbarOptions;
use plt::colors::{Color, Colorize, ColorizerOptions, colormaps};
use plt::contour::{ClabelOptions, ContourOptions, Levels, QuadField};
use plt::mplot3d::{FaceColors, Sampling, SurfaceOptions};
use plt::numeric::{Grid, linspace, meshgrid};
use plt::quiver::{QuiverOptions, VectorField};
use plt::streamplot::{StreamField, StreamplotOptions};
use plt::tri::Triangulation;
use plt_spec::rng::SpecRng;

use super::{Build, Workload, smooth_field};

/// The workloads of this module.
pub(super) const WORKLOADS: &[Workload] = &[
    Workload::new("fields/contour_1000", || contours(false)),
    Workload::new("fields/contourf_1000", || contours(true)),
    Workload::new("fields/contourf_labels_1000", contourf_labels),
    Workload::new("fields/streamplot_100", streamplot),
    Workload::new("fields/quiver_100", quiver),
    Workload::new("fields/tricontourf_1e5", tricontourf),
    Workload::new("3d/plot_surface_200", plot_surface),
    Workload::new("3d/plot_surface_200_full", plot_surface_full),
    Workload::new("3d/scatter_1e4", scatter_3d),
];

/// `contour` (or `contourf`) of a smooth 1000 × 1000 field at 20 levels.
fn contours(filled: bool) -> Build {
    let field = QuadField::new(smooth_field(1000));
    Box::new(move |fig| {
        let ax = fig.subplot();
        let options = ContourOptions {
            levels: Levels::Count(20),
            ..ContourOptions::default()
        };
        if filled {
            ax.contourf(&field, options)?;
        } else {
            ax.contour(&field, options)?;
        }
        Ok(())
    })
}

/// `contourf` of a smooth 1000 × 1000 field at 20 levels, black contour
/// lines at the same levels, labelled inline, and a colorbar.
fn contourf_labels() -> Build {
    let field = QuadField::new(smooth_field(1000));
    Box::new(move |fig| {
        let ax = fig.subplot();
        let filled = ax.contourf(
            &field,
            ContourOptions {
                levels: Levels::Count(20),
                ..ContourOptions::default()
            },
        )?;
        let lines = ax.contour(
            &field,
            ContourOptions {
                levels: Levels::Values(filled.levels()),
                colors: Some(vec![Color::from("k")]),
                linewidths: Some(vec![0.5]),
                ..ContourOptions::default()
            },
        )?;
        lines.clabel(ClabelOptions {
            fontsize: Some(8.0.into()),
            ..ClabelOptions::default()
        })?;
        fig.colorbar(&filled, ColorbarOptions::default())?;
        Ok(())
    })
}

/// A rotating field with a saddle on an `n × n` grid over `[-3, 3]²`:
/// the coordinates and the `u` and `v` components.
fn swirl(n: usize) -> (Vec<f64>, Grid<f64>, Grid<f64>) {
    let t = linspace(-3.0, 3.0, n);
    let u = Grid::from_fn(n, n, |i, j| -1.0 - t[j] * t[j] + t[i]);
    let v = Grid::from_fn(n, n, |i, j| 1.0 + t[j] - t[i] * t[i]);
    (t, u, v)
}

/// `streamplot` on a 100 × 100 grid.
fn streamplot() -> Build {
    let (t, u, v) = swirl(100);
    let field = StreamField::new(t.clone(), t, u, v);
    Box::new(move |fig| {
        fig.subplot()
            .streamplot(&field, StreamplotOptions::default())?;
        Ok(())
    })
}

/// `quiver` of 100 × 100 arrows.
fn quiver() -> Build {
    let (t, u, v) = swirl(100);
    let (x, y) = meshgrid(&t, &t);
    Box::new(move |fig| {
        let field = VectorField::on_mesh(&x, &y, &u, &v);
        fig.subplot().quiver(field, QuiverOptions::default())?;
        Ok(())
    })
}

/// `tricontourf` of a bump over 10⁵ uniform points, triangulated (Delaunay)
/// as part of the work.
fn tricontourf() -> Build {
    let mut rng = SpecRng::new(11);
    let x = rng.uniform(100_000, -3.0, 3.0);
    let y = rng.uniform(100_000, -3.0, 3.0);
    let z: Vec<f64> = x
        .iter()
        .zip(&y)
        .map(|(x, y)| (-(x * x) - y * y).exp() + 0.3 * (2.0 * x).sin())
        .collect();
    Box::new(move |fig| {
        let tri = Triangulation::delaunay(&x, &y)?;
        fig.subplot()
            .tricontourf(&tri, &z, ContourOptions::default())?;
        Ok(())
    })
}

/// `sin(r)` on a 200 × 200 grid over `[-5, 5]²`: the x, y and z grids.
fn ripple() -> (Grid<f64>, Grid<f64>, Grid<f64>) {
    let t = linspace(-5.0, 5.0, 200);
    let (x, y) = meshgrid(&t, &t);
    let z = Grid::from_fn(200, 200, |i, j| t[j].hypot(t[i]).sin());
    (x, y, z)
}

/// `plot_surface` of `sin(r)` on a 200 × 200 grid in viridis.
fn plot_surface() -> Build {
    let (x, y, z) = ripple();
    Box::new(move |fig| {
        let options = SurfaceOptions {
            colors: FaceColors::from(ColorizerOptions::default().cmap(colormaps::get("viridis")?)),
            ..SurfaceOptions::default()
        };
        fig.subplot_3d().plot_surface(&x, &y, &z, options)?;
        Ok(())
    })
}

/// `plot_surface` of `sin(r)` with every grid cell a face (strides of 1),
/// in coolwarm without edges or antialiasing.
fn plot_surface_full() -> Build {
    let (x, y, z) = ripple();
    Box::new(move |fig| {
        let options = SurfaceOptions {
            sampling: Sampling::Strides { rows: 1, cols: 1 },
            colors: FaceColors::from(ColorizerOptions::default().cmap(colormaps::get("coolwarm")?)),
            props: CollectionProps {
                antialiased: Some(vec![false]),
                ..CollectionProps::default().linewidth(0.0)
            },
            ..SurfaceOptions::default()
        };
        fig.subplot_3d().plot_surface(&x, &y, &z, options)?;
        Ok(())
    })
}

/// A 3-D scatter of 10⁴ uniform points.
fn scatter_3d() -> Build {
    let mut rng = SpecRng::new(12);
    let x = rng.uniform(10_000, 0.0, 1.0);
    let y = rng.uniform(10_000, 0.0, 1.0);
    let z = rng.uniform(10_000, 0.0, 1.0);
    Box::new(move |fig| {
        fig.subplot_3d().scatter(&x, &y, &z)?;
        Ok(())
    })
}
