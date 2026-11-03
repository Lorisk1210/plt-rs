//! Discrete Fourier transforms of arbitrary length (numpy `fft`).
//!
//! A port of the complex transforms of pocketfft, the C++ library numpy
//! 2 computes `numpy.fft.fft` with: the FFTPACK passes for radices 2, 3,
//! 4, 5, 7, 8 and 11 and a generic one for larger primes ([`cfftp`]),
//! Bluestein's algorithm for lengths with a large prime factor
//! ([`bluestein`]), pocketfft's choice between the two, and its twiddle
//! factors ([`twiddle`]), all with pocketfft's operations in its order. The
//! results are numpy's to the last bit.

mod bluestein;
mod cfftp;
mod generic;
mod radix;
mod twiddle;

use super::Complex;
use bluestein::Bluestein;
use cfftp::Cfftp;

/// A precomputed transform for one length; reuse it for many transforms of
/// that length (e.g. the segments of a spectrogram).
#[derive(Clone, Debug)]
pub struct FftPlan {
    len: usize,
    algorithm: Algorithm,
}

/// How a length is transformed (pocketfft's `pocketfft_c`).
#[derive(Clone, Debug)]
enum Algorithm {
    /// Length 0: there is nothing to transform.
    Empty,
    /// The FFTPACK passes.
    Packed(Cfftp),
    /// Bluestein's convolution.
    Bluestein(Box<Bluestein>),
}

impl FftPlan {
    /// Plans transforms of length `len`, choosing as pocketfft does:
    /// FFTPACK unless the length has a prime factor above its square root
    /// and Bluestein's convolution is estimated to be cheaper.
    pub fn new(len: usize) -> Self {
        let algorithm = if len == 0 {
            Algorithm::Empty
        } else {
            let largest = if len < 50 {
                0
            } else {
                largest_prime_factor(len)
            };
            let convolution = (largest * largest > len)
                .then(|| good_size_cmplx(2 * len - 1))
                .filter(|&n2| 2.0 * cost_guess(n2) * 1.5 < cost_guess(len));
            match convolution {
                Some(n2) => Algorithm::Bluestein(Box::new(Bluestein::new(len, n2))),
                None => Algorithm::Packed(Cfftp::new(len)),
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
        let mut data = resized(input, self.len);
        self.exec::<true>(&mut data, 1.0);
        data
    }

    /// Inverse transform `x[j] = (1/n)·Σ X[k]·exp(2πi·jk/n)` (numpy
    /// `ifft`: the backward transform scaled by `1/n`), with the same
    /// truncation/padding rule as [`Self::forward`].
    pub fn inverse(&self, input: &[Complex]) -> Vec<Complex> {
        let mut data = resized(input, self.len);
        self.exec::<false>(&mut data, 1.0 / self.len as f64);
        data
    }

    fn exec<const FWD: bool>(&self, data: &mut [Complex], fct: f64) {
        match &self.algorithm {
            Algorithm::Empty => {}
            Algorithm::Packed(plan) => plan.exec::<FWD>(data, fct),
            Algorithm::Bluestein(plan) => plan.exec::<FWD>(data, fct),
        }
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

/// The largest prime factor of `n` (pocketfft's `largest_prime_factor`).
fn largest_prime_factor(mut n: usize) -> usize {
    let mut largest = 1;
    while n.is_multiple_of(2) {
        largest = 2;
        n /= 2;
    }
    let mut x = 3;
    while x * x <= n {
        while n.is_multiple_of(x) {
            largest = x;
            n /= x;
        }
        x += 2;
    }
    if n > 1 { n } else { largest }
}

/// pocketfft's estimate of the work of an FFTPACK transform of `n`
/// points (`cost_guess`): the sum of its prime factors, those above 5
/// weighed 1.1, times `n`.
fn cost_guess(n: usize) -> f64 {
    let weight = |factor: usize| {
        if factor <= 5 {
            factor as f64
        } else {
            1.1 * factor as f64
        }
    };
    let mut rest = n;
    let mut result = 0.0;
    while rest.is_multiple_of(2) {
        result += 2.0;
        rest /= 2;
    }
    let mut x = 3;
    while x * x <= rest {
        while rest.is_multiple_of(x) {
            result += weight(x);
            rest /= x;
        }
        x += 2;
    }
    if rest > 1 {
        result += weight(rest);
    }
    result * n as f64
}

/// The smallest product of 2, 3, 5, 7 and 11 that is at least `n`
/// (pocketfft's `good_size_cmplx`).
fn good_size_cmplx(n: usize) -> usize {
    if n <= 12 {
        return n;
    }
    let mut best = 2 * n;
    let mut f11 = 1;
    while f11 < best {
        let mut f117 = f11;
        while f117 < best {
            let mut f1175 = f117;
            while f1175 < best {
                let mut x = f1175;
                while x < n {
                    x *= 2;
                }
                loop {
                    match x.cmp(&n) {
                        std::cmp::Ordering::Less => x *= 3,
                        std::cmp::Ordering::Greater => {
                            best = best.min(x);
                            if x % 2 == 1 {
                                break;
                            }
                            x /= 2;
                        }
                        std::cmp::Ordering::Equal => return n,
                    }
                }
                f1175 *= 5;
            }
            f117 *= 7;
        }
        f11 *= 11;
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The test signal of the reference values below, exact in binary64
    /// (numpy computes the same input as `complex(((j*j) % 17) / 7.0,
    /// ((3*j) % 11) / 5.0)`).
    fn signal(n: usize) -> Vec<Complex> {
        (0..n)
            .map(|j| Complex::new(((j * j) % 17) as f64 / 7.0, ((3 * j) % 11) as f64 / 5.0))
            .collect()
    }

    fn naive_dft(x: &[Complex]) -> Vec<Complex> {
        let n = x.len();
        (0..n)
            .map(|k| {
                x.iter()
                    .enumerate()
                    .map(|(j, &v)| {
                        let angle = -std::f64::consts::TAU * (j * k % n) as f64 / n as f64;
                        v * Complex::new(angle.cos(), angle.sin())
                    })
                    .sum()
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

    /// Values expected at some indices, as the bits of their parts.
    type Bits = [(usize, u64, u64)];

    /// Asserts that the values at some indices are numpy's: to the bit
    /// with glibc's `sin` and `cos`, as numpy computed them (on Linux), and
    /// within 1e-13 of the transform's scale with other C libraries, which
    /// round some twiddle factors otherwise (Windows' UCRT, Wine's).
    fn assert_bits(values: &[Complex], expected: &Bits) {
        let scale = values.iter().map(|z| z.abs()).fold(1.0, f64::max);
        for &(index, re, im) in expected {
            let value = values[index];
            if cfg!(all(target_os = "linux", target_env = "gnu")) {
                assert_eq!(
                    (value.re.to_bits(), value.im.to_bits()),
                    (re, im),
                    "index {index}: {value}"
                );
            } else {
                let numpys = Complex::new(f64::from_bits(re), f64::from_bits(im));
                assert!(
                    (value - numpys).abs() <= 1e-13 * scale,
                    "index {index}: {value} vs {numpys}"
                );
            }
        }
    }

    #[test]
    fn matches_naive_dft_for_many_lengths() {
        for n in [
            1, 2, 3, 4, 5, 6, 7, 8, 11, 12, 13, 15, 16, 20, 22, 26, 31, 33, 48, 63, 64, 67, 81, 97,
            100, 128, 134, 1009,
        ] {
            let x = signal(n);
            assert_close(&fft(&x), &naive_dft(&x), 1e-12);
        }
    }

    #[test]
    fn transforms_are_numpys_to_the_bit() {
        // numpy 2.5's `numpy.fft.fft` of `signal(n)`.
        let cases: [(usize, &Bits); 6] = [
            (
                8,
                &[
                    (0, 0x401F_6DB6_DB6D_B6DC, 0x401F_FFFF_FFFF_FFFF),
                    (1, 0xC006_F4E2_4C14_ECB0, 0x3FEA_7ABF_C5AA_765C),
                    (2, 0xBFEF_15F1_5F15_F160, 0xBFD0_7507_5075_0740),
                    (7, 0xBFFE_7652_08C3_C174, 0xBFF3_A3C6_493B_A196),
                ],
            ),
            (
                12,
                &[
                    (1, 0xC00B_F3A8_6611_CD3C, 0xBFB2_998B_CA3B_37C8),
                    (4, 0xBFC0_0304_2AF4_7610, 0xBFFA_CA2B_ECD5_E9E2),
                    (11, 0xC00B_A350_5390_E96E, 0xC011_59F7_03D4_707E),
                ],
            ),
            (
                30,
                &[
                    (1, 0x3FF6_5B34_2B93_7244, 0xBFFB_E341_BBB0_7B82),
                    (10, 0x3FF0_5058_1280_E3CC, 0xBFFF_20D1_3BE1_853D),
                    (29, 0x3FEA_77BB_F523_5420, 0x3FFC_D8B5_E661_1386),
                ],
            ),
            (
                77,
                &[
                    (0, 0x4055_DB6D_B6DB_6DB7, 0x4053_4000_0000_0000),
                    (25, 0xBFFD_F47C_EB55_133D, 0xBFE9_18BA_CDA3_9C04),
                    (76, 0xBFE1_C1FC_597B_E004, 0x3FEC_8518_8768_9D98),
                ],
            ),
            (
                26,
                &[
                    (1, 0xBFB8_47B1_7260_4D40, 0xC013_0436_CB40_23CE),
                    (8, 0x3FD5_3740_0B59_E4C0, 0xBFFC_1E3B_4975_EB57),
                    (25, 0xBFE2_1F14_D40A_3F84, 0x400E_014D_EB36_E29B),
                ],
            ),
            (
                1009,
                &[
                    (1, 0xBFF6_BE62_4412_4C63, 0xBFA9_8066_E287_136B),
                    (336, 0xBFCE_AEAC_BCD0_00E2, 0x3FD2_11BB_5A96_C0E1),
                    (1008, 0xBFF6_FB9B_0007_8680, 0x3FA9_8680_6E5F_5B5C),
                ],
            ),
        ];
        for (n, expected) in cases {
            assert_bits(&fft(&signal(n)), expected);
        }
    }

    #[test]
    fn inverse_transforms_are_numpys_to_the_bit() {
        // numpy 2.5's `numpy.fft.ifft` of `signal(n)`.
        assert_bits(
            &ifft(&signal(12)),
            &[
                (1, 0xBFD2_6CE0_37B5_F0F4, 0xBFD7_229E_AFC5_EB52),
                (11, 0xBFD2_A270_440B_DE27, 0xBF78_CCBA_62F9_9FAA),
            ],
        );
        assert_bits(
            &ifft(&signal(1009)),
            &[
                (1, 0xBF57_5312_857C_1873, 0x3F09_E7A5_197E_F65F),
                (1008, 0xBF57_14F0_CB0B_B920, 0xBF09_E174_56CC_701C),
            ],
        );
    }

    #[test]
    fn inverse_round_trips() {
        for n in [1, 7, 16, 67, 90, 1009] {
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
    fn plans_choose_as_pocketfft() {
        assert!(matches!(FftPlan::new(97).algorithm, Algorithm::Packed(_)));
        assert!(matches!(
            FftPlan::new(1009).algorithm,
            Algorithm::Bluestein(_)
        ));
        assert!(matches!(
            FftPlan::new(16_384).algorithm,
            Algorithm::Packed(_)
        ));
        assert_eq!(largest_prime_factor(11_900), 17);
        assert_eq!(good_size_cmplx(2017), 2025);
        assert_eq!(good_size_cmplx(12), 12);
        assert_eq!(cost_guess(105), (3.0 + 5.0 + 1.1 * 7.0) * 105.0);
    }

    #[test]
    fn frequencies_match_numpy() {
        assert_eq!(fftfreq(4, 0.5), vec![0.0, 0.5, -1.0, -0.5]);
        assert_eq!(fftfreq(5, 1.0), vec![0.0, 0.2, 0.4, -0.4, -0.2]);
        assert_eq!(fftfreq(0, 1.0), Vec::<f64>::new());
    }
}
