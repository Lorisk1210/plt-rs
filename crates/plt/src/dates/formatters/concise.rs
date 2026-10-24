//! Compact date labels with an offset string (matplotlib's
//! `ConciseDateFormatter`).

use super::{format_date, resolve_usetex, wrap_in_tex};
use crate::dates::DateError;
use crate::dates::convert::num2date;
use crate::dates::tz::{TimeZone, ZonedDateTime};
use crate::ticker::{FormattedTicks, Formatter, TickContext, TickerError};

/// Formats for the six labelling levels: years, months, days, hours,
/// minutes, seconds.
type Levels = [String; 6];

const DEFAULT_FORMATS: [&str; 6] = ["%Y", "%b", "%d", "%H:%M", "%H:%M", "%S.%f"];
const DEFAULT_ZERO_FORMATS: [&str; 6] = ["", "%Y", "%b", "%b-%d", "%H:%M", "%H:%M"];
const DEFAULT_OFFSET_FORMATS: [&str; 6] =
    ["", "%Y", "%Y-%b", "%Y-%b-%d", "%Y-%b-%d", "%Y-%b-%d %H:%M"];

/// The value of each level's field that marks the start of the next larger
/// unit (January, the 1st, midnight, ...).
const ZERO_VALUES: [u32; 6] = [0, 1, 1, 0, 0, 0];

/// Labels only the part of the date that changes between ticks, using the
/// larger unit at its boundaries ("Feb", "08", "15", "Mar"), and reports the
/// rest of the date as an offset string ("2020-Mar").
///
/// The labelling level is the smallest unit that differs between ticks.
/// `formats` label ordinary ticks at each level, `zero_formats` the ticks
/// at the start of the next larger unit and `offset_formats` the offset.
/// The offset describes the last tick (the first on an inverted axis) and
/// is dropped when the ticks already show the year.
#[derive(Clone, Debug, PartialEq)]
pub struct ConciseDateFormatter {
    tz: TimeZone,
    formats: Option<Levels>,
    zero_formats: Option<Levels>,
    offset_formats: Option<Levels>,
    show_offset: bool,
    usetex: Option<bool>,
}

impl Default for ConciseDateFormatter {
    fn default() -> Self {
        Self::new()
    }
}

/// Six format strings, or an error naming the argument.
fn levels(name: &str, formats: &[&str]) -> Result<Levels, DateError> {
    let formats: Vec<String> = formats.iter().map(|&f| f.to_owned()).collect();
    formats.try_into().map_err(|_| {
        DateError::InvalidArgument(format!(
            "{name} argument must be a list of 6 format strings (or None)"
        ))
    })
}

impl ConciseDateFormatter {
    /// matplotlib's defaults, in UTC.
    pub const fn new() -> Self {
        Self {
            tz: TimeZone::Utc,
            formats: None,
            zero_formats: None,
            offset_formats: None,
            show_offset: true,
            usetex: None,
        }
    }

    /// Formats in `tz` instead of UTC.
    #[must_use]
    pub fn with_tz(mut self, tz: TimeZone) -> Self {
        self.tz = tz;
        self
    }

    /// Custom formats for the six levels. Unless zero formats are set too,
    /// each level's zero format becomes the format of the level above.
    pub fn with_formats(mut self, formats: &[&str]) -> Result<Self, DateError> {
        self.formats = Some(levels("formats", formats)?);
        Ok(self)
    }

    /// Custom formats for ticks at the start of the next larger unit.
    pub fn with_zero_formats(mut self, formats: &[&str]) -> Result<Self, DateError> {
        self.zero_formats = Some(levels("zero_formats", formats)?);
        Ok(self)
    }

    /// Custom formats for the offset string.
    pub fn with_offset_formats(mut self, formats: &[&str]) -> Result<Self, DateError> {
        self.offset_formats = Some(levels("offset_formats", formats)?);
        Ok(self)
    }

    /// Whether to report the offset string (default `true`).
    #[must_use]
    pub fn with_show_offset(mut self, show_offset: bool) -> Self {
        self.show_offset = show_offset;
        self
    }

    /// Whether labels are wrapped for TeX (default: `text.usetex`).
    #[must_use]
    pub fn with_usetex(mut self, usetex: bool) -> Self {
        self.usetex = Some(usetex);
        self
    }

    /// The format of ordinary ticks at `level`.
    fn format(&self, level: usize) -> &str {
        self.formats
            .as_ref()
            .map_or(DEFAULT_FORMATS[level], |f| &f[level])
    }

    /// The format of ticks at the start of the next larger unit.
    fn zero_format(&self, level: usize) -> &str {
        match (&self.zero_formats, &self.formats) {
            (Some(zero), _) => &zero[level],
            (None, Some(formats)) => level.checked_sub(1).map_or("", |above| &formats[above]),
            (None, None) => DEFAULT_ZERO_FORMATS[level],
        }
    }

    fn offset_format(&self, level: usize) -> &str {
        self.offset_formats
            .as_ref()
            .map_or(DEFAULT_OFFSET_FORMATS[level], |f| &f[level])
    }
}

/// Year, month, day, hour, minute and second of a date-time.
fn fields(date: &ZonedDateTime) -> [u32; 6] {
    let local = date.local();
    [
        local.year() as u32,
        local.month(),
        local.day(),
        local.hour(),
        local.minute(),
        local.second(),
    ]
}

/// The labelling level and whether the year already shows in the labels.
fn labelling_level(dates: &[ZonedDateTime]) -> (usize, bool) {
    let all: Vec<[u32; 6]> = dates.iter().map(fields).collect();
    for level in (0..6).rev() {
        let first = all[0][level];
        if all.iter().any(|f| f[level] != first) {
            let shows_year = level < 2 && all.iter().any(|f| f[level] == 1);
            return (level, shows_year);
        }
    }
    (5, false)
}

/// Strips the trailing zeros all fractional labels share, and a dangling
/// decimal point.
fn trim_common_zeros(labels: &mut [String]) {
    let trailing = labels
        .iter()
        .filter(|label| label.contains('.'))
        .map(|label| label.len() - label.trim_end_matches('0').len())
        .min();
    let Some(trailing) = trailing.filter(|&n| n > 0) else {
        return;
    };
    for label in labels.iter_mut().filter(|label| label.contains('.')) {
        label.truncate(label.len() - trailing);
        let kept = label.trim_end_matches('.').len();
        label.truncate(kept);
    }
}

impl Formatter for ConciseDateFormatter {
    fn format_ticks(
        &self,
        locs: &[f64],
        ctx: &TickContext<'_>,
    ) -> Result<FormattedTicks, TickerError> {
        let dates: Vec<ZonedDateTime> = locs
            .iter()
            .map(|&x| num2date(x, &self.tz))
            .collect::<Result<_, _>>()?;
        let (Some(first), Some(last)) = (dates.first(), dates.last()) else {
            return Ok(FormattedTicks::default());
        };
        let (level, shows_year) = labelling_level(&dates);
        let mut labels: Vec<String> = dates
            .iter()
            .map(|date| {
                let at_zero = if level < 5 {
                    fields(date)[level] == ZERO_VALUES[level]
                } else {
                    date.local().second() == 0 && date.local().microsecond() == 0
                };
                let format = if at_zero {
                    self.zero_format(level)
                } else {
                    self.format(level)
                };
                date.strftime(format)
            })
            .collect();
        if level == 5 {
            trim_common_zeros(&mut labels);
        }
        let usetex = resolve_usetex(self.usetex, ctx);
        let offset = if self.show_offset && !shows_year {
            let inverted = ctx
                .axis()
                .is_some_and(|axis| axis.view_interval.0 > axis.view_interval.1);
            let anchor = if inverted { first } else { last };
            let text = anchor.strftime(self.offset_format(level));
            if usetex { wrap_in_tex(&text) } else { text }
        } else {
            String::new()
        };
        if usetex {
            labels = labels.iter().map(|label| wrap_in_tex(label)).collect();
        }
        Ok(FormattedTicks { labels, offset })
    }

    /// The year of `value` (matplotlib's `__call__` with format `%Y`).
    fn format_data(&self, value: f64, ctx: &TickContext<'_>) -> Result<String, TickerError> {
        format_date(value, "%Y", &self.tz, resolve_usetex(self.usetex, ctx))
    }

    fn format_data_short(&self, value: f64, _: &TickContext<'_>) -> Result<String, TickerError> {
        format_date(value, "%Y-%m-%d %H:%M:%S", &self.tz, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::{AutoDateLocator, datestr2num};
    use crate::ticker::{AxisInfo, Locator, TickerSettings};

    fn concise(formatter: &ConciseDateFormatter, a: &str, b: &str) -> FormattedTicks {
        let view = (datestr2num(a).unwrap(), datestr2num(b).unwrap());
        let settings = TickerSettings::default();
        let axis = AxisInfo::with_view(view.0, view.1);
        let ctx = TickContext::new(&axis, &settings);
        let ticks = AutoDateLocator::new()
            .tick_values(view.0, view.1, &ctx)
            .unwrap();
        formatter.format_ticks(&ticks, &ctx).unwrap()
    }

    #[test]
    fn labels_show_only_what_changes() {
        let labels = concise(
            &ConciseDateFormatter::new(),
            "2020-01-15T13:47:12",
            "2020-03-02",
        );
        assert_eq!(labels.labels, ["22", "Feb", "08", "15", "22", "Mar"]);
        assert_eq!(labels.offset, "2020-Mar");
        let labels = concise(
            &ConciseDateFormatter::new(),
            "2020-01-01",
            "2020-01-01T00:00:01",
        );
        assert_eq!(labels.labels[..4], ["59.8", "00:00", "00.2", "00.4"]);
        assert_eq!(labels.offset, "2020-Jan-01 00:00");
        let hidden = ConciseDateFormatter::new().with_show_offset(false);
        assert_eq!(concise(&hidden, "2020-01-01", "2020-01-02").offset, "");
    }

    #[test]
    fn zoned_labels_use_the_wall_clock() {
        let Ok(tz) = TimeZone::named("America/New_York") else {
            return; // no system zoneinfo database
        };
        let view = (
            datestr2num("2020-11-01T04:00").unwrap(),
            datestr2num("2020-11-01T08:00").unwrap(),
        );
        let settings = TickerSettings::default();
        let axis = AxisInfo::with_view(view.0, view.1);
        let ctx = TickContext::new(&axis, &settings);
        let ticks = AutoDateLocator::new()
            .with_tz(tz.clone())
            .locate(&ctx)
            .unwrap();
        let labels = ConciseDateFormatter::new()
            .with_tz(tz)
            .format_ticks(&ticks, &ctx)
            .unwrap();
        assert_eq!(
            labels.labels,
            [
                "00:00", "00:30", "01:00", "01:30", "02:00", "02:30", "03:00"
            ]
        );
        assert_eq!(labels.offset, "2020-Nov-01");
    }

    #[test]
    fn custom_formats_validate_and_cascade() {
        assert!(ConciseDateFormatter::new().with_formats(&["%Y"]).is_err());
        assert!(
            ConciseDateFormatter::new()
                .with_offset_formats(&["%Y"])
                .is_err()
        );
        let custom = ConciseDateFormatter::new()
            .with_formats(&["%y", "%B", "%d.", "%Hh", "%Mm", "%Ss"])
            .unwrap();
        assert_eq!(custom.zero_format(3), "%d.");
        assert_eq!(custom.zero_format(0), "");
        assert_eq!(ConciseDateFormatter::new().zero_format(3), "%b-%d");
    }

    #[test]
    fn trailing_zeros_are_shared() {
        let mut labels = vec![
            "00:00".to_owned(),
            "00.500000".to_owned(),
            "01.000000".to_owned(),
        ];
        trim_common_zeros(&mut labels);
        assert_eq!(labels, ["00:00", "00.5", "01.0"]);
        let mut whole = vec!["01.000000".to_owned(), "02.000000".to_owned()];
        trim_common_zeros(&mut whole);
        assert_eq!(whole, ["01", "02"]);
    }
}
