//! Where colorbars go: orientation, location, size and padding, several
//! parents, an explicit `cax` and an inset `cax`.

use plt::axes::{Axes, ImshowOptions};
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::colors::{Colorize, ColorizerOptions, colormaps};
use plt::figure::Figure;
use plt::orientation::Orientation;
use plt::rc::Aspect;
use plt::spines::SpineSide;

use super::data::field;
use crate::figures::CaseResult;

/// `fig.colorbar(imshow(_field()), ax=ax, ...)` on a single subplot.
fn image_colorbar(fig: &Figure, image: ImshowOptions, options: ColorbarOptions) -> CaseResult {
    let ax = fig.subplot();
    let im = ax.imshow(field(40), image)?;
    fig.colorbar(&im, beside(&ax, options))?;
    Ok(())
}

/// `options` stealing space from `ax`.
fn beside(ax: &Axes, options: ColorbarOptions) -> ColorbarOptions {
    ColorbarOptions {
        axes: ColorbarAxes::Parent(ax.clone()),
        ..options
    }
}

pub(super) fn image_default(fig: &Figure) -> CaseResult {
    image_colorbar(fig, ImshowOptions::default(), ColorbarOptions::default())
}

pub(super) fn horizontal(fig: &Figure) -> CaseResult {
    let options = ColorbarOptions {
        orientation: Some(Orientation::Horizontal),
        ..ColorbarOptions::default()
    };
    image_colorbar(
        fig,
        ImshowOptions::default().cmap(colormaps::get("plasma")?),
        options,
    )
}

pub(super) fn location_top(fig: &Figure) -> CaseResult {
    let options = ColorbarOptions {
        location: Some(SpineSide::Top),
        ..ColorbarOptions::default()
    };
    image_colorbar(fig, ImshowOptions::default(), options)
}

pub(super) fn location_left(fig: &Figure) -> CaseResult {
    let options = ColorbarOptions {
        location: Some(SpineSide::Left),
        ..ColorbarOptions::default()
    };
    image_colorbar(fig, ImshowOptions::default(), options)
}

pub(super) fn shrink_aspect_pad(fig: &Figure) -> CaseResult {
    let options = ColorbarOptions {
        shrink: 0.6,
        aspect: 8.0,
        pad: Some(0.15),
        ..ColorbarOptions::default()
    };
    image_colorbar(fig, ImshowOptions::default(), options)
}

pub(super) fn fraction(fig: &Figure) -> CaseResult {
    let image = ImshowOptions {
        aspect: Some(Aspect::Auto),
        ..ImshowOptions::default()
    };
    let options = ColorbarOptions {
        fraction: 0.3,
        ..ColorbarOptions::default()
    };
    image_colorbar(fig, image, options)
}

pub(super) fn multiple_axes(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 1)?;
    let limits = || {
        ImshowOptions::default()
            .colorizer(ColorizerOptions::default().limits(Some(-1.0), Some(1.0)))
    };
    let im = axs[(0, 0)].imshow(field(40), limits())?;
    axs[(1, 0)].imshow(field(40).map(|v| -v), limits())?;
    let options = ColorbarOptions {
        axes: ColorbarAxes::Parents(axs.as_slice().to_vec()),
        ..ColorbarOptions::default()
    };
    fig.colorbar(&im, options)?;
    Ok(())
}

pub(super) fn cax_explicit(fig: &Figure) -> CaseResult {
    let ax = fig.add_axes([0.1, 0.1, 0.65, 0.8]);
    let cax = fig.add_axes([0.82, 0.1, 0.04, 0.8]);
    let image = ImshowOptions {
        aspect: Some(Aspect::Auto),
        ..ImshowOptions::default()
    };
    let im = ax.imshow(field(40), image)?;
    let options = ColorbarOptions {
        axes: ColorbarAxes::Cax(cax),
        ..ColorbarOptions::default()
    };
    fig.colorbar(&im, options)?;
    Ok(())
}

pub(super) fn inset_cax_horizontal(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let im = ax.imshow(field(40), ImshowOptions::default())?;
    let cax = ax.inset_axes([0.1, 0.9, 0.8, 0.05]);
    let options = ColorbarOptions {
        axes: ColorbarAxes::Cax(cax),
        orientation: Some(Orientation::Horizontal),
        ..ColorbarOptions::default()
    };
    fig.colorbar(&im, options)?;
    Ok(())
}
