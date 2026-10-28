//! Date tick formatters (matplotlib's `DateFormatter`,
//! `AutoDateFormatter` and `ConciseDateFormatter`).
//!
//! All implement the ticker [`Formatter`] trait on Matplotlib date numbers.
//! When `usetex` is not set explicitly it follows `text.usetex` from the
//! context's settings.

mod auto;
mod concise;

pub use auto::AutoDateFormatter;
pub use concise::ConciseDateFormatter;

use super::convert::num2date;
use super::tz::TimeZone;
use crate::ticker::{FormattedTicks, Formatter, TickContext, TickerError};

/// Formats ticks with a `strftime` format string (matplotlib's
/// `DateFormatter`).
///
/// ```
/// use plt::dates::DateFormatter;
/// use plt::ticker::{Formatter, TickContext};
/// let labels = DateFormatter::new("%d %b %Y")
///     .format_ticks(&[18_262.0, 18_293.0], &TickContext::default())
///     .unwrap();
/// assert_eq!(labels.labels, ["01 Jan 2020", "01 Feb 2020"]);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct DateFormatter {
    format: String,
    tz: TimeZone,
    usetex: Option<bool>,
}

impl DateFormatter {
    /// A formatter writing ticks with `format`, in UTC.
    pub fn new(format: impl Into<String>) -> Self {
        Self {
            format: format.into(),
            tz: TimeZone::Utc,
            usetex: None,
        }
    }

    /// Formats in `tz` instead of UTC.
    #[must_use]
    pub fn with_tz(mut self, tz: TimeZone) -> Self {
        self.tz = tz;
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
        format_date(x, &self.format, &self.tz, resolve_usetex(self.usetex, ctx))
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
/// use plt::dates::wrap_in_tex;
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
}
