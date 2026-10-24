//! `axisartist` cases of `figures/cases_axes_misc.py`.

use plt::axes::AxisName;
use plt::axisartist::{self, AxislineStyle};
use plt::prelude::*;

use crate::figures::{CaseResult, FigureCase};

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("misc/axisartist_floating_axis", floating_axis).sized(4.0, 4.0, 100.0),
    FigureCase::new("misc/axisartist_axeszero_arrows", axeszero_arrows).sized(4.0, 4.0, 100.0),
];

fn floating_axis(fig: &Figure) -> CaseResult {
    let ax = axisartist::AxisArtistAxes::new(fig.subplot());
    ax.new_floating_axis("y=0", AxisName::X, 0.0);
    ax.new_floating_axis("x=0", AxisName::Y, 0.0);
    for name in ["top", "right"] {
        ax.axis(name).ok_or("no such axis")?.set_visible(false);
    }
    let x = linspace(-2.0, 2.0, 50);
    let y: Vec<f64> = x.iter().map(|x| x.powi(3) - 2.0 * x).collect();
    ax.ax().plot(x, y)?;
    ax.ax().set_ylim(-3.0, 3.0)?;
    Ok(())
}

fn axeszero_arrows(fig: &Figure) -> CaseResult {
    let ax = axisartist::AxisArtistAxes::zero(fig.subplot());
    for name in ["xzero", "yzero"] {
        let axis = ax.axis(name).ok_or("no such axis")?;
        axis.set_axisline_style(Some("-|>".parse::<AxislineStyle>()?));
        axis.set_visible(true);
    }
    for name in ["left", "right", "bottom", "top"] {
        ax.axis(name).ok_or("no such axis")?.set_visible(false);
    }
    let x = linspace(-0.5, 1.0, 50);
    let y: Vec<f64> = x.iter().map(|x| (x * std::f64::consts::PI).sin()).collect();
    ax.ax().plot(x, y)?;
    Ok(())
}
