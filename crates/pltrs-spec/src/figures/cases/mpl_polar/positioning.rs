//! `test_colorbar_positioning` of `test_colorbar.py`: colorbars on every
//! side of contoured axes, two colorbars beside one axes, and colorbars
//! shared by several axes, placed in the parent's grid cell or not.

use pltrs::colorbar::{ColorbarAxes, ColorbarOptions, ParentAnchor};
use pltrs::colors::Extend;
use pltrs::contour::{ContourOptions, ContourSet, Levels, QuadField};
use pltrs::numeric::Grid;
use pltrs::orientation::Orientation;
use pltrs::prelude::*;
use pltrs::rc::Aspect;
use pltrs::spines::SpineSide;
use pltrs_core::Anchor;

use super::{colors, ramp, ramp_levels, remove_ticks_and_titles};
use crate::figures::CaseResult;

/// The ramp filled at its levels in `ax`.
fn filled_ramp(ax: &Axes) -> Result<ContourSet, pltrs::contour::ContourError> {
    ax.contourf(&ramp(), ramp_levels())
}

/// A horizontal colorbar below one contoured axes.
fn horizontal(fig: &Figure, use_gridspec: bool) -> CaseResult {
    let cs = filled_ramp(&fig.subplot())?;
    fig.colorbar(
        &cs,
        ColorbarOptions {
            orientation: Some(Orientation::Horizontal),
            use_gridspec,
            ..ColorbarOptions::default()
        },
    )?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// A colorbar on each side of four contoured axes.
fn locations(fig: &Figure, use_gridspec: bool) -> CaseResult {
    for (i, location) in [
        SpineSide::Left,
        SpineSide::Right,
        SpineSide::Top,
        SpineSide::Bottom,
    ]
    .into_iter()
    .enumerate()
    {
        let cs = filled_ramp(&fig.add_subplot(2, 2, i + 1)?)?;
        fig.colorbar(
            &cs,
            ColorbarOptions {
                location: Some(location),
                use_gridspec,
                ..ColorbarOptions::default()
            },
        )?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// Colours and hatches on one axes, each with its colorbar.
fn double(fig: &Figure, use_gridspec: bool) -> CaseResult {
    let ax = fig.subplot();
    let blocks = [
        [2.0, 3.0, 2.0, 3.0],
        [1.5, 2.0, 2.0, 3.0],
        [2.0, 3.0, 3.0, 4.0],
    ];
    let data_2nd = QuadField::new(Grid::from_fn(30, 40, |r, c| blocks[r / 10][c / 10]));
    let color_mappable = ax.contourf(
        &ramp(),
        ContourOptions {
            extend: Extend::Both,
            ..ramp_levels()
        },
    )?;
    let hatch_levels = || Levels::Values(vec![1.0, 2.0, 3.0]);
    let hatch_mappable = ax.contourf(
        &data_2nd,
        ContourOptions {
            levels: hatch_levels(),
            colors: Some(colors(&["none"])),
            hatches: ["/", "o", "+"]
                .into_iter()
                .map(|h| Some(h.to_owned()))
                .collect(),
            extend: Extend::Max,
            ..ContourOptions::default()
        },
    )?;
    ax.contour(
        &data_2nd,
        ContourOptions {
            levels: hatch_levels(),
            colors: Some(colors(&["black"])),
            ..ContourOptions::default()
        },
    )?;
    for (mappable, location, label) in [
        (&color_mappable, SpineSide::Left, "variable 1"),
        (&hatch_mappable, SpineSide::Right, "variable 2"),
    ] {
        fig.colorbar(
            mappable,
            ColorbarOptions {
                location: Some(location),
                label: label.to_owned(),
                use_gridspec,
                ..ColorbarOptions::default()
            },
        )?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn colorbar_positioning_gridspec_1(fig: &Figure) -> CaseResult {
    horizontal(fig, true)
}

pub(super) fn colorbar_positioning_gridspec_2(fig: &Figure) -> CaseResult {
    locations(fig, true)
}

pub(super) fn colorbar_positioning_gridspec_3(fig: &Figure) -> CaseResult {
    double(fig, true)
}

pub(super) fn colorbar_positioning_1(fig: &Figure) -> CaseResult {
    horizontal(fig, false)
}

pub(super) fn colorbar_positioning_2(fig: &Figure) -> CaseResult {
    locations(fig, false)
}

pub(super) fn colorbar_positioning_3(fig: &Figure) -> CaseResult {
    double(fig, false)
}

/// Colorbars shared by three axes, one of them square and anchored to
/// the upper right.
pub(super) fn colorbar_positioning_4(fig: &Figure) -> CaseResult {
    let ax1 = fig.add_subplot(2, 1, 1)?;
    ax1.set_anchor(Anchor::NE).set_aspect(Aspect::Equal)?;
    filled_ramp(&ax1)?;
    let ax2 = fig.add_subplot(2, 2, 3)?;
    filled_ramp(&ax2)?;
    let ax3 = fig.add_subplot(2, 2, 4)?;
    let cs = filled_ramp(&ax3)?;
    let all = || ColorbarAxes::Parents(vec![ax2.clone(), ax3.clone(), ax1.clone()]);
    let shared = [
        ColorbarOptions {
            axes: all(),
            location: Some(SpineSide::Right),
            pad: Some(0.0),
            shrink: 0.5,
            panchor: ParentAnchor::Unchanged,
            ..ColorbarOptions::default()
        },
        ColorbarOptions {
            axes: all(),
            location: Some(SpineSide::Left),
            shrink: 0.5,
            panchor: ParentAnchor::Unchanged,
            ..ColorbarOptions::default()
        },
        ColorbarOptions {
            axes: ColorbarAxes::Parents(vec![ax1.clone()]),
            location: Some(SpineSide::Bottom),
            panchor: ParentAnchor::Unchanged,
            anchor: Some(Anchor::Fraction(0.8, 0.5)),
            shrink: 0.6,
            ..ColorbarOptions::default()
        },
    ];
    for options in shared {
        fig.colorbar(&cs, options)?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}
