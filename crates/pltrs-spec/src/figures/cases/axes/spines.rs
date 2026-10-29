//! Spine cases of `figures/cases_axes.py`.

use pltrs::prelude::*;
use pltrs::spines::{SpinePosition, SpineSide};

use super::line;
use crate::figures::FigureCase;

/// Hides the top and right spines.
fn hide_top_right(ax: &Axes) {
    let spines = ax.spines();
    for side in [SpineSide::Top, SpineSide::Right] {
        spines.get(side).set_visible(false);
    }
}

/// Moves the left and bottom spines to `position`.
fn move_left_bottom(ax: &Axes, position: SpinePosition) {
    let spines = ax.spines();
    for side in [SpineSide::Left, SpineSide::Bottom] {
        spines.get(side).set_position(position);
    }
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("axes/spines_hidden_top_right", |fig| {
        let ax = fig.subplot();
        line(&ax)?;
        hide_top_right(&ax);
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/spines_center_and_zero", |fig| {
        let axs = fig.subplots(1, 2)?;
        let t = linspace(-3.0, 5.0, 50);
        let y: Vec<f64> = t.iter().map(|t| t.sin() + 0.5).collect();
        let positions = [SpinePosition::Axes(0.5), SpinePosition::Data(0.0)];
        for (ax, position) in axs.as_slice().iter().zip(positions) {
            ax.plot(&t, &y)?;
            move_left_bottom(ax, position);
            hide_top_right(ax);
        }
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("axes/spines_outward_and_data", |fig| {
        let axs = fig.subplots(1, 2)?;
        line(&axs[(0, 0)])?;
        move_left_bottom(&axs[(0, 0)], SpinePosition::Outward(10.0));
        hide_top_right(&axs[(0, 0)]);
        line(&axs[(0, 1)])?;
        let spines = axs[(0, 1)].spines();
        spines
            .get(SpineSide::Left)
            .set_position(SpinePosition::Data(4.0));
        spines
            .get(SpineSide::Bottom)
            .set_position(SpinePosition::Axes(0.3));
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("axes/spine_bounds_and_style", |fig| {
        let ax = fig.subplot();
        line(&ax)?;
        let spines = ax.spines();
        spines.get(SpineSide::Left).set_bounds(Some((-1.0, 1.0)));
        spines.get(SpineSide::Bottom).set_bounds(Some((0.0, 10.0)));
        hide_top_right(&ax);
        spines.get(SpineSide::Left).set_linewidth(2.0);
        spines.get(SpineSide::Bottom).set_color("tab:red");
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
];
