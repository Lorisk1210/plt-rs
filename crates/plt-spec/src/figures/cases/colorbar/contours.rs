//! Colorbars of filled and line contours.

use plt::colorbar::ColorbarOptions;
use plt::colors::{Color, ColorizerOptions, Extend, colormaps};
use plt::contour::{ContourOptions, Levels};
use plt::figure::Figure;
use plt::numeric::linspace;

use super::data::xyz;
use crate::figures::CaseResult;

pub(super) fn extend_both_contourf(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let options = ContourOptions {
        colorizer: ColorizerOptions {
            cmap: Some(colormaps::get("RdBu_r")?),
            ..ColorizerOptions::default()
        },
        levels: Levels::Values(linspace(-0.5, 0.5, 11)),
        extend: Extend::Both,
        ..ContourOptions::default()
    };
    let cs = ax.contourf(&xyz(), options)?;
    fig.colorbar(&cs, ColorbarOptions::beside(&ax))?;
    Ok(())
}

pub(super) fn contour_lines(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let options = ContourOptions {
        levels: Levels::Count(8),
        ..ContourOptions::default()
    };
    let cs = ax.contour(&xyz(), options)?;
    fig.colorbar(&cs, ColorbarOptions::beside(&ax))?;
    Ok(())
}

pub(super) fn contourf_add_lines(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let field = xyz();
    let fills = ContourOptions {
        colorizer: ColorizerOptions {
            cmap: Some(colormaps::get("Greens")?),
            ..ColorizerOptions::default()
        },
        levels: Levels::Count(8),
        ..ContourOptions::default()
    };
    let cf = ax.contourf(&field, fills)?;
    let lines = ContourOptions {
        levels: Levels::Values(cf.levels()),
        colors: Some(vec![Color::from("k")]),
        linewidths: Some(vec![0.8]),
        ..ContourOptions::default()
    };
    let cl = ax.contour(&field, lines)?;
    let colorbar = fig.colorbar(&cf, ColorbarOptions::beside(&ax))?;
    colorbar.add_lines(&cl, true)?;
    Ok(())
}
