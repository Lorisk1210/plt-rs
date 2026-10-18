//! Colorbars following later changes of their image: new limits and
//! colormap, and a new norm.

use plt::axes::ImshowOptions;
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::colors::{LogNorm, colormaps};
use plt::figure::Figure;

use super::data::field;
use crate::figures::CaseResult;

/// Colorbar options stealing space from `ax`.
fn beside(ax: &plt::axes::Axes) -> ColorbarOptions {
    ColorbarOptions {
        axes: ColorbarAxes::Parent(ax.clone()),
        ..ColorbarOptions::default()
    }
}

pub(super) fn follows_set_clim(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let im = ax.imshow(field(40), ImshowOptions::default())?;
    fig.colorbar(&im, beside(&ax))?;
    im.set_clim(Some(-0.2), Some(0.4));
    im.set_cmap(colormaps::get("magma")?);
    Ok(())
}

pub(super) fn follows_norm_change(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let data = field(40).map(|&v| 10f64.powf(2.0 * v + 1.0));
    let im = ax.imshow(data, ImshowOptions::default())?;
    fig.colorbar(&im, beside(&ax))?;
    im.set_norm(Box::new(LogNorm::default()));
    Ok(())
}
