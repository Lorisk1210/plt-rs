//! The roots of unity of an FFT, as pocketfft computes them
//! (`sincos_2pibyn`): exact values at the octant boundaries, `cos` and
//! `sin` of small angles in between, and a product of two table entries
//! for any root, so that numpy's twiddle factors come out to the last bit.

use super::super::Complex;

/// `exp(2πi·k/n)` for every `k < n`, from two tables of `√n` entries
/// each (pocketfft's `sincos_2pibyn<double>`).
#[derive(Clone, Debug)]
pub(super) struct UnitRoots {
    n: usize,
    mask: usize,
    shift: u32,
    fine: Vec<Complex>,
    coarse: Vec<Complex>,
}

impl UnitRoots {
    /// The roots of unity of order `n`.
    pub(super) fn new(n: usize) -> Self {
        let angle = quarter_pi_over(n);
        let count = n / 2 + 1;
        let mut shift = 1_u32;
        while (1_usize << shift) * (1_usize << shift) < count {
            shift += 1;
        }
        let mask = (1_usize << shift) - 1;
        let fine = (0..=mask)
            .map(|i| {
                if i == 0 {
                    Complex::ONE
                } else {
                    octant_root(i, n, angle)
                }
            })
            .collect();
        let coarse = (0..count.div_ceil(mask + 1))
            .map(|i| {
                if i == 0 {
                    Complex::ONE
                } else {
                    octant_root(i * (mask + 1), n, angle)
                }
            })
            .collect();
        Self {
            n,
            mask,
            shift,
            fine,
            coarse,
        }
    }

    /// `exp(2πi·index/n)`, for `index <= n`.
    pub(super) fn get(&self, index: usize) -> Complex {
        let (index, conjugate) = if 2 * index <= self.n {
            (index, false)
        } else {
            (self.n - index, true)
        };
        let x1 = self.fine[index & self.mask];
        let x2 = self.coarse[index >> self.shift];
        let im = x1.re * x2.im + x1.im * x2.re;
        Complex::new(
            x1.re * x2.re - x1.im * x2.im,
            if conjugate { -im } else { im },
        )
    }
}

/// `exp(2πi·x/n)` from the cosine and sine of an angle of at most π/4,
/// mirrored into the octant holding it (pocketfft's `calc`).
fn octant_root(x: usize, n: usize, angle: f64) -> Complex {
    let at = |k: usize| (k as f64 * angle).cos();
    let sin_at = |k: usize| (k as f64 * angle).sin();
    let x = x << 3;
    if x < 4 * n {
        if x < 2 * n {
            if x < n {
                return Complex::new(at(x), sin_at(x));
            }
            return Complex::new(sin_at(2 * n - x), at(2 * n - x));
        }
        let x = x - 2 * n;
        if x < n {
            return Complex::new(-sin_at(x), at(x));
        }
        return Complex::new(-at(2 * n - x), sin_at(2 * n - x));
    }
    let x = 8 * n - x;
    if x < 2 * n {
        if x < n {
            return Complex::new(at(x), -sin_at(x));
        }
        return Complex::new(sin_at(2 * n - x), -at(2 * n - x));
    }
    let x = x - 2 * n;
    if x < n {
        return Complex::new(-sin_at(x), -at(x));
    }
    Complex::new(-at(2 * n - x), -sin_at(2 * n - x))
}

/// The significand of π in x87 extended precision (64 bits; π is
/// `PI_SIGNIFICAND · 2⁻⁶²`), as `3.141592653589793238462643383279502884197L`
/// rounds on x86-64.
const PI_SIGNIFICAND: u64 = 0xC90F_DAA2_2168_C235;

/// pocketfft's base angle `double(0.25L·π/n)`: the quotient rounded to
/// the 64-bit significand of x87 extended precision, then to a double, as
/// the C++ `long double` expression rounds it on x86-64 (its double
/// rounding differs from `(π/4)/n` in plain doubles for about one `n` in
/// four).
fn quarter_pi_over(n: usize) -> f64 {
    let n = n as u128;
    // π/4 / n = (PI_SIGNIFICAND · 2⁶⁴ / n) · 2⁻¹²⁸.
    let numerator = u128::from(PI_SIGNIFICAND) << 64;
    let (quotient, sticky) = (numerator / n, numerator % n != 0);
    let (extended, scale) = round_to_bits(quotient, 64, sticky);
    // The extended value is exact: nothing lies below its 64 bits.
    let (double, rescale) = round_to_bits(extended, 53, false);
    let exponent = i32::try_from(scale + rescale).unwrap_or(0) - 128;
    let double = u64::try_from(double).unwrap_or(u64::MAX);
    // A 53-bit integer is exact in a double, and scaling by a power of two
    // is exact while the result stays normal, as it does for every length.
    double as f64 * 2_f64.powi(exponent)
}

/// `value` rounded to `width` significant bits, ties to even (`sticky`
/// tells whether anything below `value`'s last bit was non-zero): the
/// significand and the power of two it is to be scaled by.
fn round_to_bits(value: u128, width: u32, sticky: bool) -> (u128, u32) {
    let drop = (128 - value.leading_zeros()).saturating_sub(width);
    if drop == 0 {
        return (value, 0);
    }
    let kept = value >> drop;
    let rest = value & ((1_u128 << drop) - 1);
    let half = 1_u128 << (drop - 1);
    let up = rest > half || (rest == half && (sticky || kept & 1 == 1));
    let rounded = kept + u128::from(up);
    // A carry out of the width leaves the even power `2^width`.
    if rounded >> width == 0 {
        (rounded, drop)
    } else {
        (rounded >> 1, drop + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_angles_follow_extended_precision() {
        // `double(0.25L·π/n)` on x86-64: one ulp above `(π/4)/n` in
        // doubles for these lengths...
        for (n, bits) in [
            (3_usize, 0x3FD0_C152_382D_7366_u64),
            (6, 0x3FC0_C152_382D_7366),
            (13, 0x3FAE_EEBF_2CA2_ADA8),
            (15, 0x3FAA_CEE9_F37B_EBD6),
        ] {
            assert_eq!(quarter_pi_over(n).to_bits(), bits, "{n}");
        }
        // ... and equal to it for these.
        for n in [1_usize, 8, 11_900, 16_384, 1_000_003] {
            assert_eq!(quarter_pi_over(n), std::f64::consts::FRAC_PI_4 / n as f64);
        }
    }

    #[test]
    fn roots_are_exact_at_quarter_turns() {
        let roots = UnitRoots::new(8);
        assert_eq!(roots.get(0), Complex::ONE);
        assert_eq!(roots.get(2), Complex::new(0.0, 1.0));
        assert_eq!(roots.get(4), Complex::new(-1.0, 0.0));
        assert_eq!(roots.get(6), Complex::new(0.0, -1.0));
    }

    #[test]
    fn roots_lie_on_the_unit_circle() {
        for n in [5_usize, 12, 97, 1000, 16_384] {
            let roots = UnitRoots::new(n);
            for k in (0..n).step_by(n / 5 + 1) {
                let angle = std::f64::consts::TAU * k as f64 / n as f64;
                let root = roots.get(k);
                assert!((root.re - angle.cos()).abs() < 1e-15, "{n} {k}");
                assert!((root.im - angle.sin()).abs() < 1e-15, "{n} {k}");
            }
        }
    }
}
