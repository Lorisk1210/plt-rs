//! `a * b + c` as the compiled code pltrs reproduces computes it on this
//! platform: matplotlib's extensions (with the Agg, qhull and `HarfBuzz`
//! they build in), contourpy's, and numpy's own loops.
//!
//! C and C++ let a compiler contract a product and the sum or difference
//! it feeds into one fused multiply-add, rounded once instead of twice.
//! Whether and where it does is the compiler's choice, so the reference
//! is the binary wheel from `PyPI` of each platform, and the compiler that
//! built it ([`Contraction`]):
//!
//! * the x86-64 wheels (Linux, Windows, and macOS under Rosetta) fuse
//!   nothing: GCC and MSVC do not for x86-64's baseline;
//! * the arm64 macOS wheels are clang's, which contracts within one
//!   expression, in C and C++ alike;
//! * the aarch64 Linux wheels are GCC 14's (the `manylinux_2_28` image),
//!   which contracts C++ and GNU C across statements and through inlined
//!   operators, in an order of its own, and leaves ISO C (numpy's own
//!   loops, built as C11) alone.
//!
//! A port writes each expression the reference's compiler may contract
//! through [`fused`] (C++ and GNU C) or [`fused_iso_c`] (numpy's C), in
//! clang's operand order; where GCC fuses another product, or one clang
//! leaves rounded, the port says so with a `match` on [`CONTRACTION`]
//! (or a function taking the [`Contraction`], for its tests to evaluate
//! both patterns).

/// The compiler whose contraction of products into sums the compiled
/// code pltrs reproduces here follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contraction {
    /// Every product rounded before it is added (GCC and MSVC building
    /// for x86-64's baseline).
    Unfused,
    /// clang's default (`-ffp-contract=on`), in C and C++: within one
    /// expression, of a sum or difference the left operand if it is a
    /// product, else the right one; a product kept in a variable or
    /// reached through an overloaded operator is rounded first.
    Clang,
    /// GCC's default for C++ and GNU C (`-ffp-contract=fast`): its
    /// `widening_mul` pass fuses a product into the sum or difference
    /// that uses it, across statements and inlined functions too, the
    /// product it meets first (which may be either operand); ISO C
    /// (`-std=c11`) is not contracted at all.
    Gcc,
}

/// Which product of `a * b - c * d` a compiler fuses into the difference
/// (the other is rounded first): clang the first, GCC either, by the
/// order its pass meets them in the function the expression is inlined
/// into (its GIMPLE says which).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FusedProduct {
    /// `a * b`.
    First,
    /// `c * d`.
    Second,
}

impl Contraction {
    /// Whether this compiler fuses products in C++ and GNU C.
    #[must_use]
    pub const fn fuses(self) -> bool {
        !matches!(self, Self::Unfused)
    }

    /// Whether this compiler fuses products in ISO C (numpy's own loops):
    /// clang does, GCC does not.
    #[must_use]
    pub const fn fuses_iso_c(self) -> bool {
        matches!(self, Self::Clang)
    }

    /// `a * b + c` as this compiler computes a contracted C++ expression:
    /// rounded once where it fuses, twice where it does not.
    #[inline]
    #[must_use]
    pub fn fused(self, a: f64, b: f64, c: f64) -> f64 {
        if self.fuses() {
            rounded_once(a, b, c)
        } else {
            rounded_twice(a, b, c)
        }
    }

    /// `a * b - c * d` as this compiler computes it in C++ with the product
    /// `fused` fused where it fuses (both rounded where it does not).
    #[inline]
    #[must_use]
    pub fn difference_of_products(
        self,
        fused: FusedProduct,
        a: f64,
        b: f64,
        c: f64,
        d: f64,
    ) -> f64 {
        match fused {
            FusedProduct::First => self.fused(a, b, -(c * d)),
            FusedProduct::Second => self.fused(-c, d, a * b),
        }
    }

    /// `a * b + c` as this compiler computes it in ISO C.
    #[inline]
    #[must_use]
    pub fn fused_iso_c(self, a: f64, b: f64, c: f64) -> f64 {
        if self.fuses_iso_c() {
            rounded_once(a, b, c)
        } else {
            rounded_twice(a, b, c)
        }
    }
}

/// The compiler of the reference wheels here: clang on arm64 macOS, GCC
/// on aarch64 Linux (glibc and musl alike: matplotlib has no musl aarch64
/// wheel, and a source build there is GCC's), none that fuses elsewhere.
pub const CONTRACTION: Contraction = if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
    Contraction::Clang
} else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
    Contraction::Gcc
} else {
    Contraction::Unfused
};

/// Whether matplotlib's compiled code here rounds some `a * b + c` once
/// instead of twice ([`CONTRACTION`] fuses C++).
pub const FUSES_PRODUCTS: bool = CONTRACTION.fuses();

/// `a * b + c` where matplotlib's compiled code computes it by a fused
/// multiply-add on the platforms of [`FUSES_PRODUCTS`]: rounded once
/// there, and the product and the sum rounded one after the other
/// elsewhere.
///
/// A ported expression names the product clang fuses: of `a * b + c * d`
/// the first, so `fused(a, b, c * d)`; `c - a * b` is `fused(-a, b, c)` and
/// `a * b - c` is `fused(a, b, -c)`, negation being exact. Without fusing
/// each of these is the expression as written, bit for bit.
///
/// ```
/// use pltrs_core::numeric::{FUSES_PRODUCTS, fused};
/// assert_eq!(fused(3.0, 4.0, 5.0), 17.0);
/// // 0.1 * 10.0 is 1 + 2^-54 before rounding.
/// let residue = fused(0.1, 10.0, -1.0);
/// assert_eq!(residue, if FUSES_PRODUCTS { 2f64.powi(-54) } else { 0.0 });
/// ```
#[inline]
#[must_use]
pub fn fused(a: f64, b: f64, c: f64) -> f64 {
    CONTRACTION.fused(a, b, c)
}

/// `a * b + c` as numpy's own loops (ISO C) compute it here: fused on
/// arm64 macOS (clang), rounded twice everywhere else, aarch64 Linux
/// included (GCC).
///
/// ```
/// use pltrs_core::numeric::{CONTRACTION, fused_iso_c};
/// let residue = fused_iso_c(0.1, 10.0, -1.0);
/// assert_eq!(residue, if CONTRACTION.fuses_iso_c() { 2f64.powi(-54) } else { 0.0 });
/// ```
#[inline]
#[must_use]
pub fn fused_iso_c(a: f64, b: f64, c: f64) -> f64 {
    CONTRACTION.fused_iso_c(a, b, c)
}

/// [`fused`] for single-precision operands (Agg's `float` colour types).
///
/// ```
/// use pltrs_core::numeric::fused_f32;
/// assert_eq!(fused_f32(3.0, 4.0, 5.0), 17.0);
/// ```
#[inline]
#[must_use]
pub fn fused_f32(a: f32, b: f32, c: f32) -> f32 {
    if FUSES_PRODUCTS {
        a.mul_add(b, c)
    } else {
        a * b + c
    }
}

/// `a * b + c * d` as that code computes it: where it fuses, the first
/// product is fused into the sum and the second rounded.
///
/// ```
/// use pltrs_core::numeric::sum_of_products;
/// assert_eq!(sum_of_products(2.0, 3.0, 4.0, 5.0), 26.0);
/// ```
#[inline]
#[must_use]
pub fn sum_of_products(a: f64, b: f64, c: f64, d: f64) -> f64 {
    fused(a, b, c * d)
}

/// `a * b - c * d` as that code computes it: where it fuses, the first
/// product is fused into the difference and the second rounded.
///
/// ```
/// use pltrs_core::numeric::difference_of_products;
/// assert_eq!(difference_of_products(2.0, 3.0, 4.0, 5.0), -14.0);
/// ```
#[inline]
#[must_use]
pub fn difference_of_products(a: f64, b: f64, c: f64, d: f64) -> f64 {
    fused(a, b, -(c * d))
}

/// `a * b + c` with the exact product added: a fused multiply-add.
#[inline]
fn rounded_once(a: f64, b: f64, c: f64) -> f64 {
    a.mul_add(b, c)
}

/// `a * b + c` with the product rounded before it is added.
#[inline]
fn rounded_twice(a: f64, b: f64, c: f64) -> f64 {
    a * b + c
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both forms of [`fused`], whichever the platform selects, at operands
    /// where the product's rounding shows.
    #[test]
    fn fused_products_round_once_or_twice() {
        // 0.1 * 10 = 1 + 2^-54 exactly, which rounds to 1.
        assert_eq!(rounded_twice(0.1, 10.0, -1.0), 0.0);
        assert_eq!(rounded_once(0.1, 10.0, -1.0), 2f64.powi(-54));
        // (1 + 2^-30)^2 = 1 + 2^-29 + 2^-60: the last term is rounded away.
        let x = 1.0 + 2f64.powi(-30);
        assert_eq!(rounded_twice(x, x, -(1.0 + 2f64.powi(-29))), 0.0);
        assert_eq!(rounded_once(x, x, -(1.0 + 2f64.powi(-29))), 2f64.powi(-60));
        // A determinant of dependent rows, `a * d - b * c`: with the first
        // product fused, the second product's rounding error is left.
        let third = 1.0 / 3.0;
        assert_eq!(rounded_twice(third, 3.0, -(third * 3.0)), 0.0);
        assert_eq!(rounded_once(third, 3.0, -(third * 3.0)), -(2f64.powi(-54)));
        // Exact products agree, and the forms written with a negation are
        // the differences as written.
        for (a, b, c) in [(3.0, 4.0, 5.0), (-0.5, 8.0, 4.0), (0.0, -1.0, 0.0)] {
            assert_eq!(
                rounded_once(a, b, c).to_bits(),
                rounded_twice(a, b, c).to_bits()
            );
        }
        let (a, b, c) = (0.3, 0.7, 0.21);
        assert_eq!(rounded_twice(-a, b, c).to_bits(), (c - a * b).to_bits());
        assert_eq!(rounded_twice(a, b, -c).to_bits(), (a * b - c).to_bits());
        assert_eq!(
            fused(a, b, -c),
            if FUSES_PRODUCTS {
                rounded_once(a, b, -c)
            } else {
                rounded_twice(a, b, -c)
            }
        );
        assert!(fused(f64::NAN, 1.0, 1.0).is_nan());
        assert_eq!(fused(f64::INFINITY, 1.0, 1.0), f64::INFINITY);
    }

    #[test]
    fn each_compiler_fuses_its_own_languages() {
        let (a, b, c) = (0.1, 10.0, -1.0);
        let once = 2f64.powi(-54);
        assert_eq!(Contraction::Unfused.fused(a, b, c), 0.0);
        assert_eq!(Contraction::Clang.fused(a, b, c), once);
        assert_eq!(Contraction::Gcc.fused(a, b, c), once);
        // ISO C: clang contracts it, GCC does not.
        assert_eq!(Contraction::Unfused.fused_iso_c(a, b, c), 0.0);
        assert_eq!(Contraction::Clang.fused_iso_c(a, b, c), once);
        assert_eq!(Contraction::Gcc.fused_iso_c(a, b, c), 0.0);
        assert_eq!(fused(a, b, c), CONTRACTION.fused(a, b, c));
        assert_eq!(fused_iso_c(a, b, c), CONTRACTION.fused_iso_c(a, b, c));
    }

    #[test]
    fn either_product_of_a_difference_may_be_fused() {
        // 1/3 * 3 is 1 - 2^-54 before rounding, 1 * 1 is exact: fusing the
        // first product keeps its residue, fusing the second loses it.
        let third = 1.0 / 3.0;
        let gcc = Contraction::Gcc;
        assert_eq!(
            gcc.difference_of_products(FusedProduct::First, third, 3.0, 1.0, 1.0),
            -(2f64.powi(-54))
        );
        assert_eq!(
            gcc.difference_of_products(FusedProduct::Second, third, 3.0, 1.0, 1.0),
            0.0
        );
        let unfused = Contraction::Unfused;
        assert_eq!(
            unfused.difference_of_products(FusedProduct::First, third, 3.0, 1.0, 1.0),
            0.0
        );
    }

    #[test]
    fn the_platforms_compilers() {
        let expected = if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            Contraction::Clang
        } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
            Contraction::Gcc
        } else {
            Contraction::Unfused
        };
        assert_eq!(CONTRACTION, expected);
        assert_eq!(FUSES_PRODUCTS, expected != Contraction::Unfused);
    }
}
