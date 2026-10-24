//! `numpy.histogram*` and `Axes.hist`.

use plt::axes::{HistArtists, HistPlot, HistPlotOptions};
use plt::figure::Figure;
use plt::stats::{
    self, BinEstimator, BinSample, Bins, Cumulative, HistAlign, HistBottom, HistOptions,
    HistOrientation, HistType, HistogramOptions,
};

use super::{Numbers, datasets, floats, number, object};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};

/// numpy's `bins=`: an integer, an estimator name or an edge list.
fn parse_bins(value: Option<&Json>) -> Result<Bins, ProbeError> {
    let Some(value) = value else {
        return Ok(Bins::default());
    };
    match value {
        Json::Integer(n) if *n >= 1 => Ok(Bins::Count(*n as usize)),
        Json::Integer(_) => Err(ProbeError::rejected("`bins` must be positive")),
        Json::Number(_) => Err(ProbeError::rejected("`bins` must be an integer")),
        Json::String(name) => name
            .parse::<BinEstimator>()
            .map(Bins::Estimator)
            .map_err(ProbeError::rejected),
        Json::Array(items) if items.iter().any(|v| v.as_array().is_some()) => {
            Err(ProbeError::rejected("`bins` must be 1d"))
        }
        Json::Array(_) => floats(value).map(Bins::Edges),
        _ => Err(ProbeError::BadArgs(format!("bad bins {value}"))),
    }
}

fn parse_range(args: Args<'_>) -> Result<Option<(f64, f64)>, ProbeError> {
    args.get("range")
        .map(|r| match r.as_f64_vec().as_deref() {
            Some([lo, hi]) => Ok((*lo, *hi)),
            _ => Err(ProbeError::BadArgs(format!("bad range {r}"))),
        })
        .transpose()
}

fn histogram_options<'w>(
    args: Args<'_>,
    weights: Option<&'w [f64]>,
) -> Result<HistogramOptions<'w>, ProbeError> {
    Ok(HistogramOptions {
        bins: parse_bins(args.get("bins"))?,
        range: parse_range(args)?,
        density: args.opt_bool("density")?.unwrap_or(false),
        weights,
    })
}

fn weights(args: Args<'_>) -> Result<Option<Vec<f64>>, ProbeError> {
    args.get("weights").map(floats).transpose()
}

/// Runs `f` on the data with numpy's dtype.
fn with_samples<R>(
    value: &Json,
    f: impl Fn(&dyn SampleData) -> Result<R, stats::StatsError>,
) -> Result<R, ProbeError> {
    let result = match Numbers::parse(value)? {
        Numbers::Int(v) => f(&v),
        Numbers::Float(v) => f(&v),
        Numbers::Bool(v) => f(&v),
    };
    result.map_err(ProbeError::rejected)
}

/// Object-safe access to the generic histogram functions.
trait SampleData {
    fn histogram(&self, opts: HistogramOptions<'_>) -> Result<stats::Histogram, stats::StatsError>;
    fn edges(&self, opts: HistogramOptions<'_>) -> Result<Vec<f64>, stats::StatsError>;
    fn width(&self, estimator: BinEstimator) -> Result<f64, stats::StatsError>;
}

impl<T: BinSample> SampleData for Vec<T> {
    fn histogram(&self, opts: HistogramOptions<'_>) -> Result<stats::Histogram, stats::StatsError> {
        stats::histogram(self, opts)
    }

    fn edges(&self, opts: HistogramOptions<'_>) -> Result<Vec<f64>, stats::StatsError> {
        stats::histogram_bin_edges(self, opts)
    }

    fn width(&self, estimator: BinEstimator) -> Result<f64, stats::StatsError> {
        let range = stats::outer_edges(self, None)?;
        Ok(stats::bin_width(self, estimator, range))
    }
}

pub(super) fn histogram(args: Args<'_>) -> Outcome {
    let weights = weights(args)?;
    let opts = histogram_options(args, weights.as_deref())?;
    let h = with_samples(args.json("a")?, |data| data.histogram(opts.clone()))?;
    Ok(object([
        ("hist", Json::from(h.counts)),
        ("bin_edges", Json::from(h.edges)),
    ]))
}

pub(super) fn histogram_bin_edges(args: Args<'_>) -> Outcome {
    let weights = weights(args)?;
    let opts = histogram_options(args, weights.as_deref())?;
    Ok(Json::from(with_samples(args.json("a")?, |data| {
        data.edges(opts.clone())
    })?))
}

pub(super) fn histogram_bin_width(args: Args<'_>) -> Outcome {
    let estimator = args
        .str("estimator")?
        .parse::<BinEstimator>()
        .map_err(ProbeError::rejected)?;
    Ok(Json::from(with_samples(args.json("a")?, |data| {
        data.width(estimator)
    })?))
}

pub(super) fn axes_hist(args: Args<'_>) -> Outcome {
    let histtype = match args.opt_str("histtype")?.unwrap_or("bar") {
        "bar" => HistType::Bar,
        "barstacked" => HistType::BarStacked,
        "step" => HistType::Step,
        "stepfilled" => HistType::StepFilled,
        other => return Err(ProbeError::rejected(format!("bad histtype {other:?}"))),
    };
    let align = match args.opt_str("align")?.unwrap_or("mid") {
        "left" => HistAlign::Left,
        "mid" => HistAlign::Mid,
        "right" => HistAlign::Right,
        other => return Err(ProbeError::rejected(format!("bad align {other:?}"))),
    };
    let orientation = match args.opt_str("orientation")?.unwrap_or("vertical") {
        "vertical" => HistOrientation::Vertical,
        "horizontal" => HistOrientation::Horizontal,
        other => return Err(ProbeError::rejected(format!("bad orientation {other:?}"))),
    };
    let as_ndarray = args.opt_bool("x_ndarray")?.unwrap_or(false);
    let (sets, integer) = datasets(args.json("x")?, as_ndarray)?;
    let weights = args
        .get("weights")
        .map(|w| datasets(w, false).map(|(w, _)| w))
        .transpose()?;
    let opts = HistOptions {
        bins: Some(parse_bins(args.get("bins"))?),
        range: parse_range(args)?,
        density: args.opt_bool("density")?.unwrap_or(false),
        weights: weights.as_deref(),
        cumulative: parse_cumulative(args.get("cumulative"))?,
        stacked: args.opt_bool("stacked")?.unwrap_or(false),
        histtype,
        align,
        orientation,
        rwidth: args.opt_f64("rwidth")?,
        bottom: match args.get("bottom") {
            None => HistBottom::default(),
            Some(b @ Json::Array(_)) => HistBottom::PerBin(floats(b)?),
            Some(b) => HistBottom::Uniform(number(b)?),
        },
    };
    let ax = Figure::new().subplot();
    let options = HistPlotOptions {
        layout: opts,
        log: args.opt_bool("log")?.unwrap_or(false),
        ..HistPlotOptions::default()
    };
    let plot = if integer {
        let ints: Vec<Vec<i64>> = sets
            .iter()
            .map(|s| s.iter().map(|&v| v as i64).collect())
            .collect();
        ax.hist(&ints, options)
    } else {
        ax.hist(&sets, options)
    }
    .map_err(ProbeError::rejected)?;
    let HistPlot {
        tops,
        edges,
        artists,
    } = plot;
    let n = if tops.len() == 1 {
        Json::from(tops.into_iter().next().unwrap_or_default())
    } else {
        Json::from(tops)
    };
    Ok(object([
        ("n", n),
        ("bins", Json::from(edges)),
        ("patches", patches_json(&artists)),
        ("xscale", Json::from(ax.xscale())),
        ("yscale", Json::from(ax.yscale())),
    ]))
}

/// matplotlib's `cumulative`: a bool or a number whose sign picks the
/// direction.
fn parse_cumulative(value: Option<&Json>) -> Result<Cumulative, ProbeError> {
    let forward = |on: bool| {
        if on {
            Cumulative::Forward
        } else {
            Cumulative::Off
        }
    };
    match value {
        None => Ok(Cumulative::Off),
        Some(Json::Bool(on)) => Ok(forward(*on)),
        Some(v) => {
            let x = number(v)?;
            Ok(if x < 0.0 {
                Cumulative::Reverse
            } else {
                forward(x > 0.0)
            })
        }
    }
}

/// The rectangles (`[x, y, width, height]`) or outlines of every dataset.
fn patches_json(artists: &HistArtists) -> Json {
    match artists {
        HistArtists::Bars(containers) => Json::Array(
            containers
                .iter()
                .map(|container| {
                    Json::Array(
                        container
                            .patches
                            .iter()
                            .map(|patch| {
                                let rect = patch.shape();
                                Json::from(vec![rect.xy.x, rect.xy.y, rect.width, rect.height])
                            })
                            .collect(),
                    )
                })
                .collect(),
        ),
        HistArtists::Steps(outlines) => Json::Array(
            outlines
                .iter()
                .map(|patch| {
                    let polygon = patch.shape();
                    let xy = polygon
                        .vertices()
                        .iter()
                        .map(|p| Json::from(vec![p.x, p.y]))
                        .collect();
                    Json::Array(vec![object([
                        ("xy", Json::Array(xy)),
                        ("closed", Json::from(polygon.is_closed())),
                        ("fill", Json::from(patch.style().fill)),
                    ])])
                })
                .collect(),
        ),
    }
}
