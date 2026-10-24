//! Violin-plot statistics (`matplotlib.cbook.violin_stats`).

use super::StatsError;
use super::kde::{BandwidthMethod, GaussianKde};
use super::summary::{mean, median, percentiles};
use crate::numeric::linspace;

/// Options of [`violin_stats`].
#[derive(Clone, Debug, PartialEq)]
pub struct ViolinOptions {
    /// Bandwidth of the Gaussian kernel density estimate.
    pub bw_method: BandwidthMethod,
    /// Number of points at which the density is evaluated, at most
    /// [`MAX_VIOLIN_POINTS`].
    pub points: usize,
    /// Per dataset, the quantiles (fractions in `[0, 1]`) to report;
    /// `None` reports none.
    pub quantiles: Option<Vec<Vec<f64>>>,
}

impl Default for ViolinOptions {
    fn default() -> Self {
        Self {
            bw_method: BandwidthMethod::Scott,
            points: 100,
            quantiles: None,
        }
    }
}

/// The statistics `Axes.violin` draws for one dataset.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ViolinStats {
    /// Evaluation coordinates, evenly spaced from `min` to `max`.
    pub coords: Vec<f64>,
    /// Density values at `coords`.
    pub vals: Vec<f64>,
    /// Mean.
    pub mean: f64,
    /// Median.
    pub median: f64,
    /// Minimum.
    pub min: f64,
    /// Maximum.
    pub max: f64,
    /// The requested quantiles' values.
    pub quantiles: Vec<f64>,
}

/// Violin statistics with matplotlib's default Gaussian KDE.
///
/// Non-finite samples are dropped first. A dataset whose values are all
/// equal gets a spike: density 1 where a coordinate equals the value, 0
/// elsewhere.
///
/// ```
/// use plt::stats::{ViolinOptions, violin_stats};
/// let opts = ViolinOptions { points: 3, ..ViolinOptions::default() };
/// let stats = violin_stats(&[[1.0, 2.0, 4.0]], opts).unwrap();
/// assert_eq!(stats[0].coords, vec![1.0, 2.5, 4.0]);
/// assert_eq!(stats[0].median, 2.0);
/// ```
///
/// # Errors
///
/// As [`violin_stats_with`], and when a density cannot be estimated.
pub fn violin_stats<D: AsRef<[f64]>>(
    datasets: &[D],
    opts: ViolinOptions,
) -> Result<Vec<ViolinStats>, StatsError> {
    let bw = opts.bw_method;
    violin_stats_with(
        datasets,
        opts.points,
        opts.quantiles.as_deref(),
        |x, coords| {
            if x.iter().all(|&v| v == x[0]) {
                return Ok(coords
                    .iter()
                    .map(|&c| f64::from(u8::from(c == x[0])))
                    .collect());
            }
            GaussianKde::new(x, bw)?.evaluate(coords)
        },
    )
}

/// The most points a violin's density is evaluated at: matplotlib
/// allocates whatever count is asked, until memory runs out.
pub const MAX_VIOLIN_POINTS: usize = 1 << 20;

/// Violin statistics with a custom density estimator `method(samples,
/// coords)`, which receives the finite samples of one dataset.
///
/// # Errors
///
/// When `points` is more than [`MAX_VIOLIN_POINTS`], the quantiles are
/// not one list per dataset or one is outside `[0, 1]`, or `method`
/// fails.
pub fn violin_stats_with<D, F>(
    datasets: &[D],
    points: usize,
    quantiles: Option<&[Vec<f64>]>,
    mut method: F,
) -> Result<Vec<ViolinStats>, StatsError>
where
    D: AsRef<[f64]>,
    F: FnMut(&[f64], &[f64]) -> Result<Vec<f64>, StatsError>,
{
    if points > MAX_VIOLIN_POINTS {
        return Err(StatsError::TooManyPoints(points));
    }
    let quantiles: Vec<&[f64]> = match quantiles {
        Some(q) if !q.is_empty() => q.iter().map(Vec::as_slice).collect(),
        _ => vec![&[]; datasets.len()],
    };
    if quantiles.len() != datasets.len() {
        return Err(StatsError::CountMismatch {
            what: "quantiles",
            datasets: datasets.len(),
            actual: quantiles.len(),
        });
    }
    datasets
        .iter()
        .zip(quantiles)
        .map(|(data, quantiles)| {
            let x: Vec<f64> = data
                .as_ref()
                .iter()
                .copied()
                .filter(|v| v.is_finite())
                .collect();
            if x.is_empty() {
                return Ok(ViolinStats {
                    mean: f64::NAN,
                    median: f64::NAN,
                    min: f64::NAN,
                    max: f64::NAN,
                    ..ViolinStats::default()
                });
            }
            let min = x.iter().copied().fold(f64::INFINITY, f64::min);
            let max = x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let coords = linspace(min, max, points);
            let ps: Vec<f64> = quantiles.iter().map(|q| 100.0 * q).collect();
            Ok(ViolinStats {
                vals: method(&x, &coords)?,
                coords,
                mean: mean(&x),
                median: median(&x),
                min,
                max,
                quantiles: percentiles(&x, &ps)?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_and_empty_datasets() {
        let opts = ViolinOptions {
            points: 3,
            ..ViolinOptions::default()
        };
        let stats = violin_stats(&[vec![2.0, 2.0], vec![], vec![f64::NAN, 3.0]], opts).unwrap();
        assert_eq!(stats[0].vals, vec![1.0, 1.0, 1.0]);
        assert!(stats[1].coords.is_empty() && stats[1].mean.is_nan());
        assert_eq!((stats[2].min, stats[2].max), (3.0, 3.0));
    }

    #[test]
    fn evaluation_point_counts_are_bounded() {
        // Found by fuzzing: `usize::MAX` points overflowed the capacity.
        let opts = ViolinOptions {
            points: usize::MAX,
            ..ViolinOptions::default()
        };
        assert_eq!(
            violin_stats(&[[1.0, 2.0]], opts),
            Err(StatsError::TooManyPoints(usize::MAX))
        );
    }

    #[test]
    fn quantiles_per_dataset() {
        let opts = ViolinOptions {
            points: 2,
            quantiles: Some(vec![vec![0.5], vec![]]),
            ..ViolinOptions::default()
        };
        let stats = violin_stats(&[vec![1.0, 2.0, 3.0], vec![1.0, 5.0]], opts.clone()).unwrap();
        assert_eq!(stats[0].quantiles, vec![2.0]);
        assert_eq!(stats[1].quantiles, Vec::<f64>::new());
        let bad = ViolinOptions {
            quantiles: Some(vec![vec![0.5]]),
            ..opts.clone()
        };
        assert!(violin_stats(&[vec![1.0, 2.0], vec![3.0, 4.0]], bad).is_err());
        let out_of_range = ViolinOptions {
            quantiles: Some(vec![vec![1.5]]),
            ..opts
        };
        assert!(violin_stats(&[vec![1.0, 2.0]], out_of_range).is_err());
    }

    #[test]
    fn kde_density_is_evaluated_at_coords() {
        let opts = ViolinOptions {
            points: 5,
            ..ViolinOptions::default()
        };
        let stats = violin_stats(&[[0.0, 1.0, 2.0, 3.0]], opts).unwrap();
        let kde = GaussianKde::new(&[0.0, 1.0, 2.0, 3.0], BandwidthMethod::Scott).unwrap();
        assert_eq!(stats[0].vals, kde.evaluate(&stats[0].coords).unwrap());
    }
}
