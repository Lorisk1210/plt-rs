//! Colorbars of filled and line contours.

use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::colors::{Color, Extend, colormaps};
use plt::contour::{ContourOptions, Levels};
use plt::figure::Figure;
use plt::numeric::linspace;

use super::data::xyz;
use crate::figures::CaseResult;

/// Colorbar options stealing space from `ax`.
fn beside(ax: &plt::axes::Axes) -> ColorbarOptions {
    ColorbarOptions {
        axes: ColorbarAxes::Parent(ax.clone()),
        ..ColorbarOptions::default()
    }
}

pub(super) fn extend_both_contourf(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let options = ContourOptions {
        levels: Levels::Values(linspace(-0.5, 0.5, 11)),
        extend: Extend::Both,
        cmap: Some(colormaps::get("RdBu_r")?),
        ..ContourOptions::default()
    };
    let cs = ax.contourf(&xyz(), options)?;
    fig.colorbar(&cs, beside(&ax))?;
    Ok(())
}

pub(super) fn contour_lines(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let options = ContourOptions {
        levels: Levels::Count(8),
        ..ContourOptions::default()
    };
    let cs = ax.contour(&xyz(), options)?;
    fig.colorbar(&cs, beside(&ax))?;
    Ok(())
}

pub(super) fn contourf_add_lines(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let field = xyz();
    let fills = ContourOptions {
        levels: Levels::Count(8),
        cmap: Some(colormaps::get("Greens")?),
        ..ContourOptions::default()
    };
    let cf = ax.contourf(&field, fills)?;
    let lines = ContourOptions {
        levels: Levels::Values(cf.levels().to_vec()),
        colors: Some(vec![Color::from("k")]),
        linewidths: Some(vec![0.8]),
        ..ContourOptions::default()
    };
    let cl = ax.contour(&field, lines)?;
    let mut colorbar = fig.colorbar(&cf, beside(&ax))?;
    colorbar.add_lines(&cl, true)?;
    Ok(())
}
