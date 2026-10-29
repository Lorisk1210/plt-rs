//! Clock stepping for sub-daily rules (the hourly, minutely and secondly
//! branches of dateutil's "Handle frequency and interval" block).

use super::RRuleIter;
use crate::dates::rrule::{Frequency, gcd};
use crate::dates::{DateError, Time};

impl RRuleIter<'_> {
    /// Whether `dtstart`'s own clock fields fail a `by*` rule at or below
    /// the rule's frequency (then the first period yields nothing).
    pub(super) fn start_time_excluded(&self) -> bool {
        let rule = self.rule;
        let misses = |set: &[u32], value: i64| !set.is_empty() && !set.contains(&(value as u32));
        misses(&rule.byhour, self.hour)
            || (rule.freq >= Frequency::Minutely && misses(&rule.byminute, self.minute))
            || (rule.freq >= Frequency::Secondly && misses(&rule.bysecond, self.second))
    }

    /// The times of day for the current clock position of a sub-daily rule.
    pub(super) fn clock_timeset(&self) -> Vec<Time> {
        let rule = self.rule;
        let (hour, minute, second) = (self.hour as u32, self.minute as u32, self.second as u32);
        let mut times: Vec<Time> = match rule.freq {
            Frequency::Hourly => rule
                .byminute
                .iter()
                .flat_map(|&m| rule.bysecond.iter().map(move |&s| (hour, m, s)))
                .filter_map(|(h, m, s)| Time::new(h, m, s, 0).ok())
                .collect(),
            Frequency::Minutely => rule
                .bysecond
                .iter()
                .filter_map(|&s| Time::new(hour, minute, s, 0).ok())
                .collect(),
            _ => Time::new(hour, minute, second, 0)
                .ok()
                .into_iter()
                .collect(),
        };
        times.sort_unstable();
        times
    }

    pub(super) fn advance_hourly(
        &mut self,
        filtered: bool,
        interval: i64,
    ) -> Result<bool, DateError> {
        if filtered {
            self.hour += (23 - self.hour).div_euclid(interval) * interval;
        }
        let days = self.step_unit(Unit::Hour, interval)?;
        self.day += days;
        self.timeset = self.clock_timeset();
        Ok(days != 0)
    }

    pub(super) fn advance_minutely(
        &mut self,
        filtered: bool,
        interval: i64,
    ) -> Result<bool, DateError> {
        if filtered {
            self.minute += (1439 - (self.hour * 60 + self.minute)).div_euclid(interval) * interval;
        }
        let mut fix_day = false;
        let repeat = 1440 / i64::from(gcd(self.rule.interval, 1440));
        let mut valid = false;
        for _ in 0..repeat {
            let hours = self.step_unit(Unit::Minute, interval)?;
            let days = (self.hour + hours).div_euclid(24);
            self.hour = (self.hour + hours).rem_euclid(24);
            if days != 0 {
                self.day += days;
                fix_day = true;
            }
            if matches(&self.rule.byhour, self.hour) {
                valid = true;
                break;
            }
        }
        if !valid {
            return Err(DateError::EmptyRule("interval and byhour never coincide"));
        }
        self.timeset = self.clock_timeset();
        Ok(fix_day)
    }

    pub(super) fn advance_secondly(
        &mut self,
        filtered: bool,
        interval: i64,
    ) -> Result<bool, DateError> {
        if filtered {
            let elapsed = self.hour * 3600 + self.minute * 60 + self.second;
            self.second += (86_399 - elapsed).div_euclid(interval) * interval;
        }
        let mut fix_day = false;
        let repeat = 86_400 / i64::from(gcd(self.rule.interval, 86_400));
        let mut valid = false;
        for _ in 0..repeat {
            let minutes = self.step_unit(Unit::Second, interval)?;
            let hours = (self.minute + minutes).div_euclid(60);
            self.minute = (self.minute + minutes).rem_euclid(60);
            if hours != 0 {
                self.hour += hours;
                let days = self.hour.div_euclid(24);
                self.hour = self.hour.rem_euclid(24);
                if days != 0 {
                    self.day += days;
                    fix_day = true;
                }
            }
            let rule = self.rule;
            if matches(&rule.byhour, self.hour)
                && matches(&rule.byminute, self.minute)
                && matches(&rule.bysecond, self.second)
            {
                valid = true;
                break;
            }
        }
        if !valid {
            return Err(DateError::EmptyRule(
                "interval, byhour and byminute never coincide",
            ));
        }
        self.timeset = self.clock_timeset();
        Ok(fix_day)
    }

    /// Steps the rule's own clock unit by `interval` (to the next allowed
    /// value when a `by*` set is given, dateutil's `__mod_distance`),
    /// returning the carry into the next larger unit.
    fn step_unit(&mut self, unit: Unit, interval: i64) -> Result<i64, DateError> {
        let rule = self.rule;
        let (value, allowed, base) = match unit {
            Unit::Hour => (&mut self.hour, &rule.byhour, 24),
            Unit::Minute => (&mut self.minute, &rule.byminute, 60),
            Unit::Second => (&mut self.second, &rule.bysecond, 60),
        };
        if allowed.is_empty() {
            let carry = (*value + interval).div_euclid(base);
            *value = (*value + interval).rem_euclid(base);
            return Ok(carry);
        }
        let mut carry = 0;
        for _ in 0..base {
            carry += (*value + interval).div_euclid(base);
            *value = (*value + interval).rem_euclid(base);
            if allowed.contains(&(*value as u32)) {
                return Ok(carry);
            }
        }
        Err(DateError::EmptyRule(
            "by-values unreachable with this interval",
        ))
    }
}

/// Whether `value` passes a `by*` set (an empty set allows everything).
fn matches(set: &[u32], value: i64) -> bool {
    set.is_empty() || set.contains(&(value as u32))
}

/// A clock unit a sub-daily rule steps through.
#[derive(Clone, Copy, Debug)]
enum Unit {
    Hour,
    Minute,
    Second,
}
