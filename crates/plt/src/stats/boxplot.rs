//! Box-and-whisker statistics (`matplotlib.cbook.boxplot_stats`).

use super::summary::{mean, percentile_array};
use super::{Bootstrap, StatsError};

/// How far the whiskers reach.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Whis {
    /// Whiskers reach the furthest data within `factor · IQR` beyond the
    /// quartiles (matplotlib's default factor is 1.5; infinity spans the
    /// whole data range).
    Factor(f64),
    /// Whiskers sit at these two percentiles of the data.
    Percentiles(f64, f64),
}

impl Default for Whis {
    fn default() -> Self {
        Self::Factor(1.5)
    }
}

/// Options of [`boxplot_stats`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BoxplotStatsOptions {
    /// Whisker reach.
    pub whis: Whis,
    /// One label per dataset, copied into [`BoxStats::label`].
    pub labels: Option<Vec<String>>,
    /// When the interquartile range is zero, extend the whiskers to the
    /// data's minimum and maximum instead.
    pub autorange: bool,
    /// Notches from a bootstrap of the median instead of the Gaussian
    /// approximation (`bootstrap`).
    pub bootstrap: Option<Bootstrap>,
}

/// The statistics `Axes.bxp` draws for one dataset.
///
/// The confidence interval around the median (`cilo`, `cihi`) uses the
/// Gaussian approximation `med ∓ 1.57·IQR/√n`, unless it is bootstrapped
/// ([`BoxplotStatsOptions::bootstrap`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BoxStats {
    /// The dataset's label, when labels were supplied.
    pub label: Option<String>,
    /// Arithmetic mean.
    pub mean: f64,
    /// Interquartile range `q3 - q1`.
    pub iqr: f64,
    /// Lower end of the median's confidence interval (notch).
    pub cilo: f64,
    /// Upper end of the median's confidence interval (notch).
    pub cihi: f64,
    /// Upper whisker end.
    pub whishi: f64,
    /// Lower whisker end.
    pub whislo: f64,
    /// Samples beyond the whiskers: those below `whislo` then those above
    /// `whishi`, each in data order.
    pub fliers: Vec<f64>,
    /// First quartile.
    pub q1: f64,
    /// Median.
    pub med: f64,
    /// Third quartile.
    pub q3: f64,
}

/// Box-plot statistics of each dataset.
///
/// ```
/// use plt::stats::{BoxplotStatsOptions, boxplot_stats};
/// let stats = boxplot_stats(&[[1.0, 2.0, 3.0, 4.0, 100.0]], BoxplotStatsOptions::default()).unwrap();
/// assert_eq!((stats[0].q1, stats[0].med, stats[0].q3), (2.0, 3.0, 4.0));
/// assert_eq!(stats[0].whishi, 4.0);
/// assert_eq!(stats[0].fliers, vec![100.0]);
/// ```
pub fn boxplot_stats<D: AsRef<[f64]>>(
    datasets: &[D],
    opts: BoxplotStatsOptions,
) -> Result<Vec<BoxStats>, StatsError> {
    if let Some(labels) = &opts.labels
        && labels.len() != datasets.len()
    {
        return Err(StatsError::CountMismatch {
            what: "labels",
            datasets: datasets.len(),
            actual: labels.len(),
        });
    }
    // One generator for all the datasets, as matplotlib draws every
    // resample from numpy's global one.
    let mut rng = opts.bootstrap.as_ref().map(Bootstrap::generator);
    datasets
        .iter()
        .enumerate()
        .map(|(i, data)| {
            let data = data.as_ref();
            let mut stats = box_stats(data, opts.whis, opts.autorange)?;
            stats.label = opts.labels.as_ref().map(|labels| labels[i].clone());
            if let (Some(bootstrap), Some(rng)) = (&opts.bootstrap, &mut rng)
                && !data.is_empty()
            {
                (stats.cilo, stats.cihi) = bootstrap.median_interval(data, rng)?;
            }
            Ok(stats)
        })
        .collect()
}

/// Box-plot statistics of one dataset (without label), its notches from
/// the Gaussian approximation.
pub fn box_stats(x: &[f64], whis: Whis, autorange: bool) -> Result<BoxStats, StatsError> {
    if x.is_empty() {
        return Ok(BoxStats {
            label: None,
            mean: f64::NAN,
            iqr: f64::NAN,
            cilo: f64::NAN,
            cihi: f64::NAN,
            whishi: f64::NAN,
            whislo: f64::NAN,
            fliers: Vec::new(),
            q1: f64::NAN,
            med: f64::NAN,
            q3: f64::NAN,
        });
    }
    let [q1, med, q3] = percentile_array(x, [25.0, 50.0, 75.0])?;
    let iqr = q3 - q1;
    let whis = if iqr == 0.0 && autorange {
        Whis::Percentiles(0.0, 100.0)
    } else {
        whis
    };
    let notch = 1.57 * iqr / (x.len() as f64).sqrt();
    let (loval, hival) = match whis {
        Whis::Factor(factor) => (q1 - factor * iqr, q3 + factor * iqr),
        Whis::Percentiles(lo, hi) => {
            let [lo, hi] = percentile_array(x, [lo, hi])?;
            (lo, hi)
        }
    };
    let whishi = x
        .iter()
        .copied()
        .filter(|&v| v <= hival)
        .reduce(f64::max)
        .filter(|&hi| hi >= q3)
        .unwrap_or(q3);
    let whislo = x
        .iter()
        .copied()
        .filter(|&v| v >= loval)
        .reduce(f64::min)
        .filter(|&lo| lo <= q1)
        .unwrap_or(q1);
    let fliers = x
        .iter()
        .copied()
        .filter(|&v| v < whislo)
        .chain(x.iter().copied().filter(|&v| v > whishi))
        .collect();
    Ok(BoxStats {
        label: None,
        mean: mean(x),
        iqr,
        cilo: med - notch,
        cihi: med + notch,
        whishi,
        whislo,
        fliers,
        q1,
        med,
        q3,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whiskers_and_fliers() {
        let s = box_stats(&[8.0, 1.0, 2.0, 3.0, 4.0, -9.0], Whis::default(), false).unwrap();
        assert_eq!((s.q1, s.med, s.q3), (1.25, 2.5, 3.75));
        assert_eq!((s.whislo, s.whishi), (1.0, 4.0));
        assert_eq!(s.fliers, vec![-9.0, 8.0]);
        let all = box_stats(&[8.0, 1.0, -9.0], Whis::Factor(f64::INFINITY), false).unwrap();
        assert_eq!((all.whislo, all.whishi), (-9.0, 8.0));
        assert_eq!(all.fliers, Vec::<f64>::new());
    }

    #[test]
    fn percentile_whiskers_and_autorange() {
        let data = [1.0, 3.0, 3.0, 3.0, 3.0, 9.0];
        let flat = box_stats(&data, Whis::default(), false).unwrap();
        assert_eq!((flat.whislo, flat.whishi, flat.iqr), (3.0, 3.0, 0.0));
        let auto = box_stats(&data, Whis::default(), true).unwrap();
        assert_eq!((auto.whislo, auto.whishi), (1.0, 9.0));
        let pct = box_stats(&data, Whis::Percentiles(0.0, 100.0), false).unwrap();
        assert_eq!((pct.whislo, pct.whishi), (1.0, 9.0));
        assert!(box_stats(&data, Whis::Percentiles(0.0, 120.0), false).is_err());
    }

    #[test]
    fn empty_and_nan_datasets() {
        let empty = box_stats(&[], Whis::default(), false).unwrap();
        assert!(empty.med.is_nan() && empty.fliers.is_empty());
        let nan = box_stats(&[1.0, f64::NAN], Whis::default(), false).unwrap();
        assert!(nan.med.is_nan() && nan.whishi.is_nan() && nan.fliers.is_empty());
    }

    #[test]
    fn labels_must_match() {
        let opts = BoxplotStatsOptions {
            labels: Some(vec!["a".into()]),
            ..BoxplotStatsOptions::default()
        };
        assert!(boxplot_stats(&[[1.0], [2.0]], opts.clone()).is_err());
        let one = boxplot_stats(&[[1.0]], opts).unwrap();
        assert_eq!(one[0].label.as_deref(), Some("a"));
        assert_eq!(one[0].cilo, 1.0);
    }

    #[test]
    fn bootstraps_replace_the_notches_dataset_after_dataset() {
        let data = [1.0, 2.0, 4.0, 8.0, 9.0, 12.0, 20.0];
        let bootstrap = Bootstrap {
            resamples: 400,
            seed: 11,
        };
        let opts = BoxplotStatsOptions {
            bootstrap: Some(bootstrap),
            ..BoxplotStatsOptions::default()
        };
        let both = boxplot_stats(&[&data[..], &[], &data[..]], opts.clone()).unwrap();
        let plain = box_stats(&data, Whis::default(), false).unwrap();
        for stats in [&both[0], &both[2]] {
            assert_eq!(
                (stats.q1, stats.med, stats.q3),
                (plain.q1, plain.med, plain.q3)
            );
            assert!(stats.cilo <= stats.cihi);
            assert!(stats.cilo >= 1.0 && stats.cihi <= 20.0);
        }
        assert!(both[1].cilo.is_nan(), "an empty dataset draws nothing");
        let first = bootstrap.median_interval(&data, &mut bootstrap.generator());
        assert_eq!(first, Ok((both[0].cilo, both[0].cihi)));
        let no_resamples = BoxplotStatsOptions {
            bootstrap: Some(Bootstrap {
                resamples: 0,
                seed: 0,
            }),
            ..opts
        };
        assert_eq!(
            boxplot_stats(&[&data], no_resamples),
            Err(StatsError::NoResamples)
        );
    }
}
