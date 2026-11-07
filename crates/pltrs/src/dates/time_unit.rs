//! The units of numpy's `datetime64` and their counts as instants
//! (`numpy.array(counts, dtype=f"datetime64[{unit}]")`).

use std::fmt;
use std::str::FromStr;

use super::calendar::days_from_civil;
use super::{DateError, Datetime64};

/// A unit of numpy's `datetime64`, from years to attoseconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimeUnit {
    /// `Y`: calendar years.
    Years,
    /// `M`: calendar months.
    Months,
    /// `W`: weeks of seven days.
    Weeks,
    /// `D`: days.
    Days,
    /// `h`: hours.
    Hours,
    /// `m`: minutes.
    Minutes,
    /// `s`: seconds.
    Seconds,
    /// `ms`: milliseconds.
    Milliseconds,
    /// `us`: microseconds.
    Microseconds,
    /// `ns`: nanoseconds.
    Nanoseconds,
    /// `ps`: picoseconds.
    Picoseconds,
    /// `fs`: femtoseconds.
    Femtoseconds,
    /// `as`: attoseconds.
    Attoseconds,
}

impl TimeUnit {
    /// Every unit, coarsest first.
    pub const ALL: [Self; 13] = [
        Self::Years,
        Self::Months,
        Self::Weeks,
        Self::Days,
        Self::Hours,
        Self::Minutes,
        Self::Seconds,
        Self::Milliseconds,
        Self::Microseconds,
        Self::Nanoseconds,
        Self::Picoseconds,
        Self::Femtoseconds,
        Self::Attoseconds,
    ];

    /// numpy's code of the unit (`"Y"`, `"ms"`, ...).
    pub const fn code(self) -> &'static str {
        match self {
            Self::Years => "Y",
            Self::Months => "M",
            Self::Weeks => "W",
            Self::Days => "D",
            Self::Hours => "h",
            Self::Minutes => "m",
            Self::Seconds => "s",
            Self::Milliseconds => "ms",
            Self::Microseconds => "us",
            Self::Nanoseconds => "ns",
            Self::Picoseconds => "ps",
            Self::Femtoseconds => "fs",
            Self::Attoseconds => "as",
        }
    }

    /// The unit's seconds (for units of a fixed length of a second or
    /// more) or its count per second (finer ones): `Ok(seconds)` or
    /// `Err(per_second)`; `None` for years and months.
    const fn scale(self) -> Option<Result<i64, i64>> {
        Some(match self {
            Self::Years | Self::Months => return None,
            Self::Weeks => Ok(7 * 86_400),
            Self::Days => Ok(86_400),
            Self::Hours => Ok(3600),
            Self::Minutes => Ok(60),
            Self::Seconds => Ok(1),
            Self::Milliseconds => Err(1_000),
            Self::Microseconds => Err(1_000_000),
            Self::Nanoseconds => Err(1_000_000_000),
            Self::Picoseconds => Err(1_000_000_000_000),
            Self::Femtoseconds => Err(1_000_000_000_000_000),
            Self::Attoseconds => Err(1_000_000_000_000_000_000),
        })
    }
}

impl fmt::Display for TimeUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl FromStr for TimeUnit {
    type Err = DateError;

    /// The unit of a numpy code (`"D"`, `"us"`, ...).
    fn from_str(code: &str) -> Result<Self, DateError> {
        Self::ALL
            .into_iter()
            .find(|unit| unit.code() == code)
            .ok_or_else(|| DateError::parse(code, "no datetime64 unit"))
    }
}

impl Datetime64 {
    /// The instant `count` units after 1970-01-01T00:00:00, as numpy's
    /// `datetime64[unit]` stands for it and matplotlib's `date2num` reads
    /// it (whole seconds, rounded down, and the nanoseconds beyond them,
    /// finer parts dropped).
    ///
    /// ```
    /// use pltrs::dates::{Datetime64, TimeUnit};
    /// let leap_day = Datetime64::from_count(601, TimeUnit::Months)?;
    /// assert_eq!(leap_day.seconds(), 1_580_515_200); // 2020-02-01
    /// let before = Datetime64::from_count(-5, TimeUnit::Femtoseconds)?;
    /// assert_eq!((before.seconds(), before.subsec_nanos()), (-1, 999_999_999));
    /// assert!(Datetime64::from_count(1, TimeUnit::Attoseconds).is_err());
    /// # Ok::<(), pltrs::dates::DateError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`DateError::Overflow`] for an instant beyond the seconds of an
    /// `i64`, and [`DateError::InvalidArgument`] for attoseconds, which
    /// numpy cannot convert to seconds (it overflows computing the
    /// conversion factor).
    pub fn from_count(count: i64, unit: TimeUnit) -> Result<Self, DateError> {
        let seconds_of_days = |days: Option<i64>| {
            days.and_then(|days| days.checked_mul(86_400))
                .map(|seconds| Self::new(seconds, 0))
                .ok_or(DateError::Overflow)
        };
        let civil = |year: Option<i64>, month: u32| {
            // Years beyond those of the seconds of an `i64`.
            let year = year.filter(|year| year.unsigned_abs() <= 292_277_026_596);
            seconds_of_days(year.map(|year| days_from_civil(year, month, 1)))
        };
        match unit.scale() {
            None if unit == TimeUnit::Years => civil(count.checked_add(1970), 1),
            None => civil(
                count.div_euclid(12).checked_add(1970),
                (count.rem_euclid(12) + 1) as u32,
            ),
            Some(Ok(seconds)) => count
                .checked_mul(seconds)
                .map(|seconds| Self::new(seconds, 0))
                .ok_or(DateError::Overflow),
            Some(Err(_)) if unit == TimeUnit::Attoseconds => Err(DateError::InvalidArgument(
                "Integer overflow while computing the conversion factor between NumPy \
                 datetime units s and as"
                    .to_owned(),
            )),
            Some(Err(per_second)) => {
                let rest = count.rem_euclid(per_second);
                let nanos = if per_second <= 1_000_000_000 {
                    rest * (1_000_000_000 / per_second)
                } else {
                    rest / (per_second / 1_000_000_000)
                };
                let nanos = u32::try_from(nanos).expect("the nanoseconds of a second fit a u32");
                Ok(Self::new(count.div_euclid(per_second), nanos))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(count: i64, unit: TimeUnit) -> (i64, u32) {
        let t = Datetime64::from_count(count, unit).unwrap();
        (t.seconds(), t.subsec_nanos())
    }

    #[test]
    fn calendar_units_count_from_1970() {
        assert_eq!(parts(30, TimeUnit::Years), (946_684_800, 0));
        assert_eq!(parts(-1, TimeUnit::Years), (-31_536_000, 0));
        assert_eq!(parts(-13, TimeUnit::Months), (-34_214_400, 0));
        // 1970-01-01 was a Thursday, as numpy's weeks start.
        assert_eq!(parts(-1, TimeUnit::Weeks), (-604_800, 0));
        assert!(Datetime64::from_count(i64::MAX, TimeUnit::Days).is_err());
        assert!(Datetime64::from_count(i64::MAX, TimeUnit::Years).is_err());
    }

    #[test]
    fn finer_units_round_down_to_seconds() {
        assert_eq!(parts(-1, TimeUnit::Milliseconds), (-1, 999_000_000));
        assert_eq!(parts(1_500_001, TimeUnit::Microseconds), (1, 500_001_000));
        assert_eq!(parts(-1, TimeUnit::Nanoseconds), (-1, 999_999_999));
        assert_eq!(parts(1_000_001, TimeUnit::Picoseconds), (0, 1000));
        assert_eq!(
            parts(-1_000_000_000, TimeUnit::Picoseconds),
            (-1, 999_000_000)
        );
    }

    #[test]
    fn codes_name_the_units() {
        for unit in TimeUnit::ALL {
            assert_eq!(unit.code().parse::<TimeUnit>(), Ok(unit));
            assert_eq!(unit.to_string(), unit.code());
        }
        assert!("x".parse::<TimeUnit>().is_err());
    }
}
