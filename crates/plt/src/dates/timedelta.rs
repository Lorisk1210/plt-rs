//! Signed durations with microsecond resolution (Python's `timedelta`).

use std::fmt;

use super::DateError;

/// Microseconds per second.
pub(crate) const MICROS_PER_SECOND: i64 = 1_000_000;
/// Microseconds per day.
pub(crate) const MICROS_PER_DAY: i64 = 86_400 * MICROS_PER_SECOND;

/// A signed duration with microsecond resolution.
///
/// Like Python's `timedelta` the value is exact; it is normalised into
/// [`days`](Self::days), [`seconds`](Self::seconds) (`0..86400`) and
/// [`microseconds`](Self::microseconds) (`0..1_000_000`), so negative
/// durations have a negative day count and non-negative remainders.
/// The representable range is about ±292 000 years.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeDelta {
    micros: i64,
}

impl TimeDelta {
    /// The zero duration.
    pub const ZERO: Self = Self { micros: 0 };

    /// A duration of exactly `micros` microseconds.
    pub const fn from_micros(micros: i64) -> Self {
        Self { micros }
    }

    /// `days + seconds + microseconds`, each of any sign.
    ///
    /// ```
    /// use plt::dates::TimeDelta;
    /// let d = TimeDelta::new(0, -1, 250_000).unwrap();
    /// assert_eq!((d.days(), d.seconds(), d.microseconds()), (-1, 86_399, 250_000));
    /// ```
    pub fn new(days: i64, seconds: i64, microseconds: i64) -> Result<Self, DateError> {
        let total = i128::from(days) * i128::from(MICROS_PER_DAY)
            + i128::from(seconds) * i128::from(MICROS_PER_SECOND)
            + i128::from(microseconds);
        i64::try_from(total)
            .map(Self::from_micros)
            .map_err(|_| DateError::Overflow)
    }

    /// `timedelta(days=x)` for a fractional day count, rounding to the
    /// nearest microsecond exactly as `CPython` does (the whole and
    /// fractional parts are converted separately and the final half
    /// microsecond is rounded to even).
    ///
    /// ```
    /// use plt::dates::TimeDelta;
    /// let d = TimeDelta::from_days_f64(-1e-7).unwrap();
    /// assert_eq!((d.days(), d.seconds(), d.microseconds()), (-1, 86_399, 991_360));
    /// ```
    pub fn from_days_f64(days: f64) -> Result<Self, DateError> {
        if !days.is_finite() {
            return Err(DateError::NonFinite(days));
        }
        let whole = days.trunc();
        let fraction = days - whole;
        // Exact: |whole| < 2^63 checked below before widening.
        if whole.abs() > 1e12 {
            return Err(DateError::Overflow);
        }
        let mut total = whole as i128 * i128::from(MICROS_PER_DAY);
        let scaled = MICROS_PER_DAY as f64 * fraction;
        let scaled_whole = scaled.trunc();
        total += scaled_whole as i128;
        let leftover = scaled - scaled_whole;
        if leftover != 0.0 {
            let mut rounded = leftover.round();
            #[expect(clippy::float_cmp, reason = "CPython detects exact halves this way")]
            let halfway = (rounded - leftover).abs() == 0.5;
            if halfway {
                let odd = f64::from(u8::from(total & 1 == 1));
                rounded = 2.0 * leftover.midpoint(odd).round() - odd;
            }
            total += rounded as i128;
        }
        i64::try_from(total)
            .map(Self::from_micros)
            .map_err(|_| DateError::Overflow)
    }

    /// The whole duration in microseconds.
    pub const fn total_micros(self) -> i64 {
        self.micros
    }

    /// The duration in (fractional) seconds, Python's `total_seconds()`.
    pub fn total_seconds(self) -> f64 {
        self.micros as f64 / MICROS_PER_SECOND as f64
    }

    /// Whole days, rounded towards negative infinity.
    pub const fn days(self) -> i64 {
        self.micros.div_euclid(MICROS_PER_DAY)
    }

    /// Seconds within the day, `0..86400`.
    pub const fn seconds(self) -> i64 {
        self.micros.rem_euclid(MICROS_PER_DAY) / MICROS_PER_SECOND
    }

    /// Microseconds within the second, `0..1_000_000`.
    pub const fn microseconds(self) -> i64 {
        self.micros.rem_euclid(MICROS_PER_SECOND)
    }

    /// `self + other`, or `None` on overflow.
    pub const fn checked_add(self, other: Self) -> Option<Self> {
        match self.micros.checked_add(other.micros) {
            Some(micros) => Some(Self { micros }),
            None => None,
        }
    }

    /// `self - other`, or `None` on overflow.
    pub const fn checked_sub(self, other: Self) -> Option<Self> {
        match self.micros.checked_sub(other.micros) {
            Some(micros) => Some(Self { micros }),
            None => None,
        }
    }

    /// `self * factor`, or `None` on overflow.
    pub const fn checked_mul(self, factor: i64) -> Option<Self> {
        match self.micros.checked_mul(factor) {
            Some(micros) => Some(Self { micros }),
            None => None,
        }
    }

    /// `-self`, or `None` on overflow.
    pub const fn checked_neg(self) -> Option<Self> {
        match self.micros.checked_neg() {
            Some(micros) => Some(Self { micros }),
            None => None,
        }
    }

    /// The absolute value, or `None` on overflow.
    pub const fn checked_abs(self) -> Option<Self> {
        match self.micros.checked_abs() {
            Some(micros) => Some(Self { micros }),
            None => None,
        }
    }
}

impl fmt::Display for TimeDelta {
    /// Python's `str(timedelta)`: `[D day[s], ]H:MM:SS[.ffffff]`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let days = self.days();
        if days != 0 {
            let plural = if days.abs() == 1 { "" } else { "s" };
            write!(f, "{days} day{plural}, ")?;
        }
        let seconds = self.seconds();
        write!(
            f,
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )?;
        match self.microseconds() {
            0 => Ok(()),
            us => write!(f, ".{us:06}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(d: TimeDelta) -> (i64, i64, i64) {
        (d.days(), d.seconds(), d.microseconds())
    }

    #[test]
    fn fractional_days_round_like_cpython() {
        let cases = [
            (0.0, (0, 0, 0)),
            (1.5, (1, 43_200, 0)),
            (-0.25, (-1, 64_800, 0)),
            (1e-6, (0, 0, 86_400)),
            (365.2425, (365, 20_952, 0)),
            (1e5, (100_000, 0, 0)),
            (1.123_456_789, (1, 10_666, 666_570)),
        ];
        for (days, expected) in cases {
            assert_eq!(
                parts(TimeDelta::from_days_f64(days).unwrap()),
                expected,
                "{days}"
            );
        }
        assert!(TimeDelta::from_days_f64(f64::NAN).is_err());
        assert!(TimeDelta::from_days_f64(1e300).is_err());
    }

    #[test]
    fn normalisation_and_display() {
        let d = TimeDelta::new(1, 3_661, 5).unwrap();
        assert_eq!(d.to_string(), "1 day, 1:01:01.000005");
        assert_eq!(
            TimeDelta::new(-2, 0, 0).unwrap().to_string(),
            "-2 days, 0:00:00"
        );
        assert_eq!(
            TimeDelta::from_micros(-1).to_string(),
            "-1 day, 23:59:59.999999"
        );
        assert_eq!(TimeDelta::new(0, 90, 0).unwrap().total_seconds(), 90.0);
        assert_eq!(TimeDelta::from_micros(i64::MIN).checked_neg(), None);
        assert!(TimeDelta::new(i64::MAX, 0, 0).is_err());
    }
}
