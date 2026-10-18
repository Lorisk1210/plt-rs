//! Formats chosen from the locator's tick unit (matplotlib's
//! `AutoDateFormatter`).

use super::{format_date, resolve_usetex};
use crate::dates::convert::MICROSECONDS_PER_DAY;
use crate::dates::locators::DateLocator;
use crate::dates::tz::TimeZone;
use crate::ticker::{Formatter, TickContext, TickLabels, TickerError};

/// Labels ticks with the format registered for the smallest scale at least
/// as large as the locator's unit (days per tick unit).
///
/// The default table follows the `date.autoformatter.*` rcParams:
///
/// | scale (days)  | format              |
/// |---------------|---------------------|
/// | 365           | `%Y`                |
/// | 30            | `%Y-%m`             |
/// | 1             | `%Y-%m-%d`          |
/// | 1/24          | `%m-%d %H`          |
/// | 1/1440        | `%d %H:%M`          |
/// | 1/86400       | `%H:%M:%S`          |
/// | 1/86400e6     | `%M:%S.%f`          |
///
/// Units beyond the largest scale use `defaultfmt`.
#[derive(Debug)]
pub struct AutoDateFormatter {
    locator: Box<dyn DateLocator>,
    tz: TimeZone,
    defaultfmt: String,
    scaled: Vec<(f64, String)>,
    usetex: Option<bool>,
}

impl AutoDateFormatter {
    /// A formatter following `locator`'s unit, in UTC, with the formats
    /// of the `date.autoformatter.*` rcParams.
    pub fn new(locator: impl DateLocator) -> Self {
        let rc = crate::rc::params();
        let scaled = [
            (365.0, rc.date_autoformatter_year.as_str()),
            (30.0, rc.date_autoformatter_month.as_str()),
            (1.0, rc.date_autoformatter_day.as_str()),
            (1.0 / 24.0, rc.date_autoformatter_hour.as_str()),
            (1.0 / 1440.0, rc.date_autoformatter_minute.as_str()),
            (1.0 / 86_400.0, rc.date_autoformatter_second.as_str()),
            (
                1.0 / MICROSECONDS_PER_DAY,
                rc.date_autoformatter_microsecond.as_str(),
            ),
        ];
        let mut formatter = Self {
            locator: Box::new(locator),
            tz: TimeZone::Utc,
            defaultfmt: "%Y-%m-%d".to_owned(),
            scaled: Vec::new(),
            usetex: None,
        };
        for (scale, format) in scaled {
            formatter.set_scaled(scale, format);
        }
        formatter
    }

    /// Formats in `tz` instead of UTC.
    #[must_use]
    pub fn with_tz(mut self, tz: TimeZone) -> Self {
        self.tz = tz;
        self
    }

    /// The format for units larger than every scale.
    #[must_use]
    pub fn with_defaultfmt(mut self, format: impl Into<String>) -> Self {
        self.defaultfmt = format.into();
        self
    }

    /// Whether labels are wrapped for TeX (default: `text.usetex`).
    #[must_use]
    pub fn with_usetex(mut self, usetex: bool) -> Self {
        self.usetex = Some(usetex);
        self
    }

    /// Registers (or replaces) the format for ticks `scale` days apart
    /// (matplotlib's `formatter.scaled[scale] = format`).
    pub fn set_scaled(&mut self, scale: f64, format: impl Into<String>) {
        let format = format.into();
        if let Some(entry) = self
            .scaled
            .iter_mut()
            .find(|(s, _)| s.total_cmp(&scale).is_eq())
        {
            entry.1 = format;
        } else {
            self.scaled.push((scale, format));
            self.scaled.sort_by(|a, b| a.0.total_cmp(&b.0));
        }
    }

    /// The format used for the context's view.
    pub fn format_for(&self, ctx: &TickContext<'_>) -> &str {
        let unit = self.locator.unit(ctx);
        self.scaled
            .iter()
            .find(|(scale, _)| *scale >= unit)
            .map_or(&self.defaultfmt, |(_, format)| format)
    }
}

impl Formatter for AutoDateFormatter {
    fn format_ticks(&self, locs: &[f64], ctx: &TickContext<'_>) -> Result<TickLabels, TickerError> {
        let format = self.format_for(ctx);
        let usetex = resolve_usetex(self.usetex, ctx);
        Ok(TickLabels {
            labels: locs
                .iter()
                .map(|&x| format_date(x, format, &self.tz, usetex))
                .collect::<Result<_, _>>()?,
            offset: String::new(),
        })
    }

    fn format_data(&self, value: f64, ctx: &TickContext<'_>) -> Result<String, TickerError> {
        format_date(
            value,
            self.format_for(ctx),
            &self.tz,
            resolve_usetex(self.usetex, ctx),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::{AutoDateLocator, RRuleLocator};
    use crate::ticker::{AxisInfo, TickerSettings};

    #[test]
    fn format_follows_the_locator_unit() {
        let settings = TickerSettings::default();
        let axis = AxisInfo::with_view(18_262.0, 18_263.0);
        let ctx = TickContext::new(&axis, &settings);
        let formatter = AutoDateFormatter::new(AutoDateLocator::new());
        assert_eq!(formatter.format_for(&ctx), "%m-%d %H");
        let labels = formatter
            .format_ticks(&[18_262.0, 18_262.125], &ctx)
            .unwrap();
        assert_eq!(labels.labels, ["01-01 00", "01-01 03"]);
        let mut weekly =
            AutoDateFormatter::new(RRuleLocator::weekdays(&[], 1)).with_defaultfmt("%d/%m");
        assert_eq!(weekly.format_for(&ctx), "%Y-%m-%d");
        weekly.set_scaled(1.0, "%d");
        assert_eq!(weekly.format_data(18_262.0, &ctx).unwrap(), "01");
    }

    #[test]
    fn formats_come_from_the_rc_params() {
        let settings = TickerSettings::default();
        let axis = AxisInfo::with_view(18_262.0, 18_263.0);
        let ctx = TickContext::new(&axis, &settings);
        let format = crate::rc::context(|| {
            crate::rc::set("date.autoformatter.hour", "%H h").unwrap();
            AutoDateFormatter::new(AutoDateLocator::new())
                .format_for(&ctx)
                .to_owned()
        });
        assert_eq!(format, "%H h");
    }
}
