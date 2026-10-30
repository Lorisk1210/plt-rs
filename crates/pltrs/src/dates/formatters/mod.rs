//! Date tick formatters (matplotlib's `DateFormatter`,
//! `AutoDateFormatter` and `ConciseDateFormatter`).
//!
//! All implement the ticker [`Formatter`] trait on Matplotlib date numbers.
//! When `usetex` is not set explicitly it follows `text.usetex` from the
//! context's settings. Without a zone they label in the one the
//! `timezone` rcParam names: [`DateFormatter`] reads it when it is made,
//! the other two whenever they format, as matplotlib's do.

mod auto;
mod concise;

use std::borrow::Cow;

pub use auto::AutoDateFormatter;
pub use concise::ConciseDateFormatter;

use super::DateError;
use super::convert::num2date;
use super::tz::{TickZone, TimeZone};
use crate::ticker::{FormattedTicks, Formatter, TickContext, TickerError};

/// Formats ticks with a `strftime` format string (matplotlib's
/// `DateFormatter`).
///
/// ```
/// use pltrs::dates::DateFormatter;
/// use pltrs::ticker::{Formatter, TickContext};
/// let labels = DateFormatter::new("%d %b %Y")
///     .format_ticks(&[18_262.0, 18_293.0], &TickContext::default())
///     .unwrap();
/// assert_eq!(labels.labels, ["01 Jan 2020", "01 Feb 2020"]);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct DateFormatter {
    format: String,
    tz: TickZone,
    usetex: Option<bool>,
}

pltrs_core::archive_struct!(DateFormatter { format, tz, usetex });

impl DateFormatter {
    /// A formatter writing ticks with `format`, in the zone the
    /// `timezone` rcParam names now (matplotlib's `DateFormatter(fmt)`).
    /// When the system knows no zone of that name, labelling fails with
    /// [`DateError::UnknownTimeZone`], where matplotlib's constructor
    /// raises.
    pub fn new(format: impl Into<String>) -> Self {
        Self {
            format: format.into(),
            tz: TickZone::from_rc(),
            usetex: None,
        }
    }

    /// Formats in `tz` instead (matplotlib's `tz` argument, or
    /// `set_tzinfo`).
    #[must_use]
    pub fn with_tz(mut self, tz: TimeZone) -> Self {
        self.tz = tz.into();
        self
    }

    /// Whether labels are wrapped for TeX (default: `text.usetex`).
    #[must_use]
    pub fn with_usetex(mut self, usetex: bool) -> Self {
        self.usetex = Some(usetex);
        self
    }

    /// The format string.
    pub fn format(&self) -> &str {
        &self.format
    }

    /// The label of one date number (matplotlib's `__call__`).
    pub fn label(&self, x: f64, ctx: &TickContext<'_>) -> Result<String, TickerError> {
        let tz = self.tz.get()?;
        format_date(x, &self.format, tz, resolve_usetex(self.usetex, ctx))
    }
}

impl Formatter for DateFormatter {
    fn format_ticks(
        &self,
        locs: &[f64],
        ctx: &TickContext<'_>,
    ) -> Result<FormattedTicks, TickerError> {
        FormattedTicks::each(locs, |x| self.label(x, ctx))
    }

    fn format_data(&self, value: f64, ctx: &TickContext<'_>) -> Result<String, TickerError> {
        self.label(value, ctx)
    }
}

/// `usetex`, or `text.usetex` when unset.
fn resolve_usetex(usetex: Option<bool>, ctx: &TickContext<'_>) -> bool {
    usetex.unwrap_or(ctx.settings().text_usetex)
}

/// `tz`, or the zone the `timezone` rcParam names now: the zone of a
/// formatter that keeps matplotlib's `tz=None` and resolves it whenever it
/// formats.
fn zone_or_rc(tz: Option<&TimeZone>) -> Result<Cow<'_, TimeZone>, DateError> {
    tz.map_or_else(
        || TimeZone::from_rc().map(Cow::Owned),
        |tz| Ok(Cow::Borrowed(tz)),
    )
}

/// `num2date(x, tz).strftime(format)`, TeX-wrapped if requested.
fn format_date(x: f64, format: &str, tz: &TimeZone, usetex: bool) -> Result<String, TickerError> {
    let text = num2date(x, tz)?.strftime(format);
    Ok(if usetex { wrap_in_tex(&text) } else { text })
}

/// matplotlib's `_wrap_in_tex`: letters stay upright text, everything else
/// is set in `\mathdefault`, with `-` and `:` braced so TeX does not space
/// them as operators and spaces made explicit.
///
/// ```
/// use pltrs::dates::wrap_in_tex;
/// assert_eq!(wrap_in_tex("Jan-01 10:00"), r"Jan$\mathdefault{{-}01\;10{:}00}$");
/// ```
pub fn wrap_in_tex(text: &str) -> String {
    let mut body = String::with_capacity(text.len() * 2);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_alphabetic() {
            body.push_str("}$");
            body.push(c);
            while let Some(&next) = chars.peek().filter(|n| n.is_ascii_alphabetic()) {
                body.push(next);
                chars.next();
            }
            body.push_str(r"$\mathdefault{");
        } else {
            match c {
                '-' => body.push_str("{-}"),
                ':' => body.push_str("{:}"),
                ' ' => body.push_str(r"\;"),
                other => body.push(other),
            }
        }
    }
    format!(r"$\mathdefault{{{body}}}$").replace(r"$\mathdefault{}$", "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ticker::TickerSettings;

    #[test]
    fn tex_wrapping_matches_matplotlib() {
        assert_eq!(
            wrap_in_tex("2020-01-01 00:00:00"),
            r"$\mathdefault{2020{-}01{-}01\;00{:}00{:}00}$"
        );
        assert_eq!(wrap_in_tex("Jan"), "Jan");
        assert_eq!(wrap_in_tex(""), "");
        assert_eq!(wrap_in_tex("05 Mar"), r"$\mathdefault{05\;}$Mar");
    }

    #[test]
    fn usetex_follows_settings_unless_set() {
        let settings = TickerSettings {
            text_usetex: true,
            ..TickerSettings::default()
        };
        let ctx = TickContext::detached(&settings);
        let formatter = DateFormatter::new("%H:%M");
        assert_eq!(
            formatter.label(0.5, &ctx).unwrap(),
            r"$\mathdefault{12{:}00}$"
        );
        let plain = formatter.clone().with_usetex(false);
        assert_eq!(plain.label(0.5, &ctx).unwrap(), "12:00");
        assert!(formatter.label(f64::NAN, &ctx).is_err());
        assert_eq!(formatter.format(), "%H:%M");
    }

    #[test]
    fn the_zone_is_the_rc_param_when_made() {
        if TimeZone::named("Europe/Berlin").is_err() {
            return; // no system zoneinfo database
        }
        let (berlin, unknown) = crate::rc::context(|| {
            crate::rc::set("timezone", "Europe/Berlin").unwrap();
            let berlin = DateFormatter::new("%H:%M %Z");
            crate::rc::set("timezone", "Nowhere/Atlantis").unwrap();
            (berlin, DateFormatter::new("%H:%M"))
        });
        let ctx = TickContext::default();
        // 2020-03-29T01:00Z, when Berlin's clocks skip from 02:00 to 03:00.
        let change = 18_350.0 + 1.0 / 24.0;
        assert_eq!(berlin.label(change, &ctx).unwrap(), "03:00 CEST");
        assert_eq!(
            berlin.label(change - 0.5 / 24.0, &ctx).unwrap(),
            "01:30 CET"
        );
        assert!(unknown.label(change, &ctx).is_err(), "an unknown zone");
        let utc = unknown.with_tz(TimeZone::Utc);
        assert_eq!(utc.label(change, &ctx).unwrap(), "01:00");
    }

    #[test]
    fn rc_zones_resolve_at_each_call_unless_given() {
        let fixed = TimeZone::Fixed(crate::dates::UtcOffset::from_seconds(3600).unwrap());
        assert_eq!(zone_or_rc(Some(&fixed)).unwrap().as_ref(), &fixed);
        crate::rc::context(|| {
            crate::rc::set("timezone", "UTC").unwrap();
            assert_eq!(zone_or_rc(None).unwrap().as_ref(), &TimeZone::Utc);
            crate::rc::set("timezone", "Nowhere/Atlantis").unwrap();
            assert!(zone_or_rc(None).is_err());
            assert!(
                zone_or_rc(Some(&fixed)).is_ok(),
                "a zone given never reads it"
            );
        });
    }
}
