//! `strftime` formatting with the C ("POSIX") locale.
//!
//! Python's `datetime.strftime` first substitutes `%f`, `%z`, `%:z` and
//! `%Z` itself and zero-pads `%Y`, `%G`, `%F` and `%C` for years before
//! 1000, then hands the rest to the C library. This module reproduces that
//! two-layer behaviour with glibc's conversions, including its flags
//! (`_`, `-`, `0`, `^`, `#`), field widths and `E`/`O` modifiers.
//! Unknown conversions are copied literally, as glibc does.

use std::fmt::Write as _;

use super::calendar::{Weekday, month_name};
use super::tz::UtcOffset;
use super::{Date, DateTime};

impl DateTime {
    /// Formats this naive date-time like Python's `datetime.strftime`
    /// (`%z` and `%Z` expand to nothing).
    ///
    /// ```
    /// use plt::dates::DateTime;
    /// let dt = DateTime::new(2020, 3, 5, 14, 7, 9, 123_456).unwrap();
    /// assert_eq!(dt.strftime("%a %d %b %Y, %I:%M %p (%j) %f"),
    ///            "Thu 05 Mar 2020, 02:07 PM (065) 123456");
    /// ```
    pub fn strftime(&self, format: &str) -> String {
        strftime(format, *self, None)
    }
}

/// Formats `dt` with `format`; `zone` supplies `%z`/`%Z` for aware values.
pub(crate) fn strftime(format: &str, dt: DateTime, zone: Option<(UtcOffset, &str)>) -> String {
    let mut out = String::with_capacity(format.len() + 16);
    let mut rest = format;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        rest = &rest[start + 1..];
        let mut chars = rest.chars();
        let Some(code) = chars.next() else {
            out.push('%');
            break;
        };
        let consumed = match code {
            'f' => {
                let _ = write!(out, "{:06}", dt.microsecond());
                1
            }
            'z' => {
                out.push_str(
                    &zone
                        .map(|(offset, _)| offset.format(""))
                        .unwrap_or_default(),
                );
                1
            }
            'Z' => {
                out.push_str(zone.map_or("", |(_, name)| name));
                1
            }
            ':' => match chars.next() {
                Some('z') => {
                    out.push_str(
                        &zone
                            .map(|(offset, _)| offset.format(":"))
                            .unwrap_or_default(),
                    );
                    2
                }
                // Python drops a dangling `%:`.
                None => 1,
                Some(_) => {
                    out.push_str("%:");
                    1
                }
            },
            'Y' | 'G' | 'F' | 'C' if dt.year() < 1000 => {
                out.push_str(&century_normalised(code, dt));
                1
            }
            _ => {
                if let Some(conversion) = Conversion::parse(rest) {
                    conversion.write(&mut out, dt, zone);
                    conversion.len
                } else {
                    // glibc copies an unknown or incomplete sequence verbatim.
                    let len = rest.ceil_char_boundary(rest.find('%').unwrap_or(rest.len()).max(1));
                    out.push('%');
                    out.push_str(&rest[..len]);
                    len
                }
            }
        };
        rest = &rest[rest.ceil_char_boundary(consumed)..];
    }
    out.push_str(rest);
    out
}

/// Python's zero-padded `%Y`, `%G`, `%F` and `%C` for years before 1000.
fn century_normalised(code: char, dt: DateTime) -> String {
    let date = dt.date();
    match code {
        'G' => format!("{:04}", date.iso_week().0),
        'C' => format!("{:02}", date.year() / 100),
        'F' => format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day()),
        _ => format!("{:04}", date.year()),
    }
}

/// How a numeric field is padded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pad {
    Zero,
    Space,
    None,
}

/// One parsed glibc conversion: flags, width, modifier and specifier.
#[derive(Clone, Copy, Debug)]
struct Conversion {
    pad: Option<Pad>,
    upper: bool,
    swap_case: bool,
    width: Option<usize>,
    spec: char,
    /// Bytes consumed after the `%`.
    len: usize,
}

impl Conversion {
    /// Parses the sequence following a `%`; `None` if glibc would reject it.
    fn parse(text: &str) -> Option<Self> {
        let mut conversion = Self {
            pad: None,
            upper: false,
            swap_case: false,
            width: None,
            spec: '\0',
            len: 0,
        };
        let mut chars = text.char_indices().peekable();
        while let Some(&(_, c)) = chars.peek() {
            match c {
                '_' => conversion.pad = Some(Pad::Space),
                '-' => conversion.pad = Some(Pad::None),
                '0' => conversion.pad = Some(Pad::Zero),
                '^' => conversion.upper = true,
                '#' => conversion.swap_case = true,
                _ => break,
            }
            chars.next();
        }
        let mut width = None::<usize>;
        while let Some(&(_, c @ '0'..='9')) = chars.peek() {
            width = Some(width.unwrap_or(0).saturating_mul(10) + c as usize - '0' as usize);
            chars.next();
        }
        conversion.width = width;
        let mut modifier = None;
        if let Some(&(_, c @ ('E' | 'O'))) = chars.peek() {
            modifier = Some(c);
            chars.next();
        }
        let (index, spec) = chars.next()?;
        let allowed = match modifier {
            None => true,
            Some('E') => "cCxXyY".contains(spec),
            Some(_) => "bBhdeHIklmMSuUVwWy".contains(spec),
        };
        if !allowed || !"aAbBcCdDeFgGhHIjklmMnpPrRsStTuUVwWxXyYzZ%".contains(spec) {
            return None;
        }
        conversion.spec = spec;
        conversion.len = index + spec.len_utf8();
        Some(conversion)
    }

    fn write(&self, out: &mut String, dt: DateTime, zone: Option<(UtcOffset, &str)>) {
        let date = dt.date();
        let year = i64::from(date.year());
        let hour12 = (dt.hour() + 11) % 12 + 1;
        let text = match self.spec {
            'a' => self.text(dt.weekday().short_name(), false),
            'A' => self.text(dt.weekday().name(), false),
            'b' | 'h' => self.text(&month_name(date.month())[..3], false),
            'B' => self.text(month_name(date.month()), false),
            'p' => self.text(if dt.hour() < 12 { "AM" } else { "PM" }, true),
            'P' => self.text(if dt.hour() < 12 { "am" } else { "pm" }, false),
            'Z' => self.text(zone.map_or("", |(_, name)| name), true),
            'c' => self.composite("%a %b %e %H:%M:%S %Y", dt, zone),
            'D' | 'x' => self.composite("%m/%d/%y", dt, zone),
            'F' => self.composite("%+4Y-%m-%d", dt, zone),
            'r' => self.composite("%I:%M:%S %p", dt, zone),
            'R' => self.composite("%H:%M", dt, zone),
            'T' | 'X' => self.composite("%H:%M:%S", dt, zone),
            'n' => "\n".to_owned(),
            't' => "\t".to_owned(),
            '%' => "%".to_owned(),
            'z' => zone
                .map(|(offset, _)| offset.format(""))
                .unwrap_or_default(),
            other => self.numeric(other, dt, date, year, hour12),
        };
        out.push_str(&text);
    }

    /// The numeric conversions.
    fn numeric(&self, spec: char, dt: DateTime, date: Date, year: i64, hour12: u32) -> String {
        let weekday = date.weekday();
        let yday = i64::from(date.ordinal()) - 1;
        let (value, digits, pad) = match spec {
            'C' => (year.div_euclid(100), 2, Pad::Zero),
            'd' => (i64::from(date.day()), 2, Pad::Zero),
            'e' => (i64::from(date.day()), 2, Pad::Space),
            'g' => (date.iso_week().0.rem_euclid(100), 2, Pad::Zero),
            'G' => (date.iso_week().0, 1, Pad::Zero),
            'H' => (i64::from(dt.hour()), 2, Pad::Zero),
            'I' => (i64::from(hour12), 2, Pad::Zero),
            'j' => (yday + 1, 3, Pad::Zero),
            'k' => (i64::from(dt.hour()), 2, Pad::Space),
            'l' => (i64::from(hour12), 2, Pad::Space),
            'm' => (i64::from(date.month()), 2, Pad::Zero),
            'M' => (i64::from(dt.minute()), 2, Pad::Zero),
            's' => (dt.timestamp_micros().div_euclid(1_000_000), 1, Pad::Zero),
            'S' => (i64::from(dt.second()), 2, Pad::Zero),
            'u' => (i64::from(weekday.index()) + 1, 1, Pad::Zero),
            'U' => (week_number(yday, weekday, Weekday::Sunday), 2, Pad::Zero),
            'V' => (i64::from(date.iso_week().1), 2, Pad::Zero),
            'w' => (i64::from(weekday.sunday_index()), 1, Pad::Zero),
            'W' => (week_number(yday, weekday, Weekday::Monday), 2, Pad::Zero),
            'y' => (year.rem_euclid(100), 2, Pad::Zero),
            _ => (year, 1, Pad::Zero),
        };
        // `%+4Y` (inside `%F`) pads the year to four digits.
        let digits = self.width.unwrap_or(digits);
        let magnitude = value.unsigned_abs().to_string();
        let sign = if value < 0 { "-" } else { "" };
        let fill = match self.pad.unwrap_or(pad) {
            Pad::None => return format!("{sign}{magnitude}"),
            Pad::Zero => '0',
            Pad::Space => ' ',
        };
        let padding = digits.saturating_sub(magnitude.len() + sign.len());
        let fill: String = std::iter::repeat_n(fill, padding).collect();
        if fill.starts_with('0') {
            format!("{sign}{fill}{magnitude}")
        } else {
            format!("{fill}{sign}{magnitude}")
        }
    }

    /// A textual conversion with case flags and width applied; `lower_on_swap`
    /// marks conversions that `#` lowercases instead of uppercasing.
    fn text(&self, value: &str, lower_on_swap: bool) -> String {
        let cased = if self.swap_case && lower_on_swap {
            value.to_lowercase()
        } else if self.upper || self.swap_case {
            value.to_uppercase()
        } else {
            value.to_owned()
        };
        self.pad_text(cased)
    }

    fn composite(&self, format: &str, dt: DateTime, zone: Option<(UtcOffset, &str)>) -> String {
        let mut expanded = String::new();
        let mut rest = format;
        while let Some(start) = rest.find('%') {
            expanded.push_str(&rest[..start]);
            rest = &rest[start + 1..];
            if let Some(stripped) = rest.strip_prefix("+4Y") {
                let _ = write!(expanded, "{:04}", dt.year());
                rest = stripped;
            } else if let Some(conversion) = Self::parse(rest) {
                conversion.write(&mut expanded, dt, zone);
                rest = &rest[conversion.len..];
            }
        }
        expanded.push_str(rest);
        let cased = if self.upper {
            expanded.to_uppercase()
        } else {
            expanded
        };
        self.pad_text(cased)
    }

    fn pad_text(&self, text: String) -> String {
        let width = self.width.unwrap_or(0);
        let len = text.chars().count();
        if width <= len {
            return text;
        }
        let fill = if self.pad == Some(Pad::Zero) {
            '0'
        } else {
            ' '
        };
        std::iter::repeat_n(fill, width - len)
            .chain(text.chars())
            .collect()
    }
}

/// `%U` / `%W`: weeks starting on `first`, with days before the first such
/// weekday of the year in week 0.
fn week_number(yday: i64, weekday: Weekday, first: Weekday) -> i64 {
    let days_since_first = (i64::from(weekday.index()) - i64::from(first.index())).rem_euclid(7);
    (yday + 7 - days_since_first) / 7
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32, us: u32) -> DateTime {
        DateTime::new(y, mo, d, h, mi, s, us).unwrap()
    }

    #[test]
    fn python_layer_codes() {
        let d = dt(5, 1, 1, 0, 0, 0, 0);
        assert_eq!(d.strftime("%Y|%G|%y|%C|%F"), "0005|0004|05|00|0005-01-01");
        assert_eq!(d.strftime("%c"), "Sat Jan  1 00:00:00 5");
        assert_eq!(d.strftime("%f|%z|%Z|%:z|%"), "000000||||%");
        let utc = Some((UtcOffset::UTC, "UTC"));
        assert_eq!(strftime("%z %:z %Z", d, utc), "+0000 +00:00 UTC");
        assert_eq!(d.strftime("%:"), "");
        assert_eq!(d.strftime("%:a"), "%:a");
    }

    #[test]
    fn glibc_layer_codes() {
        let d = dt(2005, 3, 7, 15, 4, 5, 123_456);
        assert_eq!(
            d.strftime("%a|%A|%b|%B|%h|%d|%e|%j|%U|%W|%w|%u|%V|%g|%G"),
            "Mon|Monday|Mar|March|Mar|07| 7|066|10|10|1|1|10|05|2005"
        );
        assert_eq!(
            d.strftime("%H|%I|%k|%l|%p|%P|%r|%R|%T|%D|%x|%X|%F"),
            "15|03|15| 3|PM|pm|03:04:05 PM|15:04|15:04:05|03/07/05|03/07/05|15:04:05|2005-03-07"
        );
        assert_eq!(
            d.strftime("%n%t%%|%q|%E|%Ey|%Oy|%Ea"),
            "\n\t%|%q|%E|05|05|%Ea"
        );
        assert_eq!(
            d.strftime("%-d|%_m|%^a|%#B|%10A|%-j"),
            "7| 3|MON|MARCH|    Monday|66"
        );
        assert_eq!(
            dt(2020, 1, 1, 0, 0, 0, 0).strftime("%I %p %s"),
            "12 AM 1577836800"
        );
    }

    #[test]
    fn week_numbers_at_year_edges() {
        let d = dt(2021, 1, 3, 0, 0, 0, 0);
        assert_eq!(d.strftime("%U %W %V %G"), "01 00 53 2020");
        let d = dt(2024, 12, 30, 0, 0, 0, 0);
        assert_eq!(d.strftime("%U %W %V %G %g"), "52 53 01 2025 25");
    }

    #[test]
    fn non_ascii_text_is_kept() {
        let d = dt(2020, 1, 1, 0, 0, 0, 0);
        assert_eq!(d.strftime("é%Yü%é"), "é2020ü%é");
    }
}
