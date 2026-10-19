//! The tests of non-linear scales on 3-D axes: every artist on log axes,
//! all the scales, data the scale rejects, log bases and subs, and
//! symlog thresholds.

use std::f64::consts::{E, PI};

use plt::collections::CollectionProps;
use plt::colors::colormaps;
use plt::contour::{ContourOptions, Levels};
use plt::mplot3d::{
    Axes3D, Axes3DError, Axes3DOptions, Axis3, Bar3DOptions, Boxes, Contour3DOptions, FaceColormap,
    FaceColors, Quiver3DOptions, Sampling, Scatter3DOptions, Stem3DOptions, SurfaceOptions,
    TrisurfOptions, WireframeOptions,
};
use plt::numeric::Grid;
use plt::prelude::*;
use plt::scale::{FuncTransform, ScaleOptions};
use plt::ticker::FuncFormatter;
use plt::tri::Triangulation;

use super::subplots_3d;
use crate::figures::CaseResult;
use crate::rng::SpecRng;

/// `_make_log_data`: a closed curve spanning 1 to about 1000.
fn log_data() -> [Vec<f64>; 3] {
    let t = linspace(0.0, 2.0 * PI, 50);
    [
        t.iter().map(|t| 10_f64.powf(t / 2.0)).collect(),
        t.iter().map(|t| 10_f64.powf(1.0 + t.sin())).collect(),
        t.iter()
            .map(|t| 10_f64.powf(2.0 * (1.0 + t.cos() / 2.0)))
            .collect(),
    ]
}

/// `_make_surface_log_data`: `z = xy` on a 20 × 20 grid over `[1, 10]²`.
fn surface_log_data() -> [Grid<f64>; 3] {
    let x = linspace(1.0, 10.0, 20);
    let (xx, yy) = meshgrid(&x, &x);
    let zz = Grid::from_fn(20, 20, |i, j| xx[(i, j)] * yy[(i, j)]);
    [xx, yy, zz]
}

/// `_make_triangulation_data`: random points with `z = xy / 10`.
fn triangulation_data() -> [Vec<f64>; 3] {
    let mut rng = SpecRng::new(42);
    let x = rng.uniform(100, 1.0, 100.0);
    let y = rng.uniform(100, 1.0, 100.0);
    let z = x.iter().zip(&y).map(|(x, y)| x * y / 10.0).collect();
    [x, y, z]
}

/// The 3-D axes in cell `index` of the 3 × 4 grid of
/// `scale3d_artists_log`.
fn panel(fig: &Figure, index: usize) -> Result<Axes3D, Axes3DError> {
    fig.add_subplot_3d(3, 4, index, Axes3DOptions::default())
}

/// `ax.set(xscale="log", yscale="log", zscale="log", title=title)`.
fn log_axes(ax: &Axes3D, title: &str) -> Result<(), Axes3DError> {
    for dir in Axis3::ALL {
        ax.set_scale_with(dir, "log", &ScaleOptions::default())?;
    }
    ax.set_title(title);
    Ok(())
}

/// Viridis for colour-mapped surfaces.
fn viridis_faces() -> Result<FaceColors, Box<dyn std::error::Error>> {
    Ok(FaceColors::Mapped(Box::new(FaceColormap::new(
        colormaps::get("viridis")?,
    ))))
}

/// The first row of `scale3d_artists_log`: plot, wireframe, scatter and
/// bar3d.
fn lines_points_bars(fig: &Figure) -> CaseResult {
    let [x, y, z] = log_data();
    let [sx, sy, sz] = surface_log_data();
    let ax = panel(fig, 1)?;
    ax.plot(&x, &y, &z)?;
    log_axes(&ax, "plot")?;

    let ax = panel(fig, 2)?;
    let options = WireframeOptions {
        sampling: Sampling::Strides { rows: 5, cols: 5 },
        ..WireframeOptions::default()
    };
    ax.plot_wireframe(&sx, &sy, &sz, options)?;
    log_axes(&ax, "wireframe")?;

    let ax = panel(fig, 3)?;
    let options = Scatter3DOptions {
        scatter: ScatterOptions {
            colors: MappedColors::new(z.clone())
                .with_cmap(colormaps::get("viridis")?)
                .into(),
            ..ScatterOptions::default()
        },
        ..Scatter3DOptions::default()
    };
    ax.scatter_with(&x, &y, &z, options)?;
    log_axes(&ax, "scatter")?;

    let ax = panel(fig, 4)?;
    let decades = [1.0, 10.0, 100.0];
    let (bx, by): (Vec<f64>, Vec<f64>) = decades
        .iter()
        .flat_map(|&y| decades.iter().map(move |&x| (x, y)))
        .unzip();
    let scaled = |v: &[f64]| v.iter().map(|v| v * 0.3).collect::<Vec<f64>>();
    let heights: Vec<f64> = bx.iter().zip(&by).map(|(x, y)| x * y / 10.0).collect();
    let boxes = Boxes {
        x: &bx,
        y: &by,
        z: &[1.0],
        dx: &scaled(&bx),
        dy: &scaled(&by),
        dz: &heights,
    };
    let options = Bar3DOptions {
        props: CollectionProps::default().alpha(0.8),
        ..Bar3DOptions::default()
    };
    ax.bar3d(boxes, options)?;
    log_axes(&ax, "bar3d")?;
    Ok(())
}

/// The second row of `scale3d_artists_log`: surface, trisurf, contour
/// and contourf.
fn surfaces_and_contours(fig: &Figure) -> CaseResult {
    let [sx, sy, sz] = surface_log_data();
    let ax = panel(fig, 5)?;
    let options = SurfaceOptions {
        colors: viridis_faces()?,
        props: CollectionProps::default().alpha(0.8),
        ..SurfaceOptions::default()
    };
    ax.plot_surface(&sx, &sy, &sz, options)?;
    log_axes(&ax, "surface")?;

    let ax = panel(fig, 6)?;
    let [tx, ty, tz] = triangulation_data();
    let options = TrisurfOptions {
        colors: viridis_faces()?,
        props: CollectionProps::default().alpha(0.8),
        ..TrisurfOptions::default()
    };
    ax.plot_trisurf(&Triangulation::delaunay(&tx, &ty)?, &tz, options)?;
    log_axes(&ax, "trisurf")?;

    let ax = panel(fig, 7)?;
    let options = Contour3DOptions {
        contour: ContourOptions {
            levels: Levels::Count(10),
            ..ContourOptions::default()
        },
        ..Contour3DOptions::default()
    };
    ax.contour(&sx, &sy, &sz, options)?;
    log_axes(&ax, "contour")?;

    let ax = panel(fig, 8)?;
    let options = Contour3DOptions {
        contour: ContourOptions {
            levels: Levels::Count(10),
            alpha: Some(0.8),
            ..ContourOptions::default()
        },
        ..Contour3DOptions::default()
    };
    ax.contourf(&sx, &sy, &sz, options)?;
    log_axes(&ax, "contourf")?;
    Ok(())
}

/// The third row of `scale3d_artists_log`: stem, quiver and text.
fn stems_arrows_texts(fig: &Figure) -> CaseResult {
    let decades = [1.0, 10.0, 100.0];
    let ax = panel(fig, 9)?;
    let options = Stem3DOptions {
        bottom: 1.0,
        ..Stem3DOptions::default()
    };
    ax.stem(&decades, &decades, &[10.0, 100.0, 1000.0], &options)?;
    log_axes(&ax, "stem")?;

    let ax = panel(fig, 10)?;
    let positions = decades.map(|q| [q; 3]);
    let vectors = decades.map(|q| [q * 0.5; 3]);
    ax.quiver(&positions, &vectors, Quiver3DOptions::default())?;
    log_axes(&ax, "quiver")?;

    let ax = panel(fig, 11)?;
    for (q, name) in decades.into_iter().zip(["Point A", "Point B", "Point C"]) {
        ax.text(q, q, q, name, None);
    }
    log_axes(&ax, "text")?;
    for dir in Axis3::ALL {
        ax.set_limits(dir, 0.5, 200.0)?;
    }
    Ok(())
}

pub(super) fn scale3d_artists_log(fig: &Figure) -> CaseResult {
    lines_points_bars(fig)?;
    surfaces_and_contours(fig)?;
    stems_arrows_texts(fig)
}

pub(super) fn scale3d_all_scales(fig: &Figure) -> CaseResult {
    let axs = subplots_3d(fig, 1, 2)?;
    let t = linspace(0.1, 0.9, 30);
    let x: Vec<f64> = t.iter().map(|t| t * 100.0).collect();
    let y: Vec<f64> = t.iter().map(|t| (t - 0.5) * 20.0).collect();
    for ax in &axs {
        ax.scatter(&x, &y, &t)?;
    }
    let (left, right) = (&axs[0], &axs[1]);
    for (dir, scale) in Axis3::ALL.into_iter().zip(["log", "symlog", "logit"]) {
        left.set_scale_with(dir, scale, &ScaleOptions::default())?;
        left.set_label(dir, scale);
    }
    right.set_xscale("asinh")?;
    let functions = ScaleOptions {
        functions: Some(FuncTransform::new(f64::sqrt, |v| v * v)),
        ..ScaleOptions::default()
    };
    right.set_scale_with(Axis3::Z, "function", &functions)?;
    for (dir, label) in Axis3::ALL.into_iter().zip(["asinh", "linear", "function"]) {
        right.set_label(dir, label);
    }
    Ok(())
}

pub(super) fn scale3d_all_clipped(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    for dir in Axis3::ALL {
        ax.set_scale_with(dir, "log", &ScaleOptions::default())?;
    }
    for dir in Axis3::ALL {
        ax.set_limits(dir, 0.1, 10.0)?;
    }
    ax.plot(
        &[-1.0, -2.0, -3.0],
        &[-4.0, -5.0, -6.0],
        &[-7.0, -8.0, -9.0],
    )?;
    Ok(())
}

/// `fmt_e`: `e^n` at the integer powers of e, nothing elsewhere.
fn power_of_e(x: f64) -> String {
    if x <= 0.0 {
        return String::new();
    }
    let exp = x.ln();
    let nearest = exp.round();
    // `np.isclose` with its default tolerances.
    if (exp - nearest).abs() <= 1e-8 + 1e-5 * nearest.abs() {
        format!("$e^{{{nearest}}}$")
    } else {
        String::new()
    }
}

/// A scatter plot of [`log_data`] with markers of size 10.
fn small_markers(ax: &Axes3D) -> CaseResult {
    let [x, y, z] = log_data();
    let options = Scatter3DOptions {
        scatter: ScatterOptions {
            sizes: Some(vec![10.0]),
            ..ScatterOptions::default()
        },
        ..Scatter3DOptions::default()
    };
    ax.scatter_with(&x, &y, &z, options)?;
    Ok(())
}

pub(super) fn scale3d_log_bases(fig: &Figure) -> CaseResult {
    let axs = subplots_3d(fig, 2, 2)?;
    let bases = [
        (10.0, "base=10", false),
        (2.0, "base=2", false),
        (E, "base=e", true),
    ];
    for (ax, (base, title, powers_of_e)) in axs.iter().zip(bases) {
        small_markers(ax)?;
        let options = ScaleOptions {
            base: Some(base),
            ..ScaleOptions::default()
        };
        for dir in Axis3::ALL {
            ax.set_scale_with(dir, "log", &options)?;
        }
        ax.set_title(title);
        if powers_of_e {
            for dir in Axis3::ALL {
                ax.axis(dir)
                    .set_major_formatter(FuncFormatter::new(|x, _| power_of_e(x)));
            }
        }
    }
    let ax = &axs[3];
    small_markers(ax)?;
    let options = ScaleOptions {
        subs: Some(vec![2.0, 5.0]),
        ..ScaleOptions::default()
    };
    for dir in Axis3::ALL {
        ax.set_scale_with(dir, "log", &options)?;
    }
    ax.set_title("subs=[2,5]");
    Ok(())
}

pub(super) fn scale3d_symlog_params(fig: &Figure) -> CaseResult {
    let axs = subplots_3d(fig, 1, 2)?;
    let t = linspace(-3.0, 3.0, 50);
    let x: Vec<f64> = t.iter().map(|t| t.sinh() * 10.0).collect();
    let y: Vec<f64> = t.iter().map(|t| t.powi(3)).collect();
    let z: Vec<f64> = t.iter().map(|t| t.signum() * t.abs().powi(2)).collect();
    for (ax, linthresh) in axs.iter().zip([0.1, 10.0]) {
        let options = Scatter3DOptions {
            scatter: ScatterOptions {
                sizes: Some(vec![10.0]),
                colors: MappedColors::new(z.iter().map(|z| z.abs()).collect::<Vec<f64>>())
                    .with_cmap(colormaps::get("viridis")?)
                    .into(),
                ..ScatterOptions::default()
            },
            ..Scatter3DOptions::default()
        };
        ax.scatter_with(&x, &y, &z, options)?;
        let options = ScaleOptions {
            linthresh: Some(linthresh),
            ..ScaleOptions::default()
        };
        for dir in Axis3::ALL {
            ax.set_scale_with(dir, "symlog", &options)?;
        }
        ax.set_title(format!("linthresh={linthresh}"));
    }
    Ok(())
}
