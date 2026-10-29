//! Trend removal (`mlab.detrend`, `detrend_mean`, `detrend_linear`).

use std::fmt;
use std::ops::{Add, Div, Mul, Sub};
use std::sync::Arc;

use super::{Complex, MlabError};
use crate::numeric::Grid;
use crate::stats::pairwise;

/// A user-supplied detrending function of one real segment.
pub type DetrendFn = Arc<dyn Fn(&[f64]) -> Vec<f64> + Send + Sync>;

/// How a signal (or each spectral segment) is detrended.
#[derive(Clone, Default)]
pub enum Detrend {
    /// Leave the data unchanged ([`detrend_none`]).
    #[default]
    None,
    /// Subtract the mean ([`detrend_mean`]).
    Mean,
    /// Subtract the least-squares line ([`detrend_linear`]).
    Linear,
    /// A custom function; it must return as many values as it receives.
    /// Complex data is detrended by applying it to the real and imaginary
    /// parts separately.
    Custom(DetrendFn),
}

impl fmt::Debug for Detrend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => f.write_str("None"),
            Self::Mean => f.write_str("Mean"),
            Self::Linear => f.write_str("Linear"),
            Self::Custom(_) => f.write_str("Custom(..)"),
        }
    }
}

impl PartialEq for Detrend {
    /// Built-in methods compare by kind, custom functions by identity.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::None, Self::None) | (Self::Mean, Self::Mean) | (Self::Linear, Self::Linear) => {
                true
            }
            (Self::Custom(a), Self::Custom(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl Detrend {
    /// Detrends one real sequence.
    pub fn apply(&self, x: &[f64]) -> Result<Vec<f64>, MlabError> {
        match self {
            Self::None => Ok(detrend_none(x)),
            Self::Mean => Ok(detrend_mean(x)),
            Self::Linear => Ok(detrend_linear(x)),
            Self::Custom(f) => checked(x.len(), f(x)),
        }
    }

    /// Detrends one complex sequence; `complex` tells whether the imaginary
    /// parts carry data (for real signals only the real parts are passed
    /// to a custom function).
    pub(crate) fn apply_complex(
        &self,
        x: &[Complex],
        complex: bool,
    ) -> Result<Vec<Complex>, MlabError> {
        match self {
            Self::None => Ok(x.to_vec()),
            Self::Mean => Ok(remove_mean(x)),
            Self::Linear => Ok(remove_line(x)),
            Self::Custom(f) => {
                let re: Vec<f64> = x.iter().map(|z| z.re).collect();
                let re = checked(x.len(), f(&re))?;
                if complex {
                    let im: Vec<f64> = x.iter().map(|z| z.im).collect();
                    let im = checked(x.len(), f(&im))?;
                    Ok(re
                        .into_iter()
                        .zip(im)
                        .map(|(r, i)| Complex::new(r, i))
                        .collect())
                } else {
                    Ok(re.into_iter().map(Complex::real).collect())
                }
            }
        }
    }
}

fn checked(expected: usize, values: Vec<f64>) -> Result<Vec<f64>, MlabError> {
    if values.len() == expected {
        Ok(values)
    } else {
        Err(MlabError::DetrendLength {
            expected,
            actual: values.len(),
        })
    }
}

/// `x` minus its mean (matplotlib `detrend_mean`).
///
/// ```
/// assert_eq!(pltrs::mlab::detrend_mean(&[1.0, 2.0, 6.0]), vec![-2.0, -1.0, 3.0]);
/// ```
pub fn detrend_mean(x: &[f64]) -> Vec<f64> {
    remove_mean(x)
}

/// `y` minus its least-squares line over the sample index (matplotlib
/// `detrend_linear`, which fits through `np.cov`).
///
/// A single sample has no defined slope and yields NaN, as in matplotlib.
pub fn detrend_linear(y: &[f64]) -> Vec<f64> {
    remove_line(y)
}

/// `x` unchanged (matplotlib `detrend_none`).
pub fn detrend_none(x: &[f64]) -> Vec<f64> {
    x.to_vec()
}

/// Detrends a 2-D array along `axis` (numpy convention: `Some(0)` treats
/// each column as a sequence, `Some(1)` each row). With `axis = None`
/// [`Detrend::Mean`] removes the overall mean and [`Detrend::None`] is the
/// identity; the sequence-based methods need an axis.
pub fn detrend_grid(
    x: &Grid<f64>,
    detrend: &Detrend,
    axis: Option<usize>,
) -> Result<Grid<f64>, MlabError> {
    if let Some(axis) = axis
        && axis >= 2
    {
        return Err(MlabError::AxisOutOfBounds { axis, ndim: 2 });
    }
    match (detrend, axis) {
        (Detrend::None, _) => Ok(x.clone()),
        (Detrend::Mean, None) => {
            let m = pairwise(x.as_slice(), 0.0) / x.as_slice().len() as f64;
            Ok(x.map(|v| v - m))
        }
        (Detrend::Linear | Detrend::Custom(_), None) => Err(MlabError::AxisRequired),
        (_, Some(0)) => Ok(detrend_rows(&x.transposed(), detrend)?.transposed()),
        (_, Some(_)) => detrend_rows(x, detrend),
    }
}

fn detrend_rows(x: &Grid<f64>, detrend: &Detrend) -> Result<Grid<f64>, MlabError> {
    let mut out = x.clone();
    if x.cols() == 0 {
        return Ok(out);
    }
    for (dst, src) in out
        .as_mut_slice()
        .chunks_exact_mut(x.cols())
        .zip(x.iter_rows())
    {
        // `apply` guarantees one output value per input value.
        dst.copy_from_slice(&detrend.apply(src)?);
    }
    Ok(out)
}

/// Arithmetic shared by real and complex detrending.
trait Field:
    Copy
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Mul<f64, Output = Self>
    + Div<f64, Output = Self>
{
    const ZERO: Self;
    fn from_real(v: f64) -> Self;
    fn conj(self) -> Self;
}

impl Field for f64 {
    const ZERO: Self = 0.0;

    fn from_real(v: f64) -> Self {
        v
    }

    fn conj(self) -> Self {
        self
    }
}

impl Field for Complex {
    const ZERO: Self = Complex::ZERO;

    fn from_real(v: f64) -> Self {
        Complex::real(v)
    }

    fn conj(self) -> Self {
        Complex::conj(self)
    }
}

fn mean_of<T: Field>(x: &[T]) -> T {
    pairwise(x, T::ZERO) / x.len() as f64
}

fn remove_mean<T: Field>(x: &[T]) -> Vec<T> {
    let m = mean_of(x);
    x.iter().map(|&v| v - m).collect()
}

/// `y - (b·i + a)` with `b = C01/C00` from `np.cov(i, y, bias=1)` (whose
/// complex form conjugates `y`) and `a = mean(y) - b·mean(i)`.
fn remove_line<T: Field>(y: &[T]) -> Vec<T> {
    let n = y.len();
    let x: Vec<T> = (0..n).map(|i| T::from_real(i as f64)).collect();
    let (x_mean, y_mean) = (mean_of(&x), mean_of(y));
    let scale = 1.0 / n as f64;
    let (c00, c01) = x
        .iter()
        .zip(y)
        .fold((T::ZERO, T::ZERO), |(c00, c01), (&xi, &yi)| {
            let (xc, yc) = (xi - x_mean, yi - y_mean);
            (c00 + xc * xc.conj(), c01 + xc * yc.conj())
        });
    let slope = (c01 * scale) / (c00 * scale);
    let intercept = y_mean - slope * x_mean;
    y.iter()
        .zip(x)
        .map(|(&yi, xi)| yi - (slope * xi + intercept))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_and_linear() {
        assert_eq!(detrend_mean(&[1.0, 3.0]), vec![-1.0, 1.0]);
        assert_eq!(detrend_mean(&[]), Vec::<f64>::new());
        assert_eq!(detrend_linear(&[1.0, 4.0]), vec![0.0, 0.0]);
        let r = detrend_linear(&[1.0, 5.0, 2.0]);
        assert!(r.iter().all(|v| v.is_finite()) && r.iter().sum::<f64>().abs() < 1e-15);
        assert_eq!(detrend_linear(&[3.0; 4]), vec![0.0; 4]);
        assert!(detrend_linear(&[1.0])[0].is_nan());
        let line: Vec<f64> = (0..10).map(|i| 2.0 + 0.5 * f64::from(i)).collect();
        assert!(detrend_linear(&line).iter().all(|v| v.abs() < 1e-14));
    }

    #[test]
    fn complex_detrending_matches_real_for_real_data() {
        let real = [3.0, 1.5, 4.0, 1.0, 5.5];
        let complex: Vec<Complex> = real.iter().map(|&v| Complex::real(v)).collect();
        for d in [Detrend::Mean, Detrend::Linear] {
            let a = d.apply(&real).unwrap();
            let b = d.apply_complex(&complex, false).unwrap();
            for (x, z) in a.iter().zip(&b) {
                assert!((x - z.re).abs() < 1e-15 && z.im == 0.0);
            }
        }
    }

    #[test]
    fn custom_functions() {
        let double: DetrendFn = Arc::new(|x| x.iter().map(|v| 2.0 * v).collect());
        let d = Detrend::Custom(double);
        assert_eq!(d.apply(&[1.0, 2.0]), Ok(vec![2.0, 4.0]));
        let z = d.apply_complex(&[Complex::new(1.0, 1.0)], true).unwrap();
        assert_eq!(z, vec![Complex::new(2.0, 2.0)]);
        let short: DetrendFn = Arc::new(|_| Vec::new());
        assert!(Detrend::Custom(short).apply(&[1.0]).is_err());
        assert_eq!(format!("{d:?}"), "Custom(..)");
    }

    #[test]
    fn grids_along_axes() {
        let g = Grid::from_rows([[1.0, 2.0, 6.0], [3.0, 4.0, 2.0]]).unwrap();
        let rows = detrend_grid(&g, &Detrend::Mean, Some(1)).unwrap();
        assert_eq!(rows.row(0), &[-2.0, -1.0, 3.0]);
        let cols = detrend_grid(&g, &Detrend::Mean, Some(0)).unwrap();
        assert_eq!(cols.row(0), &[-1.0, -1.0, 2.0]);
        let all = detrend_grid(&g, &Detrend::Mean, None).unwrap();
        assert_eq!(all.row(1), &[0.0, 1.0, -1.0]);
        assert_eq!(
            detrend_grid(&g, &Detrend::Linear, None),
            Err(MlabError::AxisRequired)
        );
        assert_eq!(
            detrend_grid(&g, &Detrend::Mean, Some(2)),
            Err(MlabError::AxisOutOfBounds { axis: 2, ndim: 2 })
        );
        let lin = detrend_grid(&g, &Detrend::Linear, Some(1)).unwrap();
        assert_eq!(lin.rows(), 2);
    }
}
