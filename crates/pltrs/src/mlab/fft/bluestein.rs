//! Bluestein's algorithm as pocketfft runs it (`fftblue`): a transform of
//! a length with a large prime factor as a convolution of a smooth length
//! (`good_size_cmplx(2n - 1)`), with pocketfft's chirp and its order of
//! operations.

use super::super::Complex;
use super::cfftp::Cfftp;
use super::radix::special_mul;
use super::twiddle::UnitRoots;

/// A planned Bluestein transform of one length.
#[derive(Clone, Debug)]
pub(super) struct Bluestein {
    n: usize,
    n2: usize,
    plan: Cfftp,
    /// The chirp `bk`: `exp(iπ·m²/n)`.
    chirp: Vec<Complex>,
    /// The first `n2/2 + 1` values of the transformed, normalised,
    /// zero-padded chirp (`bkf`).
    kernel: Vec<Complex>,
}

impl Bluestein {
    /// Plans transforms of `n` points, convolving at length `n2`.
    pub(super) fn new(n: usize, n2: usize) -> Self {
        let plan = Cfftp::new(n2);
        let roots = UnitRoots::new(2 * n);
        let mut chirp = Vec::with_capacity(n);
        chirp.push(Complex::ONE);
        let mut coeff = 0;
        for m in 1..n {
            coeff += 2 * m - 1;
            if coeff >= 2 * n {
                coeff -= 2 * n;
            }
            chirp.push(roots.get(coeff));
        }
        let scale = 1.0 / n2 as f64;
        let mut padded = vec![Complex::ZERO; n2];
        padded[0] = chirp[0] * scale;
        for m in 1..n {
            let value = chirp[m] * scale;
            padded[m] = value;
            padded[n2 - m] = value;
        }
        plan.exec::<true>(&mut padded, 1.0);
        padded.truncate(n2 / 2 + 1);
        Self {
            n,
            n2,
            plan,
            chirp,
            kernel: padded,
        }
    }

    /// Transforms `c` in place and scales it by `fct` (pocketfft's
    /// `fftblue::fft`).
    pub(super) fn exec<const FWD: bool>(&self, c: &mut [Complex], fct: f64) {
        let (n, n2) = (self.n, self.n2);
        let mut akf = vec![Complex::ZERO; n2];
        for m in 0..n {
            akf[m] = special_mul::<FWD>(c[m], self.chirp[m]);
        }
        let zero = akf[0] * 0.0;
        for slot in &mut akf[n..] {
            *slot = zero;
        }
        self.plan.exec::<true>(&mut akf, 1.0);
        let convolve = |value: Complex, kernel: Complex| {
            if FWD {
                special_mul::<false>(value, kernel)
            } else {
                special_mul::<true>(value, kernel)
            }
        };
        akf[0] = convolve(akf[0], self.kernel[0]);
        for m in 1..n2.div_ceil(2) {
            akf[m] = convolve(akf[m], self.kernel[m]);
            akf[n2 - m] = convolve(akf[n2 - m], self.kernel[m]);
        }
        if n2.is_multiple_of(2) {
            akf[n2 / 2] = convolve(akf[n2 / 2], self.kernel[n2 / 2]);
        }
        self.plan.exec::<false>(&mut akf, 1.0);
        for (out, (&value, &chirp)) in c.iter_mut().zip(akf.iter().zip(&self.chirp)) {
            *out = special_mul::<FWD>(value, chirp) * fct;
        }
    }
}
