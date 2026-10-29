//! Evenly spaced date-times: [`DateRange`], matplotlib's `drange` as
//! date-times rather than date numbers.

use std::iter::FusedIterator;

use super::{DateError, DateTime, TimeDelta};

/// The largest number of dates a [`DateRange`] holds, and of date
/// numbers [`drange`](super::drange) gives; more are refused with
/// [`DateError::TooManyRangeDates`]. matplotlib's `drange` allocates
/// whatever it is asked for, and a step of a microsecond over a few
/// years asks for more than memory holds. This allows two years of
/// seconds.
pub const MAX_RANGE_DATES: usize = 1 << 26;

/// The date-times from `start` up to, not including, `end`, `step`
/// apart: `start`, `start + step`, `start + 2 * step`, ...
/// (matplotlib's `drange(start, end, step)`, as date-times rather than
/// date numbers).
///
/// It is an iterator of [`DateTime`]s, so it plots directly, as any
/// iterator of dates does, and collects into a `Vec` or into
/// [`AxisValues`](crate::units::AxisValues). Like the date-times it
/// yields, it counts in UTC: the step is exact, whatever clock changes a
/// zone makes along the way.
///
/// ```
/// use plt::dates::{DateRange, DateTime, TimeDelta};
/// use plt::prelude::*;
///
/// let start = DateTime::new(2024, 3, 1, 0, 0, 0, 0)?;
/// let end = DateTime::new(2024, 3, 2, 0, 0, 0, 0)?;
/// let hours = DateRange::new(start, end, TimeDelta::from_hours(6))?;
/// assert_eq!(hours.len(), 4);
/// assert_eq!(hours.clone().last(), Some(DateTime::new(2024, 3, 1, 18, 0, 0, 0)?));
///
/// let ax = Figure::new().subplot();
/// let line = ax.plot(hours, [3.0, 1.0, 4.0, 1.0])?;
/// assert_eq!(line.xdata(), [19783.0, 19783.25, 19783.5, 19783.75]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DateRange {
    /// The first date-time of the whole range.
    start: DateTime,
    /// The distance between neighbours, positive.
    step: TimeDelta,
    /// The index of the next date-time from the front.
    front: usize,
    /// One past the index of the next date-time from the back.
    back: usize,
}

impl DateRange {
    /// The date-times from `start` up to, not including, `end`, `step`
    /// apart. An `end` at or before `start` gives none, as `start..end`
    /// does; matplotlib's `drange` fails instead once `end` is a step or
    /// more before `start`.
    ///
    /// # Errors
    ///
    /// [`DateError::NonPositiveStep`] for a zero or negative `step`, and
    /// [`DateError::TooManyRangeDates`] for more than
    /// [`MAX_RANGE_DATES`] date-times.
    pub fn new(start: DateTime, end: DateTime, step: TimeDelta) -> Result<Self, DateError> {
        let step_micros =
            u64::try_from(step.total_micros()).map_err(|_| DateError::NonPositiveStep(step))?;
        if step_micros == 0 {
            return Err(DateError::NonPositiveStep(step));
        }
        // Years 1 to 9999 lie well within an i64 of microseconds either way.
        let span = u64::try_from((end - start).total_micros()).unwrap_or(0);
        let count = span.div_ceil(step_micros);
        let len = usize::try_from(count)
            .ok()
            .filter(|&len| len <= MAX_RANGE_DATES)
            .ok_or(DateError::TooManyRangeDates(count))?;
        Ok(Self {
            start,
            step,
            front: 0,
            back: len,
        })
    }

    /// Date-time `index` of the whole range.
    fn at(&self, index: usize) -> DateTime {
        i64::try_from(index)
            .ok()
            .and_then(|index| index.checked_mul(self.step.total_micros()))
            .and_then(|offset| offset.checked_add(self.start.timestamp_micros()))
            .and_then(|micros| DateTime::from_timestamp_micros(micros).ok())
            .expect("the date-times of a range lie between its start and its end")
    }
}

impl Iterator for DateRange {
    type Item = DateTime;

    fn next(&mut self) -> Option<DateTime> {
        (self.front < self.back).then(|| {
            self.front += 1;
            self.at(self.front - 1)
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.back - self.front;
        (len, Some(len))
    }
}

impl DoubleEndedIterator for DateRange {
    fn next_back(&mut self) -> Option<DateTime> {
        (self.front < self.back).then(|| {
            self.back -= 1;
            self.at(self.back)
        })
    }
}

impl ExactSizeIterator for DateRange {}

impl FusedIterator for DateRange {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::{TimeZone, ZonedDateTime, drange, num2date};

    fn at(day: u32, hour: u32, micro: u32) -> DateTime {
        DateTime::new(2011, 1, day, hour, 0, 0, micro).expect("a date-time")
    }

    #[test]
    fn ranges_are_half_open() {
        let hour = TimeDelta::from_hours(1);
        let ends = [at(2, 0, 0), at(1, 23, 999_999), at(2, 0, 1)];
        let lens = ends.map(|end| DateRange::new(at(1, 0, 0), end, hour).map(|r| r.len()));
        assert_eq!(lens, [Ok(24), Ok(24), Ok(25)]);
        let four = DateRange::new(at(1, 0, 0), at(2, 0, 0), TimeDelta::from_hours(4));
        let hours: Vec<u32> = four.expect("a range").map(DateTime::hour).collect();
        assert_eq!(hours, [0, 4, 8, 12, 16, 20]);
    }

    /// matplotlib's `test_drange`: the date numbers `drange` spaces out
    /// evenly stand for the date-times of the range.
    #[test]
    fn the_date_times_are_those_of_drange() {
        let zoned = ZonedDateTime::utc;
        for (end, step) in [
            (at(2, 0, 0), TimeDelta::from_hours(1)),
            (at(1, 23, 999_999), TimeDelta::from_hours(1)),
            (at(2, 0, 1), TimeDelta::from_hours(1)),
            (at(2, 0, 0), TimeDelta::from_hours(4)),
        ] {
            let range = DateRange::new(at(1, 0, 0), end, step).expect("a range");
            let numbers = drange(&zoned(at(1, 0, 0)), &zoned(end), step).expect("numbers");
            let drange_dates: Vec<DateTime> = (numbers.iter())
                .map(|&x| num2date(x, &TimeZone::Utc).expect("a date").local())
                .collect();
            assert_eq!(range.collect::<Vec<_>>(), drange_dates, "{end}");
        }
    }

    #[test]
    fn reversed_and_empty_spans_give_nothing() {
        let day = TimeDelta::from_hours(24);
        let empty = |end| DateRange::new(at(5, 0, 0), end, day).map(|r| r.len());
        assert_eq!(empty(at(5, 0, 0)), Ok(0));
        assert_eq!(empty(at(1, 0, 0)), Ok(0));
        let mut range = DateRange::new(at(1, 0, 0), at(4, 0, 0), day).expect("a range");
        assert_eq!(range.next_back(), Some(at(3, 0, 0)));
        assert_eq!(range.next(), Some(at(1, 0, 0)));
        assert_eq!(
            (range.next(), range.next(), range.next_back()),
            (Some(at(2, 0, 0)), None, None)
        );
    }

    #[test]
    fn absurd_steps_and_spans_are_refused() {
        let (start, end) = (at(1, 0, 0), at(2, 0, 0));
        for step in [TimeDelta::ZERO, TimeDelta::from_secs(-1)] {
            let error = DateRange::new(start, end, step).err();
            assert_eq!(error, Some(DateError::NonPositiveStep(step)));
        }
        let first = DateTime::new(1, 1, 1, 0, 0, 0, 0).expect("the first date-time");
        let last = DateTime::new(9999, 12, 31, 23, 59, 59, 999_999).expect("the last");
        let micro = TimeDelta::from_micros(1);
        let count = (last - first).total_micros().unsigned_abs();
        let error = DateRange::new(first, last, micro).err();
        assert_eq!(error, Some(DateError::TooManyRangeDates(count)));
        let whole = DateRange::new(first, last, TimeDelta::from_micros(i64::MAX));
        assert_eq!(whole.map(Iterator::collect::<Vec<_>>), Ok(vec![first]));
        let limit = i64::try_from(MAX_RANGE_DATES).expect("a count");
        let widest = first
            .checked_add(TimeDelta::from_micros(limit))
            .expect("a date-time");
        let mut range = DateRange::new(first, widest, micro).expect("the widest range");
        assert_eq!(range.len(), MAX_RANGE_DATES);
        assert_eq!(
            range.next_back(),
            Some(widest.checked_sub(micro).expect("a date-time"))
        );
    }
}
