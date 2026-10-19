//! `test_legend3d.py`: legends of 3-D lines, bars, scatter plots and
//! error bars.

use plt::axes::{BarAlign, BarOptions, ErrorSize};
use plt::legend::{LegendHandle, LegendOptions};
use plt::mplot3d::{Axis3, Errorbar3DOptions};
use plt::prelude::*;

use super::remove_ticks_and_titles;
use crate::figures::CaseResult;

/// `np.arange(10)`.
fn ten() -> Vec<f64> {
    (0..10).map(f64::from).collect()
}

pub(super) fn legend_plot(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let x = ten();
    for (sign, label) in [(-1.0, "z=1"), (1.0, "z=-1")] {
        let y: Vec<f64> = x.iter().map(|x| sign * (x - 5.0)).collect();
        let props = LineProps::fmt("o")?.label(label);
        ax.plot_with(&x, &y, &[0.0], Axis3::Y, props)?;
    }
    ax.legend(LegendOptions::default())?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn legend_bar(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let x = ten();
    let reversed: Vec<f64> = x.iter().rev().copied().collect();
    let bars = |color: &str| BarOptions {
        align: BarAlign::Edge,
        color: vec![color.into()],
        ..BarOptions::default()
    };
    let up = ax.bar(&x, &x, &[0.0], Axis3::Y, bars("m"))?;
    let down = ax.bar(&x, &reversed, &[0.0], Axis3::X, bars("g"))?;
    let handles = [LegendHandle::from(&up[0]), LegendHandle::from(&down[0])];
    ax.legend(
        LegendOptions::default()
            .handles(handles)
            .labels(["up", "down"]),
    )?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn fancy(fig: &Figure) -> CaseResult {
    let ax = fig.subplot_3d();
    let x = ten();
    let props = LineProps::fmt("o--")?.label("line");
    ax.plot_with(&x, &[5.0; 10], &[5.0; 10], Axis3::Z, props)?;
    let countdown: Vec<f64> = x.iter().map(|x| 10.0 - x).collect();
    ax.scatter(&x, &countdown, &[0.0])?.set_label("scatter");
    let options = Errorbar3DOptions {
        errors: [Some(ErrorSize::from(0.5)), None, Some(ErrorSize::from(0.5))],
        line: LineProps::default().label("errorbar"),
        ..Errorbar3DOptions::default()
    };
    ax.errorbar(&[5.0; 10], &x, &[10.0; 10], options)?;
    let options = LegendOptions {
        loc: Some("lower left".into()),
        ncols: 2,
        title: Some("My legend".into()),
        numpoints: Some(1),
        ..LegendOptions::default()
    };
    ax.legend(options)?;
    remove_ticks_and_titles(fig);
    Ok(())
}
