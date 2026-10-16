//! Discrete Fourier transforms of arbitrary length (numpy `fft`).
//!
//! Lengths whose prime factors are all small use a recursive mixed-radix
//! Cooley–Tukey decimation in time with specialised radix-2 and radix-4
//! butterflies; any other length is handled by Bluestein's chirp-z
//! algorithm on a power-of-two convolution. Twiddle factors are evaluated
//! directly (with octant symmetry, so quarter turns are exact) rather than
//! by recurrence, which keeps the result within a few ulps of numpy's
//! pocketfft.

use std::f64::consts::FRAC_PI_2;

use super::Complex;

/// Prime factors larger than this make the plan fall back to Bluestein.
const MAX_DIRECT_PRIME: usize = 64;

/// A precomputed transform for one length; reuse it for many transforms of
/// that length (e.g. the segments of a spectrogram).
#[derive(Clone, Debug)]
pub struct FftPlan {
    len: usize,
    algorithm: Algorithm,
}

#[derive(Clone, Debug)]
enum Algorithm {
    /// Length 0 or 1: the transform is the identity.
    Identity,
    MixedRadix(MixedRadix),
    Bluestein(Box<Bluestein>),
}

impl FftPlan {
    /// Plans transforms of length `len`.
    pub fn new(len: usize) -> Self {
        let algorithm = if len <= 1 {
            Algorithm::Identity
        } else {
            match factorize(len) {
                Some(factors) => Algorithm::MixedRadix(MixedRadix::new(len, factors)),
                None => Algorithm::Bluestein(Box::new(Bluestein::new(len))),
            }
        };
        Self { len, algorithm }
    }

    /// The transform length.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// `true` for the zero-length plan.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Forward transform `X[k] = Σ x[j]·exp(-2πi·jk/n)`.
    ///
    /// Like numpy's `fft(x, n)`, the input is truncated or zero-padded to
    /// the plan length.
    pub fn forward(&self, input: &[Complex]) -> Vec<Complex> {
        let data = resized(input, self.len);
        match &self.algorithm {
            Algorithm::Identity => data,
            Algorithm::MixedRadix(plan) => plan.transform(&data),
            Algorithm::Bluestein(plan) => plan.transform(&data),
        }
    }

    /// Inverse transform `x[j] = (1/n)·Σ X[k]·exp(2πi·jk/n)` (numpy
    /// `ifft`), with the same truncation/padding rule as [`Self::forward`].
    pub fn inverse(&self, input: &[Complex]) -> Vec<Complex> {
        let conjugated: Vec<Complex> = resized(input, self.len)
            .into_iter()
            .map(Complex::conj)
            .collect();
        let scale = self.len as f64;
        self.forward(&conjugated)
            .into_iter()
            .map(|z| z.conj().unscale(scale))
            .collect()
    }
}

/// Forward DFT of `x` at its own length.
pub fn fft(x: &[Complex]) -> Vec<Complex> {
    FftPlan::new(x.len()).forward(x)
}

/// Forward DFT of `x` truncated or zero-padded to length `n` (numpy
/// `fft(x, n)`).
pub fn fft_n(x: &[Complex], n: usize) -> Vec<Complex> {
    FftPlan::new(n).forward(x)
}

/// Inverse DFT of `x` at its own length, normalised by `1/n`.
pub fn ifft(x: &[Complex]) -> Vec<Complex> {
    FftPlan::new(x.len()).inverse(x)
}

/// Sample frequencies of an `n`-point DFT with sample spacing `d` (numpy
/// `fftfreq`): `[0, 1, ..., ⌈n/2⌉-1, -⌊n/2⌋, ..., -1] / (n·d)`.
pub fn fftfreq(n: usize, d: f64) -> Vec<f64> {
    let val = 1.0 / (n as f64 * d);
    let positive = (n.saturating_sub(1)) / 2 + 1;
    (0..n)
        .map(|i| {
            let k = if i < positive {
                i as f64
            } else {
                -((n - i) as f64)
            };
            k * val
        })
        .collect()
}

fn resized(input: &[Complex], len: usize) -> Vec<Complex> {
    let mut data: Vec<Complex> = input.iter().take(len).copied().collect();
    data.resize(len, Complex::ZERO);
    data
}

/// Splits `n` into radices (4s first, then 2, then odd primes ascending),
/// or `None` when a prime factor exceeds [`MAX_DIRECT_PRIME`].
fn factorize(n: usize) -> Option<Vec<usize>> {
    let mut factors = Vec::new();
    let mut rest = n;
    while rest.is_multiple_of(4) {
        factors.push(4);
        rest /= 4;
    }
    if rest.is_multiple_of(2) {
        factors.push(2);
        rest /= 2;
    }
    let mut p = 3;
    while p * p <= rest {
        while rest.is_multiple_of(p) {
            factors.push(p);
            rest /= p;
        }
        p += 2;
    }
    if rest > 1 {
        factors.push(rest);
    }
    factors
        .iter()
        .all(|&f| f <= MAX_DIRECT_PRIME)
        .then_some(factors)
}

/// `exp(-2πi·j/n)`, evaluated by reduction to the first octant so that
/// multiples of a quarter turn are exact.
fn unit_root(j: usize, n: usize) -> Complex {
    let n_wide = n as u128;
    let four_j = 4 * (j as u128 % n_wide);
    let quadrant = four_j / n_wide;
    let r = four_j % n_wide;
    let (sin, cos) = if 2 * r <= n_wide {
        let phi = FRAC_PI_2 * (r as f64 / n as f64);
        (phi.sin(), phi.cos())
    } else {
        let phi = FRAC_PI_2 * ((n_wide - r) as f64 / n as f64);
        (phi.cos(), phi.sin())
    };
    let (cos_t, sin_t) = match quadrant {
        0 => (cos, sin),
        1 => (-sin, cos),
        2 => (-cos, -sin),
        _ => (sin, -cos),
    };
    Complex::new(cos_t, -sin_t)
}

/// Recursive mixed-radix decimation in time.
#[derive(Clone, Debug)]
struct MixedRadix {
    factors: Vec<usize>,
    /// `exp(-2πi·j/N)` for `j < N`.
    twiddles: Vec<Complex>,
}

impl MixedRadix {
    fn new(len: usize, factors: Vec<usize>) -> Self {
        let twiddles = (0..len).map(|j| unit_root(j, len)).collect();
        Self { factors, twiddles }
    }

    fn transform(&self, input: &[Complex]) -> Vec<Complex> {
        let mut out = vec![Complex::ZERO; input.len()];
        let widest = self.factors.iter().copied().max().unwrap_or(1);
        let mut scratch = vec![Complex::ZERO; widest];
        self.pass(input, 1, &mut out, &self.factors, 1, &mut scratch);
        out
    }

    /// Transforms the `out.len()` samples `input[0], input[stride], ...`
    /// into `out`; `tw_stride = N / out.len()` maps local twiddle indices
    /// onto the full-length table.
    fn pass(
        &self,
        input: &[Complex],
        stride: usize,
        out: &mut [Complex],
        factors: &[usize],
        tw_stride: usize,
        scratch: &mut [Complex],
    ) {
        let Some((&p, rest)) = factors.split_first() else {
            out[0] = input[0];
            return;
        };
        let m = out.len() / p;
        for (r, chunk) in out.chunks_exact_mut(m).enumerate() {
            self.pass(
                &input[r * stride..],
                stride * p,
                chunk,
                rest,
                tw_stride * p,
                scratch,
            );
        }
        match p {
            2 => self.radix2(out, m, tw_stride),
            4 => self.radix4(out, m, tw_stride),
            _ => self.radix_generic(out, p, m, tw_stride, scratch),
        }
    }

    fn twiddled(&self, value: Complex, index: usize) -> Complex {
        if index == 0 {
            value
        } else {
            value * self.twiddles[index]
        }
    }

    fn radix2(&self, out: &mut [Complex], m: usize, tw_stride: usize) {
        for k in 0..m {
            let t0 = out[k];
            let t1 = self.twiddled(out[m + k], k * tw_stride);
            out[k] = t0 + t1;
            out[m + k] = t0 - t1;
        }
    }

    fn radix4(&self, out: &mut [Complex], m: usize, tw_stride: usize) {
        for k in 0..m {
            let t0 = out[k];
            let t1 = self.twiddled(out[m + k], k * tw_stride);
            let t2 = self.twiddled(out[2 * m + k], 2 * k * tw_stride);
            let t3 = self.twiddled(out[3 * m + k], 3 * k * tw_stride);
            let a0 = t0 + t2;
            let a1 = t0 - t2;
            let a2 = t1 + t3;
            let d = t1 - t3;
            // (t1 - t3) · (-i)
            let a3 = Complex::new(d.im, -d.re);
            out[k] = a0 + a2;
            out[m + k] = a1 + a3;
            out[2 * m + k] = a0 - a2;
            out[3 * m + k] = a1 - a3;
        }
    }

    fn radix_generic(
        &self,
        out: &mut [Complex],
        p: usize,
        m: usize,
        tw_stride: usize,
        scratch: &mut [Complex],
    ) {
        let root_step = self.twiddles.len() / p;
        let terms = &mut scratch[..p];
        for k in 0..m {
            for (r, term) in terms.iter_mut().enumerate() {
                *term = self.twiddled(out[r * m + k], r * k * tw_stride);
            }
            for q in 0..p {
                out[q * m + k] = terms
                    .iter()
                    .enumerate()
                    .map(|(r, &t)| self.twiddled(t, (r * q % p) * root_step))
                    .sum();
            }
        }
    }
}

/// Bluestein's algorithm: a length-`n` DFT as a circular convolution of
/// power-of-two length.
#[derive(Clone, Debug)]
struct Bluestein {
    /// `exp(-iπ·k²/n)` for `k < n`.
    chirp: Vec<Complex>,
    /// Forward transform of the conjugate chirp kernel.
    kernel_spectrum: Vec<Complex>,
    inner: MixedRadix,
}

impl Bluestein {
    fn new(len: usize) -> Self {
        let conv_len = (2 * len - 1).next_power_of_two();
        let period = 2 * len as u128;
        let chirp: Vec<Complex> = (0..len)
            .map(|k| {
                let k = k as u128;
                let index = (k * k) % period;
                // `index < 2n` fits back into usize.
                unit_root(usize::try_from(index).unwrap_or(0), 2 * len)
            })
            .collect();
        let mut kernel = vec![Complex::ZERO; conv_len];
        kernel[0] = chirp[0].conj();
        for k in 1..len {
            kernel[k] = chirp[k].conj();
            kernel[conv_len - k] = chirp[k].conj();
        }
        let inner = MixedRadix::new(
            conv_len,
            factorize(conv_len).expect("powers of two factor into radices 2 and 4"),
        );
        let kernel_spectrum = inner.transform(&kernel);
        Self {
            chirp,
            kernel_spectrum,
            inner,
        }
    }

    fn transform(&self, input: &[Complex]) -> Vec<Complex> {
        let conv_len = self.kernel_spectrum.len();
        let mut a = vec![Complex::ZERO; conv_len];
        for ((slot, &x), &w) in a.iter_mut().zip(input).zip(&self.chirp) {
            *slot = x * w;
        }
        let spectrum: Vec<Complex> = self
            .inner
            .transform(&a)
            .into_iter()
            .zip(&self.kernel_spectrum)
            .map(|(x, &k)| (x * k).conj())
            .collect();
        let scale = conv_len as f64;
        self.inner
            .transform(&spectrum)
            .into_iter()
            .zip(&self.chirp)
            .map(|(c, &w)| c.conj().unscale(scale) * w)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn naive_dft(x: &[Complex]) -> Vec<Complex> {
        let n = x.len();
        (0..n)
            .map(|k| {
                x.iter()
                    .enumerate()
                    .map(|(j, &v)| v * unit_root(j * k % n, n))
                    .sum()
            })
            .collect()
    }

    fn signal(n: usize) -> Vec<Complex> {
        (0..n)
            .map(|i| {
                let t = i as f64;
                Complex::new((0.37 * t).sin() + 0.1 * t, (1.3 * t).cos() - 0.5)
            })
            .collect()
    }

    fn assert_close(a: &[Complex], b: &[Complex], tol: f64) {
        assert_eq!(a.len(), b.len());
        let scale = b.iter().map(|z| z.abs()).fold(1.0, f64::max);
        for (x, y) in a.iter().zip(b) {
            assert!((*x - *y).abs() <= tol * scale, "{x} vs {y}");
        }
    }

    #[test]
    fn matches_naive_dft_for_many_lengths() {
        for n in [
            1, 2, 3, 4, 5, 6, 7, 8, 12, 15, 16, 20, 31, 33, 48, 63, 64, 67, 81, 97, 100, 128, 134,
        ] {
            let x = signal(n);
            assert_close(&fft(&x), &naive_dft(&x), 1e-13);
        }
    }

    #[test]
    fn inverse_round_trips() {
        for n in [1, 7, 16, 67, 90] {
            let x = signal(n);
            assert_close(&ifft(&fft(&x)), &x, 1e-13);
        }
    }

    #[test]
    fn padding_truncation_and_empty() {
        let x = signal(5);
        assert_eq!(fft_n(&x, 8).len(), 8);
        assert_eq!(fft_n(&x, 3), fft(&x[..3]));
        assert_eq!(fft(&[]), Vec::<Complex>::new());
        assert!(FftPlan::new(0).is_empty());
        assert_eq!(FftPlan::new(9).len(), 9);
    }

    #[test]
    fn real_input_has_exact_real_dc_and_nyquist() {
        let x: Vec<Complex> = (0..16).map(|i| Complex::real(f64::from(i) * 0.3)).collect();
        let spectrum = fft(&x);
        assert_eq!(spectrum[0].im, 0.0);
        assert_eq!(spectrum[8].im, 0.0);
    }

    #[test]
    fn unit_roots_are_exact_at_quarter_turns() {
        assert_eq!(unit_root(0, 8), Complex::new(1.0, -0.0));
        assert_eq!(unit_root(2, 8), Complex::new(0.0, -1.0));
        assert_eq!(unit_root(4, 8), Complex::new(-1.0, -0.0));
        assert_eq!(unit_root(6, 8), Complex::new(-0.0, 1.0));
    }

    #[test]
    fn frequencies_match_numpy() {
        assert_eq!(fftfreq(4, 0.5), vec![0.0, 0.5, -1.0, -0.5]);
        assert_eq!(fftfreq(5, 1.0), vec![0.0, 0.2, 0.4, -0.4, -0.2]);
        assert_eq!(fftfreq(0, 1.0), Vec::<f64>::new());
    }
}
