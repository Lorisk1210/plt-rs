//! Formatters that need no numeric layout logic: empty, fixed strings,
//! closures and Python format strings.

use std::fmt;

use super::{FormattedTicks, Formatter};
use crate::ticker::pyfmt::{PrintfFormat, StrTemplate};
use crate::ticker::{TickContext, TickerError};

/// Labels every tick with the empty string (matplotlib's `NullFormatter`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NullFormatter;

impl Formatter for NullFormatter {
    fn format_ticks(
        &self,
        locs: &[f64],
        _: &TickContext<'_>,
    ) -> Result<FormattedTicks, TickerError> {
        Ok(FormattedTicks {
            labels: vec![String::new(); locs.len()],
            offset: String::new(),
        })
    }

    fn format_data(&self, _: f64, _: &TickContext<'_>) -> Result<String, TickerError> {
        Ok(String::new())
    }
}

/// Labels the `i`-th tick with the `i`-th string, regardless of its value;
/// meant for use with a `FixedLocator` (matplotlib's `FixedFormatter`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FixedFormatter {
    seq: Vec<String>,
    offset: String,
}

impl FixedFormatter {
    /// Labels taken from `seq` by position.
    pub fn new(seq: Vec<String>) -> Self {
        Self {
            seq,
            offset: String::new(),
        }
    }

    /// Sets the offset text.
    #[must_use]
    pub fn with_offset_string(mut self, offset: impl Into<String>) -> Self {
        self.offset = offset.into();
        self
    }

    /// The label sequence.
    pub fn seq(&self) -> &[String] {
        &self.seq
    }
}

impl Formatter for FixedFormatter {
    fn format_ticks(
        &self,
        locs: &[f64],
        _: &TickContext<'_>,
    ) -> Result<FormattedTicks, TickerError> {
        let labels = (0..locs.len())
            .map(|pos| self.seq.get(pos).cloned().unwrap_or_default())
            .collect();
        Ok(FormattedTicks {
            labels,
            offset: self.offset.clone(),
        })
    }

    /// Without a position there is no label.
    fn format_data(&self, _: f64, _: &TickContext<'_>) -> Result<String, TickerError> {
        Ok(String::new())
    }
}

/// The signature of a [`FuncFormatter`] callback: `(value, position)`.
type FormatFn = dyn Fn(f64, Option<usize>) -> String;

/// Labels produced by a user function of the value and the tick index
/// (matplotlib's `FuncFormatter`).
pub struct FuncFormatter {
    func: Box<FormatFn>,
    offset: String,
}

impl FuncFormatter {
    /// Formats each tick with `func(value, Some(index))`. `func` runs
    /// while the figure is laid out and drawn and gets what it needs as
    /// arguments: it is `Send`, so it cannot use the figure's handles.
    ///
    /// ```
    /// use plt::prelude::*;
    /// use plt::ticker::FuncFormatter;
    /// let ax = Figure::new().subplot();
    /// ax.set_xticks([0.0, 0.5]);
    /// ax.xaxis().set_major_formatter(FuncFormatter::new(|x, _| format!("{:.0}%", 100.0 * x)));
    /// assert_eq!(ax.xaxis().ticklabels(false)?, ["0%", "50%"]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// A handle of the figure does not go into `func`:
    ///
    /// ```compile_fail
    /// use plt::prelude::*;
    /// use plt::ticker::FuncFormatter;
    /// let ax = Figure::new().subplot();
    /// let other = ax.clone();
    /// let labels = FuncFormatter::new(move |x, _| format!("{x} of {:?}", other.xlim()));
    /// ```
    pub fn new(func: impl Fn(f64, Option<usize>) -> String + Send + 'static) -> Self {
        Self::from_fn(func)
    }

    /// A formatter running plt-rs' own function, which never uses a
    /// figure's handles, or a function a public constructor already took.
    pub(crate) fn from_fn(func: impl Fn(f64, Option<usize>) -> String + 'static) -> Self {
        Self {
            func: Box::new(func),
            offset: String::new(),
        }
    }

    /// Sets the offset text.
    #[must_use]
    pub fn with_offset_string(mut self, offset: impl Into<String>) -> Self {
        self.offset = offset.into();
        self
    }
}

impl fmt::Debug for FuncFormatter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FuncFormatter")
            .field("offset", &self.offset)
            .finish_non_exhaustive()
    }
}

impl Formatter for FuncFormatter {
    fn format_ticks(
        &self,
        locs: &[f64],
        _: &TickContext<'_>,
    ) -> Result<FormattedTicks, TickerError> {
        Ok(FormattedTicks {
            labels: locs
                .iter()
                .enumerate()
                .map(|(pos, &x)| (self.func)(x, Some(pos)))
                .collect(),
            offset: self.offset.clone(),
        })
    }

    fn format_data(&self, value: f64, _: &TickContext<'_>) -> Result<String, TickerError> {
        Ok((self.func)(value, None))
    }
}

/// Labels from an old-style `%` format string such as `"%1.2f"` applied to
/// the value (matplotlib's `FormatStrFormatter`). Minus signs stay ASCII.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatStrFormatter {
    format: PrintfFormat,
}

impl FormatStrFormatter {
    /// Parses `fmt`, which must hold exactly one float-compatible
    /// conversion.
    pub fn new(fmt: &str) -> Result<Self, TickerError> {
        Ok(Self {
            format: PrintfFormat::parse(fmt)?,
        })
    }

    /// The format string.
    pub fn fmt(&self) -> &str {
        self.format.template()
    }
}

impl Formatter for FormatStrFormatter {
    fn format_ticks(
        &self,
        locs: &[f64],
        _: &TickContext<'_>,
    ) -> Result<FormattedTicks, TickerError> {
        let labels = locs
            .iter()
            .map(|&x| self.format.format(x))
            .collect::<Result<_, _>>()?;
        Ok(FormattedTicks {
            labels,
            offset: String::new(),
        })
    }

    fn format_data(&self, value: f64, _: &TickContext<'_>) -> Result<String, TickerError> {
        Ok(self.format.format(value)?)
    }
}

/// Labels from a `str.format` template with the fields `x` (value) and
/// `pos` (tick index), e.g. `"{x:.2f}"`; minus signs in the formatted
/// fields follow `axes.unicode_minus` (matplotlib's `StrMethodFormatter`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrMethodFormatter {
    template: StrTemplate,
}

impl StrMethodFormatter {
    /// Parses and validates `fmt`.
    pub fn new(fmt: &str) -> Result<Self, TickerError> {
        Ok(Self {
            template: StrTemplate::parse(fmt)?,
        })
    }

    /// The template.
    pub fn fmt(&self) -> &str {
        self.template.template()
    }

    fn render(
        &self,
        x: f64,
        pos: Option<usize>,
        ctx: &TickContext<'_>,
    ) -> Result<String, TickerError> {
        let settings = ctx.settings();
        Ok(self
            .template
            .render(x, pos, |field| settings.fix_minus(&field))?)
    }
}

impl Formatter for StrMethodFormatter {
    fn format_ticks(
        &self,
        locs: &[f64],
        ctx: &TickContext<'_>,
    ) -> Result<FormattedTicks, TickerError> {
        let labels = locs
            .iter()
            .enumerate()
            .map(|(pos, &x)| self.render(x, Some(pos), ctx))
            .collect::<Result<_, _>>()?;
        Ok(FormattedTicks {
            labels,
            offset: String::new(),
        })
    }

    fn format_data(&self, value: f64, ctx: &TickContext<'_>) -> Result<String, TickerError> {
        self.render(value, None, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(formatter: &impl Formatter, locs: &[f64]) -> Vec<String> {
        formatter
            .format_ticks(locs, &TickContext::default())
            .unwrap()
            .labels
    }

    #[test]
    fn null_and_fixed() {
        assert_eq!(labels(&NullFormatter, &[1.0, 2.0]), ["", ""]);
        let fixed = FixedFormatter::new(vec!["a".into(), "b".into()]);
        assert_eq!(labels(&fixed, &[1.0, 2.0, 3.0]), ["a", "b", ""]);
    }

    #[test]
    fn func_formatter_sees_positions() {
        let func = FuncFormatter::new(|x, pos| format!("{x}@{pos:?}"));
        assert_eq!(labels(&func, &[1.5]), ["1.5@Some(0)"]);
        assert_eq!(
            func.format_data(2.0, &TickContext::default()).unwrap(),
            "2@None"
        );
    }

    #[test]
    fn python_format_strings() {
        let printf = FormatStrFormatter::new("%d").unwrap();
        assert_eq!(labels(&printf, &[-1.0]), ["-1"]);
        assert!(FormatStrFormatter::new("%x").is_err());
        let method = StrMethodFormatter::new("{x:.1e}").unwrap();
        assert_eq!(labels(&method, &[-1e-5]), ["\u{2212}1.0e\u{2212}05"]);
        assert!(StrMethodFormatter::new("{y}").is_err());
    }
}
