//! Colorbars under constrained layout.

use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::figure::Figure;
use plt::image::ColorizerOptions;
use plt::layout::{ConstrainedLayout, LayoutEngine};
use plt::spines::SpineSide;

use super::data::{cmap, colored, field};
use crate::figures::CaseResult;

/// `fig.set_layout_engine("constrained")`.
fn constrained(fig: &Figure) {
    fig.set_layout_engine(Some(
        LayoutEngine::Constrained(ConstrainedLayout::default()),
    ));
}

/// `_field(20) * gain`.
fn scaled_field(gain: f64) -> plt::numeric::Grid<f64> {
    field(20).map(|v| v * gain)
}

pub(super) fn constrained_layout(fig: &Figure) -> CaseResult {
    constrained(fig);
    let axs = fig.subplots(2, 2)?;
    for (i, ax) in axs.as_slice().iter().enumerate() {
        let im = ax.imshow(scaled_field(i as f64 + 1.0), cmap("viridis")?)?;
        let options = ColorbarOptions {
            axes: ColorbarAxes::Parent(ax.clone()),
            ..ColorbarOptions::default()
        };
        fig.colorbar(&im, options)?;
    }
    Ok(())
}

pub(super) fn constrained_shared(fig: &Figure) -> CaseResult {
    constrained(fig);
    let axs = fig.subplots(2, 2)?;
    let limits = || colored(ColorizerOptions::default().limits(Some(-2.0), Some(2.0)));
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
