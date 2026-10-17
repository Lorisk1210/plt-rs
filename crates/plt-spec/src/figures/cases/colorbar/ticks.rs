//! Tick positions, tick formats, minor ticks and labels.

use std::rc::Rc;

use plt::axes::{ImshowOptions, LabelLoc};
use plt::colorbar::{ColorbarAxes, ColorbarFormat, ColorbarOptions, ColorbarTicks};
use plt::figure::Figure;
use plt::ticker::PercentFormatter;

use super::data::{cmap, field};
use crate::figures::CaseResult;

pub(super) fn ticks_format_label(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let im = ax.imshow(field(40), ImshowOptions::default())?;
    let options = ColorbarOptions {
        axes: ColorbarAxes::Parent(ax),
        ticks: Some(ColorbarTicks::Values(vec![-0.6, -0.2, 0.0, 0.2, 0.6])),
        format: Some(ColorbarFormat::Template("%.2f".into())),
        label: "amplitude".into(),
        ..ColorbarOptions::default()
    };
    fig.colorbar(&im, options)?;
    Ok(())
}

pub(super) fn formatter_minorticks(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let im = ax.imshow(field(40).map(|v| v.abs()), cmap("magma")?)?;
    let options = ColorbarOptions {
        axes: ColorbarAxes::Parent(ax),
        format: Some(ColorbarFormat::Formatter(Rc::new(PercentFormatter::new(
            1.0,
        )))),
        ..ColorbarOptions::default()
    };
    let colorbar = fig.colorbar(&im, options)?;
    colorbar.minorticks_on();
    colorbar.set_label("fraction", Some(LabelLoc::End));
    Ok(())
}
