//! `_native.cbook`: the numeric helpers of matplotlib's `cbook` (box-plot
//! statistics, stripping mathtext), computed by pltrs.

use pltrs::stats::{Bootstrap, BoxStats, BoxplotStatsOptions, StatsError, Whis, boxplot_stats};

use crate::binding::prelude::*;

/// The seed of the bootstrapped notches: matplotlib draws them from
/// numpy's global generator, pltrs (Rust) from a seeded one (see
/// [`Bootstrap`]).
const BOOTSTRAP_SEED: u64 = 0;

/// `strip_math(s)`: the text of a fully-mathtext string without its `$`s
/// and common TeX commands.
fn strip_math(text: String) -> PyResult<String> {
    Ok(pltrs::ticker::strip_math(&text))
}

/// The whisker reach: a factor of the interquartile range, or two
/// percentiles.
fn whis(object: &Bound<'_>) -> PyResult<Whis> {
    if let Ok(factor) = object.extract::<f64>() {
        return Ok(Whis::Factor(factor));
    }
    let (lo, hi): (f64, f64) = object.extract()?;
    Ok(Whis::Percentiles(lo, hi))
}

/// The statistics of one dataset as `((mean, iqr, cilo, cihi), (whishi,
/// whislo), fliers, (q1, med, q3))`.
type BoxTuple = ((f64, f64, f64, f64), (f64, f64), NdArray, (f64, f64, f64));

/// `boxplot_stats` of one dataset `x`, with whiskers `whis` (a factor or
/// two percentiles), `autorange` and `bootstrap` resamples of the median.
fn box_stats(
    x: NdArray,
    whis_spec: Owned,
    autorange: bool,
    bootstrap: Option<usize>,
) -> PyResult<BoxTuple> {
    let options = BoxplotStatsOptions {
        whis: Python::with_gil(|py| whis(&whis_spec.bind(py)))?,
        labels: None,
        autorange,
        bootstrap: bootstrap.map(|resamples| Bootstrap {
            resamples,
            seed: BOOTSTRAP_SEED,
        }),
    };
    let stats = boxplot_stats(&[x.data], options)
        .map_err(|err: StatsError| PyErr::value_error(err.to_string()))?;
    let BoxStats {
        mean,
        iqr,
        cilo,
        cihi,
        whishi,
        whislo,
        fliers,
        q1,
        med,
        q3,
        ..
    } = stats
        .into_iter()
        .next()
        .ok_or_else(|| PyErr::value_error("no statistics for the dataset"))?;
    Ok((
        (mean, iqr, cilo, cihi),
        (whishi, whislo),
        NdArray::from_vec(fliers),
        (q1, med, q3),
    ))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("strip_math", strip_math)?;
    module.add("box_stats", box_stats)
}
