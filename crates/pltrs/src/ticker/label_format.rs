//! How a number becomes a label text: the `fmt`, `autopct` and `format`
//! arguments of matplotlib's labelling functions (`pie`, `bar_label`,
//! `clabel`, `colorbar`, `Sankey`).

use std::fmt;
use std::rc::Rc;

use pltrs_core::archive::{External, ProgramFn};

use super::pyfmt::{FormatError, PrintfFormat, auto_format_value};
use super::{FormatStrFormatter, Formatter, FuncFormatter, StrMethodFormatter, TickerError};

/// A template or a function turning a number into a label. matplotlib
/// reads a template differently depending on where it is given, and so
/// does pltrs: each use says how (a printf-style template only, or a
/// printf-style template falling back to `str.format`).
///
/// ```
/// use pltrs::ticker::LabelFormat;
/// let percent = LabelFormat::from("%1.1f%%");
/// let words = LabelFormat::func(|value| format!("{value} units"));
/// assert!(matches!(percent, LabelFormat::Template(_)));
/// assert!(matches!(words, LabelFormat::Func(_)));
/// ```
#[derive(Clone)]
pub enum LabelFormat {
    /// A printf-style template such as `"%1.1f%%"`, or where matplotlib
    /// accepts one, a `str.format` template such as `"{:.0%}"`.
    Template(String),
    /// A function of the value; a figure labelled by one is archived only
    /// if the function has an origin (see [`ProgramFn`]).
    Func(ProgramFn<ValueFn>),
}

/// A function turning a value into its label.
pub type ValueFn = dyn Fn(f64) -> String;

pltrs_core::archive_enum!(LabelFormat {
    Template(template),
    Func(function),
});

impl LabelFormat {
    /// The labels `f` writes. Colorbars run `f` while their figure is
    /// laid out: it is `Send`, so it cannot use the figure's handles.
    pub fn func(f: impl Fn(f64) -> String + Send + 'static) -> Self {
        Self::Func(ProgramFn::new(Rc::new(f)))
    }

    /// The labels `f` writes, archived as `origin`, what the program made
    /// `f` from.
    pub fn func_with_origin(f: impl Fn(f64) -> String + Send + 'static, origin: External) -> Self {
        Self::Func(ProgramFn::with_origin(Rc::new(f), origin))
    }

    /// The label of `value`: the template as a printf-style template only
    /// (Python's `template % value`), or the function's text.
    pub(crate) fn printf(&self, value: f64) -> Result<String, FormatError> {
        match self {
            Self::Template(template) => PrintfFormat::parse(template)?.format(value),
            Self::Func(f) => Ok((f.get())(value)),
        }
    }

    /// The label of `value`: the template as a printf-style template, or
    /// as a positional `str.format` one when Python's `%` rejects it
    /// (matplotlib's `cbook._auto_format_str`), with `value` a Python
    /// `int` when `integer`; or the function's text.
    pub(crate) fn printf_or_format(
        &self,
        value: f64,
        integer: bool,
    ) -> Result<String, FormatError> {
        match self {
            Self::Template(template) => auto_format_value(template, value, integer),
            Self::Func(f) => Ok((f.get())(value)),
        }
    }

    /// The tick formatter writing these labels: a `FormatStrFormatter` for
    /// a printf-style template, else a `StrMethodFormatter` (fields `x`
    /// and `pos`), as matplotlib's colorbar chooses; a `FuncFormatter`
    /// for a function.
    pub(crate) fn formatter(&self) -> Result<Rc<dyn Formatter>, TickerError> {
        Ok(match self {
            Self::Template(template) => match FormatStrFormatter::new(template) {
                Ok(printf) => Rc::new(printf),
                Err(_) => Rc::new(StrMethodFormatter::new(template)?),
            },
            Self::Func(f) => Rc::new(FuncFormatter::of_values(f.clone())),
        })
    }
}

impl From<&str> for LabelFormat {
    fn from(template: &str) -> Self {
        Self::Template(template.to_owned())
    }
}

impl From<String> for LabelFormat {
    fn from(template: String) -> Self {
        Self::Template(template)
    }
}

impl fmt::Debug for LabelFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Template(template) => f.debug_tuple("Template").field(template).finish(),
            Self::Func(_) => f.write_str("Func(..)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ticker::{AxisInfo, TickContext, TickerSettings};

    #[test]
    fn printf_reads_templates_as_printf_only() {
        assert_eq!(LabelFormat::from("%1.1f%%").printf(12.34).unwrap(), "12.3%");
        assert!(LabelFormat::from("{:.1f}").printf(1.0).is_err());
        let twice = LabelFormat::func(|v| format!("{}", v * 2.0));
        assert_eq!(twice.printf(1.5).unwrap(), "3");
    }

    #[test]
    fn printf_or_format_falls_back_to_str_format() {
        let format = LabelFormat::from("{:.0%}");
        assert_eq!(format.printf_or_format(0.43, false).unwrap(), "43%");
        let integer = LabelFormat::from("{:d}");
        assert_eq!(integer.printf_or_format(5.0, true).unwrap(), "5");
        assert!(integer.printf_or_format(5.0, false).is_err());
    }

    #[test]
    fn formatters_follow_the_colorbar_choice() {
        let axis = AxisInfo::with_view(0.0, 1.0);
        let settings = TickerSettings::default();
        let ctx = TickContext::new(&axis, &settings);
        let label = |format: &str| {
            LabelFormat::from(format)
                .formatter()
                .unwrap()
                .format_ticks(&[0.5], &ctx)
                .unwrap()
                .labels
        };
        assert_eq!(label("%.2f"), ["0.50"]);
        assert_eq!(label("{x:.1f}"), ["0.5"]);
        assert!(LabelFormat::from("{y}").formatter().is_err());
        let func = LabelFormat::func(|v| format!("<{v}>")).formatter().unwrap();
        assert_eq!(func.format_ticks(&[1.0], &ctx).unwrap().labels, ["<1>"]);
    }
}
