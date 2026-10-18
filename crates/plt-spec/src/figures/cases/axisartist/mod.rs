//! `figures/cases_axisartist.py`: the direction and styling of axis
//! artists in one submodule, curvilinear grids and floating axes in the
//! other.

mod curvilinear;
mod directions;

use std::f64::consts::PI;

use plt::axisartist::angle_helper::{ExtremeFinderCycle, FormatterDms, LocatorDms};
use plt::axisartist::grid_finder::{GridFinderOptions, GridLocator};
use plt::axisartist::grid_helper_curvelinear::GridHelperCurveLinear;
use plt::axisartist::{self, AuxTransform, AxisArtist};
use plt::transforms::Affine2D;

use crate::figures::FigureCase;

/// Every case of the group.
pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new(
        "axisartist/simple_axis_direction01",
        directions::simple_axis_direction01,
    )
    .sized(4.0, 2.5, 100.0),
    FigureCase::new(
        "axisartist/simple_axis_direction03",
        directions::simple_axis_direction03,
    )
    .sized(5.0, 2.0, 100.0),
    FigureCase::new(
        "axisartist/ticklabel_alignment",
        directions::ticklabel_alignment,
    )
    .sized(3.0, 5.0, 100.0),
    FigureCase::new(
        "axisartist/ticklabel_direction",
        directions::ticklabel_direction,
    )
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("axisartist/axis_styling", directions::axis_styling).sized(6.0, 4.0, 100.0),
    FigureCase::new("axisartist/axis_direction", directions::axis_direction).sized(8.0, 4.0, 100.0),
    FigureCase::new("axisartist/simple_axis_pad", curvilinear::simple_axis_pad)
        .sized(9.0, 3.0, 100.0),
    FigureCase::new("axisartist/curvelinear_grid", curvilinear::curvelinear_grid)
        .sized(7.0, 4.0, 100.0),
    FigureCase::new(
        "axisartist/curvelinear_grid2",
        curvilinear::curvelinear_grid2,
    )
    .sized(7.0, 4.0, 100.0),
    FigureCase::new("axisartist/floating_axes", curvilinear::floating_axes).sized(8.0, 4.0, 100.0),
    FigureCase::new("axisartist/floating_axis", curvilinear::floating_axis).sized(5.0, 5.0, 100.0),
];

/// `ax.axis[name]`.
fn artist(ax: &axisartist::Axes, name: &str) -> Result<AxisArtist, &'static str> {
    ax.axis(name).ok_or("no such axis artist")
}

/// `axis.label.set_text(text); axis.label.set_visible(True)`.
fn set_text(axis: &AxisArtist, text: &str) {
    axis.label().set_text(text).set_visible(true);
}

/// `Affine2D().scale(np.pi / 180, 1) + PolarAxes.PolarTransform()`.
fn polar_degrees() -> AuxTransform {
    AuxTransform::affine(Affine2D::scaling(PI / 180.0, 1.0)).then(AuxTransform::polar())
}

/// `_polar_grid_helper(grid_locator2)`: the polar grid in degrees of
/// `demo_curvelinear_grid` in a rectangular box.
fn polar_grid_helper(grid_locator2: Option<Box<dyn GridLocator>>) -> GridHelperCurveLinear {
    GridHelperCurveLinear::new(
        polar_degrees(),
        GridFinderOptions {
            extreme_finder: Some(Box::new(ExtremeFinderCycle {
                lat_minmax: Some((0.0, f64::INFINITY)),
                ..ExtremeFinderCycle::new(20, 20)
            })),
            grid_locator1: Some(Box::new(LocatorDms::new(12))),
            grid_locator2,
            tick_formatter1: Some(Box::new(FormatterDms)),
            ..GridFinderOptions::default()
        },
    )
}
