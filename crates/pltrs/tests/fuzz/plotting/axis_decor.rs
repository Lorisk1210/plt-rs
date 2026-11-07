//! Axis decorations: tick parameters, tick formatters and locators with
//! random templates and counts, explicit ticks, grids and limits, the
//! tick label format and locator parameters, the axes box (its aspect,
//! adjustable and anchor) and secondary axes mapped by any scale.

use pltrs::axes::{
    Adjustable, Axes, GridOptions, LocatorParams, SecondaryLocation, TickDirection, TickLevel,
    TickParams, TicklabelFormat,
};
use pltrs::fonts::FontSize;
use pltrs::scale::FuncTransform;
use pltrs::ticker::{
    EngFormatter, FormatStrFormatter, LinearLocator, LogLocator, MaxNLocator, MultipleLocator,
    Nbins, OffsetMode, PercentFormatter, StrMethodFormatter,
};
use pltrs::transforms::Anchor;

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
    TickFormat {
        scientific: bool,
        scilimits: (i32, i32),
        offset: Option<f64>,
        nbins: usize,
        integer: bool,
        symmetric: bool,
    },
    BoxLayout {
        box_aspect: Option<f64>,
        datalim: bool,
        anchor: Anchor,
        minor_ticks: bool,
    },
    Secondary {
        x: bool,
        location: f64,
        scale: f64,
    },
}

/// Anchors of a shrunken axes: the named ones and fractions of any size.
const ANCHORS: [Anchor; 5] = [
    Anchor::C,
    Anchor::SW,
    Anchor::NE,
    Anchor::Fraction(0.25, 0.75),
    Anchor::Fraction(-3.0, f64::NAN),
];

impl AxisDecor {
    /// A random decoration.
    pub(crate) fn generate(g: &mut Gen) -> Self {
        match g.below(9) {
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
            5 => Self::Limits {
                x: (g.number(), g.number()),
                y: (g.number(), g.number()),
            },
            6 => Self::TickFormat {
                scientific: g.chance(0.5),
                scilimits: (g.below(12) as i32 - 6, g.below(12) as i32 - 6),
                offset: g.chance(0.3).then(|| g.number()),
                nbins: count(g, 12),
                integer: g.chance(0.3),
                symmetric: g.chance(0.3),
            },
            7 => Self::BoxLayout {
                box_aspect: g.chance(0.7).then(|| g.number()),
                datalim: g.chance(0.5),
                anchor: g.pick(&ANCHORS),
                minor_ticks: g.chance(0.5),
            },
            _ => Self::Secondary {
                x: g.chance(0.5),
                location: g.number_in(-0.5, 1.5),
                scale: g.number(),
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
            Self::TickFormat {
                scientific,
                scilimits,
                offset,
                nbins,
                integer,
                symmetric,
            } => tick_format(
                ax,
                *scientific,
                *scilimits,
                *offset,
                *nbins,
                (*integer, *symmetric),
            ),
            Self::BoxLayout {
                box_aspect,
                datalim,
                anchor,
                minor_ticks,
            } => box_layout(ax, *box_aspect, *datalim, *anchor, *minor_ticks),
            Self::Secondary { x, location, scale } => secondary(ax, *x, *location, *scale),
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

/// The tick label format (scientific or not, its limits, an offset) and
/// the major locators' parameters (bins, integer or symmetric ticks).
fn tick_format(
    ax: &Axes,
    scientific: bool,
    scilimits: (i32, i32),
    offset: Option<f64>,
    nbins: usize,
    (integer, symmetric): (bool, bool),
) {
    let format = TicklabelFormat {
        scientific: Some(scientific),
        scilimits: Some(scilimits),
        use_offset: Some(offset.map_or(OffsetMode::Auto, OffsetMode::Fixed)),
        ..TicklabelFormat::default()
    };
    let params = LocatorParams {
        nbins: Some(Nbins::Fixed(nbins)),
        integer: Some(integer),
        symmetric: Some(symmetric),
        ..LocatorParams::default()
    };
    let _ = ax.ticklabel_format(format);
    let _ = ax.locator_params(params);
}

/// The axes box: its aspect, what gives way to the data's aspect, where a
/// shrunken box sits, and minor ticks.
fn box_layout(ax: &Axes, box_aspect: Option<f64>, datalim: bool, anchor: Anchor, minor: bool) {
    let _ = ax.set_box_aspect(box_aspect);
    let adjustable = if datalim {
        Adjustable::Datalim
    } else {
        Adjustable::Box
    };
    let _ = ax.set_adjustable(adjustable);
    ax.set_anchor(anchor);
    if minor {
        ax.minorticks_on();
    }
}

/// A secondary x or y axis at `location` across the axes, its values the
/// host's times `scale` (zero, negative and NaN included).
fn secondary(ax: &Axes, x: bool, location: f64, scale: f64) {
    let functions = FuncTransform::new(move |v| v * scale, move |v| v / scale);
    let location = SecondaryLocation::Fraction(location);
    let _ = if x {
        ax.secondary_xaxis(location, Some(functions))
    } else {
        ax.secondary_yaxis(location, Some(functions))
    };
}
