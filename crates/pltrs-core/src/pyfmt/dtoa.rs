//! Decimal digit generation and `CPython`'s `format_float_short` layout.
//!
//! `CPython` formats floats in two stages: David Gay's `dtoa` produces the
//! correctly rounded decimal digits (trailing zeros stripped) and the
//! position of the decimal point, then `format_float_short` lays them out
//! with padding zeros, a decimal point and an optional exponent. Rust's
//! exact float formatting rounds identically (half to even on the exact
//! binary value), so it serves as the digit generator here.

use std::fmt::Write as _;

/// One of `CPython`'s float presentations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Presentation {
    /// `e`: one digit before the point, `precision` after, exponent.
    Exponent,
    /// `f`: `precision` digits after the point.
    Fixed,
    /// `g`: `precision` significant digits, exponent when out of range.
    General,
    /// `r`: the shortest digits that round-trip (`repr`).
    Repr,
}

crate::archive_enum!(Presentation {
    Exponent,
    Fixed,
    General,
    Repr,
});

/// Everything `PyOS_double_to_string` takes besides the value.
#[expect(
    clippy::struct_excessive_bools,
    reason = "mirrors CPython's independent Py_DTSF_* flags"
)]
#[derive(Clone, Copy, Debug)]
pub(super) struct FloatStyle {
    /// The presentation type.
    pub(super) presentation: Presentation,
    /// Its precision (ignored by [`Presentation::Repr`]).
    pub(super) precision: usize,
    /// Upper-case `E`, `INF` and `NAN`.
    pub(super) upper: bool,
    /// Always emit a sign (`Py_DTSF_SIGN`).
    pub(super) always_sign: bool,
    /// Keep at least one digit after the point (`Py_DTSF_ADD_DOT_0`).
    pub(super) add_dot_0: bool,
    /// The `#` alternate form: keep trailing zeros and the point.
    pub(super) alternate: bool,
    /// The `z` option: drop the sign of a value that rounds to zero.
    pub(super) no_negative_zero: bool,
}

impl FloatStyle {
    /// A plain style for `presentation` at `precision`.
    pub(super) const fn new(presentation: Presentation, precision: usize) -> Self {
        Self {
            presentation,
            precision,
            upper: false,
            always_sign: false,
            add_dot_0: false,
            alternate: false,
            no_negative_zero: false,
        }
    }

    /// The style of Python's `repr(float)`.
    pub(super) const REPR: Self = Self {
        add_dot_0: true,
        ..Self::new(Presentation::Repr, 0)
    };

    /// The precision the layout pads to and the digits `dtoa` produces.
    fn digit_mode(&self) -> (usize, Mode) {
        match self.presentation {
            Presentation::Exponent => {
                let significant = self.precision.saturating_add(1);
                (significant, Mode::Significant(significant))
            }
            Presentation::Fixed => (self.precision, Mode::Fixed(self.precision)),
            Presentation::General => {
                let p = self.precision.max(1);
                (p, Mode::Significant(p))
            }
            Presentation::Repr => (0, Mode::Shortest),
        }
    }
}

/// How many digits `dtoa` should produce.
#[derive(Clone, Copy, Debug)]
enum Mode {
    /// Mode 0: shortest round-tripping digits.
    Shortest,
    /// Mode 2: this many significant digits.
    Significant(usize),
    /// Mode 3: this many digits after the decimal point.
    Fixed(usize),
}

/// `dtoa`'s result: digits without trailing zeros and the decimal point
/// position (`0.123 -> ("123", 0)`, `12.3 -> ("123", 2)`).
#[derive(Debug, PartialEq)]
struct Digits {
    digits: Vec<u8>,
    decpt: i64,
}

/// The most significant digits the exact decimal value of a double has.
const EXACT_SIGNIFICANT_DIGITS: usize = 767;

/// The most decimal places the exact decimal value of a double has.
pub(super) const EXACT_DECIMAL_PLACES: usize = 1074;

fn dtoa(abs: f64, mode: Mode) -> Digits {
    if abs == 0.0 {
        return Digits {
            digits: b"0".to_vec(),
            decpt: 1,
        };
    }
    // Digits beyond the exact value are zeros, which `Digits` drops, so
    // asking for no more than the exact digits changes nothing (and keeps
    // within the precision `format!` accepts).
    match mode {
        Mode::Shortest => scientific(&format!("{abs:e}")),
        Mode::Significant(n) => scientific(&format!(
            "{abs:.*e}",
            n.clamp(1, EXACT_SIGNIFICANT_DIGITS) - 1
        )),
        Mode::Fixed(n) => positional(&format!("{abs:.*}", n.min(EXACT_DECIMAL_PLACES)), n),
    }
}

/// Parses Rust's `d.ddde±x` output.
fn scientific(text: &str) -> Digits {
    let (mantissa, exponent) = text
        .split_once('e')
        .expect("Rust's exponential output contains an exponent");
    let exponent: i64 = exponent
        .parse()
        .expect("Rust's exponent is a decimal integer");
    let mut digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).collect();
    while digits.len() > 1 && digits.last() == Some(&b'0') {
        digits.pop();
    }
    Digits {
        digits,
        decpt: exponent + 1,
    }
}

/// Parses Rust's `ddd.ddd` output produced with `places` decimals.
fn positional(text: &str, places: usize) -> Digits {
    let (int_part, frac_part) = text.split_once('.').unwrap_or((text, ""));
    let mut digits: Vec<u8> = int_part.bytes().chain(frac_part.bytes()).collect();
    let leading = digits.iter().take_while(|&&d| d == b'0').count();
    if leading == digits.len() {
        // dtoa returns no digits and `decpt = -ndigits` for a value that
        // rounds to zero.
        return Digits {
            digits: Vec::new(),
            decpt: -(places as i64),
        };
    }
    digits.drain(..leading);
    while digits.last() == Some(&b'0') {
        digits.pop();
    }
    Digits {
        digits,
        decpt: int_part.len() as i64 - leading as i64,
    }
}

fn push_zeros(out: &mut String, count: i64) {
    for _ in 0..count.max(0) {
        out.push('0');
    }
}

/// `CPython`'s `PyOS_double_to_string` for one value.
pub(super) fn format_float(x: f64, style: FloatStyle) -> String {
    let mut out = String::new();
    if !x.is_finite() {
        // dtoa ignores the sign of a NaN.
        if x == f64::NEG_INFINITY {
            out.push('-');
        } else if style.always_sign {
            out.push('+');
        }
        let word = if x.is_nan() { "nan" } else { "inf" };
        out.push_str(&if style.upper {
            word.to_ascii_uppercase()
        } else {
            word.to_owned()
        });
        return out;
    }

    let (precision, mode) = style.digit_mode();
    let Digits { digits, mut decpt } = dtoa(x.abs(), mode);
    let precision = precision as i64;
    let digits_len = digits.len() as i64;

    let mut negative = x.is_sign_negative();
    if style.no_negative_zero && (digits.is_empty() || digits == b"0") {
        negative = false;
    }

    let mut vdigits_end = digits_len;
    let mut use_exp = false;
    match style.presentation {
        Presentation::Exponent => {
            use_exp = true;
            vdigits_end = precision;
        }
        Presentation::Fixed => vdigits_end = decpt + precision,
        Presentation::General => {
            let limit = if style.add_dot_0 {
                precision - 1
            } else {
                precision
            };
            use_exp = decpt <= -4 || decpt > limit;
            if style.alternate {
                vdigits_end = precision;
            }
        }
        Presentation::Repr => use_exp = decpt <= -4 || decpt > 16,
    }

    let mut exponent = 0;
    if use_exp {
        exponent = decpt - 1;
        decpt = 1;
    }
    let vdigits_start = if decpt <= 0 { decpt - 1 } else { 0 };
    let min_end = if !use_exp && style.add_dot_0 {
        decpt + 1
    } else {
        decpt
    };
    vdigits_end = vdigits_end.max(min_end);

    if negative {
        out.push('-');
    } else if style.always_sign {
        out.push('+');
    }
    if decpt <= 0 {
        push_zeros(&mut out, decpt - vdigits_start);
        out.push('.');
        push_zeros(&mut out, -decpt);
    } else {
        push_zeros(&mut out, -vdigits_start);
    }
    let text = String::from_utf8(digits).expect("dtoa digits are ASCII");
    if 0 < decpt && decpt <= digits_len {
        let (int_digits, frac_digits) = text.split_at(decpt as usize);
        out.push_str(int_digits);
        out.push('.');
        out.push_str(frac_digits);
    } else {
        out.push_str(&text);
    }
    if digits_len < decpt {
        push_zeros(&mut out, decpt - digits_len);
        out.push('.');
        push_zeros(&mut out, vdigits_end - decpt);
    } else {
        push_zeros(&mut out, vdigits_end - digits_len);
    }
    if out.ends_with('.') && !style.alternate {
        out.pop();
    }
    if use_exp {
        out.push(if style.upper { 'E' } else { 'e' });
        write!(out, "{exponent:+03}").expect("writing to a String cannot fail");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(x: f64, presentation: Presentation, precision: usize) -> String {
        format_float(x, FloatStyle::new(presentation, precision))
    }

    #[test]
    fn digit_generation() {
        assert_eq!(
            dtoa(0.0012, Mode::Fixed(4)),
            Digits {
                digits: b"12".to_vec(),
                decpt: -2
            }
        );
        assert_eq!(
            dtoa(0.0001, Mode::Fixed(2)),
            Digits {
                digits: Vec::new(),
                decpt: -2
            }
        );
        assert_eq!(
            dtoa(1500.0, Mode::Shortest),
            Digits {
                digits: b"15".to_vec(),
                decpt: 4
            }
        );
    }

    #[test]
    fn precisions_beyond_the_exact_digits_pad_with_zeros() {
        // Found by fuzzing: `format!` panicked on precisions above 65535.
        let long = fmt(5.990_139_303_742_726, Presentation::General, 326_666);
        assert_eq!(long, "5.990139303742726184509592712856829166412353515625");
        let fixed = fmt(0.5, Presentation::Fixed, 70_000);
        assert_eq!(fixed.len(), 70_002);
        assert!(fixed.starts_with("0.5000"));
        let exact = fmt(5e-324, Presentation::Exponent, 800);
        assert!(exact.starts_with("4.9406564584124654417656879286822137236505"));
        assert!(exact.ends_with("000e-324"));
    }

    #[test]
    fn repr_matches_python() {
        let cases = [
            (1.0, "1.0"),
            (0.1, "0.1"),
            (-1e-5, "-1e-05"),
            (1e16, "1e+16"),
            (1e15, "1000000000000000.0"),
            (0.0001, "0.0001"),
            (123_456_789.0, "123456789.0"),
            (1.5e300, "1.5e+300"),
            (-0.0, "-0.0"),
            (5e-324, "5e-324"),
            (f64::NAN, "nan"),
            (f64::NEG_INFINITY, "-inf"),
            (0.300_000_000_000_000_04, "0.30000000000000004"),
        ];
        for (x, want) in cases {
            assert_eq!(format_float(x, FloatStyle::REPR), want, "repr({x})");
        }
    }

    #[test]
    fn fixed_exponent_general() {
        assert_eq!(fmt(1500.25, Presentation::Fixed, 1), "1500.2");
        assert_eq!(fmt(0.125, Presentation::Fixed, 2), "0.12");
        assert_eq!(fmt(-1e-5, Presentation::Fixed, 2), "-0.00");
        assert_eq!(fmt(2.5, Presentation::Fixed, 0), "2");
        assert_eq!(fmt(1500.25, Presentation::Exponent, 3), "1.500e+03");
        assert_eq!(fmt(1e-300, Presentation::Exponent, 0), "1e-300");
        assert_eq!(fmt(1e7, Presentation::General, 6), "1e+07");
        assert_eq!(fmt(1500.25, Presentation::General, 6), "1500.25");
        assert_eq!(fmt(1e-5, Presentation::General, 6), "1e-05");
        assert_eq!(fmt(0.0001, Presentation::General, 6), "0.0001");
        assert_eq!(fmt(123_456_789.0, Presentation::General, 6), "1.23457e+08");
        assert_eq!(fmt(9.9999, Presentation::General, 2), "10");
        assert_eq!(fmt(0.0, Presentation::General, 0), "0");
        let alt = FloatStyle {
            alternate: true,
            ..FloatStyle::new(Presentation::General, 1)
        };
        assert_eq!(format_float(2.0, alt), "2.");
        let alt = FloatStyle {
            alternate: true,
            ..FloatStyle::new(Presentation::General, 5)
        };
        assert_eq!(format_float(0.5, alt), "0.50000");
    }
}
