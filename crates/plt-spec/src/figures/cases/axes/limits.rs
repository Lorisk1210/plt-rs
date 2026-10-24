//! Limit, margin, autoscaling and frame cases of `figures/cases_axes.py`.

use plt::axes::{Adjustable, AxisMode, AxisName};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::transforms::Anchor;

use super::line;
use crate::figures::FigureCase;

/// `CIRCLE = np.linspace(0, 2 * np.pi, 100)`.
fn circle() -> Vec<f64> {
    linspace(0.0, 2.0 * std::f64::consts::PI, 100)
}

/// `_ellipse(ax)`: `ax.plot(3 * np.cos(CIRCLE), np.sin(CIRCLE))`.
fn ellipse(ax: &Axes) -> Result<(), plt::axes::PlotError> {
    let x: Vec<f64> = circle().iter().map(|t| 3.0 * t.cos()).collect();
    let y: Vec<f64> = circle().iter().map(|t| t.sin()).collect();
    ax.plot(x, y).map(drop)
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("axes/set_limits", |fig| {
        let ax = fig.subplot();
        line(&ax)?;
        ax.set_xlim(2.0, 8.0)?.set_ylim(-2.0, 1.5)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/reversed_limits", |fig| {
        let ax = fig.subplot();
        line(&ax)?;
        ax.set_xlim(10.0, 0.0)?.set_ylim(1.5, -1.5)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/invert_axes", |fig| {
        let axs = fig.subplots(1, 2)?;
        for (ax, name) in axs.as_slice().iter().zip([AxisName::X, AxisName::Y]) {
            line(ax)?;
            let axis = ax.axis_of(name);
            axis.set_inverted(!axis.inverted());
        }
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("axes/margins", |fig| {
        let axs = fig.subplots(1, 2)?;
        line(&axs[(0, 0)])?;
        axs[(0, 0)].set_margins(0.25, 0.25)?;
        line(&axs[(0, 1)])?;
        axs[(0, 1)].set_margins(0.0, -0.25)?;
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("axes/autoscale_tight", |fig| {
        let ax = fig.subplot();
        line(&ax)?;
        ax.autoscale(None, Some(true));
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/relim", |fig| {
        let grid = fig.subplots(1, 2)?;
        let (ax1, ax2) = (&grid[(0, 0)], &grid[(0, 1)]);
        let line = ax1.plot([0.0, 1.0], [0.0, 1.0])?;
        ax1.bar([2.0], [1.0], BarOptions::default())?;
        ax1.scatter([3.5], [-1.5])?;
        line.set_data(&[0.0, 4.0], &[-2.0, 3.0]);
        ax1.relim(false).autoscale_view(None);
        ax2.plot([0.0, 1.0], [0.0, 1.0])?;
        ax2.plot([-5.0, 5.0], [-5.0, 5.0])?.set_visible(false);
        ax2.relim(true).autoscale_view(None);
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("axes/aspect_equal_box", |fig| {
        let ax = fig.subplot();
        ellipse(&ax)?;
        ax.set_aspect(Aspect::Equal)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/aspect_equal_datalim", |fig| {
        let ax = fig.subplot();
        ellipse(&ax)?;
        ax.set_aspect(Aspect::Equal)?
            .set_adjustable(Adjustable::Datalim)?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/aspect_numeric_anchor", |fig| {
        let ax = fig.subplot();
        ellipse(&ax)?;
        ax.set_aspect(Aspect::Ratio(2.0))?.set_anchor(Anchor::SW);
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/box_aspect", |fig| {
        let axs = fig.subplots(1, 2)?;
        for (ax, ratio) in axs.as_slice().iter().zip([1.0, 0.5]) {
            line(ax)?;
            ax.set_box_aspect(Some(ratio))?;
        }
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("axes/axis_modes_equal_scaled_tight_square", |fig| {
        let axs = fig.subplots(2, 2)?;
        fig.subplots_adjust(&plt::gridspec::SubplotOverrides {
            hspace: Some(0.4),
            ..plt::gridspec::SubplotOverrides::default()
        })?;
        let modes = [
            (AxisMode::Equal, "equal"),
            (AxisMode::Scaled, "scaled"),
            (AxisMode::Tight, "tight"),
            (AxisMode::Square, "square"),
        ];
        for (ax, (mode, name)) in axs.as_slice().iter().zip(modes) {
            ellipse(ax)?;
            ax.axis_mode(mode);
            ax.set_title(name);
        }
        Ok(())
    })
    .sized(7.0, 6.0, 100.0),
    FigureCase::new("axes/axis_off", |fig| {
        let ax = fig.subplot();
        ellipse(&ax)?;
        ax.set_title("no axis decorations");
        ax.set_axis_off();
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("axes/frame_off_and_facecolor", |fig| {
        let axs = fig.subplots(1, 2)?;
        line(&axs[(0, 0)])?;
        axs[(0, 0)].set_frame_on(false);
        line(&axs[(0, 1)])?;
        axs[(0, 1)].set_facecolor("lightyellow");
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
];
