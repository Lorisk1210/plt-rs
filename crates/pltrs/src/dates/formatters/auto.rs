//! Formats chosen from the locator's tick unit (matplotlib's
//! `AutoDateFormatter`).

use std::rc::Rc;

use super::{format_date, resolve_usetex, zone_or_rc};
use crate::dates::convert::MICROSECONDS_PER_DAY;
use crate::dates::locators::DateLocator;
use crate::dates::tz::TimeZone;
use crate::ticker::{FormattedTicks, Formatter, Locator, TickContext, TickerError};

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
/// Units beyond the largest scale use `defaultfmt`. An axis given a new
/// major locator hands it to the formatter, as matplotlib's does; a
/// locator that is not a date locator counts as a unit of one day, so
/// explicit ticks on a date axis are labelled with dates.
///
/// Without [`with_tz`](Self::with_tz) the formatter labels in the zone the
/// `timezone` rcParam names when it formats, as matplotlib's makes a
/// `DateFormatter` with `tz=None` for each label; a name the system does
/// not know makes formatting fail, where matplotlib's constructor already
/// raises.
#[derive(Clone, Debug)]
pub struct AutoDateFormatter {
    locator: Rc<dyn Locator>,
    tz: Option<TimeZone>,
    defaultfmt: String,
    scaled: Vec<(f64, String)>,
    usetex: Option<bool>,
}

pltrs_core::archive_struct!(AutoDateFormatter {
    locator,
    tz,
    defaultfmt,
    scaled,
    usetex
});

impl AutoDateFormatter {
    /// A formatter following `locator`'s unit, in the zone of the
    /// `timezone` rcParam, with the formats of the `date.autoformatter.*`
    /// rcParams.
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
            locator: Rc::new(locator),
            tz: None,
            defaultfmt: "%Y-%m-%d".to_owned(),
            scaled: Vec::new(),
            usetex: None,
        };
        for (scale, format) in scaled {
            formatter.set_scaled(scale, format);
        }
        formatter
    }

    /// Formats in `tz` instead of the zone of the `timezone` rcParam.
    #[must_use]
    pub fn with_tz(mut self, tz: TimeZone) -> Self {
        self.tz = Some(tz);
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

    /// The format used for the context's view: that of the locator's
    /// tick unit, or of a day when the locator is no date locator
    /// (matplotlib's fallback when it has no `_get_unit`).
    pub fn format_for(&self, ctx: &TickContext<'_>) -> &str {
        let unit = self.locator.date_unit(ctx).unwrap_or(1.0);
        self.scaled
            .iter()
            .find(|(scale, _)| *scale >= unit)
            .map_or(&self.defaultfmt, |(_, format)| format)
    }
}

impl Formatter for AutoDateFormatter {
    fn format_ticks(
        &self,
        locs: &[f64],
        ctx: &TickContext<'_>,
    ) -> Result<FormattedTicks, TickerError> {
        let format = self.format_for(ctx);
        let usetex = resolve_usetex(self.usetex, ctx);
        let tz = zone_or_rc(self.tz.as_ref())?;
        FormattedTicks::each(locs, |x| format_date(x, format, &tz, usetex))
    }

    fn format_data(&self, value: f64, ctx: &TickContext<'_>) -> Result<String, TickerError> {
        let tz = zone_or_rc(self.tz.as_ref())?;
        format_date(
            value,
            self.format_for(ctx),
            &tz,
            resolve_usetex(self.usetex, ctx),
        )
    }

    /// The formatter choosing its formats from `locator`'s unit
    /// (matplotlib's `AutoDateFormatter._set_locator`).
    fn following(&self, locator: &Rc<dyn Locator>) -> Option<Rc<dyn Formatter>> {
        Some(Rc::new(Self {
            locator: Rc::clone(locator),
            ..self.clone()
        }))
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

    #[test]
    fn labels_take_the_rc_zone_when_formatting() {
        if TimeZone::named("America/New_York").is_err() {
            return; // no system zoneinfo database
        }
        let settings = TickerSettings::default();
        let ctx = TickContext::detached(&settings);
        let formatter = AutoDateFormatter::new(RRuleLocator::hours(&[], 1));
        let utc = formatter.clone().with_tz(TimeZone::Utc);
        // 2020-03-08T07:00Z, when New York's clocks skip from 02:00 to 03:00.
        let change = 18_329.0 + 7.0 / 24.0;
        let label = |formatter: &AutoDateFormatter, zone: &str| {
            crate::rc::context(|| {
                crate::rc::set("timezone", zone).unwrap();
                formatter.format_data(change, &ctx)
            })
        };
        assert_eq!(label(&formatter, "UTC").unwrap(), "03-08 07");
        assert_eq!(label(&formatter, "America/New_York").unwrap(), "03-08 03");
        assert!(label(&formatter, "Nowhere/Atlantis").is_err());
        assert_eq!(label(&utc, "Nowhere/Atlantis").unwrap(), "03-08 07");
    }

    #[test]
    fn a_locator_handed_over_chooses_the_format() {
        let settings = TickerSettings::default();
        let axis = AxisInfo::with_view(3_757.0, 18_577.0);
        let ctx = TickContext::new(&axis, &settings);
        let formatter = AutoDateFormatter::new(AutoDateLocator::new());
        assert_eq!(formatter.format_for(&ctx), "%Y");
        let fixed: Rc<dyn Locator> = Rc::new(crate::ticker::FixedLocator::new(vec![4000.0]));
        let follower = formatter
            .following(&fixed)
            .expect("date formatters follow their locator");
        let labels = follower.format_ticks(&[4000.0], &ctx).unwrap().labels;
        assert_eq!(labels, ["1980-12-14"], "no date unit counts as a day");
        let weekdays: Rc<dyn Locator> = Rc::new(RRuleLocator::weekdays(&[], 1));
        let daily = formatter.following(&weekdays).unwrap();
        assert_eq!(
            daily.format_ticks(&[4000.0], &ctx).unwrap().labels,
            ["1980-12-14"]
        );
    }
}
