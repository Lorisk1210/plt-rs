//! pocketfft's complex FFTPACK transform (`cfftp`): the length split into
//! radices (8s, then 4s, a 2 moved to the front, then odd primes), one
//! pass per radix with its twiddle factors, ping-ponging between the data
//! and a scratch buffer.

use super::super::Complex;
use super::generic::pass_generic;
use super::radix::{Layout, odd_radix_roots, pass_odd, pass2, pass4, pass8};
use super::twiddle::UnitRoots;

/// One radix of the factorisation and its twiddle factors.
#[derive(Clone, Debug)]
struct Factor {
    radix: usize,
    /// `WA`: `(radix - 1) · (ido - 1)` factors.
    twiddles: Vec<Complex>,
    /// The `radix`-th roots of unity, for the radices without a
    /// hard-coded pass (above 11).
    roots: Vec<Complex>,
}

/// A planned complex transform of one length.
#[derive(Clone, Debug)]
pub(super) struct Cfftp {
    length: usize,
    factors: Vec<Factor>,
}

impl Cfftp {
    /// Plans transforms of `length` (at least 1) points.
    pub(super) fn new(length: usize) -> Self {
        let radices = if length == 1 {
            Vec::new()
        } else {
            factorize(length)
        };
        let roots = UnitRoots::new(length);
        let mut l1 = 1;
        let factors = radices
            .into_iter()
            .map(|radix| {
                let ido = length / (l1 * radix);
                let twiddles = (1..radix)
                    .flat_map(|j| (1..ido).map(move |i| (j, i)))
                    .map(|(j, i)| roots.get(j * l1 * i))
                    .collect();
                let roots = if radix > 11 {
                    (0..radix).map(|j| roots.get(j * l1 * ido)).collect()
                } else {
                    Vec::new()
                };
                l1 *= radix;
                Factor {
                    radix,
                    twiddles,
                    roots,
                }
            })
            .collect();
        Self { length, factors }
    }

    /// Transforms `c` in place, forward or backward, and scales the result
    /// by `fct` unless it is 1 (pocketfft's `pass_all`).
    pub(super) fn exec<const FWD: bool>(&self, c: &mut [Complex], fct: f64) {
        if self.length == 1 {
            c[0] = c[0] * fct;
            return;
        }
        let mut scratch = vec![Complex::ZERO; self.length];
        let mut in_place = true;
        let mut l1 = 1;
        for factor in &self.factors {
            let layout = Layout {
                ido: self.length / (factor.radix * l1),
                l1,
                radix: factor.radix,
                twiddles: &factor.twiddles,
            };
            let (from, to): (&mut [Complex], &mut [Complex]) = if in_place {
                (&mut *c, scratch.as_mut_slice())
            } else {
                (scratch.as_mut_slice(), &mut *c)
            };
            if pass::<FWD>(layout, &factor.roots, from, to) {
                in_place = !in_place;
            }
            l1 *= factor.radix;
        }
        if !in_place {
            if fct == 1.0 {
                c.copy_from_slice(&scratch);
            } else {
                for (out, &value) in c.iter_mut().zip(&scratch) {
                    *out = value * fct;
                }
            }
        } else if fct != 1.0 {
            for value in c.iter_mut() {
                *value = *value * fct;
            }
        }
    }
}

/// One pass from `from` into `to`; whether the result is in `to` (the
/// generic pass leaves it in `from`).
fn pass<const FWD: bool>(
    layout: Layout<'_>,
    roots: &[Complex],
    from: &mut [Complex],
    to: &mut [Complex],
) -> bool {
    match layout.radix {
        2 => pass2::<FWD>(layout, from, to),
        4 => pass4::<FWD>(layout, from, to),
        8 => pass8::<FWD>(layout, from, to),
        radix => {
            if let Some(table) = odd_radix_roots(radix) {
                pass_odd::<FWD>(layout, table, from, to);
            } else {
                pass_generic::<FWD>(layout, roots, from, to);
                return false;
            }
        }
    }
    true
}

/// pocketfft's factorisation: 8s, then 4s, then a 2 (moved to the
/// front), then odd factors in increasing order.
fn factorize(length: usize) -> Vec<usize> {
    let mut factors = Vec::new();
    let mut rest = length;
    while rest.is_multiple_of(8) {
        factors.push(8);
        rest /= 8;
    }
    while rest.is_multiple_of(4) {
        factors.push(4);
        rest /= 4;
    }
    if rest.is_multiple_of(2) {
        rest /= 2;
        factors.push(2);
        let last = factors.len() - 1;
        factors.swap(0, last);
    }
    let mut divisor = 3;
    while divisor * divisor <= rest {
        while rest.is_multiple_of(divisor) {
            factors.push(divisor);
            rest /= divisor;
        }
        divisor += 2;
    }
    if rest > 1 {
        factors.push(rest);
    }
    factors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factors_follow_pocketfft() {
        assert_eq!(factorize(16_384), [8, 8, 8, 8, 4]);
        assert_eq!(factorize(12), [4, 3]);
        assert_eq!(factorize(6), [2, 3]);
        assert_eq!(factorize(2 * 8 * 9 * 13), [2, 8, 3, 3, 13]);
        assert_eq!(factorize(11_900), [4, 5, 5, 7, 17]);
        assert_eq!(factorize(97), [97]);
    }
}
