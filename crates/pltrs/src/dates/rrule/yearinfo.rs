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

    /// dateutil's `wnomask`: which days of the year (and of the next
    /// seven) fall in the weeks numbered `byweekno`, weeks starting on
    /// `wkst` and week 1 being the first with at least four days in the
    /// year; negative numbers count from the year's last week.
    pub(super) fn week_number_mask(&self, byweekno: &[i32], wkst: Weekday) -> Vec<bool> {
        let wkst = i64::from(wkst.index());
        let mut mask = vec![false; (self.len + 7) as usize];
        let first_wkst = (7 - self.first_weekday + wkst).rem_euclid(7);
        let (no1wkst, week_year_len) = if first_wkst >= 4 {
            // Week 1 starts last year: count the days taken from it.
            (0, self.len + (self.first_weekday - wkst).rem_euclid(7))
        } else {
            // The days before week 1 belong to last year's last week.
            (first_wkst, self.len - first_wkst)
        };
        let weeks = week_year_len / 7 + week_year_len % 7 / 4;
        let week_start = |n: i64| {
            let mut i = no1wkst + (n - 1) * 7;
            if no1wkst != first_wkst {
                i -= 7 - first_wkst;
            }
            i
        };
        let mut mark_week = |mut i: i64| {
            for _ in 0..7 {
                mask[i as usize] = true;
                i += 1;
                if i64::from(self.weekday(i).index()) == wkst {
                    break;
                }
            }
        };
        for &n in byweekno {
            let n = if n < 0 {
                i64::from(n) + weeks + 1
            } else {
                i64::from(n)
            };
            if 0 < n && n <= weeks {
                mark_week(if n > 1 { week_start(n) } else { no1wkst });
            }
        }
        if byweekno.contains(&1) {
            // Week 1 of next year may start in this one.
            let i = week_start(weeks + 1);
            if i < self.len {
                mark_week(i);
            }
        }
        if no1wkst != 0 {
            // The days of last year's last week in this year.
            let last_weeks = if byweekno.contains(&-1) {
                -1
            } else {
                let last_len = i64::from(calendar::days_in_year(self.year - 1));
                let last_first_weekday = (self.first_weekday - last_len).rem_euclid(7);
                if (7 - last_first_weekday + wkst).rem_euclid(7) >= 4 {
                    52 + (last_len + (last_first_weekday - wkst).rem_euclid(7)) % 7 / 4
                } else {
                    // dateutil counts with this year's length here.
                    52 + (self.len - no1wkst) % 7 / 4
                }
            };
            if byweekno.iter().any(|&n| i64::from(n) == last_weeks) {
                mask[..no1wkst as usize].fill(true);
            }
        }
        mask
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

    /// The day indices `mask` marks.
    fn marked(mask: &[bool]) -> Vec<usize> {
        mask.iter()
            .enumerate()
            .filter_map(|(i, &on)| on.then_some(i))
            .collect()
    }

    #[test]
    fn week_numbers_follow_iso_8601() {
        // 2021 starts on a Friday: its days up to Sunday 3 January are in
        // week 53 of 2020, and week 1 runs from Monday 4 January.
        let info = YearInfo::new(2021);
        let monday = Weekday::Monday;
        assert_eq!(
            marked(&info.week_number_mask(&[1], monday)),
            (3..10).collect::<Vec<_>>()
        );
        assert_eq!(marked(&info.week_number_mask(&[53], monday)), [0, 1, 2]);
        // Week 52 is the last of 2021, from Monday 27 December; dateutil
        // also counts the days of last year's week as week -1.
        assert_eq!(
            marked(&info.week_number_mask(&[-1], monday)),
            [0, 1, 2].into_iter().chain(360..367).collect::<Vec<_>>()
        );
        // 2020 starts on a Wednesday: week 1 starts on 30 December 2019,
        // and its 53 weeks end with the first three days of 2021.
        let info = YearInfo::new(2020);
        assert_eq!(
            marked(&info.week_number_mask(&[1], monday)),
            [0, 1, 2, 3, 4]
        );
        assert_eq!(
            marked(&info.week_number_mask(&[53], monday)),
            (362..369).collect::<Vec<_>>()
        );
        // Weeks starting on Sunday move week 1 of 2020 to 29 December 2019.
        assert_eq!(
            marked(&info.week_number_mask(&[1], Weekday::Sunday)),
            [0, 1, 2, 3]
        );
    }
}
