//! Colorbars following later changes of their image: new limits and
//! colormap, and a new norm.

use pltrs::axes::ImshowOptions;
use pltrs::colorbar::ColorbarOptions;
use pltrs::colors::{LogNorm, colormaps};
use pltrs::figure::Figure;
use pltrs::numeric::pow;

use super::data::field;
use crate::figures::CaseResult;

pub(super) fn follows_set_clim(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let im = ax.imshow(field(40), ImshowOptions::default())?;
    fig.colorbar(&im, ColorbarOptions::beside(&ax))?;
    im.set_clim(Some(-0.2), Some(0.4));
    im.set_cmap(colormaps::get("magma")?);
    Ok(())
}

pub(super) fn follows_norm_change(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let data = field(40).map(|&v| pow(10.0, 2.0 * v + 1.0));
    let im = ax.imshow(data, ImshowOptions::default())?;
    fig.colorbar(&im, ColorbarOptions::beside(&ax))?;
    im.set_norm(Box::new(LogNorm::default()));
    Ok(())
}
