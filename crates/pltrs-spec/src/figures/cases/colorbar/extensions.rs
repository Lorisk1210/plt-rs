//! Triangular and rectangular extensions for out-of-range values.

use pltrs::axes::ImshowOptions;
use pltrs::colorbar::{ColorbarAxes, ColorbarOptions, ExtendFrac};
use pltrs::colors::{Colorize, ColorizerOptions, Extend, colormaps};
use pltrs::figure::Figure;
use pltrs::orientation::Orientation;

use super::data::field;
use crate::figures::CaseResult;

/// An image of the field coloured by `colors`, with a colorbar beside it.
fn extended(fig: &Figure, colorizer: ColorizerOptions, options: ColorbarOptions) -> CaseResult {
    let ax = fig.subplot();
    let im = ax.imshow(field(40), ImshowOptions::default().colorizer(colorizer))?;
    let options = ColorbarOptions {
        axes: ColorbarAxes::Parent(ax),
        ..options
    };
    fig.colorbar(&im, options)?;
    Ok(())
}

pub(super) fn extend_min_extendfrac(fig: &Figure) -> CaseResult {
    let options = ColorbarOptions {
        extend: Some(Extend::Min),
        extendfrac: ExtendFrac::Both(0.25),
        ..ColorbarOptions::default()
    };
    extended(
        fig,
        ColorizerOptions::default().limits(Some(-0.3), None),
        options,
    )
}

pub(super) fn extend_max_extendrect(fig: &Figure) -> CaseResult {
    let options = ColorbarOptions {
        extend: Some(Extend::Max),
        extendrect: true,
        ..ColorbarOptions::default()
    };
    extended(
        fig,
        ColorizerOptions::default().limits(None, Some(0.3)),
        options,
    )
}

pub(super) fn extend_both_horizontal_auto(fig: &Figure) -> CaseResult {
    let colors = ColorizerOptions::default()
        .cmap(colormaps::get("coolwarm")?)
        .limits(Some(-0.3), Some(0.3));
    let options = ColorbarOptions {
        orientation: Some(Orientation::Horizontal),
        extend: Some(Extend::Both),
        extendfrac: ExtendFrac::Auto,
        ..ColorbarOptions::default()
    };
    extended(fig, colors, options)
}
