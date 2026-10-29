//! Per-year lookup tables for rule iteration (dateutil's `_iterinfo`).
//!
//! Days are indexed from 0 (January 1). Every table extends seven days
//! into the next January so weekly periods can cross the year boundary.

use crate::dates::calendar::{self, Weekday};

/// Calendar facts about one year, indexed by day of year.
#[derive(Clone, Debug)]
pub(super) struct YearInfo {
    /// The year.
    pub(super) year: i64,
    /// Days in the year.
    pub(super) len: i64,
    /// Days in the following year.
    pub(super) next_len: i64,
    /// Days from 1970-01-01 to January 1.
    pub(super) start: i64,
    /// Day index of Western Easter Sunday.
    pub(super) easter: i64,
    /// The weekday index (Monday = 0) of January 1.
    first_weekday: i64,
    /// Day index of the first day of each month, plus the year length.
    month_start: [i64; 13],
    /// Month of each day.
    month: Vec<u8>,
    /// Day of the month of each day.
    month_day: Vec<i8>,
    /// Day of the month counted from its end (`-1` = last day).
    neg_month_day: Vec<i8>,
}

impl YearInfo {
    pub(super) fn new(year: i64) -> Self {
        let len = i64::from(calendar::days_in_year(year));
        let start = calendar::days_from_civil(year, 1, 1);
        let mut month_start = [len; 13];
        let mut month = Vec::with_capacity(len as usize + 7);
        let mut month_day = Vec::with_capacity(len as usize + 7);
        let mut neg_month_day = Vec::with_capacity(len as usize + 7);
        for m in 1..=12 {
            month_start[m as usize - 1] = i64::from(calendar::days_before_month(year, m));
            let days = calendar::days_in_month(year, m);
            for d in 1..=days {
                month.push(m as u8);
                month_day.push(d as i8);
                neg_month_day.push(d as i8 - days as i8 - 1);
            }
        }
        for d in 1..=7_i8 {
            month.push(1);
            month_day.push(d);
            neg_month_day.push(d - 32);
        }
        Self {
            year,
            len,
            next_len: i64::from(calendar::days_in_year(year + 1)),
            start,
            easter: {
                let (month, day) = calendar::easter(year);
                calendar::days_from_civil(year, month, day) - start
            },
            first_weekday: i64::from(Weekday::from_days_since_epoch(start).index()),
            month_start,
            month,
            month_day,
            neg_month_day,
        }
    }

    /// The day-index range `[first, end)` of `month` (1-12).
    pub(super) fn month_range(&self, month: u32) -> (i64, i64) {
        let m = month as usize;
        (self.month_start[m - 1], self.month_start[m])
    }

    /// The month (1-12) of day `i`.
    pub(super) fn month_of(&self, i: i64) -> u32 {
        u32::from(self.month[i as usize])
    }

    /// The day of the month of day `i`.
    pub(super) fn month_day(&self, i: i64) -> i32 {
        i32::from(self.month_day[i as usize])
    }

    /// The day of the month of day `i`, counted from the month's end.
    pub(super) fn neg_month_day(&self, i: i64) -> i32 {
        i32::from(self.neg_month_day[i as usize])
    }

    /// The weekday of day `i` (any index, even outside the year).
    pub(super) fn weekday(&self, i: i64) -> Weekday {
        Weekday::from_index(self.first_weekday + i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_cover_the_year_and_next_week() {
        let info = YearInfo::new(2020);
        assert_eq!((info.len, info.next_len), (366, 365));
        assert_eq!(info.month_range(2), (31, 60));
        assert_eq!(info.month_range(12), (335, 366));
        assert_eq!(
            (
                info.month_of(59),
                info.month_day(59),
                info.neg_month_day(59)
            ),
            (2, 29, -1)
        );
        assert_eq!(
            (
                info.month_of(366),
                info.month_day(366),
                info.neg_month_day(366)
            ),
            (1, 1, -31)
        );
        assert_eq!(info.weekday(0), Weekday::Wednesday);
        assert_eq!(info.weekday(-1), Weekday::Tuesday);
        let info = YearInfo::new(2021);
        assert_eq!(info.month_range(3), (59, 90));
    }
}
