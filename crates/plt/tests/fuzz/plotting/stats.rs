//! Plotting calls of statistics and spectra: histograms, event plots,
//! box plots, violins, pies, empirical distributions and spectral
//! estimates, with random data and options (huge bin, point and segment
//! counts included).

use plt::axes::{
    Axes, BoxplotOptions, EcdfOptions, EventplotOptions, HistPlotOptions, PieOptions,
    SpecgramOptions, SpectralPlotOptions, ViolinplotOptions,
};
use plt::lines::LineProps;
use plt::mlab::XcorrOptions;
use plt::stats::{Bins, Cumulative, HistOrientation, HistType, Whis};
use plt::ticker::LabelFormat;

use super::data::{count, datasets, few_in, length, series};
use crate::harness::Gen;

/// The histogram types, by number.
const HIST_TYPES: [HistType; 4] = [
    HistType::Bar,
    HistType::BarStacked,
    HistType::Step,
    HistType::StepFilled,
];

/// Pie label templates, printf and `str.format`.
const PERCENTS: [&str; 5] = ["%1.1f%%", "{:.0%}", "%d", "%.999f", "%%"];

/// A plotting call of statistics or spectra.
#[derive(Debug)]
pub(crate) enum StatCall {
    Hist {
        data: Vec<Vec<f64>>,
        bins: usize,
        kind: usize,
        density: bool,
        log: bool,
        range: Option<(f64, f64)>,
    },
    Eventplot {
        positions: Vec<Vec<f64>>,
        lengths: Vec<f64>,
    },
    Boxplot {
        data: Vec<Vec<f64>>,
        whis: f64,
        notch: bool,
    },
    Violin {
        data: Vec<Vec<f64>>,
        points: usize,
    },
    Pie {
        values: Vec<f64>,
        explode: Vec<f64>,
        radius: f64,
        autopct: String,
        wedge: Option<f64>,
    },
    Ecdf {
        x: Vec<f64>,
    },
    Spectrum {
        x: Vec<f64>,
        nfft: usize,
        overlap: usize,
        kind: usize,
    },
}

impl StatCall {
    /// A random call.
    pub(crate) fn generate(g: &mut Gen) -> Self {
        let n = length(g);
        match g.below(7) {
            0 => Self::Hist {
                data: datasets(g),
                bins: count(g, 30),
                kind: g.below(HIST_TYPES.len()),
                density: g.chance(0.3),
                log: g.chance(0.1),
                range: g.chance(0.2).then(|| (g.number(), g.number())),
            },
            1 => Self::Eventplot {
                positions: datasets(g),
                lengths: few_in(g, 3, 0.0, 2.0),
            },
            2 => Self::Boxplot {
                data: datasets(g),
                whis: g.number_in(0.0, 3.0),
                notch: g.chance(0.3),
            },
            3 => Self::Violin {
                data: datasets(g),
                points: count(g, 100),
            },
            4 => Self::Pie {
                values: series(g, n.min(12)),
                explode: few_in(g, 4, 0.0, 0.5),
                radius: g.number_in(0.1, 2.0),
                autopct: g.pick(&PERCENTS).to_owned(),
                wedge: g.chance(0.3).then(|| g.number_in(0.0, 1.0)),
            },
            5 => Self::Ecdf { x: series(g, n) },
            _ => Self::Spectrum {
                x: series(g, n),
                nfft: count(g, 64),
                overlap: count(g, 32),
                kind: g.below(5),
            },
        }
    }

    /// Runs the call on `ax`; errors are fine.
    pub(crate) fn run(&self, ax: &Axes) {
        match self {
            Self::Hist {
                data,
                bins,
                kind,
                density,
                log,
                range,
            } => {
                let options = HistPlotOptions {
                    bins: Some(Bins::Count(*bins)),
                    histtype: HIST_TYPES[*kind],
                    density: *density,
                    range: *range,
                    cumulative: if *density {
                        Cumulative::Forward
                    } else {
                        Cumulative::Off
                    },
                    orientation: if *kind == 3 {
                        HistOrientation::Horizontal
                    } else {
                        HistOrientation::Vertical
                    },
                    log: *log,
                    ..HistPlotOptions::default()
                };
                drop(ax.hist(data, options));
            }
            Self::Eventplot { positions, lengths } => {
                let options = EventplotOptions {
                    linelengths: lengths.clone(),
                    ..EventplotOptions::default()
                };
                drop(ax.eventplot(positions, options));
            }
            Self::Boxplot { data, whis, notch } => {
                let mut options = BoxplotOptions {
                    whis: Some(Whis::Factor(*whis)),
                    ..BoxplotOptions::default()
                };
                options.draw.shownotches = Some(*notch);
                drop(ax.boxplot(data, options));
            }
            Self::Violin { data, points } => {
                let mut options = ViolinplotOptions::default();
                options.stats.points = *points;
                drop(ax.violinplot(data, options));
            }
            Self::Pie {
                values,
                explode,
                radius,
                autopct,
                wedge,
            } => {
                let options = PieOptions {
                    explode: explode.clone(),
                    radius: *radius,
                    autopct: Some(LabelFormat::Template(autopct.clone())),
                    wedge_width: *wedge,
                    ..PieOptions::default()
                };
                drop(ax.pie(values, options));
            }
            Self::Ecdf { x } => drop(ax.ecdf(x, EcdfOptions::default())),
            Self::Spectrum {
                x,
                nfft,
                overlap,
                kind,
            } => spectrum(ax, x, *nfft, *overlap, *kind),
        }
    }
}

/// A spectral plot of `x`: power, specgram, autocorrelation, coherence
/// or cross spectral density.
fn spectrum(ax: &Axes, x: &[f64], nfft: usize, overlap: usize, kind: usize) {
    let mut options = SpectralPlotOptions::default();
    options.spectral.nfft = nfft;
    options.spectral.noverlap = Some(overlap);
    match kind {
        0 => drop(ax.psd(x, options)),
        1 => {
            let specgram = SpecgramOptions {
                spectral: options.spectral,
                ..SpecgramOptions::default()
            };
            drop(ax.specgram(x, specgram));
        }
        2 => {
            let correlation = XcorrOptions {
                maxlags: Some(overlap),
                ..XcorrOptions::default()
            };
            drop(ax.acorr(
                x,
                correlation,
                overlap.is_multiple_of(2),
                LineProps::default(),
            ));
        }
        3 => drop(ax.cohere(x, x, options)),
        _ => drop(ax.csd(x, x, options)),
    }
}
