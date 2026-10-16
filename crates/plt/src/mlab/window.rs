//! Tapering windows for spectral estimates.

use std::f64::consts::PI;

use super::MlabError;

/// The window applied to each segment before its Fourier transform.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Window {
    /// The Hann window ([`window_hanning`], matplotlib's default).
    #[default]
    Hanning,
    /// No tapering ([`window_none`]).
    Rectangular,
    /// Explicit window values; there must be one per segment sample.
    Values(Vec<f64>),
}

impl Window {
    /// The `n` window values (matplotlib applies a window function to
    /// `ones(n)`).
    pub fn values(&self, n: usize) -> Result<Vec<f64>, MlabError> {
        match self {
            Self::Hanning => Ok(hanning(n)),
            Self::Rectangular => Ok(vec![1.0; n]),
            Self::Values(values) if values.len() == n => Ok(values.clone()),
            Self::Values(values) => Err(MlabError::WindowLength {
                expected: n,
                actual: values.len(),
            }),
        }
    }
}

/// The `m`-point Hann window (numpy `hanning`):
/// `0.5 + 0.5·cos(π·k/(m-1))` for `k = 1-m, 3-m, ..., m-1`.
///
/// ```
/// assert_eq!(plt::mlab::hanning(3), vec![0.0, 1.0, 0.0]);
/// ```
pub fn hanning(m: usize) -> Vec<f64> {
    match m {
        0 => Vec::new(),
        1 => vec![1.0],
        _ => {
            let denom = (m - 1) as f64;
            (0..m)
                .map(|i| {
                    let k = 2.0 * i as f64 + 1.0 - m as f64;
                    0.5 + 0.5 * (PI * k / denom).cos()
                })
                .collect()
        }
    }
}

/// `x` multiplied by a Hann window of its length (matplotlib
/// `window_hanning`).
pub fn window_hanning(x: &[f64]) -> Vec<f64> {
    hanning(x.len()).iter().zip(x).map(|(w, v)| w * v).collect()
}

/// `x` unchanged (matplotlib `window_none`).
pub fn window_none(x: &[f64]) -> Vec<f64> {
    x.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hann_window_shape() {
        assert_eq!(hanning(0), Vec::<f64>::new());
        assert_eq!(hanning(1), vec![1.0]);
        let h = hanning(9);
        assert_eq!((h[0], h[4], h[8]), (0.0, 1.0, 0.0));
        for i in 0..9 {
            assert!((h[i] - h[8 - i]).abs() < 1e-16);
        }
        assert_eq!(window_hanning(&[2.0, 2.0, 2.0]), vec![0.0, 2.0, 0.0]);
        assert_eq!(window_none(&[2.0, -1.0]), vec![2.0, -1.0]);
    }

    #[test]
    fn window_values() {
        assert_eq!(Window::Rectangular.values(2), Ok(vec![1.0, 1.0]));
        assert_eq!(Window::Values(vec![0.5]).values(1), Ok(vec![0.5]));
        assert_eq!(
            Window::Values(vec![0.5]).values(2),
            Err(MlabError::WindowLength {
                expected: 2,
                actual: 1
            })
        );
    }
}
