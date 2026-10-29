//! Axis decorations: tick parameters, tick formatters and locators with
//! random templates and counts, explicit ticks, grids and limits.

use pltrs::axes::{Axes, GridOptions, TickDirection, TickLevel, TickParams};
use pltrs::fonts::FontSize;
use pltrs::ticker::{
    EngFormatter, FormatStrFormatter, LinearLocator, LogLocator, MaxNLocator, MultipleLocator,
    Nbins, PercentFormatter, StrMethodFormatter,
};

use super::data::{count, length, series};
use crate::harness::Gen;

/// Tick label templates, printf and `str.format`.
pub(crate) const TEMPLATES: [&str; 5] = ["%1.2f", "%d m", "{x:.3g}", "{x:>20}", "%.30e"];

/// A decoration of the axes' x or y axis.
#[derive(Debug)]
pub(crate) enum AxisDecor {
    TickParams {
        length: f64,
        width: f64,
        pad: f64,
        rotation: f64,
        labelsize: f64,
        inward: bool,
    },
    Formatter {
        x: bool,
        kind: usize,
        template: String,
        decimals: i32,
    },
    Locator {
        x: bool,
        kind: usize,
        value: f64,
        count: usize,
    },
    Ticks {
        x: bool,
        positions: Vec<f64>,
        labels: bool,
    },
    Grid {
        alpha: f64,
        width: f64,
        minor: bool,
    },
    Limits {
        x: (f64, f64),
        y: (f64, f64),
    },
}

impl AxisDecor {
    /// A random decoration.
    pub(crate) fn generate(g: &mut Gen) -> Self {
        match g.below(6) {
            0 => Self::TickParams {
                length: g.number_in(0.0, 20.0),
                width: g.number_in(0.0, 5.0),
                pad: g.number_in(-5.0, 20.0),
                rotation: g.number_in(-180.0, 180.0),
                labelsize: g.number_in(1.0, 30.0),
                inward: g.chance(0.5),
            },
            1 => Self::Formatter {
                x: g.chance(0.5),
                kind: g.below(4),
                template: g.text(&TEMPLATES, &["%", "{", "}", ".", "9", "f", "x"]),
                decimals: g.below(12) as i32 - 2,
            },
            2 => Self::Locator {
                x: g.chance(0.5),
                kind: g.below(4),
                value: g.number(),
                count: count(g, 10),
            },
            3 => {
                let len = length(g);
                Self::Ticks {
                    x: g.chance(0.5),
                    positions: series(g, len),
                    labels: g.chance(0.5),
                }
            }
            4 => Self::Grid {
                alpha: g.number_in(0.0, 1.0),
                width: g.number_in(0.0, 3.0),
                minor: g.chance(0.3),
            },
            _ => Self::Limits {
                x: (g.number(), g.number()),
                y: (g.number(), g.number()),
            },
        }
    }

    /// Applies the decoration to `ax`; errors are fine.
    pub(crate) fn run(&self, ax: &Axes) {
        match self {
            Self::TickParams {
                length,
                width,
                pad,
                rotation,
                labelsize,
                inward,
            } => {
                let params = TickParams {
                    which: TickLevel::Both,
                    length: Some(*length),
                    width: Some(*width),
                    pad: Some(*pad),
                    labelrotation: Some(*rotation),
                    labelsize: Some(FontSize::Points(*labelsize)),
                    direction: Some(if *inward {
                        TickDirection::In
                    } else {
                        TickDirection::Out
                    }),
                    ..TickParams::default()
                };
                ax.tick_params(params);
            }
            Self::Formatter {
                x,
                kind,
                template,
                decimals,
            } => formatter(ax, *x, *kind, template, *decimals),
            Self::Locator {
                x,
                kind,
                value,
                count,
            } => locator(ax, *x, *kind, *value, *count),
            Self::Ticks {
                x,
                positions,
                labels,
            } => {
                let axis = if *x { ax.xaxis() } else { ax.yaxis() };
                axis.set_ticks(positions.clone());
                if *labels {
                    let texts: Vec<String> = positions.iter().map(|p| format!("{p:e}")).collect();
                    drop(axis.set_ticklabels(texts));
                }
            }
            Self::Grid {
                alpha,
                width,
                minor,
            } => {
                let options = GridOptions {
                    which: if *minor {
                        TickLevel::Both
                    } else {
                        TickLevel::Major
                    },
                    alpha: Some(*alpha),
                    linewidth: Some(*width),
                    ..GridOptions::default()
                };
                ax.grid_with(true, options);
            }
            Self::Limits { x, y } => {
                let _ = ax.set_xlim(x.0, x.1);
                let _ = ax.set_ylim(y.0, y.1);
            }
        }
    }
}

/// A tick formatter: printf, `str.format`, percent or engineering.
fn formatter(ax: &Axes, x: bool, kind: usize, template: &str, decimals: i32) {
    let axis = if x { ax.xaxis() } else { ax.yaxis() };
    match kind {
        0 => {
            if let Ok(formatter) = FormatStrFormatter::new(template) {
                axis.set_major_formatter(formatter);
            }
        }
        1 => {
            if let Ok(formatter) = StrMethodFormatter::new(template) {
                axis.set_major_formatter(formatter);
            }
        }
        2 => {
            axis.set_major_formatter(PercentFormatter::new(100.0).with_decimals(Some(decimals)));
        }
        _ => {
            let places = usize::try_from(decimals).ok();
            axis.set_major_formatter(EngFormatter::new().with_places(places));
        }
    }
}

/// A tick locator with random parameters.
fn locator(ax: &Axes, x: bool, kind: usize, value: f64, count: usize) {
    let axis = if x { ax.xaxis() } else { ax.yaxis() };
    match kind {
        0 => {
            axis.set_major_locator(MaxNLocator::new(Nbins::Fixed(count)));
        }
        1 => {
            if let Ok(locator) = MultipleLocator::new(value) {
                axis.set_major_locator(locator);
            }
        }
        2 => {
            axis.set_major_locator(LinearLocator::new(Some(count)));
        }
        _ => {
            if let Ok(locator) = LogLocator::new(value) {
                axis.set_major_locator(locator);
            }
        }
    }
}
