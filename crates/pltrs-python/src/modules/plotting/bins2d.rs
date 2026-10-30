//! Two-dimensional binning: `hexbin` and `hist2d`.

use pltrs::axes::{
    Axes, HexBins, HexGridSize, HexReduce, HexScale, HexbinOptions, Hist2dOptions, PcolorOptions,
};
use pltrs::collections::Collection;
use pltrs::image::Extent;
use pltrs::stats::{AxisBins, Histogram2dOptions};

use super::mapping::{Mapping, colorizer};
use crate::binding::prelude::*;

/// A reduction of the values in a hexagon (matplotlib's
/// `reduce_C_function`): numpy's name for the ones pltrs (Rust) has
/// (`mean`, `sum`, `median`, `max`, `min`), else a Python function called
/// with each hexagon's values as an array.
pub(super) struct PyReduce(HexReduce);

impl FromPy<'_> for PyReduce {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let Ok(name) = object.extract::<String>() else {
            let function: Callback = object.extract()?;
            return Ok(Self(HexReduce::custom(move |values| {
                function
                    .call::<f64>((NdArray::from_vec(values.to_vec()),))
                    .unwrap_or(f64::NAN)
            })));
        };
        Ok(Self(match name.as_str() {
            "mean" => HexReduce::Mean,
            "sum" => HexReduce::Sum,
            "median" => HexReduce::Median,
            "max" => HexReduce::Max,
            "min" => HexReduce::Min,
            _ => return Err(PyErr::value_error(format!("unknown reduction {name:?}"))),
        }))
    }
}

/// A binning scale: `"linear"` or `"log"`.
fn scale(name: &str) -> HexScale {
    if name == "log" {
        HexScale::Log
    } else {
        HexScale::Linear
    }
}

/// How hexagon values are discretised: `None`, `"log"`, a count, or
/// edges.
pub(super) struct PyHexBins(Option<HexBins>);

impl FromPy<'_> for PyHexBins {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if object.is_none() {
            return Ok(Self(None));
        }
        if let Ok(name) = object.extract::<String>() {
            return if name == "log" {
                Ok(Self(Some(HexBins::Log)))
            } else {
                Err(PyErr::value_error(format!(
                    "{name:?} is not a valid value for bins; supported values are 'log'"
                )))
            };
        }
        if let Ok(count) = object.extract::<usize>() {
            return Ok(Self(Some(HexBins::Count(count))));
        }
        Ok(Self(Some(HexBins::Edges(
            object.extract::<NdArray>()?.into_vec()?,
        ))))
    }
}

/// The binning grid: `(gridsize (nx, ny or None), bins, xscale, yscale,
/// extent, mincnt)`.
type HexGrid = (
    (usize, Option<usize>),
    PyHexBins,
    (String, String),
    Option<(f64, f64, f64, f64)>,
    Option<usize>,
);

/// Hexagonal binning of the points `x`, `y` (matplotlib's `hexbin`):
/// their counts, or the values `c` reduced by `reduce`, mapped as
/// `mapping` says, with marginal bars when `marginals`.
pub(super) fn hexbin(
    axes: Axes,
    (x, y): (NdArray, NdArray),
    (c, reduce): (Option<NdArray>, PyReduce),
    ((nx, ny), bins, (xscale, yscale), extent, mincnt): HexGrid,
    mapping: Mapping,
    marginals: bool,
) -> PyResult<Collection> {
    let options = HexbinOptions {
        values: c.map(NdArray::into_vec).transpose()?,
        reduce: reduce.0,
        gridsize: match ny {
            Some(ny) => HexGridSize::Shape(nx, ny),
            None => HexGridSize::Count(nx),
        },
        bins: bins.0,
        xscale: scale(&xscale),
        yscale: scale(&yscale),
        extent: extent.map(|(left, right, bottom, top)| Extent::new(left, right, bottom, top)),
        mincnt,
        colorizer: colorizer(mapping),
        marginals,
        ..HexbinOptions::default()
    };
    Ok(axes.hexbin(x.into_vec()?, y.into_vec()?, options)?)
}

/// The bins of one axis of a 2-D histogram: a count or the edges.
pub(super) struct PyAxisBins(AxisBins);

impl FromPy<'_> for PyAxisBins {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if let Ok(count) = object.extract::<usize>() {
            return Ok(Self(AxisBins::Count(count)));
        }
        Ok(Self(AxisBins::Edges(
            object.extract::<NdArray>()?.into_vec()?,
        )))
    }
}

/// A 2-D histogram: `(counts, xedges, yedges, mesh)`.
type Hist2d = (NdArray, NdArray, NdArray, Collection);

/// The histogram: `(bins along x and y, ranges, density, weights)`.
type Binning = (
    (PyAxisBins, PyAxisBins),
    (Option<(f64, f64)>, Option<(f64, f64)>),
    bool,
    Option<NdArray>,
);

/// A 2-D histogram of the points `x`, `y` drawn as a mesh (matplotlib's
/// `hist2d`), counts outside `(cmin, cmax)` blanked.
pub(super) fn hist2d(
    axes: Axes,
    (x, y): (NdArray, NdArray),
    ((xbins, ybins), (xrange, yrange), density, weights): Binning,
    (cmin, cmax): (Option<f64>, Option<f64>),
    mapping: Mapping,
) -> PyResult<Hist2d> {
    let weights = weights.map(NdArray::into_vec).transpose()?;
    let options = Hist2dOptions {
        histogram: Histogram2dOptions {
            bins: [xbins.0, ybins.0],
            range: [xrange, yrange],
            density,
            weights: weights.as_deref(),
        },
        cmin,
        cmax,
        mesh: PcolorOptions {
            colorizer: colorizer(mapping),
            ..PcolorOptions::default()
        },
    };
    let hist = axes.hist2d(x.into_vec()?, y.into_vec()?, options)?;
    let (rows, cols) = hist.counts.shape();
    Ok((
        NdArray::with_shape(hist.counts.into_vec(), vec![rows, cols])?,
        NdArray::from_vec(hist.xedges),
        NdArray::from_vec(hist.yedges),
        hist.mesh,
    ))
}
