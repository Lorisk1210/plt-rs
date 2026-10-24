//! Plotting calls on 2-D axes: lines, markers, bars, fills and reference
//! lines, with random data and options.

use plt::axes::{
    Axes, BandAlign, BarOptions, ErrorSize, ErrorbarOptions, FillBetweenOptions, MappedColors,
    ScatterColors, ScatterOptions, StackplotOptions, StairsOptions, StemOptions,
};
use plt::colors::Alpha;
use plt::lines::{AxLineThrough, LineProps, LineStyle, MarkEvery, StepWhere};
use plt::markers::MarkerStyle;
use plt::patches::PatchProps;
use plt::transforms::Point;

use super::Mappables;
use super::data::{each_or_one_in, few_in, length, partner, series};
use crate::harness::Gen;

/// How a line is styled.
#[derive(Debug)]
pub(crate) struct LineSpec {
    format: String,
    width: f64,
    dashes: Option<(f64, Vec<f64>)>,
    marker_size: f64,
    markevery: Option<MarkEvery>,
    alpha: f64,
    zorder: f64,
}

/// A plotting call of lines, markers, bars or fills.
#[derive(Debug)]
pub(crate) enum Call {
    Plot {
        x: Vec<f64>,
        y: Vec<f64>,
        line: LineSpec,
    },
    Scatter {
        x: Vec<f64>,
        y: Vec<f64>,
        sizes: Vec<f64>,
        marker: String,
        alpha: f64,
        mapped: bool,
    },
    Bar {
        x: Vec<f64>,
        heights: Vec<f64>,
        widths: Vec<f64>,
        horizontal: bool,
        log: bool,
        hatch: String,
    },
    FillBetween {
        x: Vec<f64>,
        y1: Vec<f64>,
        y2: Vec<f64>,
        step: bool,
        interpolate: bool,
    },
    Errorbar {
        x: Vec<f64>,
        y: Vec<f64>,
        yerr: Vec<f64>,
        capsize: f64,
        every: MarkEvery,
    },
    Stem {
        x: Vec<f64>,
        y: Vec<f64>,
    },
    Step {
        x: Vec<f64>,
        y: Vec<f64>,
    },
    Stairs {
        values: Vec<f64>,
        edges: Option<Vec<f64>>,
        fill: bool,
    },
    Segments {
        at: Vec<f64>,
        low: Vec<f64>,
        high: Vec<f64>,
        vertical: bool,
    },
    Reference {
        kind: usize,
        a: f64,
        b: f64,
        slope: f64,
    },
    Fill {
        x: Vec<f64>,
        y: Vec<f64>,
        hatch: String,
    },
    BrokenBarh {
        ranges: Vec<(f64, f64)>,
        y: f64,
        height: f64,
    },
    Stackplot {
        x: Vec<f64>,
        ys: Vec<Vec<f64>>,
    },
}

/// Line format strings, markers and hatches the generators pick from.
const FORMATS: [&str; 8] = ["-", "o--", "C2^:", "k.", "rs-.", "", "|", "$x$"];
const MARKERS: [&str; 8] = ["o", "s", "^", "$\\alpha$", "8", "None", "x", "|"];
const HATCHES: [&str; 6] = ["", "/", "x", "o.", "|||", "*"];

impl LineSpec {
    fn generate(g: &mut Gen) -> Self {
        Self {
            format: g.pick(&FORMATS).to_owned(),
            width: g.number_in(0.0, 5.0),
            dashes: g
                .chance(0.2)
                .then(|| (g.number_in(0.0, 3.0), few_in(g, 5, 0.0, 8.0))),
            marker_size: g.number_in(0.0, 20.0),
            markevery: g.chance(0.2).then(|| markevery(g)),
            alpha: g.number_in(0.0, 1.0),
            zorder: g.number_in(-3.0, 3.0),
        }
    }

    /// The line properties: the format string's, then the others.
    pub(crate) fn props(&self) -> LineProps {
        let mut props = LineProps::fmt(&self.format)
            .unwrap_or_default()
            .linewidth(self.width)
            .markersize(self.marker_size)
            .alpha(self.alpha)
            .zorder(self.zorder)
            .label("line");
        if let Some((offset, pattern)) = &self.dashes {
            props = props.linestyle(LineStyle::dashes(*offset, pattern.clone()));
        }
        if let Some(every) = &self.markevery {
            props = props.markevery(every.clone());
        }
        props
    }
}

impl Call {
    /// A random call.
    pub(crate) fn generate(g: &mut Gen) -> Self {
        let n = length(g);
        match g.below(13) {
            0 | 1 => Self::Plot {
                x: series(g, n),
                y: partner(g, n),
                line: LineSpec::generate(g),
            },
            2 => Self::Scatter {
                x: series(g, n),
                y: partner(g, n),
                sizes: each_or_one_in(g, n, 0.0, 400.0),
                marker: g.pick(&MARKERS).to_owned(),
                alpha: g.number_in(0.0, 1.0),
                mapped: g.chance(0.5),
            },
            3 => Self::Bar {
                x: series(g, n),
                heights: partner(g, n),
                widths: few_in(g, 3, 0.0, 1.5),
                horizontal: g.chance(0.3),
                log: g.chance(0.1),
                hatch: g.pick(&HATCHES).to_owned(),
            },
            4 => Self::FillBetween {
                x: series(g, n),
                y1: partner(g, n),
                y2: partner(g, n),
                step: g.chance(0.3),
                interpolate: g.chance(0.5),
            },
            5 => Self::Errorbar {
                x: series(g, n),
                y: partner(g, n),
                yerr: each_or_one_in(g, n, 0.0, 2.0),
                capsize: g.number_in(0.0, 10.0),
                every: markevery(g),
            },
            6 => Self::Stem {
                x: series(g, n),
                y: partner(g, n),
            },
            7 => Self::Step {
                x: series(g, n),
                y: partner(g, n),
            },
            8 => Self::Stairs {
                values: series(g, n),
                edges: g.chance(0.5).then(|| partner(g, n + 1)),
                fill: g.chance(0.5),
            },
            9 => Self::Segments {
                at: series(g, n),
                low: partner(g, n),
                high: partner(g, n),
                vertical: g.chance(0.5),
            },
            10 => Self::Reference {
                kind: g.below(5),
                a: g.number_in(-1.0, 2.0),
                b: g.number_in(-1.0, 2.0),
                slope: g.number(),
            },
            11 => Self::Fill {
                x: series(g, n),
                y: partner(g, n),
                hatch: g.pick(&HATCHES).to_owned(),
            },
            _ if g.chance(0.5) => {
                let ranges = (0..g.len(5))
                    .map(|_| (g.number_in(0.0, 10.0), g.number_in(0.0, 3.0)))
                    .collect();
                Self::BrokenBarh {
                    ranges,
                    y: g.number_in(0.0, 5.0),
                    height: g.number_in(0.0, 3.0),
                }
            }
            _ => {
                let rows = g.len(4);
                Self::Stackplot {
                    x: series(g, n),
                    ys: (0..rows).map(|_| partner(g, n)).collect(),
                }
            }
        }
    }

    /// Runs the call on `ax`, keeping what a colorbar or bar labels can
    /// use in `mappables`; errors are fine.
    pub(crate) fn run(&self, ax: &Axes, mappables: &mut Mappables) {
        match self {
            Self::Plot { x, y, line } => drop(ax.plot_with(x.clone(), y.clone(), line.props())),
            Self::Scatter { .. } => self.scatter(ax, mappables),
            Self::Bar { .. } => self.bar(ax, mappables),
            Self::FillBetween {
                x,
                y1,
                y2,
                step,
                interpolate,
            } => {
                let options = FillBetweenOptions {
                    interpolate: *interpolate,
                    step: step.then_some(StepWhere::Mid),
                    ..FillBetweenOptions::default()
                };
                drop(ax.fill_between(x.clone(), y1.clone(), y2.clone(), options));
            }
            Self::Errorbar {
                x,
                y,
                yerr,
                capsize,
                every,
            } => {
                let options = ErrorbarOptions {
                    yerr: Some(ErrorSize::Symmetric(yerr.clone().into())),
                    capsize: Some(*capsize),
                    errorevery: Some(every.clone()),
                    ..ErrorbarOptions::default()
                };
                drop(ax.errorbar(x.clone(), y.clone(), options));
            }
            Self::Stem { x, y } => drop(ax.stem(x.clone(), y.clone(), StemOptions::default())),
            Self::Step { x, y } => {
                drop(ax.step(x.clone(), y.clone(), StepWhere::Post, LineProps::default()));
            }
            Self::Stairs {
                values,
                edges,
                fill,
            } => {
                let options = StairsOptions {
                    edges: edges.clone(),
                    fill: *fill,
                    ..StairsOptions::default()
                };
                drop(ax.stairs(values.clone(), options));
            }
            Self::Segments {
                at,
                low,
                high,
                vertical,
            } => {
                let lines = if *vertical {
                    ax.vlines(at.clone(), low.clone(), high.clone())
                } else {
                    ax.hlines(at.clone(), low.clone(), high.clone())
                };
                drop(lines);
            }
            Self::Reference { kind, a, b, slope } => reference(ax, *kind, *a, *b, *slope),
            Self::Fill { x, y, hatch } => {
                drop(ax.fill(
                    x.clone(),
                    y.clone(),
                    PatchProps::default().hatch(hatch.as_str()),
                ));
            }
            Self::BrokenBarh { ranges, y, height } => {
                drop(ax.broken_barh(ranges, (*y, *height), BandAlign::Center));
            }
            Self::Stackplot { x, ys } => {
                drop(ax.stackplot(x.clone(), ys, StackplotOptions::default()));
            }
        }
    }

    /// Runs a [`Call::Scatter`].
    fn scatter(&self, ax: &Axes, mappables: &mut Mappables) {
        let Self::Scatter {
            x,
            y,
            sizes,
            marker,
            alpha,
            mapped,
        } = self
        else {
            return;
        };
        let mut options = ScatterOptions {
            sizes: Some(sizes.clone()),
            marker: marker.parse::<MarkerStyle>().ok(),
            alpha: Alpha::Uniform(*alpha),
            ..ScatterOptions::default()
        };
        if *mapped {
            options.colors = ScatterColors::Mapped(Box::new(MappedColors::new(y.clone())));
        }
        if let Ok(collection) = ax.scatter_with(x.clone(), y.clone(), options) {
            mappables.collections.push(collection);
        }
    }

    /// Runs a [`Call::Bar`].
    fn bar(&self, ax: &Axes, mappables: &mut Mappables) {
        let Self::Bar {
            x,
            heights,
            widths,
            horizontal,
            log,
            hatch,
        } = self
        else {
            return;
        };
        let options = BarOptions {
            thickness: widths.clone().into(),
            log: *log,
            hatch: vec![hatch.clone()],
            label: Some("bars".to_owned()),
            ..BarOptions::default()
        };
        let bars = if *horizontal {
            ax.barh(x.clone(), heights, options)
        } else {
            ax.bar(x.clone(), heights, options)
        };
        if let Ok(bars) = bars {
            mappables.bars.push(bars);
        }
    }
}

/// A marker subsampling: strides (zero and negative included) or
/// distances along the line.
fn markevery(g: &mut Gen) -> MarkEvery {
    if g.chance(0.7) {
        MarkEvery::Every {
            start: g.below(7) as i64 - 3,
            step: g.below(9) as i64 - 3,
        }
    } else {
        MarkEvery::Distance {
            start: g.number_in(0.0, 1.0),
            step: g.number_in(0.0, 0.5),
        }
    }
}

/// A reference line or span.
fn reference(ax: &Axes, kind: usize, a: f64, b: f64, slope: f64) {
    match kind {
        0 => drop(ax.axhline(a, LineProps::default())),
        1 => drop(ax.axvline(a, LineProps::default())),
        2 => drop(ax.axhspan(a, b)),
        3 => drop(ax.axvspan(a, b)),
        _ => drop(ax.axline(
            Point::new(a, b),
            AxLineThrough::Slope(slope),
            LineProps::default(),
        )),
    }
}
