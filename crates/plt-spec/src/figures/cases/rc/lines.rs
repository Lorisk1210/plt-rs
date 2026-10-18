//! The `lines.*`, `markers.*`, `pcolor*`, `patch.*` and `hatch.*` cases.

use plt::axes::{Mesh, PcolorOptions};
use plt::numeric::{Grid, linspace, meshgrid};
use plt::patches::{Circle, Polygon, Rectangle};
use plt::prelude::*;

use super::{SMALL, curves, map, rc_cases, t};
use crate::figures::{CaseResult, FigureCase};

/// `_reference_lines`: lines that take `lines.color`: axhline, axvline,
/// hlines, vlines.
fn reference_lines(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.axhline(0.5, LineProps::default());
    ax.axvline(0.25, LineProps::default());
    ax.hlines([0.2, 0.8], [0.4], [0.9])?;
    ax.vlines([0.6, 0.75], [0.1], [0.45])?;
    Ok(())
}

/// `_dashes`: dashed, dash-dot and dotted curves.
fn dashes(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let t = t();
    for (i, style) in ["--", "-.", ":"].into_iter().enumerate() {
        let shift = i as f64;
        let props = LineProps::default()
            .linestyle(style.parse::<LineStyle>()?)
            .linewidth(2.0);
        ax.plot_with(&t, map(&t, |x| x.sin() + shift), props)?;
    }
    Ok(())
}

/// `_zigzags`: a thick solid and a thick dashed zigzag; joins and caps
/// show.
fn zigzags(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x: Vec<f64> = (0..5).map(|i| 1.2 + 0.4 * f64::from(i)).collect();
    let thick = LineProps::default().linewidth(12.0);
    ax.plot_with(&x, [0.0, 1.0, 0.0, 1.0, 0.0], thick.clone())?;
    ax.plot_with(
        &x,
        [1.6, 2.6, 1.6, 2.6, 1.6],
        thick.linestyle(LineStyle::Dashed),
    )?;
    ax.set_xlim(-0.6, 4.6).set_ylim(-0.6, 3.2);
    Ok(())
}

/// `_markers`: circle markers on a line, and squares and triangles on
/// their own.
fn markers(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x: Vec<f64> = t().into_iter().step_by(3).collect();
    let marked = |symbol: MarkerSymbol| LineProps::default().marker(Marker::from(symbol));
    ax.plot_with(&x, map(&x, f64::sin), marked(MarkerSymbol::Circle))?;
    ax.plot_with(
        &x,
        map(&x, f64::cos),
        marked(MarkerSymbol::Square).linestyle(LineStyle::None),
    )?;
    ax.plot_with(
        &x,
        map(&x, |v| 0.5 * v.sin() - 1.2),
        marked(MarkerSymbol::TriangleUp).linestyle(LineStyle::None),
    )?;
    Ok(())
}

/// `_mesh`: a pcolormesh whose coordinates have the shape of the data.
fn mesh(fig: &Figure) -> CaseResult {
    let (x, y) = meshgrid(&linspace(0.0, 1.0, 7), &linspace(0.0, 1.0, 5));
    let values = Grid::from_fn(5, 7, |r, c| (3.0 * x[(r, c)]).sin() + y[(r, c)]);
    fig.subplot()
        .pcolormesh(Mesh::with_coords(x, y, values), PcolorOptions::default())?;
    Ok(())
}

/// `_patches`: a filled rectangle and circle and an unfilled triangle, all
/// with the patch defaults.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn patches(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.add_patch(Rectangle::new((0.08, 0.1), 0.3, 0.45));
    ax.add_patch(Circle::new((0.65, 0.32), 0.2));
    ax.add_patch(Polygon::new(
        vec![(0.2, 0.62).into(), (0.5, 0.95).into(), (0.8, 0.62).into()],
        true,
    ))
    .update_props(PatchProps::default().fill(false));
    ax.set_xlim(0.0, 1.0).set_ylim(0.0, 1.0);
    Ok(())
}

/// `_hatches`: a hatched patch with an edge colour and one without an
/// edge.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn hatches(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.add_patch(Rectangle::new((0.05, 0.1), 0.4, 0.8))
        .update_props(
            PatchProps::default()
                .hatch("//")
                .facecolor("white")
                .edgecolor("tab:blue"),
        );
    ax.add_patch(Rectangle::new((0.55, 0.1), 0.4, 0.8))
        .update_props(PatchProps::default().hatch("x").facecolor("lightgray"));
    ax.set_xlim(0.0, 1.0).set_ylim(0.0, 1.0);
    Ok(())
}

pub(crate) const CASES: &[FigureCase] = rc_cases! {
    curves, SMALL => {
        "lines_linewidth": [("lines.linewidth", "4")],
        "lines_linestyle": [("lines.linestyle", "--")],
        "lines_marker": [("lines.marker", "o")],
        "lines_antialiased": [("lines.antialiased", "False")],
    }
    reference_lines, SMALL => {
        "lines_color": [("lines.color", "tab:red")],
    }
    markers, SMALL => {
        "lines_markersize": [("lines.markersize", "10")],
        "lines_markeredgewidth": [("lines.markeredgewidth", "3")],
        "lines_marker_colors": [
            ("lines.markerfacecolor", "white"),
            ("lines.markeredgecolor", "black"),
        ],
        "markers_fillstyle_left": [("markers.fillstyle", "left")],
        "markers_fillstyle_none": [("markers.fillstyle", "none")],
    }
    zigzags, SMALL => {
        "lines_solid_joinstyle": [("lines.solid_joinstyle", "miter")],
        "lines_solid_capstyle": [("lines.solid_capstyle", "round")],
        "lines_dash_joinstyle": [("lines.dash_joinstyle", "bevel")],
        "lines_dash_capstyle": [("lines.dash_capstyle", "round")],
    }
    dashes, SMALL => {
        "lines_dash_patterns": [
            ("lines.dashed_pattern", "6, 2"),
            ("lines.dashdot_pattern", "8, 2, 2, 2"),
            ("lines.dotted_pattern", "1, 3"),
        ],
        "lines_scale_dashes": [("lines.scale_dashes", "False")],
    }
    mesh, SMALL => {
        "pcolor_shading": [("pcolor.shading", "gouraud")],
        "pcolormesh_snap": [("pcolormesh.snap", "False")],
    }
    patches, SMALL => {
        "patch_facecolor": [("patch.facecolor", "salmon")],
        "patch_edgecolor_linewidth": [("patch.edgecolor", "tab:red"), ("patch.linewidth", "3")],
        "patch_force_edgecolor": [("patch.force_edgecolor", "True")],
        "patch_antialiased": [("patch.antialiased", "False")],
    }
    hatches, SMALL => {
        "hatch_color": [("hatch.color", "tab:red")],
        "hatch_linewidth": [("hatch.linewidth", "3")],
    }
};
