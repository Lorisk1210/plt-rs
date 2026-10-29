//! The `polaraxes.*`, `axes3d.*` and `axes.zmargin` cases.

use pltrs::axes::ScatterOptions;
use pltrs::figure::Figure;
use pltrs::mplot3d::{Axes3D, Axes3DOptions, ProjType, Scatter3DOptions};

use super::{SQUARE, map, rc_cases, t};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `_polar`: a curve on polar axes.
fn polar(fig: &Figure) -> CaseResult {
    let t = t();
    fig.polar_subplot()
        .plot(&t, map(&t, |x| 1.0 + 0.5 * (3.0 * x).sin()))?;
    Ok(())
}

/// `_scatter3d`: a 3-D scatter plot.
fn scatter3d(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let mut g = SpecRng::new(3);
    let x = g.uniform(30, 0.0, 1.0);
    let y = g.uniform(30, 0.0, 1.0);
    let z = g.uniform(30, 0.0, 1.0);
    let options = Scatter3DOptions {
        scatter: ScatterOptions {
            sizes: Some(vec![30.0]),
            ..ScatterOptions::default()
        },
        ..Scatter3DOptions::default()
    };
    ax.scatter_with(&x, &y, &z, options)?;
    Ok(())
}

/// A helix of `T` on 3-D axes.
fn helix(ax: &Axes3D) -> CaseResult {
    let t = t();
    ax.plot(map(&t, f64::cos), map(&t, f64::sin), map(&t, |x| x / 6.0))?;
    Ok(())
}

/// `_ortho_helix`: a 3-D helix, autoscaled in an orthographic projection
/// (perspective axes have no z margin).
fn ortho_helix(fig: &Figure) -> CaseResult {
    helix(&fig.subplot_3d_with(Axes3DOptions {
        proj_type: ProjType::Ortho,
        ..Axes3DOptions::default()
    })?)
}

/// `_limited_helix`: a 3-D helix with explicitly set limits.
fn limited_helix(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    helix(&ax)?;
    ax.set_xlim(-1.0, 1.0)?;
    ax.set_ylim(-1.0, 1.0)?;
    ax.set_zlim(0.0, 1.0)?;
    Ok(())
}

pub(crate) const CASES: &[FigureCase] = rc_cases! {
    polar, SQUARE => {
        "polaraxes_grid": [("polaraxes.grid", "False")],
    }
    scatter3d, SQUARE => {
        "axes3d_grid": [("axes3d.grid", "False")],
        "axes3d_panecolor": [
            ("axes3d.xaxis.panecolor", "lightsteelblue"),
            ("axes3d.yaxis.panecolor", "(1.0, 0.9, 0.7, 0.8)"),
            ("axes3d.zaxis.panecolor", "#c8e6c8"),
        ],
        "axes3d_depthshade": [("axes3d.depthshade", "False")],
        "axes3d_depthshade_minalpha": [("axes3d.depthshade_minalpha", "0.8")],
    }
    limited_helix, SQUARE => {
        "axes3d_automargin": [("axes3d.automargin", "True")],
    }
    ortho_helix, SQUARE => {
        "axes_zmargin": [("axes.zmargin", "0.4")],
    }
};
