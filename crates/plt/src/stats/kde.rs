//! Gaussian kernel density estimation (`matplotlib.mlab.GaussianKDE`).

use std::f64::consts::PI;

use super::StatsError;
use super::linalg::Lu;
use super::summary::{mean, pairwise_sum};
use crate::numeric::Grid;

/// How the kernel bandwidth factor is chosen.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum BandwidthMethod {
    /// Scott's rule: `n^(-1/(d+4))`.
    #[default]
    Scott,
    /// Silverman's rule: `(n·(d+2)/4)^(-1/(d+4))`.
    Silverman,
    /// A fixed factor multiplying the data's standard deviation.
    Factor(f64),
}

impl BandwidthMethod {
    /// The bandwidth factor for `n` samples of `dim` dimensions
    /// (matplotlib's `scotts_factor`, `silverman_factor`, or the fixed one).
    ///
    /// ```
    /// use plt::stats::BandwidthMethod;
    /// assert_eq!(BandwidthMethod::Scott.factor(1, 32), 0.5);
    /// assert_eq!(BandwidthMethod::Factor(0.3).factor(2, 10), 0.3);
    /// ```
    pub fn factor(self, dim: usize, n: usize) -> f64 {
        let exponent = -1.0 / (dim as f64 + 4.0);
        match self {
            Self::Scott => (n as f64).powf(exponent),
            Self::Silverman => (n as f64 * (dim as f64 + 2.0) / 4.0).powf(exponent),
            Self::Factor(f) => f,
        }
    }
}

/// A Gaussian kernel density estimate of a `d`-dimensional dataset.
///
/// ```
/// use plt::stats::{BandwidthMethod, GaussianKde};
/// let kde = GaussianKde::new(&[1.0, 2.0, 4.0], BandwidthMethod::Scott).unwrap();
/// let density = kde.evaluate(&[2.0, 10.0]).unwrap();
/// assert!(density[0] > density[1]);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct GaussianKde {
    /// `dim × num_points` samples.
    dataset: Grid<f64>,
    factor: f64,
    covariance: Grid<f64>,
    inv_cov: Grid<f64>,
    norm_factor: f64,
}

impl GaussianKde {
    /// A one-dimensional estimate of `samples`.
    pub fn new(samples: &[f64], bw_method: BandwidthMethod) -> Result<Self, StatsError> {
        Self::from_grid(
            Grid::from_fn(1, samples.len(), |_, i| samples[i]),
            bw_method,
        )
    }

    /// A multi-dimensional estimate; each row of `dataset` is one
    /// dimension, each column one sample.
    pub fn from_grid(dataset: Grid<f64>, bw_method: BandwidthMethod) -> Result<Self, StatsError> {
        if dataset.as_slice().len() <= 1 {
            return Err(StatsError::TooFewSamples);
        }
        let (dim, n) = dataset.shape();
        let factor = bw_method.factor(dim, n);
        let data_covariance = sample_covariance(&dataset);
        let data_inv_cov = Lu::new(&data_covariance)
            .ok_or(StatsError::SingularCovariance)?
            .inverse();
        let factor_sq = factor * factor;
        let covariance = data_covariance.map(|c| c * factor_sq);
        let inv_cov = data_inv_cov.map(|c| c / factor_sq);
        let det = Lu::new(&covariance.map(|c| 2.0 * PI * c)).map_or(0.0, |lu| lu.determinant());
        let norm_factor = det.sqrt() * n as f64;
        Ok(Self {
            dataset,
            factor,
            covariance,
            inv_cov,
            norm_factor,
        })
    }

    /// Number of dimensions.
    pub fn dim(&self) -> usize {
        self.dataset.rows()
    }

    /// Number of samples.
    pub fn num_points(&self) -> usize {
        self.dataset.cols()
    }

    /// The bandwidth factor.
    pub fn factor(&self) -> f64 {
        self.factor
    }

    /// The kernel covariance: the data covariance times `factor²`.
    pub fn covariance(&self) -> &Grid<f64> {
        &self.covariance
    }

    /// The inverse kernel covariance.
    pub fn inv_cov(&self) -> &Grid<f64> {
        &self.inv_cov
    }

    /// The normalisation `√det(2π·covariance) · n`.
    pub fn norm_factor(&self) -> f64 {
        self.norm_factor
    }

    /// Evaluates a one-dimensional estimate at `points`.
    pub fn evaluate(&self, points: &[f64]) -> Result<Vec<f64>, StatsError> {
        self.evaluate_grid(&Grid::from_fn(1, points.len(), |_, i| points[i]))
    }

    /// Evaluates the estimate at the columns of `points` (`dim × m`).
    pub fn evaluate_grid(&self, points: &Grid<f64>) -> Result<Vec<f64>, StatsError> {
        let (dim, m) = points.shape();
        if dim != self.dim() {
            return Err(StatsError::DimensionMismatch {
                expected: self.dim(),
                actual: dim,
            });
        }
        let n = self.num_points();
        let kernel = |sample: usize, point: usize| {
            let diff = |r: usize| self.dataset[(r, sample)] - points[(r, point)];
            let energy = (0..dim).fold(0.0, |acc, r| {
                let t = (0..dim).fold(0.0, |s, c| s + self.inv_cov[(r, c)] * diff(c));
                acc + diff(r) * t
            }) / 2.0;
            (-energy).exp()
        };
        // matplotlib loops over whichever of samples and points is fewer,
        // which fixes the summation order.
        let sums: Vec<f64> = if m >= n {
            (0..m)
                .map(|j| (0..n).fold(0.0, |acc, i| acc + kernel(i, j)))
                .collect()
        } else {
            (0..m)
                .map(|j| pairwise_sum(&(0..n).map(|i| kernel(i, j)).collect::<Vec<_>>()))
                .collect()
        };
        Ok(sums.into_iter().map(|s| s / self.norm_factor).collect())
    }
}

/// `np.cov(dataset, rowvar=True, ddof=1)`: rows are variables.
fn sample_covariance(dataset: &Grid<f64>) -> Grid<f64> {
    let (dim, n) = dataset.shape();
    let centred: Vec<Vec<f64>> = dataset
        .iter_rows()
        .map(|row| {
            let m = mean(row);
            row.iter().map(|v| v - m).collect()
        })
        .collect();
    let scale = 1.0 / (n as f64 - 1.0);
    Grid::from_fn(dim, dim, |a, b| {
        centred[a]
            .iter()
            .zip(&centred[b])
            .fold(0.0, |acc, (x, y)| acc + x * y)
            * scale
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_dimensional_parameters() {
        let kde = GaussianKde::new(&[1.0, 2.0, 3.0, 4.0], BandwidthMethod::Factor(0.5)).unwrap();
        assert_eq!((kde.dim(), kde.num_points(), kde.factor()), (1, 4, 0.5));
        let var = 5.0 / 3.0;
        assert!((kde.covariance()[(0, 0)] - var * 0.25).abs() < 1e-15);
        assert!((kde.inv_cov()[(0, 0)] - 4.0 / var).abs() < 1e-14);
        let expected_norm = (2.0 * PI * var * 0.25).sqrt() * 4.0;
        assert!((kde.norm_factor() - expected_norm).abs() < 1e-14);
        let scott = GaussianKde::new(&[1.0, 2.0, 4.0], BandwidthMethod::Scott).unwrap();
        assert_eq!(scott.factor(), 3_f64.powf(-0.2));
        let silverman = GaussianKde::new(&[1.0, 2.0, 4.0], BandwidthMethod::Silverman).unwrap();
        assert_eq!(silverman.factor(), 2.25_f64.powf(-0.2));
    }

    #[test]
    fn density_integrates_to_one() {
        let kde = GaussianKde::new(&[0.0, 1.0, 3.0], BandwidthMethod::Scott).unwrap();
        let xs: Vec<f64> = (0..4000).map(|i| -20.0 + f64::from(i) * 0.01).collect();
        let total: f64 = kde.evaluate(&xs).unwrap().iter().sum::<f64>() * 0.01;
        assert!((total - 1.0).abs() < 1e-9);
        // Fewer points than samples takes the other summation branch.
        let few = kde.evaluate(&[1.0]).unwrap()[0];
        let many = kde.evaluate(&[1.0, 1.0, 1.0, 1.0]).unwrap()[0];
        assert!((few - many).abs() < 1e-16);
    }

    #[test]
    fn errors() {
        assert_eq!(
            GaussianKde::new(&[1.0], BandwidthMethod::Scott),
            Err(StatsError::TooFewSamples)
        );
        assert_eq!(
            GaussianKde::new(&[2.0, 2.0], BandwidthMethod::Scott),
            Err(StatsError::SingularCovariance)
        );
        let collinear = Grid::from_rows([[1.0, 2.0, 3.0], [2.0, 4.0, 6.0]]).unwrap();
        assert_eq!(
            GaussianKde::from_grid(collinear, BandwidthMethod::Scott),
            Err(StatsError::SingularCovariance)
        );
        let kde = GaussianKde::new(&[1.0, 2.0], BandwidthMethod::Scott).unwrap();
        assert_eq!(
            kde.evaluate_grid(&Grid::filled(2, 1, 0.0)),
            Err(StatsError::DimensionMismatch {
                expected: 1,
                actual: 2
            })
        );
        assert_eq!(kde.evaluate(&[]), Ok(Vec::new()));
    }

    #[test]
    fn two_dimensional_estimate_is_symmetric() {
        let data = Grid::from_rows([[0.0, 1.0, 0.0, -1.0], [1.0, 0.0, -1.0, 0.0]]).unwrap();
        let kde = GaussianKde::from_grid(data, BandwidthMethod::Scott).unwrap();
        let pts = Grid::from_rows([[0.5, -0.5], [0.0, 0.0]]).unwrap();
        let v = kde.evaluate_grid(&pts).unwrap();
        assert!((v[0] - v[1]).abs() < 1e-15);
    }
}
