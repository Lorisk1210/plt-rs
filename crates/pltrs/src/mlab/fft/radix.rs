//! The radix-2, 3, 4, 5, 7, 8 and 11 passes of pocketfft's complex
//! FFTPACK transform (`cfftp::pass2` ... `pass11`), with its operations
//! in its order, so that every value rounds as numpy's.
//!
//! A pass combines `radix` interleaved sub-transforms of `ido` points,
//! for `l1` blocks: `CC(a, b, c)` is point `a` of sub-transform `b` of
//! block `c`, `CH(a, b, c)` point `a` of block `b` of output `c`, and the
//! twiddle factors `WA(x, i)` rotate output `x + 1` at point `i`.

use super::super::Complex;

/// The shape of one pass and its twiddle factors.
#[derive(Clone, Copy, Debug)]
pub(super) struct Layout<'a> {
    /// Points per sub-transform.
    pub(super) ido: usize,
    /// Blocks.
    pub(super) l1: usize,
    /// Sub-transforms combined.
    pub(super) radix: usize,
    /// `WA`, `(radix - 1) · (ido - 1)` factors.
    pub(super) twiddles: &'a [Complex],
}

impl Layout<'_> {
    /// The index of `CC(a, b, c)`.
    pub(super) fn input(&self, a: usize, b: usize, c: usize) -> usize {
        a + self.ido * (b + self.radix * c)
    }

    /// The index of `CH(a, b, c)`.
    pub(super) fn output(&self, a: usize, b: usize, c: usize) -> usize {
        a + self.ido * (b + self.l1 * c)
    }

    /// `WA(x, i)`.
    pub(super) fn twiddle(&self, x: usize, i: usize) -> Complex {
        self.twiddles[i - 1 + x * (self.ido - 1)]
    }

    /// Output `u` at point `i`: rotated by its twiddle factor except for
    /// the first output and at the first point, which pocketfft leaves
    /// alone.
    fn twiddled<const FWD: bool>(&self, value: Complex, u: usize, i: usize) -> Complex {
        if u == 0 || i == 0 {
            value
        } else {
            special_mul::<FWD>(value, self.twiddle(u - 1, i))
        }
    }
}

/// `v·w` for the backward transform, `v·conj(w)` for the forward one
/// (pocketfft's `special_mul`).
pub(super) fn special_mul<const FWD: bool>(v: Complex, w: Complex) -> Complex {
    if FWD {
        Complex::new(v.re * w.re + v.im * w.im, v.im * w.re - v.re * w.im)
    } else {
        Complex::new(v.re * w.re - v.im * w.im, v.re * w.im + v.im * w.re)
    }
}

/// `(c + d, c - d)` (pocketfft's `PM`).
pub(super) fn pm(c: Complex, d: Complex) -> (Complex, Complex) {
    (c + d, c - d)
}

/// A quarter turn: by `-i` forward, by `i` backward (`ROTX90`).
fn rotx90<const FWD: bool>(a: Complex) -> Complex {
    if FWD {
        Complex::new(a.im, -a.re)
    } else {
        Complex::new(-a.im, a.re)
    }
}

/// `√½` (pocketfft's `hsqt2`, a `long double` literal that rounds to this
/// double).
const HALF_SQRT2: f64 = std::f64::consts::FRAC_1_SQRT_2;

/// An eighth of a turn (`ROTX45`).
fn rotx45<const FWD: bool>(a: Complex) -> Complex {
    if FWD {
        Complex::new(HALF_SQRT2 * (a.re + a.im), HALF_SQRT2 * (a.im - a.re))
    } else {
        Complex::new(HALF_SQRT2 * (a.re - a.im), HALF_SQRT2 * (a.im + a.re))
    }
}

/// Three eighths of a turn (`ROTX135`).
fn rotx135<const FWD: bool>(a: Complex) -> Complex {
    if FWD {
        Complex::new(HALF_SQRT2 * (a.im - a.re), HALF_SQRT2 * (-a.re - a.im))
    } else {
        Complex::new(HALF_SQRT2 * (-a.re - a.im), HALF_SQRT2 * (a.re - a.im))
    }
}

/// pocketfft's `pass2`.
pub(super) fn pass2<const FWD: bool>(p: Layout<'_>, cc: &[Complex], ch: &mut [Complex]) {
    for k in 0..p.l1 {
        for i in 0..p.ido {
            let (sum, difference) = pm(cc[p.input(i, 0, k)], cc[p.input(i, 1, k)]);
            ch[p.output(i, k, 0)] = sum;
            ch[p.output(i, k, 1)] = p.twiddled::<FWD>(difference, 1, i);
        }
    }
}

/// pocketfft's `pass4`.
pub(super) fn pass4<const FWD: bool>(p: Layout<'_>, cc: &[Complex], ch: &mut [Complex]) {
    for k in 0..p.l1 {
        for i in 0..p.ido {
            let (t2, t1) = pm(cc[p.input(i, 0, k)], cc[p.input(i, 2, k)]);
            let (t3, t4) = pm(cc[p.input(i, 1, k)], cc[p.input(i, 3, k)]);
            let t4 = rotx90::<FWD>(t4);
            ch[p.output(i, k, 0)] = t2 + t3;
            ch[p.output(i, k, 1)] = p.twiddled::<FWD>(t1 + t4, 1, i);
            ch[p.output(i, k, 2)] = p.twiddled::<FWD>(t2 - t3, 2, i);
            ch[p.output(i, k, 3)] = p.twiddled::<FWD>(t1 - t4, 3, i);
        }
    }
}

/// pocketfft's `pass8`.
pub(super) fn pass8<const FWD: bool>(p: Layout<'_>, cc: &[Complex], ch: &mut [Complex]) {
    for k in 0..p.l1 {
        for i in 0..p.ido {
            let at = |b: usize| cc[p.input(i, b, k)];
            let (a1, a5) = pm(at(1), at(5));
            let (a3, a7) = pm(at(3), at(7));
            let (a1, a3) = pm(a1, a3);
            let a3 = rotx90::<FWD>(a3);
            let (a5, a7) = pm(a5, rotx90::<FWD>(a7));
            let a5 = rotx45::<FWD>(a5);
            let a7 = rotx135::<FWD>(a7);
            let (a0, a4) = pm(at(0), at(4));
            let (a2, a6) = pm(at(2), at(6));
            let (a0, a2) = pm(a0, a2);
            let (a4, a6) = pm(a4, rotx90::<FWD>(a6));
            let outputs = [
                (0, a0 + a1),
                (4, a0 - a1),
                (2, a2 + a3),
                (6, a2 - a3),
                (1, a4 + a5),
                (5, a4 - a5),
                (3, a6 + a7),
                (7, a6 - a7),
            ];
            for (u, value) in outputs {
                ch[p.output(i, k, u)] = p.twiddled::<FWD>(value, u, i);
            }
        }
    }
}

/// The cosines and sines of `2π·j/radix` for `j = 1, ..., (radix-1)/2`
/// of the odd radices pocketfft hard-codes: the doubles its 34-digit
/// `long double` literals round to (through x87 extended precision, which
/// rounds none of them otherwise), written here in their shortest form.
pub(super) fn odd_radix_roots(radix: usize) -> Option<(&'static [f64], &'static [f64])> {
    Some(match radix {
        3 => (&[-0.5], &[0.866_025_403_784_438_6]),
        5 => (
            &[0.309_016_994_374_947_45, -0.809_016_994_374_947_5],
            &[0.951_056_516_295_153_5, 0.587_785_252_292_473_1],
        ),
        7 => (
            &[
                0.623_489_801_858_733_5,
                -0.222_520_933_956_314_4,
                -0.900_968_867_902_419_1,
            ],
            &[
                0.781_831_482_468_029_8,
                0.974_927_912_181_823_6,
                0.433_883_739_117_558_1,
            ],
        ),
        11 => (
            &[
                0.841_253_532_831_181_2,
                0.415_415_013_001_886_44,
                -0.142_314_838_273_285_14,
                -0.654_860_733_945_285_1,
                -0.959_492_973_614_497_4,
            ],
            &[
                0.540_640_817_455_597_6,
                0.909_631_995_354_518_3,
                0.989_821_441_880_932_7,
                0.755_749_574_354_258_3,
                0.281_732_556_841_429_67,
            ],
        ),
        _ => return None,
    })
}

/// pocketfft's `pass3`, `pass5`, `pass7` and `pass11`, which share one
/// shape: the input pairs `j` and `radix - j` are added and subtracted,
/// and output pair `u`, `radix - u` takes the sums times the cosines and
/// the differences times the sines of `2π·u·j/radix`, accumulated in
/// `j` order.
pub(super) fn pass_odd<const FWD: bool>(
    p: Layout<'_>,
    (cosines, sines): (&[f64], &[f64]),
    cc: &[Complex],
    ch: &mut [Complex],
) {
    let half = p.radix / 2;
    let sines: Vec<f64> = sines.iter().map(|&s| if FWD { -s } else { s }).collect();
    let mut sums = vec![Complex::ZERO; half];
    let mut differences = vec![Complex::ZERO; half];
    for k in 0..p.l1 {
        for i in 0..p.ido {
            let t0 = cc[p.input(i, 0, k)];
            for j in 1..=half {
                let pair = pm(cc[p.input(i, j, k)], cc[p.input(i, p.radix - j, k)]);
                (sums[j - 1], differences[j - 1]) = pair;
            }
            ch[p.output(i, k, 0)] = sums.iter().fold(t0, |acc, &s| acc + s);
            for u in 1..=half {
                let mut ca = t0;
                let (mut real, mut imaginary) = (0.0, 0.0);
                for j in 1..=half {
                    let turn = u * j % p.radix;
                    let (index, sign) = if turn <= half {
                        (turn, 1.0)
                    } else {
                        (p.radix - turn, -1.0)
                    };
                    let (c, s) = (cosines[index - 1], sign * sines[index - 1]);
                    ca = Complex::new(ca.re + c * sums[j - 1].re, ca.im + c * sums[j - 1].im);
                    let difference = differences[j - 1];
                    if j == 1 {
                        (real, imaginary) = (s * difference.im, s * difference.re);
                    } else {
                        real += s * difference.im;
                        imaginary += s * difference.re;
                    }
                }
                let cb = Complex::new(-real, imaginary);
                ch[p.output(i, k, u)] = p.twiddled::<FWD>(ca + cb, u, i);
                ch[p.output(i, k, p.radix - u)] = p.twiddled::<FWD>(ca - cb, p.radix - u, i);
            }
        }
    }
}
