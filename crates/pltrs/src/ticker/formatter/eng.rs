//! `EngFormatter`: engineering notation with SI prefixes (`10 MHz`).

use std::collections::BTreeMap;

use super::scalar::{ScalarLayout, compute_offset, decimals};
use super::{FormattedTicks, Formatter, OffsetMode};
use crate::ticker::pyfmt::{fixed, general};
use crate::ticker::pymath::{floor_int, log_base, pow10, pow10_int, py_round};
use crate::ticker::{TickContext, TickerError, TickerSettings};

/// matplotlib's `EngFormatter.ENG_PREFIXES`: the SI prefix of each power
/// of 1000 from `10^-30` to `10^30`.
pub const SI_PREFIXES: [(i64, &str); 21] = [
    (-30, "q"),
    (-27, "r"),
    (-24, "y"),
    (-21, "z"),
    (-18, "a"),
    (-15, "f"),
    (-12, "p"),
    (-9, "n"),
    (-6, "\u{b5}"),
    (-3, "m"),
    (0, ""),
    (3, "k"),
    (6, "M"),
    (9, "G"),
    (12, "T"),
    (15, "P"),
    (18, "E"),
    (21, "Z"),
    (24, "Y"),
    (27, "R"),
    (30, "Q"),
];

/// Formats values as a mantissa in `[1, 1000)` with an SI prefix and unit,
/// e.g. `4.7 kΩ` (matplotlib's `EngFormatter`).
///
/// ```
/// use pltrs::ticker::{EngFormatter, Formatter, TickContext};
/// let hz = EngFormatter::new().with_unit("Hz");
/// assert_eq!(hz.format_data(1e7, &TickContext::default()).unwrap(), "10 MHz");
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct EngFormatter {
    unit: String,
    places: Option<usize>,
    sep: String,
    use_mathtext: bool,
    usetex: bool,
    use_offset: OffsetMode,
    offset_threshold: i32,
    prefixes: BTreeMap<i64, String>,
}

impl EngFormatter {
    /// No unit, `%g`-style mantissas, a space separator and no offset;
    /// other settings from matplotlib's default rcParams.
    pub fn new() -> Self {
        Self::from_settings(&TickerSettings::DEFAULT)
    }

    /// Like [`EngFormatter::new`] with mathtext/TeX and the offset
    /// threshold taken from `settings`.
    pub fn from_settings(settings: &TickerSettings) -> Self {
        Self {
            unit: String::new(),
            places: None,
            sep: " ".to_owned(),
            use_mathtext: settings.axes_formatter_use_mathtext,
            usetex: settings.text_usetex,
            use_offset: OffsetMode::Off,
            offset_threshold: settings.axes_formatter_offset_threshold,
            prefixes: SI_PREFIXES
                .iter()
                .map(|&(exponent, prefix)| (exponent, prefix.to_owned()))
                .collect(),
        }
    }

    /// The unit appended after the prefix, e.g. `"Hz"`.
    #[must_use]
    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = unit.into();
        self
    }

    /// Digits after the decimal point (`None`: `%g`, up to 6 significant
    /// digits).
    #[must_use]
    pub fn with_places(mut self, places: Option<usize>) -> Self {
        self.places = places;
        self
    }

    /// Separator between the number and the prefix/unit.
    #[must_use]
    pub fn with_sep(mut self, sep: impl Into<String>) -> Self {
        self.sep = sep.into();
        self
    }

    /// Wraps the number in mathtext `$...$`.
    #[must_use]
    pub fn with_use_mathtext(mut self, use_mathtext: bool) -> Self {
        self.use_mathtext = use_mathtext;
        self
    }

    /// Wraps the number in TeX math mode.
    #[must_use]
    pub fn with_usetex(mut self, usetex: bool) -> Self {
        self.usetex = usetex;
        self
    }

    /// Offset notation; an automatic offset is rounded to the view centre
    /// so labels stay within three digits.
    #[must_use]
    pub fn with_use_offset(mut self, use_offset: OffsetMode) -> Self {
        self.use_offset = use_offset;
        self
    }

    /// The prefix of each power of ten (by default [`SI_PREFIXES`]):
    /// numbers are written with the prefix of the power of 1000 below
    /// them, within the table's range (matplotlib's `ENG_PREFIXES`, which
    /// code may change).
    ///
    /// ```
    /// use pltrs::ticker::{EngFormatter, Formatter, TickContext};
    /// let bytes = EngFormatter::new()
    ///     .with_unit("B")
    ///     .with_prefixes([(0, String::new()), (3, "K".to_owned())].into());
    /// let ctx = TickContext::default();
    /// assert_eq!(bytes.format_data(2048.0, &ctx).unwrap(), "2.048 KB");
    /// assert_eq!(bytes.format_data(3e6, &ctx).unwrap(), "3000 KB");
    /// ```
    #[must_use]
    pub fn with_prefixes(mut self, prefixes: BTreeMap<i64, String>) -> Self {
        self.prefixes = prefixes;
        self
    }

    /// The unit.
    pub fn unit(&self) -> &str {
        &self.unit
    }

    /// The fixed number of decimals, if any.
    pub fn places(&self) -> Option<usize> {
        self.places
    }

    /// The separator.
    pub fn sep(&self) -> &str {
        &self.sep
    }

    fn math(&self) -> bool {
        self.use_mathtext || self.usetex
    }

    fn mantissa_text(&self, mantissa: f64) -> String {
        match self.places {
            Some(places) => fixed(mantissa, places),
            None => general(mantissa, 6),
        }
    }

    /// matplotlib's `format_eng` (no Unicode minus).
    fn format_eng(&self, value: f64) -> Result<String, TickerError> {
        let (sign, magnitude) = if value < 0.0 {
            (-1.0, -value)
        } else {
            (1.0, value)
        };
        let (mut exponent, magnitude) = if magnitude == 0.0 {
            // Avoid "-0" for negative zero.
            (0, 0.0)
        } else {
            let decade = floor_int(magnitude.log10() / 3.0, "value")?;
            (decade * 3, magnitude)
        };
        let (Some((&lowest, _)), Some((&highest, _))) = (
            self.prefixes.first_key_value(),
            self.prefixes.last_key_value(),
        ) else {
            return Err(TickerError::invalid("ENG_PREFIXES", "no prefixes"));
        };
        exponent = exponent.clamp(lowest, highest);
        let mut mantissa = sign * magnitude / pow10(exponent as f64);
        // 999.9... may round up to 1000: use the next prefix instead.
        let rounded: f64 = self
            .mantissa_text(mantissa)
            .parse()
            .map_err(|_| TickerError::NonFinite("mantissa"))?;
        if rounded.abs() >= 1000.0 && exponent < highest {
            mantissa /= 1000.0;
            exponent += 3;
        }
        let prefix = self.prefixes.get(&exponent).ok_or_else(|| {
            TickerError::invalid("ENG_PREFIXES", format!("no prefix for 10^{exponent}"))
        })?;
        let suffix = if self.unit.is_empty() && prefix.is_empty() {
            String::new()
        } else {
            format!("{}{prefix}{}", self.sep, self.unit)
        };
        let number = self.mantissa_text(mantissa);
        Ok(if self.math() {
            format!("${number}${suffix}")
        } else {
            format!("{number}{suffix}")
        })
    }

    /// The offset text (only shown when an offset is in use).
    fn offset_text(
        &self,
        layout: &ScalarLayout,
        settings: &TickerSettings,
    ) -> Result<String, TickerError> {
        if layout.offset == 0.0 {
            return Ok(String::new());
        }
        let mut offset = self.format_eng(layout.offset)?;
        if layout.offset > 0.0 {
            offset.insert(0, '+');
        }
        let sci = self.format_eng(pow10_int(layout.oom))?;
        let text = if self.math() {
            let sci = if sci.is_empty() {
                sci
            } else {
                format!("\\times{sci}")
            };
            format!("${sci}{offset}$")
        } else {
            sci + &offset
        };
        Ok(settings.fix_minus(&text))
    }
}

impl Default for EngFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for EngFormatter {
    fn format_ticks(
        &self,
        locs: &[f64],
        ctx: &TickContext<'_>,
    ) -> Result<FormattedTicks, TickerError> {
        if locs.is_empty() {
            return Ok(FormattedTicks::default());
        }
        let settings = ctx.settings();
        let view = ctx.view_interval();
        let (vmin, vmax) = if view.1 < view.0 {
            (view.1, view.0)
        } else {
            view
        };
        let offset = match self.use_offset {
            OffsetMode::Auto => {
                if compute_offset(locs, view, self.offset_threshold) == 0.0 {
                    0.0
                } else {
                    py_round(vmin.midpoint(vmax), 3)
                }
            }
            mode => mode.fixed_offset(),
        };
        let span = vmax - vmin;
        if span <= 0.0 {
            return Err(TickerError::Domain("log of an empty view"));
        }
        let oom = i32::try_from(floor_int(log_base(span, 1000.0), "view span")? * 3)
            .map_err(|_| TickerError::NonFinite("order of magnitude"))?;
        let layout = ScalarLayout {
            offset,
            oom,
            decimals: decimals(locs, view, offset, oom)?,
        };
        let labels = locs
            .iter()
            .map(|&x| {
                if offset == 0.0 {
                    Ok(settings.fix_minus(&self.format_eng(x)?))
                } else {
                    Ok(layout.label(x, self.math(), settings))
                }
            })
            .collect::<Result<_, TickerError>>()?;
        Ok(FormattedTicks {
            labels,
            offset: self.offset_text(&layout, settings)?,
        })
    }

    /// The engineering representation; note that matplotlib keeps an
    /// ASCII hyphen here (`-1 k`) while tick labels use `axes.unicode_minus`.
    fn format_data(&self, value: f64, _: &TickContext<'_>) -> Result<String, TickerError> {
        self.format_eng(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ticker::AxisInfo;

    fn eng(formatter: &EngFormatter, value: f64) -> String {
        formatter
            .format_data(value, &TickContext::default())
            .unwrap()
    }

    #[test]
    fn engineering_notation() {
        let f = EngFormatter::new();
        assert_eq!(eng(&f, 0.0), "0");
        assert_eq!(eng(&f, 999.9999), "1 k");
        assert_eq!(eng(&f, -1.5e-6), "-1.5 \u{b5}");
        assert_eq!(eng(&f, 1e33), "1000 Q");
        assert_eq!(eng(&f, 1e-33), "0.001 q");
        let places = EngFormatter::new().with_places(Some(1));
        assert_eq!(eng(&places, 999.99), "1.0 k");
        let math = EngFormatter::new().with_use_mathtext(true).with_unit("W");
        assert_eq!(eng(&math, 1000.0), "$1$ kW");
        assert!(f.format_data(f64::NAN, &TickContext::default()).is_err());
        assert!(
            f.format_data(f64::INFINITY, &TickContext::default())
                .is_err()
        );
    }

    #[test]
    fn a_changed_prefix_table_names_and_bounds_the_prefixes() {
        let mut prefixes: BTreeMap<i64, String> = SI_PREFIXES
            .iter()
            .map(|&(exponent, prefix)| (exponent, prefix.to_owned()))
            .collect();
        prefixes.insert(0, "_".to_owned());
        let marked = EngFormatter::new().with_prefixes(prefixes);
        assert_eq!(eng(&marked, 12.0), "12 _");
        assert_eq!(eng(&marked, 1.2e4), "12 k");
        let narrow =
            EngFormatter::new().with_prefixes([(-3, "m".to_owned()), (3, "k".to_owned())].into());
        assert_eq!(eng(&narrow, 1e7), "10000 k");
        assert_eq!(eng(&narrow, 1e-7), "0.0001 m");
        assert!(narrow.format_data(5.0, &TickContext::default()).is_err());
        let empty = EngFormatter::new().with_prefixes(BTreeMap::new());
        assert!(empty.format_data(5.0, &TickContext::default()).is_err());
    }

    #[test]
    fn tick_labels_use_unicode_minus() {
        let f = EngFormatter::new().with_places(Some(1));
        let axis = AxisInfo::with_view(-5e9, 5e9);
        let settings = TickerSettings::default();
        let out = f
            .format_ticks(&[-2e9, 0.0, 2e9], &TickContext::new(&axis, &settings))
            .unwrap();
        assert_eq!(out.labels, ["\u{2212}2.0 G", "0.0", "2.0 G"]);
        assert_eq!(out.offset, "");
    }
}
