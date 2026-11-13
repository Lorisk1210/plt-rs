//! numpy's own results, where numpy is in the process.
//!
//! On Linux with AVX-512 numpy computes `sinh`, `log`, `power` and their
//! kin with Intel's SVML, whose results differ from the C library's in the
//! last bit of some values (see [`CpuLevel`](super::CpuLevel)). pltrs
//! computes with the C library, which is numpy's at every other level, and
//! one bit is nothing a figure shows, except where matplotlib rounds the
//! result to something discrete: the decade a tick is rounded to, a
//! boundary lying on a pixel's centre. At those places pltrs asks for the
//! values through [`Ufunc::map`]: pltrs (Python) installs numpy's own
//! ufuncs behind it ([`evaluate_with`]), so that matplotlib's decision is
//! reproduced whatever numpy runs; pltrs (Rust) alone computes them with
//! the C library.

use std::sync::OnceLock;

/// A ufunc of numpy evaluated on `float64` arrays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ufunc {
    /// `np.arcsinh(x)`.
    Arcsinh,
    /// `np.sinh(x)`.
    Sinh,
    /// `np.log(x)`.
    Log,
    /// `np.log2(x)`.
    Log2,
    /// `np.log10(x)`.
    Log10,
    /// `np.exp(x)`.
    Exp,
    /// `np.exp2(x)`.
    Exp2,
    /// `np.power(base, x)` for a scalar `base`.
    PowerOf(f64),
}

/// Evaluates a ufunc on values in place, or returns `false` leaving them
/// as they are.
pub type Evaluator = fn(Ufunc, &mut [f64]) -> bool;

/// numpy's evaluator, where one is installed.
static NUMPY: OnceLock<Evaluator> = OnceLock::new();

/// Installs numpy's own evaluation of the ufuncs for this process; the
/// first evaluator installed stays.
pub fn evaluate_with(numpy: Evaluator) {
    NUMPY.get_or_init(|| numpy);
}

impl Ufunc {
    /// The ufunc of each of `values`: numpy's results where numpy
    /// evaluates them ([`evaluate_with`]), and `c_library` of each value
    /// otherwise (the caller's computation, kept as it is).
    ///
    /// ```
    /// use pltrs_core::numeric::Ufunc;
    /// assert_eq!(Ufunc::Log10.map(&[100.0, 1.0], f64::log10), [2.0, 0.0]);
    /// ```
    pub fn map(self, values: &[f64], c_library: impl Fn(f64) -> f64) -> Vec<f64> {
        let mut results = values.to_vec();
        let evaluated =
            !values.is_empty() && NUMPY.get().is_some_and(|numpy| numpy(self, &mut results));
        if !evaluated {
            for (result, &value) in results.iter_mut().zip(values) {
                *result = c_library(value);
            }
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_c_library_computes_without_numpy() {
        // No evaluator is installed in this crate's tests.
        assert_eq!(
            Ufunc::Sinh.map(&[0.0, 1.0], f64::sinh),
            [0.0, 1.0_f64.sinh()]
        );
        assert_eq!(Ufunc::PowerOf(2.0).map(&[3.0], |x| 2f64.powf(x)), [8.0]);
        assert_eq!(Ufunc::Log.map(&[], f64::ln), Vec::<f64>::new());
        let nan = Ufunc::Arcsinh.map(&[f64::NAN], f64::asinh);
        assert!(nan[0].is_nan());
    }
}
