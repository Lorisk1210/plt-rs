//! `figures/cases_axisartist.py`: the direction and styling of axis
//! artists in one submodule, curvilinear grids and floating axes in the
//! other.

mod curvilinear;
mod directions;

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
