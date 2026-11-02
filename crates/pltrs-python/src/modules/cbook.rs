//! `_native.cbook`: the helpers of matplotlib's `cbook` and its package (box-plot
//! statistics, stripping mathtext, the configuration and cache directories),
//! computed by pltrs.

use pltrs::config::{self, ConfigError};
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

impl From<ConfigError> for PyErr {
    /// matplotlib's `OSError` when no writable directory can be found.
    fn from(err: ConfigError) -> Self {
        PyErr::new(ExceptionKind::OSError, err.to_string())
    }
}

/// The configuration directory (matplotlib's `get_configdir`).
fn config_dir() -> PyResult<String> {
    Ok(config::config_dir()?.display().to_string())
}

/// The cache directory (matplotlib's `get_cachedir`).
fn cache_dir() -> PyResult<String> {
    Ok(config::cache_dir()?.display().to_string())
}

/// The rc file pltrs reads, or the template of the defaults (matplotlib's
/// `matplotlib_fname`).
fn rc_file_or_template() -> PyResult<String> {
    config::rc_file_or_template()
        .map(|path| path.display().to_string())
        .map_err(|err| PyErr::runtime_error(err.to_string()))
}

/// The temporary directory standing in for the configuration directory,
/// looked up first, if one does, with the warning matplotlib logs on
/// import then: `(directory, message)`.
fn temp_dir_notice() -> PyResult<Option<(String, String)>> {
    config::config_dir()?;
    Ok(config::temp_dir_notice()
        .map(|notice| (notice.dir().display().to_string(), notice.to_string())))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("config_dir", config_dir)?;
    module.add("temp_dir_notice", temp_dir_notice)?;
    module.add("cache_dir", cache_dir)?;
    module.add("rc_file_or_template", rc_file_or_template)?;
    module.add("strip_math", strip_math)?;
    module.add("box_stats", box_stats)
}
