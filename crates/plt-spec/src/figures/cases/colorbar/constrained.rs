//! Colorbars under constrained layout.

use plt::axes::ImshowOptions;
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::colors::colormaps;
use plt::figure::Figure;
use plt::image::ColorizerOptions;
use plt::layout::LayoutEngine;
use plt::spines::SpineSide;

use super::data::field;
use crate::figures::CaseResult;

/// `_field(20) * gain`.
fn scaled_field(gain: f64) -> plt::numeric::Grid<f64> {
    field(20).map(|v| v * gain)
}

pub(super) fn constrained_layout(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained());
    let axs = fig.subplots(2, 2)?;
    for (i, ax) in axs.as_slice().iter().enumerate() {
        let im = ax.imshow(
            scaled_field(i as f64 + 1.0),
            ImshowOptions::default().cmap(colormaps::get("viridis")?),
        )?;
        let options = ColorbarOptions {
            axes: ColorbarAxes::Parent(ax.clone()),
            ..ColorbarOptions::default()
        };
        fig.colorbar(&im, options)?;
    }
    Ok(())
}

pub(super) fn constrained_shared(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained());
    let axs = fig.subplots(2, 2)?;
    let limits = || {
        ImshowOptions::default().colors(ColorizerOptions::default().limits(Some(-2.0), Some(2.0)))
    };
    let mut last = None;
    for (i, ax) in axs.as_slice().iter().enumerate() {
        last = Some(ax.imshow(scaled_field(i as f64 + 1.0), limits())?);
    }
    let im = last.ok_or("no image")?;
    let options = ColorbarOptions {
        axes: ColorbarAxes::Parents(axs.as_slice().to_vec()),
        shrink: 0.8,
        location: Some(SpineSide::Bottom),
        ..ColorbarOptions::default()
    };
    fig.colorbar(&im, options)?;
    Ok(())
}
