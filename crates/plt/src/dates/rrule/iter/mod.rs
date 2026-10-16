//! Occurrence generation: a faithful port of `dateutil.rrule.rrule._iter`.
//!
//! Each step expands one period (a year, month, week, day, or one clock
//! position for sub-daily rules) into candidate days, filters them through
//! the `by*` rules, combines the survivors with the times of day and then
//! advances to the next period.

use std::collections::VecDeque;

mod clock;

use super::yearinfo::YearInfo;
use super::{Frequency, RRule};
use crate::dates::calendar::days_in_month;
use crate::dates::datetime::MAX_YEAR;
use crate::dates::{Date, DateError, DateTime, Time};

/// Iterator over the occurrences of an [`RRule`], in increasing order.
///
/// Yields an error (and then stops) if the rule turns out to be empty
/// while iterating or an occurrence would fall after year 9999.
#[derive(Debug)]
pub struct RRuleIter<'a> {
    rule: &'a RRule,
    info: YearInfo,
    /// dateutil's `nwdaymask` for the current period, if any.
    nth_mask: Option<Vec<bool>>,
    nth_key: Option<(i64, u32)>,
    year: i64,
    month: u32,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
    /// Weekday index (Monday = 0) of `day`, maintained for weekly rules.
    weekday: i64,
    timeset: Vec<Time>,
    pending: VecDeque<DateTime>,
    remaining: Option<u32>,
    finished: bool,
}

impl<'a> RRuleIter<'a> {
    pub(super) fn new(rule: &'a RRule) -> Self {
        let start = rule.dtstart;
        let mut iter = Self {
            rule,
            info: YearInfo::new(i64::from(start.year())),
            nth_mask: None,
            nth_key: None,
            year: i64::from(start.year()),
            month: start.month(),
            day: i64::from(start.day()),
            hour: i64::from(start.hour()),
            minute: i64::from(start.minute()),
            second: i64::from(start.second()),
            weekday: i64::from(start.weekday().index()),
            timeset: Vec::new(),
            pending: VecDeque::new(),
            remaining: rule.count,
            finished: false,
        };
        iter.rebuild();
        iter.timeset = if rule.freq < Frequency::Hourly {
            rule.timeset.clone()
        } else if iter.start_time_excluded() {
            Vec::new()
        } else {
            iter.clock_timeset()
        };
        iter
    }

    /// dateutil's `_iterinfo.rebuild` for the current year and month.
    fn rebuild(&mut self) {
        if self.info.year != self.year {
            self.info = YearInfo::new(self.year);
        }
        let rule = self.rule;
        if rule.bynweekday.is_empty() || self.nth_key == Some((self.year, self.month)) {
            return;
        }
        self.nth_key = Some((self.year, self.month));
        let ranges: Vec<(i64, i64)> = match rule.freq {
            Frequency::Yearly if rule.bymonth.is_empty() => vec![(0, self.info.len)],
            Frequency::Yearly => rule
                .bymonth
                .iter()
                .map(|&m| self.info.month_range(m))
                .collect(),
            Frequency::Monthly => vec![self.info.month_range(self.month)],
            _ => return,
        };
        let mut mask = vec![false; self.info.len as usize];
        for (first, end) in ranges {
            let last = end - 1;
            for &(weekday, n) in &rule.bynweekday {
                let target = i64::from(weekday.index());
                let i = if n < 0 {
                    let i = last + (i64::from(n) + 1) * 7;
                    i - (i64::from(self.info.weekday(i).index()) - target).rem_euclid(7)
                } else {
                    let i = first + (i64::from(n) - 1) * 7;
                    i + (7 - i64::from(self.info.weekday(i).index()) + target).rem_euclid(7)
                };
                if (first..=last).contains(&i) {
                    mask[i as usize] = true;
                }
            }
        }
        self.nth_mask = Some(mask);
    }

    /// The day-index range `[start, end)` of the current period.
    fn period(&self) -> (i64, i64) {
        let info = &self.info;
        let current = || {
            crate::dates::calendar::days_from_civil(self.year, self.month, self.day as u32)
                - info.start
        };
        match self.rule.freq {
            Frequency::Yearly => (0, info.len),
            Frequency::Monthly => info.month_range(self.month),
            Frequency::Weekly => {
                let start = current();
                let mut end = start;
                for _ in 0..7 {
                    end += 1;
                    if info.weekday(end) == self.rule.wkst {
                        break;
                    }
                }
                (start, end)
            }
            _ => {
                let day = current();
                (day, day + 1)
            }
        }
    }

    /// Whether day index `i` fails one of the day-level `by*` rules.
    fn excluded(&self, i: i64) -> bool {
        let rule = self.rule;
        let info = &self.info;
        let missing_yearday = |day: i64, from_end: i64| {
            !rule.byyearday.contains(&(day as i32)) && !rule.byyearday.contains(&(from_end as i32))
        };
        (!rule.bymonth.is_empty() && !rule.bymonth.contains(&info.month_of(i)))
            || (!rule.byweekday.is_empty() && !rule.byweekday.contains(&info.weekday(i)))
            || self
                .nth_mask
                .as_ref()
                .is_some_and(|mask| !mask.get(i as usize).copied().unwrap_or(false))
            || ((!rule.bymonthday.is_empty() || !rule.bynmonthday.is_empty())
                && !rule.bymonthday.contains(&info.month_day(i))
                && !rule.bynmonthday.contains(&info.neg_month_day(i)))
            || (!rule.byyearday.is_empty()
                && if i < info.len {
                    missing_yearday(i + 1, i - info.len)
                } else {
                    missing_yearday(i + 1 - info.len, i - info.len - info.next_len)
                })
    }

    fn date_at(&self, i: i64) -> Result<Date, DateError> {
        Date::from_days_since_epoch(self.info.start + i)
    }

    /// Queues `occurrence` unless it ends the iteration; returns `false`
    /// once the rule is exhausted.
    fn emit(&mut self, occurrence: DateTime) -> bool {
        if self.rule.until.is_some_and(|until| occurrence > until) {
            self.finished = true;
            return false;
        }
        if occurrence >= self.rule.dtstart {
            if let Some(remaining) = self.remaining.as_mut() {
                if *remaining == 0 {
                    self.finished = true;
                    return false;
                }
                *remaining -= 1;
            }
            self.pending.push_back(occurrence);
        }
        true
    }

    /// Whether the current period starts after `until`, so no further
    /// occurrence can be produced (a shortcut dateutil does not take; it
    /// gives the same result without scanning to year 9999).
    fn past_until(&self) -> bool {
        let Some(until) = self.rule.until else {
            return false;
        };
        let (month, day) = match self.rule.freq {
            Frequency::Yearly => (1, 1),
            Frequency::Monthly => (self.month, 1),
            _ => (self.month, self.day as u32),
        };
        Date::new(self.year as i32, month, day)
            .map_or(true, |date| DateTime::midnight(date) > until)
    }

    /// Expands the current period into `pending`, then advances.
    fn step(&mut self) -> Result<(), DateError> {
        if self.past_until() {
            self.finished = true;
            return Ok(());
        }
        let (start, end) = self.period();
        let days: Vec<i64> = (start..end).filter(|&i| !self.excluded(i)).collect();
        let filtered = days.len() as i64 != end - start;
        if !self.output(&days)? {
            return Ok(());
        }
        self.advance(filtered)
    }

    /// Emits the occurrences of the current period; `false` if exhausted.
    fn output(&mut self, days: &[i64]) -> Result<bool, DateError> {
        let timeset = std::mem::take(&mut self.timeset);
        let result = if !self.rule.bysetpos.is_empty() && !timeset.is_empty() {
            self.output_positions(days, &timeset)
        } else {
            self.output_all(days, &timeset)
        };
        self.timeset = timeset;
        result
    }

    fn output_all(&mut self, days: &[i64], timeset: &[Time]) -> Result<bool, DateError> {
        for &i in days {
            let date = self.date_at(i)?;
            for &time in timeset {
                if !self.emit(DateTime::from_parts(date, time)) {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    /// `bysetpos`: picks occurrences by position within the period, with
    /// Python's negative indexing.
    fn output_positions(&mut self, days: &[i64], timeset: &[Time]) -> Result<bool, DateError> {
        let times = timeset.len() as i64;
        let mut chosen: Vec<DateTime> = Vec::new();
        for &pos in &self.rule.bysetpos {
            let flat = if pos < 0 {
                i64::from(pos)
            } else {
                i64::from(pos) - 1
            };
            let (day_pos, time_pos) = (flat.div_euclid(times), flat.rem_euclid(times));
            let index = if day_pos < 0 {
                days.len() as i64 + day_pos
            } else {
                day_pos
            };
            let Some(&i) = usize::try_from(index)
                .ok()
                .and_then(|index| days.get(index))
            else {
                continue;
            };
            let occurrence = DateTime::from_parts(self.date_at(i)?, timeset[time_pos as usize]);
            if !chosen.contains(&occurrence) {
                chosen.push(occurrence);
            }
        }
        chosen.sort_unstable();
        Ok(chosen.into_iter().all(|occurrence| self.emit(occurrence)))
    }

    /// Moves to the next period (dateutil's "Handle frequency and
    /// interval" block).
    fn advance(&mut self, filtered: bool) -> Result<(), DateError> {
        let interval = i64::from(self.rule.interval);
        let mut fix_day = false;
        match self.rule.freq {
            Frequency::Yearly => {
                self.year += interval;
                if self.year > i64::from(MAX_YEAR) {
                    self.finished = true;
                    return Ok(());
                }
                self.rebuild();
            }
            Frequency::Monthly => {
                let months = i64::from(self.month) + interval;
                if months > 12 {
                    self.year += (months - 1).div_euclid(12);
                    if self.year > i64::from(MAX_YEAR) {
                        self.finished = true;
                        return Ok(());
                    }
                }
                self.month = ((months - 1).rem_euclid(12) + 1) as u32;
                self.rebuild();
            }
            Frequency::Weekly => {
                let wkst = i64::from(self.rule.wkst.index());
                self.day += if wkst > self.weekday {
                    -(self.weekday + 1 + (6 - wkst)) + interval * 7
                } else {
                    -(self.weekday - wkst) + interval * 7
                };
                self.weekday = wkst;
                fix_day = true;
            }
            Frequency::Daily => {
                self.day += interval;
                fix_day = true;
            }
            Frequency::Hourly => fix_day = self.advance_hourly(filtered, interval)?,
            Frequency::Minutely => fix_day = self.advance_minutely(filtered, interval)?,
            Frequency::Secondly => fix_day = self.advance_secondly(filtered, interval)?,
        }
        if fix_day {
            self.fix_day();
        }
        Ok(())
    }

    /// Rolls an overflowing day into the following months.
    fn fix_day(&mut self) {
        if self.day <= 28 {
            return;
        }
        let mut month_len = i64::from(days_in_month(self.year, self.month));
        if self.day <= month_len {
            return;
        }
        while self.day > month_len {
            self.day -= month_len;
            self.month += 1;
            if self.month == 13 {
                self.month = 1;
                self.year += 1;
                if self.year > i64::from(MAX_YEAR) {
                    self.finished = true;
                    return;
                }
            }
            month_len = i64::from(days_in_month(self.year, self.month));
        }
        self.rebuild();
    }
}

impl Iterator for RRuleIter<'_> {
    type Item = Result<DateTime, DateError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(occurrence) = self.pending.pop_front() {
                return Some(Ok(occurrence));
            }
            if self.finished {
                return None;
            }
            if let Err(err) = self.step() {
                self.finished = true;
                return Some(Err(err));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{NthWeekday, RRuleSpec};
    use super::*;
    use crate::dates::calendar::Weekday;

    fn dt(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> DateTime {
        DateTime::new(y, mo, d, h, mi, s, 0).unwrap()
    }

    fn occurrences(spec: &RRuleSpec, limit: usize) -> Vec<String> {
        RRule::new(spec)
            .unwrap()
            .iter()
            .take(limit)
            .map(|o| o.unwrap().to_string())
            .collect()
    }

    fn spec(freq: Frequency, start: DateTime) -> RRuleSpec {
        RRuleSpec {
            dtstart: Some(start),
            ..RRuleSpec::new(freq)
        }
    }

    #[test]
    fn monthly_skips_missing_days() {
        // dateutil docstring: monthly from 2014-12-31 skips short months.
        let got = occurrences(&spec(Frequency::Monthly, dt(2014, 12, 31, 0, 0, 0)), 4);
        assert_eq!(
            got,
            [
                "2014-12-31T00:00:00",
                "2015-01-31T00:00:00",
                "2015-03-31T00:00:00",
                "2015-05-31T00:00:00"
            ]
        );
    }

    #[test]
    fn last_friday_of_each_month() {
        let mut rule = spec(Frequency::Monthly, dt(2020, 1, 1, 0, 0, 0));
        rule.byweekday = vec![Weekday::Friday.into()];
        rule.bysetpos = vec![-1];
        assert_eq!(
            occurrences(&rule, 3),
            [
                "2020-01-31T00:00:00",
                "2020-02-28T00:00:00",
                "2020-03-27T00:00:00"
            ]
        );
        rule.bysetpos.clear();
        rule.byweekday = vec![NthWeekday {
            weekday: Weekday::Monday,
            nth: Some(2),
        }];
        assert_eq!(
            occurrences(&rule, 2),
            ["2020-01-13T00:00:00", "2020-02-10T00:00:00"]
        );
    }

    #[test]
    fn sub_daily_rules_wrap_days_and_honour_until() {
        let mut rule = spec(Frequency::Hourly, dt(2020, 2, 28, 22, 0, 0));
        rule.interval = 3;
        assert_eq!(
            occurrences(&rule, 3),
            [
                "2020-02-28T22:00:00",
                "2020-02-29T01:00:00",
                "2020-02-29T04:00:00"
            ]
        );
        let mut rule = spec(Frequency::Minutely, dt(2020, 1, 1, 23, 0, 0));
        rule.interval = 90;
        rule.until = Some(dt(2020, 1, 2, 3, 0, 0));
        assert_eq!(
            occurrences(&rule, 10),
            [
                "2020-01-01T23:00:00",
                "2020-01-02T00:30:00",
                "2020-01-02T02:00:00"
            ]
        );
        let mut rule = spec(Frequency::Secondly, dt(2020, 1, 1, 0, 0, 58));
        rule.bysecond = vec![0, 30];
        rule.count = Some(2);
        assert_eq!(
            occurrences(&rule, 10),
            ["2020-01-01T00:01:00", "2020-01-01T00:01:30"]
        );
    }

    #[test]
    fn yearly_yeardays_and_weekly_crossing_new_year() {
        let mut rule = spec(Frequency::Daily, dt(2020, 1, 1, 0, 0, 0));
        rule.byyearday = vec![1, -1];
        assert_eq!(
            occurrences(&rule, 3),
            [
                "2020-01-01T00:00:00",
                "2020-12-31T00:00:00",
                "2021-01-01T00:00:00"
            ]
        );
        let mut rule = spec(Frequency::Weekly, dt(2020, 12, 28, 0, 0, 0));
        rule.byweekday = vec![Weekday::Friday.into(), Weekday::Sunday.into()];
        assert_eq!(
            occurrences(&rule, 3),
            [
                "2021-01-01T00:00:00",
                "2021-01-03T00:00:00",
                "2021-01-08T00:00:00"
            ]
        );
    }

    #[test]
    fn impossible_rules_end_without_looping_forever() {
        let mut rule = spec(Frequency::Daily, dt(2020, 1, 1, 0, 0, 0));
        rule.bymonth = vec![2];
        rule.bymonthday = vec![30];
        rule.until = Some(dt(2030, 1, 1, 0, 0, 0));
        assert_eq!(occurrences(&rule, 5), Vec::<String>::new());
        let mut rule = spec(Frequency::Yearly, dt(9998, 1, 1, 0, 0, 0));
        rule.interval = 1;
        assert_eq!(occurrences(&rule, 5).len(), 2);
        let all = RRule::new(&spec(Frequency::Daily, dt(2020, 1, 1, 0, 0, 0))).unwrap();
        let found = all
            .between(dt(2020, 1, 2, 0, 0, 0), dt(2020, 1, 4, 0, 0, 0), false)
            .unwrap();
        assert_eq!(found, vec![dt(2020, 1, 3, 0, 0, 0)]);
    }
}
