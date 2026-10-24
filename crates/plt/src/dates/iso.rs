//! ISO 8601 parsing: Python's `datetime.fromisoformat` and numpy's
//! `datetime64` string syntax.

use super::calendar::{self, Weekday};
use super::timedelta::{MICROS_PER_DAY, MICROS_PER_SECOND};
use super::tz::{TimeZone, UtcOffset, ZonedDateTime};
use super::{Date, DateError, DateTime, TimeDelta};

/// Parses a string the way Python's `datetime.fromisoformat` does,
/// returning the wall-clock date-time and its UTC offset, if any.
///
/// Accepted: calendar (`2020-03-05`, `20200305`) and week dates
/// (`2020-W10-4`), any single separator character, times `HH[:MM[:SS[.f]]]`
/// (basic format too, `,` or `.` before up to any number of fraction
/// digits, truncated to microseconds), `24:00` as the next midnight, and
/// offsets `Z` / `±HH[:MM[:SS[.ffffff]]]`.
///
/// ```
/// use plt::dates::iso::fromisoformat;
/// let (dt, offset) = fromisoformat("2020-06-15T12:00:00+02:00").unwrap();
/// assert_eq!(dt.to_string(), "2020-06-15T12:00:00");
/// assert_eq!(offset.unwrap().to_string(), "+02:00");
/// ```
pub fn fromisoformat(text: &str) -> Result<(DateTime, Option<UtcOffset>), DateError> {
    let invalid = || DateError::parse(text, "invalid isoformat string");
    let mut cursor = Cursor::new(text);
    let date = parse_date(&mut cursor).ok_or_else(invalid)??;
    if cursor.at_end() {
        return Ok((DateTime::midnight(date), None));
    }
    cursor.next_char().ok_or_else(invalid)?;
    let (micros_of_day, offset) = parse_time(&mut cursor).ok_or_else(invalid)??;
    if !cursor.at_end() {
        return Err(invalid());
    }
    let start = DateTime::midnight(date);
    let dt = start.checked_add(TimeDelta::from_micros(micros_of_day))?;
    Ok((dt, offset))
}

impl ZonedDateTime {
    /// Parses an ISO 8601 string with [`fromisoformat`]; strings without an
    /// offset are taken as UTC (matplotlib's convention for naive dates),
    /// strings with one get a fixed-offset zone.
    pub fn parse_iso(text: &str) -> Result<Self, DateError> {
        let (local, offset) = fromisoformat(text)?;
        let tz = match offset {
            None => TimeZone::Utc,
            Some(offset) if offset == UtcOffset::UTC => TimeZone::Utc,
            Some(offset) => TimeZone::Fixed(offset),
        };
        Ok(Self::from_local(local, tz))
    }
}

fn parse_date(cursor: &mut Cursor<'_>) -> Option<Result<Date, DateError>> {
    let year = cursor.digits(4)?;
    let extended = cursor.eat('-');
    if cursor.eat('W') {
        let week = cursor.digits(2)?;
        let weekday = if cursor
            .peek()
            .is_some_and(|c| c.is_ascii_digit() || c == '-')
        {
            if extended && !cursor.eat('-') {
                return None;
            }
            cursor.digits(1)?
        } else {
            1
        };
        return Some(week_date(year, week, weekday));
    }
    let month = cursor.digits(2)?;
    if extended && !cursor.eat('-') {
        return None;
    }
    let day = cursor.digits(2)?;
    Some(Date::new(year as i32, month as u32, day as u32))
}

/// The date of ISO week `week`, day `weekday` (1 = Monday) of `year`.
fn week_date(year: i64, week: i64, weekday: i64) -> Result<Date, DateError> {
    if !(1..=53).contains(&week) {
        return Err(DateError::field("week", week));
    }
    if !(1..=7).contains(&weekday) {
        return Err(DateError::field("weekday", weekday));
    }
    let jan4 = calendar::days_from_civil(year, 1, 4);
    let monday = jan4 - i64::from(Weekday::from_days_since_epoch(jan4).index());
    Date::from_days_since_epoch(monday + (week - 1) * 7 + weekday - 1)
}

/// A time of day (in microseconds, `24:00` allowed) and optional offset.
fn parse_time(cursor: &mut Cursor<'_>) -> Option<Result<(i64, Option<UtcOffset>), DateError>> {
    let (hour, minute, second, micros) = parse_clock(cursor)?;
    let offset = if cursor.eat('Z') {
        Some(Ok(UtcOffset::UTC))
    } else if let Some(sign) = cursor.eat_sign() {
        let (h, m, s, us) = parse_clock(cursor)?;
        Some(validate_clock(h, m, s, us).and_then(|total| UtcOffset::from_micros(sign * total)))
    } else {
        None
    };
    Some(
        validate_clock(hour, minute, second, micros).and_then(|of_day| {
            let offset = offset.transpose()?;
            Ok((of_day, offset))
        }),
    )
}

/// `HH[:MM[:SS[.fff]]]` or the basic `HH[MM[SS[.fff]]]`.
fn parse_clock(cursor: &mut Cursor<'_>) -> Option<(i64, i64, i64, i64)> {
    let hour = cursor.digits(2)?;
    let mut fields = [0_i64; 2];
    let mut extended = None;
    for field in &mut fields {
        let colon = cursor.peek() == Some(':');
        if !colon && !cursor.peek().is_some_and(|c| c.is_ascii_digit()) {
            return Some((hour, fields[0], fields[1], 0));
        }
        match extended {
            None => extended = Some(colon),
            Some(e) if e != colon => return None,
            Some(_) => {}
        }
        if colon {
            cursor.next_char();
        }
        *field = cursor.digits(2)?;
    }
    let micros = if cursor.eat('.') || cursor.eat(',') {
        cursor.fraction_micros()?
    } else {
        0
    };
    Some((hour, fields[0], fields[1], micros))
}

fn validate_clock(hour: i64, minute: i64, second: i64, micros: i64) -> Result<i64, DateError> {
    if hour == 24 && minute == 0 && second == 0 && micros == 0 {
        return Ok(MICROS_PER_DAY);
    }
    for (field, value, limit) in [
        ("hour", hour, 23),
        ("minute", minute, 59),
        ("second", second, 59),
    ] {
        if value > limit {
            return Err(DateError::field(field, value));
        }
    }
    Ok(((hour * 60 + minute) * 60 + second) * MICROS_PER_SECOND + micros)
}

/// A numpy `datetime64` value: an instant with nanosecond resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Datetime64 {
    seconds: i64,
    nanos: u32,
}

impl Datetime64 {
    /// The instant `seconds + nanos` after 1970-01-01T00:00:00 (the whole
    /// seconds saturating at the ends of `i64`).
    pub const fn new(seconds: i64, nanos: u32) -> Self {
        Self {
            seconds: seconds.saturating_add((nanos / 1_000_000_000) as i64),
            nanos: nanos % 1_000_000_000,
        }
    }

    /// Whole seconds since the Unix epoch, rounded down (`astype('M8[s]')`).
    pub const fn seconds(self) -> i64 {
        self.seconds
    }

    /// Nanoseconds past [`seconds`](Self::seconds), `0..1_000_000_000`.
    pub const fn subsec_nanos(self) -> u32 {
        self.nanos
    }

    /// Microseconds since the Unix epoch, rounded down and saturating at
    /// the ends of `i64` (beyond about 292,000 years from 1970).
    pub const fn timestamp_micros(self) -> i64 {
        self.seconds
            .saturating_mul(MICROS_PER_SECOND)
            .saturating_add((self.nanos / 1000) as i64)
    }

    /// Parses numpy's `datetime64` string syntax: `YYYY[-MM[-DD[THH[:MM[:SS[.f]]]]]]`
    /// with an optional sign on the year, `T` or a space before the time,
    /// up to 18 fraction digits (kept to nanoseconds) and an optional `Z`
    /// or `±HH[:MM]` offset. `"NaT"` (any case) and the empty string are
    /// not-a-time and give `None`.
    ///
    /// ```
    /// use plt::dates::iso::Datetime64;
    /// let t = Datetime64::parse("1970-01-01T00:00:01.5").unwrap().unwrap();
    /// assert_eq!((t.seconds(), t.subsec_nanos()), (1, 500_000_000));
    /// assert_eq!(Datetime64::parse("NaT"), Ok(None));
    /// ```
    pub fn parse(text: &str) -> Result<Option<Self>, DateError> {
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("nat") {
            return Ok(None);
        }
        let invalid = || DateError::parse(text, "invalid datetime64 string");
        let mut cursor = Cursor::new(trimmed);
        let sign = cursor.eat_sign().unwrap_or(1);
        let year = sign * cursor.integer().ok_or_else(invalid)?;
        let mut fields = [1_i64, 1, 0, 0, 0];
        let separators = ['-', '-', 'T', ':', ':'];
        let mut parsed = 0;
        for (field, separator) in fields.iter_mut().zip(separators) {
            let matched = if separator == 'T' {
                cursor.eat('T') || cursor.eat(' ')
            } else {
                cursor.eat(separator)
            };
            if !matched {
                break;
            }
            *field = cursor.digits(2).ok_or_else(invalid)?;
            parsed += 1;
        }
        let nanos = if parsed == fields.len() && cursor.eat('.') {
            cursor.fraction_nanos().ok_or_else(invalid)?
        } else {
            0
        };
        let offset_seconds = if cursor.eat('Z') {
            0
        } else if let Some(sign) = cursor.eat_sign() {
            let hours = cursor.digits(2).ok_or_else(invalid)?;
            cursor.eat(':');
            let minutes = cursor.digits(2).unwrap_or(0);
            sign * (hours * 3600 + minutes * 60)
        } else {
            0
        };
        if !cursor.at_end() {
            return Err(invalid());
        }
        // The year `i64::MAX` seconds reach: beyond it, no instant exists
        // (and the calendar arithmetic would overflow).
        if year.unsigned_abs() > 292_277_026_596 {
            return Err(DateError::field("year", year));
        }
        let [month, day, hour, minute, second] = fields;
        let valid_day = (1..=12).contains(&month)
            && day >= 1
            && day <= i64::from(calendar::days_in_month(year, month as u32));
        if !valid_day || hour > 23 || minute > 59 || second > 59 {
            return Err(invalid());
        }
        let days = calendar::days_from_civil(year, month as u32, day as u32);
        let seconds = days
            .checked_mul(86_400)
            .and_then(|s| s.checked_add(hour * 3600 + minute * 60 + second - offset_seconds))
            .ok_or(DateError::Overflow)?;
        Ok(Some(Self::new(seconds, nanos)))
    }
}

/// A byte cursor over ASCII-structured text.
struct Cursor<'a> {
    text: &'a str,
    pos: usize,
}

impl<'a> Cursor<'a> {
    const fn new(text: &'a str) -> Self {
        Self { text, pos: 0 }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.text.len()
    }

    fn peek(&self) -> Option<char> {
        self.text[self.pos..].chars().next()
    }

    fn next_char(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn eat(&mut self, expected: char) -> bool {
        let matched = self.peek() == Some(expected);
        if matched {
            self.pos += expected.len_utf8();
        }
        matched
    }

    fn eat_sign(&mut self) -> Option<i64> {
        if self.eat('+') {
            Some(1)
        } else if self.eat('-') {
            Some(-1)
        } else {
            None
        }
    }

    /// Exactly `count` ASCII digits.
    fn digits(&mut self, count: usize) -> Option<i64> {
        let slice = self.text.get(self.pos..self.pos + count)?;
        if !slice.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        self.pos += count;
        slice.parse().ok()
    }

    /// One or more ASCII digits.
    fn integer(&mut self) -> Option<i64> {
        let len = self.text[self.pos..]
            .bytes()
            .take_while(u8::is_ascii_digit)
            .count();
        let value = self.text[self.pos..self.pos + len].parse().ok()?;
        self.pos += len;
        Some(value)
    }

    /// One or more fraction digits scaled to `10^-scale`, truncated.
    fn fraction(&mut self, scale: u32) -> Option<i64> {
        let digits = &self.text[self.pos..];
        let len = digits.bytes().take_while(u8::is_ascii_digit).count();
        if len == 0 {
            return None;
        }
        self.pos += len;
        let kept = &digits[..len.min(scale as usize)];
        let value: i64 = kept.parse().ok()?;
        Some(value * 10_i64.pow(scale - kept.len() as u32))
    }

    fn fraction_micros(&mut self) -> Option<i64> {
        self.fraction(6)
    }

    fn fraction_nanos(&mut self) -> Option<u32> {
        self.fraction(9).map(|n| n as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iso(text: &str) -> String {
        let (dt, offset) = fromisoformat(text).unwrap();
        format!("{dt}{}", offset.map(|o| o.to_string()).unwrap_or_default())
    }

    #[test]
    fn fromisoformat_matches_python() {
        assert_eq!(iso("2020-01-01T00:00:00.5"), "2020-01-01T00:00:00.500000");
        assert_eq!(
            iso("2020-01-01T00:00:00.1234567"),
            "2020-01-01T00:00:00.123456"
        );
        assert_eq!(iso("2020-01-01 12"), "2020-01-01T12:00:00");
        assert_eq!(iso("20200101T123045"), "2020-01-01T12:30:45");
        assert_eq!(iso("2020-W01-1"), "2019-12-30T00:00:00");
        assert_eq!(iso("2020-01-01T24:00:00"), "2020-01-02T00:00:00");
        assert_eq!(iso("2020-01-01T00:00:00Z"), "2020-01-01T00:00:00+00:00");
        assert_eq!(iso("2020-01-01T00:00+0530"), "2020-01-01T00:00:00+05:30");
        assert_eq!(iso("2020-01-01x00:00:00,5"), "2020-01-01T00:00:00.500000");
        assert_eq!(
            iso("2020-01-01T00:00:00+05:30:15.5"),
            "2020-01-01T00:00:00+05:30:15.500000"
        );
        assert_eq!(iso("0001-01-01"), "0001-01-01T00:00:00");
        for bad in [
            "2020-01-01T00:00,5",
            "2020-001",
            "2020-1-1",
            "2020-01-01T1:00",
            "2020-01-01T00:00:00.",
            "2020-01-01T23:59:60",
            "",
            "2020-02-30",
        ] {
            assert!(fromisoformat(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn datetime64_strings() {
        let parse = |s: &str| Datetime64::parse(s).unwrap().unwrap();
        assert_eq!(parse("2020-01-01"), Datetime64::new(1_577_836_800, 0));
        assert_eq!(
            parse("1970-01-01T00:00:00.000000001"),
            Datetime64::new(0, 1)
        );
        assert_eq!(
            parse("1969-12-31T23:59:59.5"),
            Datetime64::new(-1, 500_000_000)
        );
        assert_eq!(parse("0000-12-31").seconds(), -719_163 * 86_400);
        assert_eq!(parse("1970"), Datetime64::new(0, 0));
        assert_eq!(parse("1970-01-01T01:00+01:00"), Datetime64::new(0, 0));
        assert_eq!(Datetime64::parse(" nat "), Ok(None));
        assert!(Datetime64::parse("2020-13-01").is_err());
        assert!(Datetime64::parse("2020-01-01T").is_err());
    }

    #[test]
    fn extreme_datetime64_years_are_rejected_or_saturate() {
        assert!(Datetime64::parse("292277026596-01-01").is_ok());
        assert!(Datetime64::parse("9223372036854775807").is_err());
        assert!(Datetime64::parse("-9223372036854775807-12-31").is_err());
        // Found by fuzzing: microseconds of far instants overflowed.
        let far = Datetime64::parse("99202099").unwrap().unwrap();
        assert_eq!(far.timestamp_micros(), i64::MAX);
        assert_eq!(Datetime64::new(i64::MAX, 2_000_000_000).seconds(), i64::MAX);
    }
}
