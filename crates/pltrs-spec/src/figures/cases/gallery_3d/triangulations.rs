//! Triangulated surfaces and contours on triangulations: `trisurf3d`,
//! `trisurf3d_2`, `tricontour3d` and `tricontourf3d`.

use std::f64::consts::PI;

use pltrs::collections::CollectionProps;
use pltrs::colors::{ColorizerOptions, colormaps};
use pltrs::contour::ContourOptions;
use pltrs::figure::Figure;
use pltrs::mplot3d::{Axes3DOptions, Contour3DOptions, TrisurfOptions};
use pltrs::numeric::{linspace, linspace_with, meshgrid};
use pltrs::tri::{TriError, Triangulation};

use super::{Fill, ring_points};
use crate::figures::cases::common::face_cmap;
use crate::figures::{CaseResult, FigureCase};

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_3d/trisurf3d", trisurf3d),
    FigureCase::new("gallery_3d/trisurf3d_2", trisurf3d_2).sized(9.6, 4.8, 100.0),
    FigureCase::new("gallery_3d/tricontour3d", tricontour3d),
    FigureCase::new("gallery_3d/tricontourf3d", tricontourf3d),
];

/// The Delaunay triangulation of `(x, y)` without the triangles whose
/// centroid `masked(x, y)` says lies inside the inner ring.
fn ring_triangulation(
    x: &[f64],
    y: &[f64],
    masked: impl Fn(f64, f64) -> bool,
) -> Result<Triangulation, TriError> {
    let tri = Triangulation::delaunay(x, y)?;
    let mask = tri
        .triangles()
        .iter()
        .map(|t| {
            let mean = |v: &[f64]| (v[t[0]] + v[t[1]] + v[t[2]]) / 3.0;
            masked(mean(x), mean(y))
        })
        .collect();
    tri.with_mask(Some(mask))
}

/// `tricontour3d` and `tricontourf3d`: `cmap="CMRmap"` contours of
/// [`ring_points`] over their triangulation without the inner disc
/// (`np.hypot(xmid, ymid) < min_radius`), seen from 45° up.
fn ring_contours(fig: &Figure, fill: Fill) -> CaseResult {
    let [x, y, z] = ring_points(48);
    let tri = ring_triangulation(&x, &y, |x, y| x.hypot(y) < 0.25)?;
    let options = Contour3DOptions {
        contour: ContourOptions {
            colorizer: ColorizerOptions {
                cmap: Some(colormaps::get("CMRmap")?),
                ..ColorizerOptions::default()
            },
            ..ContourOptions::default()
        },
        ..Contour3DOptions::default()
    };
    let ax = fig.subplot_3d();
    match fill {
        Fill::Lines => ax.tricontour(&tri, &z, options)?,
        Fill::Filled => ax.tricontourf(&tri, &z, options)?,
    };
    let view = ax.view();
    ax.view_init(45.0, view.azim, view.roll);
    Ok(())
}

fn tricontour3d(fig: &Figure) -> CaseResult {
    ring_contours(fig, Fill::Lines)
}

fn tricontourf3d(fig: &Figure) -> CaseResult {
    ring_contours(fig, Fill::Filled)
}

fn trisurf3d(fig: &Figure) -> CaseResult {
    let radii = linspace(0.125, 1.0, 8);
    let angles = linspace_with(0.0, 2.0 * PI, 36, false);
    let mut x = vec![0.0];
    let mut y = vec![0.0];
    for angle in &angles {
        x.extend(radii.iter().map(|r| r * angle.cos()));
        y.extend(radii.iter().map(|r| r * angle.sin()));
    }
    let z: Vec<f64> = x.iter().zip(&y).map(|(x, y)| (-x * y).sin()).collect();
    let options = TrisurfOptions {
        props: CollectionProps {
            antialiased: Some(vec![true]),
            ..CollectionProps::default().linewidth(0.2)
        },
        ..TrisurfOptions::default()
    };
    fig.subplot_3d()
        .plot_trisurf(&Triangulation::delaunay(&x, &y)?, &z, options)?;
    Ok(())
}

fn trisurf3d_2(fig: &Figure) -> CaseResult {
    let (u, v) = meshgrid(&linspace(0.0, 2.0 * PI, 50), &linspace(-0.5, 0.5, 10));
    let (u, v) = (u.as_slice(), v.as_slice());
    let radius: Vec<f64> = u
        .iter()
        .zip(v)
        .map(|(u, v)| 1.0 + 0.5 * v * (u / 2.0).cos())
        .collect();
    let x: Vec<f64> = radius.iter().zip(u).map(|(r, u)| r * u.cos()).collect();
    let y: Vec<f64> = radius.iter().zip(u).map(|(r, u)| r * u.sin()).collect();
    let z: Vec<f64> = u
        .iter()
        .zip(v)
        .map(|(u, v)| 0.5 * v * (u / 2.0).sin())
        .collect();
    // The triangles of the parameter space connect the points of the band.
    let parameters = Triangulation::delaunay(u, v)?;
    let mobius = Triangulation::new(&x, &y, parameters.triangles().to_vec())?;
    let ax = fig.add_subplot_3d(1, 2, 1, Axes3DOptions::default())?;
    let options = TrisurfOptions {
        colors: face_cmap("Spectral")?,
        ..TrisurfOptions::default()
    };
    ax.plot_trisurf(&mobius, &z, options)?;
    ax.set_zlim(-1.0, 1.0)?;

    let [x, y, z] = ring_points(36);
    let tri = ring_triangulation(&x, &y, |x, y| x.powi(2) + y.powi(2) < 0.25_f64.powi(2))?;
    let options = TrisurfOptions {
        colors: face_cmap("CMRmap")?,
        ..TrisurfOptions::default()
    };
    fig.add_subplot_3d(1, 2, 2, Axes3DOptions::default())?
        .plot_trisurf(&tri, &z, options)?;
    Ok(())
}
