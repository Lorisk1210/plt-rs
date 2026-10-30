//! Formatters that need no numeric layout logic: empty, fixed strings,
//! closures and Python format strings.

use std::fmt;
use std::rc::Rc;

use pltrs_core::archive::{External, ProgramFn};

use super::{FormattedTicks, Formatter};
use crate::ticker::pyfmt::{PrintfFormat, StrTemplate};
use crate::ticker::{TickContext, TickerError, ValueFn};

/// Labels every tick with the empty string (matplotlib's `NullFormatter`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NullFormatter;

pltrs_core::archive_struct!(NullFormatter);

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

pltrs_core::archive_struct!(FixedFormatter { seq, offset });

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

/// The function of a [`FuncFormatter`].
#[derive(Clone, Debug)]
enum LabelFn {
    /// A function of the value and the tick index.
    Ticks(ProgramFn<FormatFn>),
    /// A function of the value alone (a
    /// [`LabelFormat::Func`](crate::ticker::LabelFormat::Func)).
    Values(ProgramFn<ValueFn>),
    /// The label of each tick position, and nothing elsewhere (matplotlib's
    /// `Axis._format_with_dict`, which `set_ticklabels` formats with).
    Table(Vec<(f64, String)>),
}

pltrs_core::archive_enum!(LabelFn {
    Ticks(function),
    Values(function),
    Table(labels),
});

impl LabelFn {
    /// The label of `value` at tick `position`.
    fn label(&self, value: f64, position: Option<usize>) -> String {
        match self {
            Self::Ticks(function) => (function.get())(value, position),
            Self::Values(function) => (function.get())(value),
            Self::Table(labels) => label_at(labels, value),
        }
    }
}

/// The label of the tick exactly at `value`, or nothing (matplotlib's
/// `_format_with_dict`).
#[expect(
    clippy::float_cmp,
    reason = "labels belong to exact tick positions, as in matplotlib's dict lookup"
)]
fn label_at(table: &[(f64, String)], value: f64) -> String {
    table
        .iter()
        .find(|(loc, _)| *loc == value)
        .map(|(_, label)| label.clone())
        .unwrap_or_default()
}

/// Labels produced by a user function of the value and the tick index
/// (matplotlib's `FuncFormatter`). A figure with one is archived only if
/// the function has an origin ([`FuncFormatter::with_origin`]).
pub struct FuncFormatter {
    func: LabelFn,
    offset: String,
}

pltrs_core::archive_struct!(FuncFormatter { func, offset });

impl FuncFormatter {
    /// Formats each tick with `func(value, Some(index))`. `func` runs
    /// while the figure is laid out and drawn and gets what it needs as
    /// arguments: it is `Send`, so it cannot use the figure's handles.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::ticker::FuncFormatter;
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
    /// use pltrs::prelude::*;
    /// use pltrs::ticker::FuncFormatter;
    /// let ax = Figure::new().subplot();
    /// let other = ax.clone();
    /// let labels = FuncFormatter::new(move |x, _| format!("{x} of {:?}", other.xlim()));
    /// ```
    pub fn new(func: impl Fn(f64, Option<usize>) -> String + Send + 'static) -> Self {
        Self::labelling(LabelFn::Ticks(ProgramFn::new(Rc::new(func))))
    }

    /// A formatter calling `func` as [`new`](Self::new) does, archived as
    /// `origin`, what the program made it from (see [`ProgramFn`]).
    pub fn with_origin(
        func: impl Fn(f64, Option<usize>) -> String + Send + 'static,
        origin: External,
    ) -> Self {
        Self::labelling(LabelFn::Ticks(ProgramFn::with_origin(
            Rc::new(func),
            origin,
        )))
    }

    /// A formatter labelling each tick with `func(value)`, the function of
    /// a [`LabelFormat::Func`](crate::ticker::LabelFormat::Func).
    pub(crate) fn of_values(func: ProgramFn<ValueFn>) -> Self {
        Self::labelling(LabelFn::Values(func))
    }

    /// A formatter labelling the ticks at the positions of `labels` with
    /// their texts, and nothing elsewhere (matplotlib's
    /// `FuncFormatter(partial(axis._format_with_dict, tickd))`).
    pub(crate) fn of_table(labels: Vec<(f64, String)>) -> Self {
        Self::labelling(LabelFn::Table(labels))
    }

    /// A formatter labelling with `func`, without offset text.
    fn labelling(func: LabelFn) -> Self {
        Self {
            func,
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
                .map(|(pos, &x)| self.func.label(x, Some(pos)))
                .collect(),
            offset: self.offset.clone(),
        })
    }

    fn format_data(&self, value: f64, _: &TickContext<'_>) -> Result<String, TickerError> {
        Ok(self.func.label(value, None))
    }
}

/// Labels from an old-style `%` format string such as `"%1.2f"` applied to
/// the value (matplotlib's `FormatStrFormatter`). Minus signs stay ASCII.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatStrFormatter {
    format: PrintfFormat,
}

pltrs_core::archive_struct!(FormatStrFormatter { format });

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
        FormattedTicks::each(locs, |x| Ok(self.format.format(x)?))
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

pltrs_core::archive_struct!(StrMethodFormatter { template });

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
