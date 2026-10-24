//! Choosing contour levels and deriving the extended levels, layers and z
//! bands from them (matplotlib's `_process_contour_level_args`,
//! `_autolev`, `_process_levels` and `_get_lowers_and_uppers`).

use std::any::Any;

use super::ContourError;
use crate::colors::Extend;
use crate::ticker::{
    Locator, LogLocator, MaxNLocator, Nbins, NumTicks, TickContext, TickerError, TickerSettings,
    drawable_ticks,
};

/// Which contour levels to draw.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Levels {
    /// About seven automatic levels spanning the data (boolean data:
    /// `[0.5]` for lines, `[0, 0.5, 1]` for fills).
    #[default]
    Auto,
    /// About `n` automatic levels: the locator gets `n + 1` bins (or `n`
    /// ticks when log-scaled).
    Count(i64),
    /// Explicit, strictly increasing levels.
    Values(Vec<f64>),
}

/// The level-selection inputs of a contour set.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LevelRequest<'a> {
    pub(crate) levels: &'a Levels,
    pub(crate) locator: Option<&'a dyn Locator>,
    pub(crate) logscale: bool,
    pub(crate) filled: bool,
    pub(crate) extend: Extend,
    /// Whether z holds booleans.
    pub(crate) boolean: bool,
}

/// Whether `locator` makes a contour set log-scaled.
pub(crate) fn is_log_locator(locator: &dyn Locator) -> bool {
    (locator as &dyn Any).is::<LogLocator>()
}

impl LevelRequest<'_> {
    /// The levels for data spanning `[zmin, zmax]` (`None` when all z is
    /// masked).
    pub(crate) fn choose(
        &self,
        zmin: Option<f64>,
        zmax: Option<f64>,
        settings: &TickerSettings,
    ) -> Result<Vec<f64>, ContourError> {
        let levels = match self.levels {
            Levels::Values(values) => values.clone(),
            Levels::Auto if self.boolean => {
                if self.filled {
                    vec![0.0, 0.5, 1.0]
                } else {
                    vec![0.5]
                }
            }
            Levels::Auto => self.autolev(None, zmin, zmax, settings)?,
            Levels::Count(n) => self.autolev(Some(*n), zmin, zmax, settings)?,
        };
        if self.filled && levels.len() < 2 {
            return Err(ContourError::TooFewFilledLevels(levels.len()));
        }
        if !increasing(&levels) {
            return Err(ContourError::LevelsNotIncreasing);
        }
        Ok(levels)
    }

    /// Levels from the locator, trimmed to keep at most one level beyond
    /// each end of the data (one fewer at an extended end), unless fewer
    /// than three would remain or the locator is symmetric.
    fn autolev(
        &self,
        n: Option<i64>,
        zmin: Option<f64>,
        zmax: Option<f64>,
        settings: &TickerSettings,
    ) -> Result<Vec<f64>, ContourError> {
        let default;
        let locator = if let Some(locator) = self.locator {
            locator
        } else {
            default = self.default_locator(n)?;
            default.as_ref()
        };
        let ctx = TickContext::detached(settings);
        // Fully masked data compares like 0 in the locator and never trims.
        // Like the ticks of an axis, no more levels than are drawable: each
        // is contoured and drawn (matplotlib takes minutes over a million).
        let lev =
            drawable_ticks(locator.tick_values(zmin.unwrap_or(0.0), zmax.unwrap_or(0.0), &ctx)?)?;
        let symmetric = (locator as &dyn Any)
            .downcast_ref::<MaxNLocator>()
            .is_some_and(MaxNLocator::symmetric);
        let (Some(zmin), Some(zmax), false) = (zmin, zmax, symmetric) else {
            return Ok(lev);
        };
        let len = lev.len();
        let mut i0 = lev.iter().rposition(|&v| v < zmin).unwrap_or(0);
        let mut i1 = lev.iter().position(|&v| v > zmax).map_or(len, |i| i + 1);
        if self.extend.extends_min() {
            i0 += 1;
        }
        if self.extend.extends_max() {
            i1 = i1.saturating_sub(1);
        }
        if i1 < i0 + 3 {
            return Ok(lev);
        }
        Ok(lev[i0..i1].to_vec())
    }

    /// matplotlib's `_ensure_locator_exists`: `LogLocator(numticks=n)` when
    /// log-scaled, else `MaxNLocator(n + 1, min_n_ticks=1)` with `n = 7`
    /// by default.
    fn default_locator(&self, n: Option<i64>) -> Result<Box<dyn Locator>, ContourError> {
        if self.logscale {
            let numticks = match n {
                None => NumTicks::Auto,
                Some(n) => NumTicks::Fixed(positive(n)?),
            };
            return Ok(Box::new(LogLocator::new(10.0)?.with_numticks(numticks)));
        }
        let nbins = positive(n.unwrap_or(7).saturating_add(1))?;
        Ok(Box::new(
            MaxNLocator::new(Nbins::Fixed(nbins)).with_min_n_ticks(1),
        ))
    }
}

/// A count the locators accept.
fn positive(n: i64) -> Result<usize, ContourError> {
    usize::try_from(n)
        .ok()
        .filter(|&n| n > 0)
        .ok_or_else(|| TickerError::invalid("levels", "must be positive").into())
}

/// numpy's `np.min(np.diff(levels)) > 0` (a NaN difference passes, as the
/// minimum is then NaN).
fn increasing(levels: &[f64]) -> bool {
    let diffs: Vec<f64> = levels.windows(2).map(|w| w[1] - w[0]).collect();
    diffs.iter().any(|d| d.is_nan()) || diffs.iter().all(|&d| d > 0.0)
}

/// The levels with the `extend` sentinels added (matplotlib's `_levels`):
/// `±1e250`, or `1e-250`/`1e250` when log-scaled.
pub(crate) fn extended(levels: &[f64], extend: Extend, logscale: bool) -> Vec<f64> {
    let (lower, upper) = if logscale {
        (1e-250, 1e250)
    } else {
        (-1e250, 1e250)
    };
    let mut out = Vec::with_capacity(levels.len() + 2);
    if extend.extends_min() {
        out.push(lower);
    }
    out.extend_from_slice(levels);
    if extend.extends_max() {
        out.push(upper);
    }
    out
}

/// The value each level (lines) or band (fills) is coloured by: the levels
/// themselves, or the band midpoints (geometric when log-scaled).
pub(crate) fn layers(levels: &[f64], extended: &[f64], filled: bool, logscale: bool) -> Vec<f64> {
    if !filled {
        return levels.to_vec();
    }
    extended
        .windows(2)
        .map(|w| {
            if logscale {
                // The square roots avoid overflow.
                w[0].sqrt() * w[1].sqrt()
            } else {
                f64::midpoint(w[0], w[1])
            }
        })
        .collect()
}

/// The z intervals of the bands of a filled contour: consecutive extended
/// levels, with the lowest bound lowered (by 1, or to 99% when
/// log-scaled) if it equals `zmin`, so that the minimum is included.
#[expect(
    clippy::float_cmp,
    reason = "matplotlib tests the minimum for exact equality"
)]
pub(crate) fn bands(extended: &[f64], zmin: Option<f64>, logscale: bool) -> Vec<(f64, f64)> {
    let mut lowers: Vec<f64> = extended
        .iter()
        .take(extended.len().saturating_sub(1))
        .copied()
        .collect();
    if let (Some(first), Some(zmin)) = (lowers.first_mut(), zmin)
        && *first == zmin
    {
        *first = if logscale { 0.99 * zmin } else { *first - 1.0 };
    }
    lowers
        .into_iter()
        .zip(extended.iter().skip(1).copied())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(levels: &Levels, filled: bool, extend: Extend) -> LevelRequest<'_> {
        LevelRequest {
            levels,
            locator: None,
            logscale: false,
            filled,
            extend,
            boolean: false,
        }
    }

    fn choose(
        levels: &Levels,
        filled: bool,
        extend: Extend,
        z: (f64, f64),
    ) -> Result<Vec<f64>, ContourError> {
        request(levels, filled, extend).choose(Some(z.0), Some(z.1), &TickerSettings::DEFAULT)
    }

    #[test]
    fn automatic_levels_are_trimmed_to_the_data() {
        let auto = choose(&Levels::Auto, false, Extend::Neither, (0.0, 2.0)).unwrap();
        assert_eq!(auto, [0.0, 0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0]);
        let three = choose(&Levels::Count(3), false, Extend::Neither, (0.05, 1.95)).unwrap();
        assert_eq!(three, [0.0, 0.5, 1.0, 1.5, 2.0]);
        let both = choose(&Levels::Count(3), false, Extend::Both, (0.05, 1.95)).unwrap();
        assert_eq!(both, [0.5, 1.0, 1.5]);
    }

    #[test]
    fn constant_and_masked_data() {
        let constant = choose(&Levels::Auto, false, Extend::Neither, (1.0, 1.0)).unwrap();
        assert_eq!(
            constant,
            [0.999_999_999_999_975, 1.0, 1.000_000_000_000_025]
        );
        let masked = request(&Levels::Auto, false, Extend::Neither)
            .choose(None, None, &TickerSettings::DEFAULT)
            .unwrap();
        assert_eq!(masked.len(), 9);
        assert_eq!((masked[0], masked[8]), (-1e-13, 1e-13));
    }

    #[test]
    fn validation() {
        let bad = Levels::Values(vec![1.0, 0.5]);
        assert_eq!(
            choose(&bad, false, Extend::Neither, (0.0, 1.0)),
            Err(ContourError::LevelsNotIncreasing)
        );
        let single = Levels::Values(vec![0.5]);
        assert_eq!(
            choose(&single, true, Extend::Neither, (0.0, 1.0)),
            Err(ContourError::TooFewFilledLevels(1))
        );
        let none = choose(&Levels::Values(vec![]), false, Extend::Neither, (0.0, 1.0));
        assert_eq!(none, Ok(Vec::new()));
        assert!(choose(&Levels::Count(-1), false, Extend::Neither, (0.0, 1.0)).is_err());
        // Found by fuzzing: a million levels took minutes to fill.
        assert_eq!(
            choose(&Levels::Count(1 << 20), true, Extend::Neither, (0.0, 1.0)),
            Err(ContourError::Ticker(TickerError::TooManyTicks(1_000_001)))
        );
        let nan = Levels::Values(vec![1.0, f64::NAN, 0.5]);
        assert!(
            choose(&nan, false, Extend::Neither, (0.0, 1.0)).is_ok(),
            "numpy's NaN minimum passes"
        );
    }

    #[test]
    fn derived_levels() {
        let ext = extended(&[0.0, 0.5, 1.0], Extend::Both, false);
        assert_eq!(ext, [-1e250, 0.0, 0.5, 1.0, 1e250]);
        assert_eq!(
            layers(&[0.0, 0.5, 1.0], &ext, true, false),
            [-5e249, 0.25, 0.75, 5e249]
        );
        assert_eq!(
            layers(&[0.0, 0.5, 1.0], &ext, false, false),
            [0.0, 0.5, 1.0]
        );
        assert_eq!(
            bands(&[0.0, 1.0, 2.0], Some(0.0), false),
            [(-1.0, 1.0), (1.0, 2.0)]
        );
        assert_eq!(bands(&[1.0, 10.0], Some(1.0), true), [(0.99, 10.0)]);
        assert_eq!(bands(&[0.0, 1.0], None, false), [(0.0, 1.0)]);
    }
}
