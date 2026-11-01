//! `hist`.

use pltrs::axes::{Axes, HistArtists, HistPlotOptions};
use pltrs::colors::Color;
use pltrs::container::Container;
use pltrs::patches::{Patch, Polygon, Rectangle};
use pltrs::stats::{
    BinEstimator, Bins, Cumulative, HistAlign, HistBottom, HistOrientation, HistType,
};

use crate::binding::prelude::*;

/// matplotlib's `bins`: a count, an estimator name or the edges.
pub(super) struct PyBins(Bins);

impl FromPy<'_> for PyBins {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if let Ok(name) = object.extract::<String>() {
            return name
                .parse::<BinEstimator>()
                .map(|estimator| Self(Bins::Estimator(estimator)))
                .map_err(|err| PyErr::value_error(err.to_string()));
        }
        if object.type_name() == "int" || object.type_name().starts_with("int") {
            let count: usize = object.extract()?;
            return Ok(Self(Bins::Count(count)));
        }
        let edges: NdArray = object.extract()?;
        Ok(Self(Bins::Edges(edges.into_vec()?)))
    }
}

/// The binning: `(bins, range, density, cumulative, stacked)`, the
/// cumulation `0` (off), positive (forward) or negative (reverse).
pub(super) type Binning = (PyBins, Option<(f64, f64)>, bool, i64, bool);

/// The layout: `(histtype, align, orientation, rwidth, bottom, log)`.
pub(super) type Layout = (String, String, String, Option<f64>, Option<NdArray>, bool);

/// The bars of one dataset: their container, rectangles and values.
pub(super) type DatasetBars = (Container, Vec<Patch<Rectangle>>, Vec<f64>);

/// The tops (one row per dataset), the bin edges, and the bars of each
/// dataset or its step outlines.
pub(super) type HistResult = (
    Vec<Vec<f64>>,
    Vec<f64>,
    Vec<DatasetBars>,
    Vec<Patch<Polygon>>,
);

/// A histogram of each dataset (matplotlib's `hist`, whose input
/// reshaping and checks the Python side did), in `colors` (one per
/// dataset).
pub(super) fn hist(
    axes: Axes,
    datasets: Vec<NdArray>,
    weights: Option<Vec<NdArray>>,
    (bins, range, density, cumulative, stacked): Binning,
    (histtype, align, orientation, rwidth, bottom, log): Layout,
    colors: Vec<Color>,
) -> PyResult<HistResult> {
    let datasets = datasets
        .into_iter()
        .map(NdArray::into_vec)
        .collect::<PyResult<Vec<_>>>()?;
    let weights = weights
        .map(|weights| {
            weights
                .into_iter()
                .map(NdArray::into_vec)
                .collect::<PyResult<Vec<_>>>()
        })
        .transpose()?;
    let options = HistPlotOptions {
        bins: Some(bins.0),
        range,
        density,
        weights: weights.as_deref(),
        cumulative: match cumulative {
            0 => Cumulative::Off,
            c if c < 0 => Cumulative::Reverse,
            _ => Cumulative::Forward,
        },
        stacked,
        histtype: match histtype.as_str() {
            "barstacked" => HistType::BarStacked,
            "step" => HistType::Step,
            "stepfilled" => HistType::StepFilled,
            _ => HistType::Bar,
        },
        align: match align.as_str() {
            "left" => HistAlign::Left,
            "right" => HistAlign::Right,
            _ => HistAlign::Mid,
        },
        orientation: match orientation.as_str() {
            "horizontal" => HistOrientation::Horizontal,
            _ => HistOrientation::Vertical,
        },
        rwidth,
        bottom: match bottom {
            Some(bottom) if bottom.data.len() == 1 => HistBottom::Uniform(bottom.data[0]),
            Some(bottom) => HistBottom::PerBin(bottom.into_vec()?),
            None => HistBottom::default(),
        },
        log,
        color: colors,
        ..HistPlotOptions::default()
    };
    let plot = axes.hist(&datasets, options)?;
    let (bars, steps) = match plot.artists {
        HistArtists::Bars(containers) => (
            containers
                .into_iter()
                .map(|bars| (Container::Bar(bars.clone()), bars.patches, bars.datavalues))
                .collect(),
            Vec::new(),
        ),
        HistArtists::Steps(outlines) => (Vec::new(), outlines),
    };
    Ok((plot.tops, plot.edges, bars, steps))
}
