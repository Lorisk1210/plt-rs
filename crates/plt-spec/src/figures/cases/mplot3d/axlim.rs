//! `figures/cases_mplot3d_api.py`, `axlim_clip` part: every kind of 3-D
//! artist with limits cutting through its data, hiding what lies outside.

use std::f64::consts::PI;

use plt::axes::{MappedColors, ScatterColors, ScatterOptions};
use plt::collections::{CollectionProps, CollectionSpec, EdgeColors};
use plt::colors::colormaps;
use plt::figure::Figure;
use plt::lines::{LineProps, MarkEvery};
use plt::mplot3d::{
    Array3, Axes3D, Axes3DError, Axis3, Bar3DOptions, Boxes, Errorbar3DOptions, FaceColormap,
    FaceColors, FillBetween3DOptions, FillMode, Quiver3DOptions, Sampling, Scatter3DOptions,
    Stem3DOptions, SurfaceOptions, TrisurfOptions, VoxelColors, VoxelOptions, WireframeOptions,
};
use plt::numeric::linspace;
use plt::transforms::Point;
use plt::tri::Triangulation;

use super::api::ellipse_helix;
use super::api_plots::polar_points;
use super::surface;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `ax.set(xlim=..., ylim=..., zlim=...)`.
fn limits(ax: &Axes3D, x: (f64, f64), y: (f64, f64), z: (f64, f64)) -> Result<(), Axes3DError> {
    ax.set_xlim(x.0, x.1)?
        .set_ylim(y.0, y.1)?
        .set_zlim(z.0, z.1)
        .map(drop)
}

/// `_axlim_scatter(fig, axlim_clip)`: colour-mapped markers of random
/// sizes, depth shaded, and unshaded red triangles above them.
fn axlim_scatter(fig: &Figure, axlim_clip: bool) -> CaseResult {
    let ax = fig.subplot_3d();
    let mut g = SpecRng::new(44);
    let (x, y, z) = (
        g.uniform(120, -1.0, 1.0),
        g.uniform(120, -1.0, 1.0),
        g.uniform(120, -1.0, 1.0),
    );
    let sizes = g
        .uniform(120, 0.0, 1.0)
        .iter()
        .map(|u| 10.0 + 60.0 * u)
        .collect();
    let scatter = ScatterOptions {
        sizes: Some(sizes),
        colors: MappedColors::new(z.clone())
            .with_cmap(colormaps::get("viridis")?)
            .into(),
        ..ScatterOptions::default()
    };
    let options = Scatter3DOptions {
        axlim_clip,
        scatter,
        ..Scatter3DOptions::default()
    };
    ax.scatter_with(&x, &y, &z, options)?;
    let half = |values: &[f64]| -> Vec<f64> { values[..30].iter().map(|v| 0.5 * v).collect() };
    let raised: Vec<f64> = z[..30].iter().map(|z| z + 0.6).collect();
    // `color=` sets both the faces and the edges.
    let scatter = ScatterOptions {
        sizes: Some(vec![30.0]),
        colors: ScatterColors::Single("tab:red".into()),
        edgecolors: Some(EdgeColors::Colors(vec!["tab:red".into()])),
        marker: Some("^".parse()?),
        ..ScatterOptions::default()
    };
    let options = Scatter3DOptions {
        depthshade: Some(false),
        axlim_clip,
        scatter,
        ..Scatter3DOptions::default()
    };
    ax.scatter_with(&half(&x), &half(&y), &raised, options)?;
    limits(&ax, (-0.6, 0.65), (-0.8, 0.75), (-0.5, 1.05))?;
    Ok(())
}

/// `_axlim_surface(fig, axlim_clip)`: a colour-mapped surface with edges.
fn axlim_surface(fig: &Figure, axlim_clip: bool) -> CaseResult {
    let ax = fig.subplot_3d();
    let (x, y, z) = surface(20);
    let options = SurfaceOptions {
        colors: FaceColors::Mapped(Box::new(FaceColormap::new(colormaps::get("viridis")?))),
        props: CollectionProps::default().edgecolor("k").linewidth(0.3),
        axlim_clip,
        ..SurfaceOptions::default()
    };
    ax.plot_surface(&x, &y, &z, options)?;
    limits(&ax, (-2.05, 2.55), (-2.55, 2.05), (-0.1, 0.3))?;
    Ok(())
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("mplot3d/axlim_plot_text", |fig| {
        let ax = fig.subplot_3d();
        let [x, y, z] = ellipse_helix();
        let line = ax.plot(&x, &y, &z)?;
        ax.set_axlim_clip(&line, true)?;
        let every_sixth =
            |values: &[f64]| -> Vec<f64> { values.iter().step_by(6).copied().collect() };
        let markers = ax.plot_with(
            &every_sixth(&x),
            &every_sixth(&y),
            &every_sixth(&z),
            Axis3::Z,
            LineProps::fmt("o")?.markersize(4.0),
        )?;
        ax.set_axlim_clip(&markers, true)?;
        let inside = ax.text(0.0, 0.0, 1.0, "inside", Some(Axis3::X));
        ax.set_axlim_clip(&inside, true)?;
        let hidden = ax.text(1.8, 0.0, 2.5, "hidden", None);
        ax.set_axlim_clip(&hidden, true)?;
        ax.text(-1.8, 0.0, 2.8, "unclipped", None)
            .set_color("tab:red");
        limits(&ax, (-1.5, 1.5), (-1.0, 1.0), (-0.2, 2.2))?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/axlim_scatter", |fig| axlim_scatter(fig, true)).sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/axlim_scatter_unclipped", |fig| {
        axlim_scatter(fig, false)
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/axlim_surface", |fig| axlim_surface(fig, true)).sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/axlim_surface_unclipped", |fig| {
        axlim_surface(fig, false)
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/axlim_wireframe", |fig| {
        let ax = fig.subplot_3d();
        let (x, y, z) = surface(20);
        let options = WireframeOptions {
            sampling: Sampling::Strides { rows: 2, cols: 2 },
            props: CollectionProps::default().color("0.3").linewidth(0.6),
            axlim_clip: true,
        };
        ax.plot_wireframe(&x, &y, &z, options)?;
        ax.set_xlim(-2.05, 2.55)?.set_zlim(-0.1, 0.3)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/axlim_trisurf", |fig| {
        let ax = fig.subplot_3d();
        let [x, y, z] = polar_points();
        let tri = Triangulation::delaunay(&x, &y)?;
        let options = TrisurfOptions {
            colors: FaceColors::Mapped(Box::new(FaceColormap::new(colormaps::get("Spectral")?))),
            props: CollectionProps::default().edgecolor("grey").linewidth(0.2),
            axlim_clip: true,
            ..TrisurfOptions::default()
        };
        ax.plot_trisurf(&tri, &z, options)?;
        ax.set_xlim(-0.8, 1.05)?.set_zlim(-1.0, 1.2)?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/axlim_fill_between", |fig| {
        let ax = fig.subplot_3d();
        let t = linspace(0.0, 2.0 * PI, 30);
        let (c, s): (Vec<f64>, Vec<f64>) = t.iter().map(|t| (t.cos(), t.sin())).unzip();
        let top: Vec<f64> = t.iter().map(|t| t / 2.0).collect();
        let bottom: Vec<f64> = t.iter().map(|t| 0.2 * (3.0 * t).sin()).collect();
        let options = FillBetween3DOptions {
            props: CollectionProps::default().alpha(0.6),
            axlim_clip: true,
            ..FillBetween3DOptions::default()
        };
        ax.fill_between([&c, &s, &top], [&c, &s, &bottom], options)?;
        let x = linspace(-1.0, 1.0, 25);
        let wave: Vec<f64> = x.iter().map(|x| (3.0 * x).cos() + 1.5).collect();
        let options = FillBetween3DOptions {
            mask: Some(x.iter().map(|x| (x - 0.45).abs() > 0.1).collect()),
            mode: FillMode::Polygon,
            props: CollectionProps::default()
                .facecolor("tab:orange")
                .edgecolor("k")
                .linewidth(0.5),
            axlim_clip: true,
            ..FillBetween3DOptions::default()
        };
        ax.fill_between([&x, &[-1.1], &wave], [&x, &[-1.1], &[0.0]], options)?;
        limits(&ax, (-0.8, 0.9), (-1.2, 1.2), (-0.1, 2.2))?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/axlim_bar3d", |fig| {
        let ax = fig.subplot_3d();
        // np.meshgrid(np.arange(4), np.arange(3)), raveled.
        let (x, y): (Vec<f64>, Vec<f64>) = (0..3)
            .flat_map(|row| (0..4).map(move |col| (f64::from(col), f64::from(row))))
            .unzip();
        let heights: Vec<f64> = x
            .iter()
            .zip(&y)
            .map(|(x, y)| 1.0 + (x * 3.0 + y * 5.0) % 7.0)
            .collect();
        let boxes = Boxes {
            x: &x,
            y: &y,
            z: &[0.0],
            dx: &[0.6],
            dy: &[0.6],
            dz: &heights,
        };
        let options = Bar3DOptions {
            color: vec!["tab:cyan".into()],
            props: CollectionProps::default().edgecolor("k").linewidth(0.5),
            axlim_clip: true,
            ..Bar3DOptions::default()
        };
        ax.bar3d(boxes, options)?;
        limits(&ax, (0.3, 3.8), (-0.2, 2.8), (-0.5, 4.5))?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/axlim_quiver", |fig| {
        let ax = fig.subplot_3d();
        let g = linspace(-1.0, 1.0, 4);
        // np.meshgrid(g, g, g): xx[i, j, k] = g[j], yy = g[i], zz = g[k].
        let mut positions = Vec::new();
        let mut vectors = Vec::new();
        for &gi in &g {
            for &gj in &g {
                for &gk in &g {
                    positions.push([gj, gi, gk]);
                    vectors.push([-gi, gj, 0.3 * gk]);
                }
            }
        }
        let options = Quiver3DOptions {
            length: 0.4,
            normalize: true,
            axlim_clip: true,
            ..Quiver3DOptions::default()
        };
        ax.quiver(&positions, &vectors, options)?;
        limits(&ax, (-0.8, 1.2), (-1.2, 0.9), (-1.2, 1.2))?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/axlim_errorbar", |fig| {
        let ax = fig.subplot_3d();
        let t = linspace(0.0, 2.0 * PI, 9);
        let x: Vec<f64> = t.iter().map(|t| t.cos()).collect();
        let y: Vec<f64> = t.iter().map(|t| t.sin()).collect();
        let z: Vec<f64> = t.iter().map(|t| t / 2.0).collect();
        let only = |index: usize| -> Vec<bool> { (0..9).map(|i| i == index).collect() };
        let options = Errorbar3DOptions {
            errors: [Some(0.15.into()), None, Some(0.4.into())],
            fmt: "o-".parse()?,
            capsize: Some(3.0),
            errorevery: Some(MarkEvery::Every { start: 1, step: 2 }),
            uplims: [Vec::new(), Vec::new(), only(1)],
            lolims: [Vec::new(), Vec::new(), only(7)],
            ecolor: Some("tab:red".into()),
            axlim_clip: true,
            ..Errorbar3DOptions::default()
        };
        ax.errorbar(&x, &y, &z, options)?;
        limits(&ax, (-0.95, 1.2), (-1.2, 1.2), (0.2, 2.9))?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/axlim_stem", |fig| {
        let ax = fig.subplot_3d();
        let t = linspace(0.0, 2.0 * PI, 16);
        let x: Vec<f64> = t.iter().map(|t| t.cos()).collect();
        let y: Vec<f64> = t.iter().map(|t| t.sin()).collect();
        let options = Stem3DOptions {
            axlim_clip: true,
            ..Stem3DOptions::default()
        };
        ax.stem(&x, &y, &t, options)?;
        limits(&ax, (-0.8, 1.2), (-1.2, 1.2), (-0.5, 4.5))?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/axlim_voxels", |fig| {
        let ax = fig.subplot_3d();
        let centered = |v: usize| (v as f64 - 1.5).powi(2);
        let filled = Array3::from_fn([4, 4, 4], |[x, y, z]| {
            centered(x) + centered(y) + centered(z) < 4.0
        });
        let colors = Array3::from_fn([4, 4, 4], |[_, _, z]| {
            if z < 2 { "tab:blue" } else { "tab:orange" }.into()
        });
        let options = VoxelOptions {
            facecolors: Some(VoxelColors::PerCell(colors)),
            props: CollectionProps::default().edgecolor("k").linewidth(0.3),
            axlim_clip: true,
            ..VoxelOptions::default()
        };
        ax.voxels(&filled, options)?;
        limits(&ax, (0.5, 4.2), (-0.2, 3.4), (0.3, 3.6))?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
    FigureCase::new("mplot3d/axlim_add_collection3d", |fig| {
        let ax = fig.subplot_3d();
        let x = linspace(0.0, 3.0, 10);
        let polygons: Vec<Vec<Point>> = (0..3)
            .map(|k| {
                let k = f64::from(k);
                std::iter::once(Point::new(0.0, 0.0))
                    .chain(x.iter().map(|&x| Point::new(x, (x + k).sin().abs() + 0.2)))
                    .chain(std::iter::once(Point::new(3.0, 0.0)))
                    .collect()
            })
            .collect();
        let mut props = CollectionProps::default().alpha(0.6);
        props.facecolors = Some(vec!["r".into(), "g".into(), "b".into()]);
        let polys = CollectionSpec::polygons(polygons).with_props(props);
        let polys = ax.add_collection3d(polys, &[0.0, 1.0, 5.0], Axis3::Y, true)?;
        ax.set_axlim_clip(&polys, true)?;
        let lines: Vec<Vec<Point>> = (0..2)
            .map(|k| {
                let k = f64::from(k);
                x.iter().map(|&x| Point::new(x, (x + k).cos())).collect()
            })
            .collect();
        let lines = CollectionSpec::lines(lines)
            .with_props(CollectionProps::default().edgecolor("k").linewidth(1.5));
        let lines = ax.add_collection3d(lines, &[0.5, 1.5], Axis3::Y, true)?;
        ax.set_axlim_clip(&lines, true)?;
        limits(&ax, (-0.2, 3.2), (-0.5, 2.0), (-0.5, 1.4))?;
        Ok(())
    })
    .sized(4.0, 4.0, 100.0),
];
