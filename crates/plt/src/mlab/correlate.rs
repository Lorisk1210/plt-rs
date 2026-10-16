//! Auto- and cross-correlation as plotted by `Axes.acorr` / `Axes.xcorr`.

use super::{Detrend, MlabError};

/// Options of [`xcorr`] and [`acorr`]; the defaults are matplotlib's.
#[derive(Clone, Debug, PartialEq)]
pub struct XcorrOptions {
    /// Normalise by `√(x·x · y·y)` so that a perfect match is 1.
    pub normed: bool,
    /// Detrending applied to both whole signals first.
    pub detrend: Detrend,
    /// Largest lag reported; `None` reports all `len - 1` lags. Must be at
    /// least 1 and below the signal length.
    pub maxlags: Option<usize>,
}

impl Default for XcorrOptions {
    fn default() -> Self {
        Self {
            normed: true,
            detrend: Detrend::None,
            maxlags: Some(10),
        }
    }
}

/// Correlation values by lag.
#[derive(Clone, Debug, PartialEq)]
pub struct Correlation {
    /// Lags `-maxlags..=maxlags`.
    pub lags: Vec<i64>,
    /// The correlation at each lag.
    pub values: Vec<f64>,
}

/// The cross-correlation of `x` and `y` (`Axes.xcorr` without drawing):
/// at lag `k`, `Σₙ x[n+k]·y[n]`.
///
/// ```
/// use plt::mlab::{XcorrOptions, xcorr};
/// let opts = XcorrOptions { normed: false, maxlags: Some(1), ..XcorrOptions::default() };
/// let c = xcorr(&[1.0, 0.0, 0.0], &[0.0, 1.0, 0.0], &opts).unwrap();
/// assert_eq!(c.lags, vec![-1, 0, 1]);
/// assert_eq!(c.values, vec![1.0, 0.0, 0.0]);
/// ```
pub fn xcorr(x: &[f64], y: &[f64], opts: &XcorrOptions) -> Result<Correlation, MlabError> {
    let n = x.len();
    if n != y.len() {
        return Err(MlabError::LengthMismatch { x: n, y: y.len() });
    }
    let x = opts.detrend.apply(x)?;
    let y = opts.detrend.apply(y)?;
    let mut values = correlate_full(&x, &y);
    if opts.normed {
        let norm = (dot(&x, &x) * dot(&y, &y)).sqrt();
        for v in &mut values {
            *v /= norm;
        }
    }
    let maxlags = opts.maxlags.unwrap_or(n.saturating_sub(1));
    if maxlags >= n || maxlags < 1 {
        return Err(MlabError::InvalidMaxLags { maxlags, len: n });
    }
    let lag = maxlags as i64;
    Ok(Correlation {
        lags: (-lag..=lag).collect(),
        values: values[n - 1 - maxlags..n + maxlags].to_vec(),
    })
}

/// The autocorrelation of `x` (`Axes.acorr`, i.e. `xcorr(x, x)`).
pub fn acorr(x: &[f64], opts: &XcorrOptions) -> Result<Correlation, MlabError> {
    xcorr(x, x, opts)
}

/// numpy `correlate(a, v, mode='full')`: entry `j` is the lag
/// `k = j - (len(v) - 1)`, `Σₙ a[n+k]·v[n]`.
pub fn correlate_full(a: &[f64], v: &[f64]) -> Vec<f64> {
    if a.is_empty() || v.is_empty() {
        return Vec::new();
    }
    let (na, nv) = (a.len() as isize, v.len() as isize);
    (-(nv - 1)..na)
        .map(|k| {
            let start = (-k).max(0);
            let end = nv.min(na - k);
            (start..end).fold(0.0, |acc, n| acc + a[(n + k) as usize] * v[n as usize])
        })
        .collect()
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).fold(0.0, |acc, (x, y)| acc + x * y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_correlation_matches_numpy() {
        assert_eq!(
            correlate_full(&[1.0, 2.0, 3.0], &[0.0, 1.0, 0.5]),
            vec![0.5, 2.0, 3.5, 3.0, 0.0]
        );
        assert_eq!(correlate_full(&[1.0, 2.0], &[1.0]), vec![1.0, 2.0]);
        assert_eq!(correlate_full(&[], &[1.0]), Vec::<f64>::new());
    }

    #[test]
    fn normalised_autocorrelation_peaks_at_one() {
        let x = [1.0, -2.0, 3.0, 0.5, -1.0];
        let c = acorr(
            &x,
            &XcorrOptions {
                maxlags: None,
                ..XcorrOptions::default()
            },
        )
        .unwrap();
        assert_eq!(c.lags.len(), 9);
        assert!((c.values[4] - 1.0).abs() < 1e-15);
        assert_eq!(c.values[0], c.values[8]);
    }

    #[test]
    fn errors() {
        let opts = XcorrOptions::default();
        assert_eq!(
            xcorr(&[1.0], &[1.0, 2.0], &opts),
            Err(MlabError::LengthMismatch { x: 1, y: 2 })
        );
        assert_eq!(
            acorr(&[1.0; 5], &opts),
            Err(MlabError::InvalidMaxLags {
                maxlags: 10,
                len: 5
            })
        );
        let none = XcorrOptions {
            maxlags: None,
            ..opts
        };
        assert_eq!(
            acorr(&[1.0], &none),
            Err(MlabError::InvalidMaxLags { maxlags: 0, len: 1 })
        );
        let zeros = acorr(
            &[0.0; 4],
            &XcorrOptions {
                maxlags: Some(1),
                ..XcorrOptions::default()
            },
        );
        assert!(zeros.unwrap().values.iter().all(|v| v.is_nan()));
    }
}
