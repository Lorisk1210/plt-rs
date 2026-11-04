//! The POSIX `TZ` strings that end version 2+ `TZif` files
//! (`PST8PDT,M3.2.0,M11.1.0`): the rules a zone follows after its last
//! transition, as CPython 3.14's `zoneinfo` reads and applies them
//! (`_parse_tz_str` and `_TZStr`).

use crate::dates::DateError;
use crate::dates::calendar::{Weekday, days_from_civil, days_in_month, is_leap_year};

/// Seconds in a day.
const SECONDS_PER_DAY: i64 = 86_400;

/// One local time type: its offset east of UTC in seconds and its
/// abbreviation (`zoneinfo`'s `_ttinfo`, without the `dstoff` only
/// `dst()` reads).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct LocalType {
    /// Seconds east of UTC.
    pub(super) offset: i64,
    /// The abbreviation, e.g. `"PST"`.
    pub(super) abbreviation: String,
}

pltrs_core::archive_struct!(LocalType {
    offset,
    abbreviation
});

/// What a `TZ` string says: one local type, or standard and daylight
/// time with the rules switching between them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum TzRule {
    /// A constant local type.
    Fixed(LocalType),
    /// Standard and daylight time.
    Seasons(Seasons),
}

pltrs_core::archive_enum!(TzRule {
    Fixed(v0),
    Seasons(v0),
});

/// Standard and daylight time, and when daylight time starts and ends
/// each year (`zoneinfo`'s `_TZStr`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Seasons {
    standard: LocalType,
    daylight: LocalType,
    start: Transition,
    end: Transition,
}

pltrs_core::archive_struct!(Seasons {
    standard,
    daylight,
    start,
    end
});

/// The day of a transition in its year.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TransitionDay {
    /// `Jn` (`julian`, February 29 never counted) or `n`: CPython's
    /// `_DayOffset`.
    YearDay { day: i64, julian: bool },
    /// `Mm.w.d`: day `d` (0 is Sunday) of week `w` (5 is the last) of
    /// month `m` (`_CalendarOffset`).
    MonthWeekDay { month: u32, week: u32, weekday: u32 },
}

pltrs_core::archive_enum!(TransitionDay {
    YearDay { day, julian },
    MonthWeekDay {
        month,
        week,
        weekday
    },
});

/// A transition: its day, and its local time of day in seconds (2:00 by
/// default; negative or past 24 hours in version 3 files).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Transition {
    day: TransitionDay,
    time: i64,
}

pltrs_core::archive_struct!(Transition { day, time });

impl Transition {
    /// The transition in `year`, in seconds since 1970-01-01 on the local
    /// clock it is given in (`year_to_epoch`).
    fn in_year(self, year: i32) -> i64 {
        let year = i64::from(year);
        let days = match self.day {
            // CPython's `_post_epoch_days_before_year` counts from the
            // day before January 1.
            TransitionDay::YearDay { day, julian } => {
                let leap_skip = i64::from(julian && day >= 59 && is_leap_year(year));
                days_from_civil(year, 1, 1) - 1 + day + leap_skip
            }
            TransitionDay::MonthWeekDay {
                month,
                week,
                weekday,
            } => {
                let first = days_from_civil(year, month, 1);
                let first_weekday = i64::from(Weekday::from_days_since_epoch(first).index());
                let mut day = (i64::from(weekday) - (first_weekday + 1)).rem_euclid(7) + 1;
                day += i64::from(week - 1) * 7;
                if day > i64::from(days_in_month(year, month)) {
                    day -= 7;
                }
                first + day - 1
            }
        };
        days * SECONDS_PER_DAY + self.time
    }
}

impl Seasons {
    /// Daylight time's start and end in `year`, on the local clock.
    fn transitions(&self, year: i32) -> (i64, i64) {
        (self.start.in_year(year), self.end.in_year(year))
    }

    /// Daylight time minus standard time, in seconds.
    fn shift(&self) -> i64 {
        self.daylight.offset - self.standard.offset
    }

    /// The local type at the wall-clock timestamp `ts` of `year`, `fold`
    /// choosing between the two readings of a repeated wall time
    /// (`_get_trans_info`).
    pub(super) fn at_local(&self, ts: i64, year: i32, fold: bool) -> &LocalType {
        let (mut start, mut end) = self.transitions(year);
        let shift = self.shift();
        if fold == (shift >= 0) {
            end -= shift;
        } else {
            start += shift;
        }
        if in_season(start, end, ts) {
            &self.daylight
        } else {
            &self.standard
        }
    }

    /// The local type at the UTC timestamp `ts` of `year`, and whether
    /// its wall time is the second of a repeated one
    /// (`_get_trans_info_fromutc`).
    pub(super) fn at_utc(&self, ts: i64, year: i32) -> (&LocalType, bool) {
        let (start, end) = self.transitions(year);
        let (start, end) = (start - self.standard.offset, end - self.daylight.offset);
        let local = if in_season(start, end, ts) {
            &self.daylight
        } else {
            &self.standard
        };
        let shift = self.shift();
        let (ambiguous_start, ambiguous_end) = if shift > 0 {
            (end, end + shift)
        } else {
            (start, start - shift)
        };
        (local, (ambiguous_start..ambiguous_end).contains(&ts))
    }
}

/// Whether `ts` is in daylight time running from `start` to `end`, which
/// wraps around the new year when `end` comes first.
fn in_season(start: i64, end: i64, ts: i64) -> bool {
    if start < end {
        (start..end).contains(&ts)
    } else {
        !(end..start).contains(&ts)
    }
}

/// The error of a malformed `TZ` string.
fn invalid() -> DateError {
    DateError::InvalidTimeZoneData("invalid TZ string")
}

/// Parses a `TZ` string, `std offset [dst [offset] ,start[/time],end[/time]]`,
/// as CPython's `_parse_tz_str` does.
pub(super) fn parse(text: &str) -> Result<TzRule, DateError> {
    let (zones, rules) = match text.split_once(',') {
        Some((zones, rules)) => (zones, Some(rules)),
        None => (text, None),
    };
    let mut cursor = Cursor(zones);
    let standard_name = cursor.abbreviation().ok_or_else(invalid)?;
    let standard_offset = cursor.signed_time().ok_or_else(invalid)?;
    let standard_offset = posix_offset(standard_offset)?;
    let daylight = cursor
        .abbreviation()
        .map(|name| {
            let offset = match cursor.signed_time() {
                Some(time) => posix_offset(time)?,
                None => standard_offset + 3600,
            };
            Ok::<_, DateError>((name, offset))
        })
        .transpose()?;
    if !cursor.0.is_empty() {
        return Err(invalid());
    }
    let standard = LocalType {
        offset: standard_offset,
        abbreviation: standard_name.to_owned(),
    };
    let Some((daylight_name, daylight_offset)) = daylight else {
        return match rules {
            Some(_) => Err(invalid()),
            None => Ok(TzRule::Fixed(standard)),
        };
    };
    let (start, end) = rules
        .and_then(|rules| rules.split_once(','))
        .ok_or_else(invalid)?;
    Ok(TzRule::Seasons(Seasons {
        standard,
        daylight: LocalType {
            offset: daylight_offset,
            abbreviation: daylight_name.to_owned(),
        },
        start: transition(start)?,
        end: transition(end)?,
    }))
}

/// A POSIX offset (`_parse_tz_delta`): hours west of UTC, so `+5` is
/// five hours behind, at most 24 hours.
fn posix_offset((seconds, hours): (i64, i64)) -> Result<i64, DateError> {
    if hours > 24 {
        return Err(invalid());
    }
    Ok(-seconds)
}

/// A transition, `date[/time]` (`_parse_dst_start_end`).
fn transition(text: &str) -> Result<Transition, DateError> {
    let (date, time) = match text.split_once('/') {
        Some((date, time)) => (date, Some(time)),
        None => (text, None),
    };
    let day = transition_day(date).ok_or_else(invalid)?;
    let time = match time {
        None => 2 * 3600,
        Some(time) => {
            let mut cursor = Cursor(time);
            let (seconds, hours) = cursor.signed_time().ok_or_else(invalid)?;
            if !cursor.0.is_empty() || hours > 167 {
                return Err(invalid());
            }
            seconds
        }
    };
    Ok(Transition { day, time })
}

/// The day of a transition: `Mm.w.d`, `Jn` or `n`, as CPython validates
/// them.
fn transition_day(date: &str) -> Option<TransitionDay> {
    if let Some(rest) = date.strip_prefix('M') {
        let mut parts = rest.split('.');
        let mut part = |max_digits: usize| {
            let text = parts.next()?;
            (!text.is_empty()
                && text.len() <= max_digits
                && text.bytes().all(|b| b.is_ascii_digit()))
            .then(|| text.parse::<u32>().ok())
            .flatten()
        };
        let (month, week, weekday) = (part(2)?, part(1)?, part(1)?);
        let valid = parts.next().is_none()
            && (1..=12).contains(&month)
            && (1..=5).contains(&week)
            && weekday <= 6;
        return valid.then_some(TransitionDay::MonthWeekDay {
            month,
            week,
            weekday,
        });
    }
    let (digits, julian) = match date.strip_prefix('J') {
        Some(digits) => (digits, true),
        None => (date, false),
    };
    if digits.is_empty() || digits.len() > 3 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let day: i64 = digits.parse().ok()?;
    (i64::from(julian)..=365)
        .contains(&day)
        .then_some(TransitionDay::YearDay { day, julian })
}

/// A cursor over the zone part of a `TZ` string.
struct Cursor<'a>(&'a str);

impl<'a> Cursor<'a> {
    /// An abbreviation: letters, or `<...>` of letters, digits and signs.
    fn abbreviation(&mut self) -> Option<&'a str> {
        if let Some(rest) = self.0.strip_prefix('<') {
            let end = rest.find('>')?;
            let name = &rest[..end];
            let valid = !name.is_empty()
                && name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'-');
            if !valid {
                return None;
            }
            self.0 = &rest[end + 1..];
            return Some(name);
        }
        let end = self
            .0
            .find(|c: char| !c.is_ascii_alphabetic())
            .unwrap_or(self.0.len());
        let (name, rest) = self.0.split_at(end);
        (!name.is_empty()).then(|| {
            self.0 = rest;
            name
        })
    }

    /// `[+-]hhh[:mm[:ss]]` (one to three digits of hours): the signed
    /// seconds, with the hours for range checks.
    fn signed_time(&mut self) -> Option<(i64, i64)> {
        let mut rest = self.0;
        let sign = match rest.as_bytes().first() {
            Some(b'-') => -1,
            Some(b'+') => 1,
            _ => 0,
        };
        if sign != 0 {
            rest = &rest[1..];
        }
        let digits = |text: &'a str, min: usize, max: usize| {
            let count = text.bytes().take_while(u8::is_ascii_digit).count();
            (min..=max)
                .contains(&count)
                .then(|| (text[..count].parse::<i64>().ok(), &text[count..]))
                .and_then(|(value, rest)| Some((value?, rest)))
        };
        let (hours, mut rest) = digits(rest, 1, 3)?;
        let mut seconds = hours * 3600;
        for scale in [60, 1] {
            let Some(after) = rest.strip_prefix(':') else {
                break;
            };
            let (value, after) = digits(after, 2, 2)?;
            seconds += value * scale;
            rest = after;
        }
        self.0 = rest;
        Some((if sign < 0 { -seconds } else { seconds }, hours))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Seconds since the epoch of a local date-time.
    fn at(year: i64, month: u32, day: u32, hour: i64) -> i64 {
        days_from_civil(year, month, day) * SECONDS_PER_DAY + hour * 3600
    }

    fn seasons(text: &str) -> Seasons {
        match parse(text).unwrap() {
            TzRule::Seasons(seasons) => seasons,
            TzRule::Fixed(_) => panic!("{text} has daylight time"),
        }
    }

    #[test]
    fn us_rules_switch_on_the_second_and_first_sundays() {
        let pacific = seasons("PST8PDT,M3.2.0,M11.1.0");
        assert_eq!(pacific.standard.offset, -8 * 3600);
        assert_eq!(pacific.daylight.offset, -7 * 3600);
        assert_eq!(
            pacific.transitions(2020),
            (at(2020, 3, 8, 2), at(2020, 11, 1, 2))
        );
        assert_eq!(
            pacific
                .at_local(at(2020, 7, 1, 12), 2020, false)
                .abbreviation,
            "PDT"
        );
        assert_eq!(
            pacific
                .at_local(at(2020, 1, 1, 12), 2020, false)
                .abbreviation,
            "PST"
        );
        // 01:30 on 2020-11-01 happens twice: PDT first, PST with fold.
        let repeated = at(2020, 11, 1, 1) + 1800;
        assert_eq!(pacific.at_local(repeated, 2020, false).abbreviation, "PDT");
        assert_eq!(pacific.at_local(repeated, 2020, true).abbreviation, "PST");
        let (local, fold) = pacific.at_utc(at(2020, 11, 1, 9) + 1800, 2020);
        assert_eq!((local.abbreviation.as_str(), fold), ("PST", true));
    }

    #[test]
    fn southern_rules_wrap_around_the_new_year() {
        let sydney = seasons("AEST-10AEDT,M10.1.0,M4.1.0/3");
        assert_eq!(sydney.standard.offset, 10 * 3600);
        assert_eq!(
            sydney
                .at_local(at(2021, 1, 15, 12), 2021, false)
                .abbreviation,
            "AEDT"
        );
        assert_eq!(
            sydney
                .at_local(at(2021, 6, 15, 12), 2021, false)
                .abbreviation,
            "AEST"
        );
        assert_eq!(sydney.end.time, 3 * 3600);
    }

    #[test]
    fn day_rules_count_as_cpython_does() {
        let julian = Transition {
            day: TransitionDay::YearDay {
                day: 60,
                julian: true,
            },
            time: 0,
        };
        assert_eq!(julian.in_year(2020), at(2020, 3, 1, 0));
        assert_eq!(julian.in_year(2021), at(2021, 3, 1, 0));
        let zero_based = Transition {
            day: TransitionDay::YearDay {
                day: 0,
                julian: false,
            },
            time: 0,
        };
        assert_eq!(zero_based.in_year(2021), at(2020, 12, 31, 0));
        let last_sunday = transition("M3.5.0/-1").unwrap();
        assert_eq!(last_sunday.in_year(2021), at(2021, 3, 28, -1));
    }

    #[test]
    fn fixed_zones_and_angle_brackets() {
        assert_eq!(
            parse("<+0330>-3:30").unwrap(),
            TzRule::Fixed(LocalType {
                offset: 3 * 3600 + 1800,
                abbreviation: "+0330".to_owned()
            })
        );
        let default_shift = seasons("EST5EDT,M3.2.0,M11.1.0");
        assert_eq!(default_shift.daylight.offset, -4 * 3600);
    }

    #[test]
    fn malformed_strings_are_refused() {
        for text in [
            "",
            "EST",
            "5EST",
            "EST25",
            "EST5EDT",
            "EST5EDT,M3.2.0",
            "EST5,M3.2.0,M11.1.0",
            "EST5EDT,M13.2.0,M11.1.0",
            "EST5EDT,M3.6.0,M11.1.0",
            "EST5EDT,J0,J100",
            "EST5EDT,M3.2.0/168,M11.1.0",
            "EST5:3EDT",
            "<>5",
            "EST5EDT4x,J1,J2",
        ] {
            assert!(parse(text).is_err(), "{text:?}");
        }
    }
}
