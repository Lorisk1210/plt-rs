//! The plotting tests: error bars, stems, colour-mapped and spiral
//! scatter plots, texts, clipping to the view limits, and surfaces
//! reaching infinity.

use std::f64::consts::PI;

use plt::axes::ErrorSize;
use plt::collections::CollectionProps;
use plt::colors::colormaps;
use plt::layout::{ConstrainedLayout, LayoutEngine};
use plt::legend::LegendOptions;
use plt::lines::MarkEvery;
use plt::mplot3d::{
    Axis3, Errorbar3DOptions, FaceColormap, FaceColors, Quiver3DOptions, Sampling,
    Scatter3DOptions, Stem3DOptions, SurfaceOptions,
};
use plt::numeric::Grid;
use plt::patches::{Arc, PathShape};
use plt::prelude::*;
use plt_core::Path;

use super::{product3, remove_ticks_and_titles, subplots_3d};
use crate::figures::CaseResult;

pub(super) fn errorbar3d_errorevery(fig: &Figure) -> CaseResult {
    let t = arange(0.0, 2.0 * PI + 0.1, 0.01);
    let x: Vec<f64> = t.iter().map(|t| t.sin()).collect();
    let y: Vec<f64> = t.iter().map(|t| (3.0 * t).cos()).collect();
    let z: Vec<f64> = t.iter().map(|t| (5.0 * t).sin()).collect();
    let estep = 15;
    let limits = |phase: usize| -> Vec<bool> {
        (0..t.len())
            .map(|i| i % estep == 0 && (i / estep) % 3 == phase)
            .collect()
    };
    let options = Errorbar3DOptions {
        errors: [None, None, Some(0.2.into())],
        uplims: [Vec::new(), Vec::new(), limits(0)],
        lolims: [Vec::new(), Vec::new(), limits(2)],
        errorevery: Some(MarkEvery::Every { start: 0, step: 15 }),
        ..Errorbar3DOptions::default()
    };
    fig.subplot_3d().errorbar(&x, &y, &z, options)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn errorbar3d(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let d = [1.0, 2.0, 3.0, 4.0, 5.0];
    let e = || Some(ErrorSize::Symmetric(vec![0.5; 5]));
    let options = Errorbar3DOptions {
        errors: [e(), e(), e()],
        capsize: Some(3.0),
        uplims: [Vec::new(), vec![true], vec![false, true, false, true, true]],
        lolims: [
            Vec::new(),
            Vec::new(),
            vec![true, false, false, true, false],
        ],
        ecolor: Some("purple".into()),
        line: LineProps::default().label("Error lines"),
        ..Errorbar3DOptions::default()
    };
    ax.errorbar(&d, &d, &d, options)?;
    ax.legend(LegendOptions::default())?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn stem3d(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(Some(
        LayoutEngine::Constrained(ConstrainedLayout::default()),
    ));
    let axs = subplots_3d(fig, 2, 3)?;
    let theta = linspace(0.0, 2.0 * PI, 50);
    let x: Vec<f64> = theta.iter().map(|t| (t - PI / 2.0).cos()).collect();
    let y: Vec<f64> = theta.iter().map(|t| (t - PI / 2.0).sin()).collect();
    for (ax, zdir) in axs[..3].iter().zip(Axis3::ALL) {
        let options = Stem3DOptions {
            orientation: zdir,
            ..Stem3DOptions::default()
        };
        ax.stem(&x, &y, &theta, options)?;
        ax.set_title(format!("orientation={}", zdir.name()));
    }
    let x = linspace(-PI / 2.0, PI / 2.0, 20);
    let z: Vec<f64> = x.iter().map(|x| x.cos()).collect();
    for (ax, zdir) in axs[3..].iter().zip(Axis3::ALL) {
        let options = Stem3DOptions {
            linefmt: "C4-.".parse()?,
            markerfmt: "C1D".parse()?,
            basefmt: "C2".parse()?,
            orientation: zdir,
            ..Stem3DOptions::default()
        };
        let stem = ax.stem(&x, &[1.0; 20], &z, options)?;
        ax.set_title(format!("orientation={}", zdir.name()));
        stem.markerline
            .set_markerfacecolor("none")
            .set_markeredgewidth(2.0);
        stem.baseline.set_linewidth(3.0);
    }
    Ok(())
}

pub(super) fn scalarmap_update(fig: &Figure) -> CaseResult {
    let range: Vec<f64> = (0..5).map(f64::from).collect();
    let points = product3(&range, &range, &range);
    let [x, y, z] = [0, 1, 2].map(|k| points.iter().map(|p| p[k]).collect::<Vec<f64>>());
    let c: Vec<f64> = x.iter().zip(&y).map(|(x, y)| x + y).collect();
    let ax = fig.subplot_3d();
    let options = Scatter3DOptions {
        scatter: ScatterOptions {
            sizes: Some(vec![40.0]),
            colors: MappedColors::new(c)
                .with_cmap(colormaps::get("viridis")?)
                .into(),
            ..ScatterOptions::default()
        },
        ..Scatter3DOptions::default()
    };
    ax.scatter_with(&x, &y, &z, options)?;
    fig.layout();
    Ok(())
}

pub(super) fn pathpatch_3d(fig: &Figure) -> CaseResult {
    let square = PathShape(Path::unit_rectangle());
    let zs = [0.0, 0.5, 0.7, 1.0, 0.0];
    fig.subplot_3d()
        .add_path_patch3d(&square, PatchProps::default(), &zs, Axis3::Y)?;
    Ok(())
}

pub(super) fn arc_pathpatch(fig: &Figure) -> CaseResult {
    let arc = Arc::new((0.5, 0.5), 0.5, 0.9)
        .with_angle(20.0)
        .between(10.0, 130.0);
    fig.subplot_3d()
        .add_path_patch3d(&arc, PatchProps::default(), &[0.0], Axis3::Z)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
pub(super) fn text_3d(fig: &Figure) -> CaseResult {
    // A `Text3D` follows the z axis unless told otherwise, and
    // `add_artist` clips it to the axes.
    fig.subplot_3d()
        .text(0.5, 0.5, 1.0, r"Foo bar $\int$", Some(Axis3::Z))
        .set_clip_on(true);
    Ok(())
}

pub(super) fn scatter_spiral(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let th = linspace(0.0, 2.0 * PI * 6.0, 256);
    let x: Vec<f64> = th.iter().map(|t| t.sin()).collect();
    let y: Vec<f64> = th.iter().map(|t| t.cos()).collect();
    let options = Scatter3DOptions {
        scatter: ScatterOptions {
            sizes: Some(th.iter().map(|t| 1.0 + t * 5.0).collect()),
            colors: MappedColors::new(th.iter().map(|t| t * t).collect::<Vec<f64>>()).into(),
            ..ScatterOptions::default()
        },
        ..Scatter3DOptions::default()
    };
    ax.scatter_with(&x, &y, &th, options)?;
    fig.layout();
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn mutating_input_arrays_y_and_z(fig: &Figure) -> CaseResult {
    fig.subplot_3d()
        .plot_fmt(&[1.0, 2.0, 3.0], &[0.0; 3], &[0.0; 3], "o-")?;
    Ok(())
}

pub(super) fn surface3d_zsort_inf(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let axis = mgrid_axis_step(-2.0, 2.0, 0.1);
    let n = axis.len();
    let x = Grid::from_fn(n, n, |i, _| axis[i]);
    let y = Grid::from_fn(n, n, |_, j| axis[j]);
    let z = Grid::from_fn(n, n, |i, j| {
        if i >= n / 2 && j >= n / 2 {
            f64::INFINITY
        } else {
            x[(i, j)].sin().powi(2) + y[(i, j)].cos().powi(2)
        }
    });
    let options = SurfaceOptions {
        colors: FaceColors::Mapped(Box::new(FaceColormap::new(colormaps::get("jet")?))),
        ..SurfaceOptions::default()
    };
    ax.plot_surface(&x, &y, &z, options)?;
    ax.view_init(45.0, 145.0, 0.0);
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `np.mgrid[start:stop:step]` along one axis: `start + i·step`.
fn mgrid_axis_step(start: f64, stop: f64, step: f64) -> Vec<f64> {
    let n = ((stop - start) / step).ceil() as usize;
    (0..n).map(|i| i as f64 * step + start).collect()
}

/// The surface, line, scatter, quiver, line and text of `axlim_clip`,
/// with the points beyond `x = 0.5` hidden.
pub(super) fn axlim_clip(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let x = linspace(0.0, 1.0, 11);
    let (xx, yy) = meshgrid(&x, &x);
    let zz = Grid::from_fn(11, 11, |i, j| xx[(i, j)] + yy[(i, j)]);
    // `facecolor` replaces the shaded cycle colour (which is still taken).
    let options = SurfaceOptions {
        props: CollectionProps::default().facecolor("C1"),
        // Given counts override classic mode's default strides.
        sampling: Sampling::Counts { rows: 50, cols: 50 },
        axlim_clip: true,
        ..SurfaceOptions::default()
    };
    ax.plot_surface(&xx, &yy, &zz, options)?;
    let props = LineProps::default().color("k").linewidth(3.0).zorder(5.0);
    let line = ax.plot_with(&[0.5, 0.5], &[0.0, 1.0], &[0.5, 1.5], Axis3::Z, props)?;
    ax.set_axlim_clip(&line, true)?;
    let (flat_x, flat_y) = (xx.as_slice(), yy.as_slice());
    let lifted = |dz: f64| zz.as_slice().iter().map(|z| z + dz).collect::<Vec<f64>>();
    let options = Scatter3DOptions {
        axlim_clip: true,
        ..Scatter3DOptions::default()
    };
    ax.scatter_with(flat_x, flat_y, &lifted(1.0), options)?;
    let positions: Vec<[f64; 3]> = (flat_x.iter().zip(flat_y).zip(lifted(2.0)))
        .map(|((&x, &y), z)| [x, y, z])
        .collect();
    let options = Quiver3DOptions {
        arrow_length_ratio: 0.0,
        axlim_clip: true,
        ..Quiver3DOptions::default()
    };
    ax.quiver(&positions, &vec![[0.0, 0.0, 1.0]; positions.len()], options)?;
    let first_row = |g: &Grid<f64>| g.row(0).to_vec();
    let top: Vec<f64> = first_row(&zz).iter().map(|z| z + 3.0).collect();
    let props = LineProps::default().color("C2");
    let line = ax.plot_with(&first_row(&xx), &first_row(&yy), &top, Axis3::Z, props)?;
    ax.set_axlim_clip(&line, true)?;
    let text = ax.text(1.1, 0.5, 4.0, "test", None);
    ax.set_axlim_clip(&text, true)?;
    ax.set_xlim(0.0, 0.5)?;
    ax.set_ylim(0.0, 1.0)?;
    ax.set_zlim(0.0, 5.0)?;
    Ok(())
}
