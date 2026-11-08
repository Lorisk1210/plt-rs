//! How many ticks fit along an axis (matplotlib's `Axis.get_tick_space`).

use super::TickerError;

/// How many ticks fit along an axis, counted as matplotlib's
/// `Axis.get_tick_space` counts them: `int(np.floor(length / size))`, the
/// axis' length in points over the room one tick label takes.
///
/// The count is negative along an axes of negative size, and there is
/// none along an axis whose length is no number (an axes placed at NaN):
/// matplotlib's `int` then raises where a locator asks for the count,
/// which [`count`](Self::count) reproduces.
///
/// ```
/// use pltrs::ticker::TickSpace;
/// assert_eq!(TickSpace::along(100.0, 30.0).count(), Ok(3));
/// assert_eq!(TickSpace::along(-100.0, 30.0).count(), Ok(-4));
/// assert!(TickSpace::along(f64::NAN, 30.0).count().is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TickSpace(f64);

pltrs_core::archive_struct!(TickSpace(v0));

impl TickSpace {
    /// Room for `count` ticks.
    pub const fn of(count: i64) -> Self {
        Self(count as f64)
    }

    /// The room along an axis `length` points long whose tick labels take
    /// `label_room` points each; matplotlib's `2**31 - 1` ticks when they
    /// take none.
    pub fn along(length: f64, label_room: f64) -> Self {
        if label_room > 0.0 {
            Self((length / label_room).floor())
        } else {
            Self::of(i64::from(i32::MAX))
        }
    }

    /// The number of ticks; Python's `ValueError` (a NaN) or
    /// `OverflowError` (an infinity) converting a length that is no finite
    /// number to an integer.
    ///
    /// # Errors
    ///
    /// [`TickerError::NotAnInteger`] when the room is no finite number.
    pub fn count(self) -> Result<i64, TickerError> {
        if self.0.is_finite() {
            // The callers clip the count to a few ticks, so a count beyond
            // `i64` (a Python int) saturates harmlessly.
            Ok(self.0 as i64)
        } else {
            Err(TickerError::NotAnInteger {
                infinite: self.0.is_infinite(),
            })
        }
    }

    /// The count clipped to `[low, high]` (numpy's `np.clip(space, low,
    /// high)`, which is `min(max(space, low), high)` even when `low >
    /// high`).
    ///
    /// # Errors
    ///
    /// As [`count`](Self::count).
    pub fn clipped(self, low: usize, high: usize) -> Result<usize, TickerError> {
        let count = self.count()?;
        let low = i64::try_from(low).unwrap_or(i64::MAX);
        let high = i64::try_from(high).unwrap_or(i64::MAX);
        Ok(usize::try_from(count.max(low).min(high)).unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::TickSpace;
    use crate::ticker::TickerError;

    #[test]
    fn counts_floor_the_quotient_with_its_sign() {
        assert_eq!(TickSpace::along(89.9, 30.0).count(), Ok(2));
        assert_eq!(TickSpace::along(-0.5, 30.0).count(), Ok(-1));
        assert_eq!(TickSpace::along(0.0, 30.0).count(), Ok(0));
        assert_eq!(TickSpace::of(7).count(), Ok(7));
    }

    #[test]
    fn labels_taking_no_room_leave_room_for_matplotlibs_maximum() {
        assert_eq!(TickSpace::along(100.0, 0.0).count(), Ok(2_147_483_647));
        assert_eq!(TickSpace::along(f64::NAN, -1.0).count(), Ok(2_147_483_647));
    }

    #[test]
    fn lengths_that_are_no_numbers_have_no_count() {
        assert_eq!(
            TickSpace::along(f64::NAN, 30.0).count(),
            Err(TickerError::NotAnInteger { infinite: false })
        );
        assert_eq!(
            TickSpace::along(f64::INFINITY, 30.0).count(),
            Err(TickerError::NotAnInteger { infinite: true })
        );
        assert!(TickSpace::along(f64::NAN, 30.0).clipped(1, 9).is_err());
    }

    #[test]
    fn clipping_is_numpys_even_for_crossed_bounds() {
        assert_eq!(TickSpace::of(-5).clipped(1, 9), Ok(1));
        assert_eq!(TickSpace::of(50).clipped(2, 9), Ok(9));
        assert_eq!(TickSpace::of(5).clipped(12, 9), Ok(9));
        assert_eq!(TickSpace::along(1e300, 1.0).clipped(2, 9), Ok(9));
    }
}
