//! Specs of the locators and formatters pltrs (Rust) installs on an axis
//! by itself (a scale's defaults, fixed ticks, units, polar axes): the
//! inverse of [`super::locator`] and [`super::formatter`], so that the
//! Python side can make the matching object of matplotlib's class
//! (`axis.get_major_locator()` after `set_xscale("log")` is a
//! `LogLocator` with its base and subs). A helper without a Python
//! counterpart describes as `None`.

use std::any::Any;

use pltrs::dates::{AutoDateFormatter, AutoDateLocator, ConciseDateFormatter, DateFormatter};
use pltrs::polar::{RadialLocator, ThetaFormatter, ThetaLocator};
use pltrs::ticker::{
    AsinhLocator, AutoLocator, AutoMinorLocator, EngFormatter, FixedFormatter, FixedLocator,
    FormatStrFormatter, Formatter, IndexLocator, LinearLocator, Locator, LogFormatter,
    LogLabelStyle, LogLocator, LogSubs, LogitFormatter, LogitLocator, MaxNLocator, MinorDivisions,
    MultipleLocator, Nbins, NullFormatter, NullLocator, NumTicks, OffsetMode, PercentFormatter,
    Prune, ScalarFormatter, StrMethodFormatter, SymmetricalLogLocator,
};
use pltrs::units::{CategoryFormatter, CategoryLocator};

use crate::binding::new_dict;
use crate::binding::prelude::*;

/// A spec being written: the dict of a helper's kind and parameters.
struct Spec<'py>(Bound<'py>);

impl<'py> Spec<'py> {
    /// The spec of a helper of `kind`.
    fn new(py: Python<'py>, kind: &str) -> PyResult<Self> {
        let dict = new_dict(py)?;
        dict.set_item("kind", kind)?;
        Ok(Self(dict))
    }

    /// The spec with the parameter `key` set to `value`.
    fn with(self, key: &str, value: impl IntoPy) -> PyResult<Self> {
        self.0.set_item(key, value)?;
        Ok(self)
    }
}

/// `"auto"` or a count, as matplotlib spells `nbins` and `numticks`.
fn auto_or_count(py: Python<'_>, count: Option<usize>) -> PyResult<Bound<'_>> {
    match count {
        Some(count) => count.into_py(py),
        None => "auto".into_py(py),
    }
}

/// The parameters of a `MaxNLocator` added to `spec`.
fn with_max_n<'py>(py: Python<'py>, spec: Spec<'py>, locator: &MaxNLocator) -> PyResult<Spec<'py>> {
    let nbins = match locator.nbins() {
        Nbins::Auto => None,
        Nbins::Fixed(count) => Some(count),
    };
    let prune = locator.prune().map(|prune| match prune {
        Prune::Lower => "lower",
        Prune::Upper => "upper",
        Prune::Both => "both",
    });
    spec.with("nbins", auto_or_count(py, nbins)?)?
        .with("steps", locator.steps().to_vec())?
        .with("integer", locator.integer())?
        .with("symmetric", locator.symmetric())?
        .with("prune", prune)?
        .with("min_n_ticks", locator.min_n_ticks())
}

/// The spec of `locator`, or `None` for one without a Python counterpart.
pub(crate) fn locator<'py>(py: Python<'py>, locator: &dyn Locator) -> PyResult<Option<Bound<'py>>> {
    let any: &dyn Any = locator;
    let spec = if any.is::<NullLocator>() {
        Spec::new(py, "null")?
    } else if any.is::<AutoLocator>() {
        Spec::new(py, "auto")?
    } else if let Some(maxn) = any.downcast_ref::<MaxNLocator>() {
        with_max_n(py, Spec::new(py, "maxn")?, maxn)?
    } else if let Some(logit) = any.downcast_ref::<LogitLocator>() {
        let nbins = match logit.as_max_n().nbins() {
            Nbins::Auto => None,
            Nbins::Fixed(count) => Some(count),
        };
        Spec::new(py, "logit")?
            .with("minor", logit.minor())?
            .with("nbins", auto_or_count(py, nbins)?)?
    } else if let Some(fixed) = any.downcast_ref::<FixedLocator>() {
        Spec::new(py, "fixed")?
            .with("locs", fixed.locs().to_vec())?
            .with("nbins", fixed.nbins())?
    } else if let Some(index) = any.downcast_ref::<IndexLocator>() {
        Spec::new(py, "index")?
            .with("base", index.base())?
            .with("offset", index.offset())?
    } else if let Some(linear) = any.downcast_ref::<LinearLocator>() {
        Spec::new(py, "linear")?.with("numticks", linear.numticks())?
    } else if let Some(multiple) = any.downcast_ref::<MultipleLocator>() {
        Spec::new(py, "multiple")?
            .with("base", multiple.base())?
            .with("offset", multiple.offset())?
    } else if let Some(log) = any.downcast_ref::<LogLocator>() {
        log_locator(py, log)?
    } else if let Some(symlog) = any.downcast_ref::<SymmetricalLogLocator>() {
        Spec::new(py, "symlog")?
            .with("base", symlog.base())?
            .with("linthresh", symlog.linthresh())?
            .with("subs", symlog.subs().to_vec())?
            .with("numticks", symlog.numticks())?
    } else if let Some(asinh) = any.downcast_ref::<AsinhLocator>() {
        Spec::new(py, "asinh")?
            .with("linear_width", asinh.linear_width())?
            .with("numticks", asinh.numticks())?
            .with("symthresh", asinh.symthresh())?
            .with("base", asinh.base())?
            .with("subs", asinh.subs().map(<[f64]>::to_vec))?
    } else if let Some(minor) = any.downcast_ref::<AutoMinorLocator>() {
        let n = match minor.ndivs() {
            None => Bound::none(py),
            Some(MinorDivisions::Auto) => "auto".into_py(py)?,
            Some(MinorDivisions::Count(count)) => count.into_py(py)?,
        };
        Spec::new(py, "autominor")?.with("n", n)?
    } else if let Some(theta) = any.downcast_ref::<ThetaLocator>() {
        return wrapping(py, "theta", theta.base());
    } else if let Some(radial) = any.downcast_ref::<RadialLocator>() {
        return wrapping(py, "radial", radial.base());
    } else if any.is::<CategoryLocator>() {
        Spec::new(py, "category")?
    } else if any.is::<AutoDateLocator>() {
        Spec::new(py, "date_auto")?
    } else {
        return Ok(None);
    };
    Ok(Some(spec.0))
}

/// The spec of a `LogLocator`.
fn log_locator<'py>(py: Python<'py>, log: &LogLocator) -> PyResult<Spec<'py>> {
    let subs = match log.subs() {
        LogSubs::Auto => "auto".into_py(py)?,
        LogSubs::All => "all".into_py(py)?,
        LogSubs::Explicit(subs) => subs.clone().into_py(py)?,
    };
    let numticks = match log.numticks() {
        NumTicks::Auto => None,
        NumTicks::Fixed(count) => Some(count),
    };
    Spec::new(py, "log")?
        .with("base", log.base())?
        .with("subs", subs)?
        .with("numticks", auto_or_count(py, numticks)?)
}

/// The spec of a locator of `kind` wrapping `base` (polar axes' θ and
/// radial locators), `None` when the base has no spec.
fn wrapping<'py>(py: Python<'py>, kind: &str, base: &dyn Locator) -> PyResult<Option<Bound<'py>>> {
    let Some(base) = locator(py, base)? else {
        return Ok(None);
    };
    Ok(Some(Spec::new(py, kind)?.with("base", base)?.0))
}

/// The kind of a log formatter of `style`, as [`super::log_style`] reads
/// it.
fn log_kind(style: LogLabelStyle) -> &'static str {
    match style {
        LogLabelStyle::Plain => "log",
        LogLabelStyle::Exponent => "log_exponent",
        LogLabelStyle::Mathtext => "log_mathtext",
        LogLabelStyle::SciNotation => "log_scinotation",
    }
}

/// The spec of `formatter`, or `None` for one without a Python
/// counterpart.
pub(crate) fn formatter<'py>(
    py: Python<'py>,
    formatter: &dyn Formatter,
) -> PyResult<Option<Bound<'py>>> {
    let any: &dyn Any = formatter;
    let spec = if any.is::<NullFormatter>() {
        Spec::new(py, "null")?
    } else if let Some(fixed) = any.downcast_ref::<FixedFormatter>() {
        Spec::new(py, "fixed")?.with("seq", fixed.seq().to_vec())?
    } else if let Some(format) = any.downcast_ref::<FormatStrFormatter>() {
        Spec::new(py, "format_str")?.with("fmt", format.fmt())?
    } else if let Some(format) = any.downcast_ref::<StrMethodFormatter>() {
        Spec::new(py, "str_method")?.with("fmt", format.fmt())?
    } else if let Some(scalar) = any.downcast_ref::<ScalarFormatter>() {
        scalar_formatter(py, scalar)?
    } else if let Some(log) = any.downcast_ref::<LogFormatter>() {
        let (subset, all) = log.minor_thresholds();
        Spec::new(py, log_kind(log.style()))?
            .with("base", log.base())?
            .with("label_only_base", log.label_only_base())?
            .with("minor_thresholds", (subset, all))?
            .with("linthresh", log.linthresh())?
    } else if let Some(logit) = any.downcast_ref::<LogitFormatter>() {
        Spec::new(py, "logit")?
            .with("use_overline", logit.use_overline())?
            .with("one_half", logit.one_half())?
            .with("minor", logit.minor())?
            .with("minor_threshold", logit.minor_threshold())?
            .with("minor_number", logit.minor_number())?
    } else if let Some(eng) = any.downcast_ref::<EngFormatter>() {
        Spec::new(py, "eng")?
            .with("unit", eng.unit())?
            .with("places", eng.places())?
            .with("sep", eng.sep())?
    } else if let Some(percent) = any.downcast_ref::<PercentFormatter>() {
        Spec::new(py, "percent")?
            .with("xmax", percent.xmax())?
            .with("decimals", percent.decimals())?
    } else if any.is::<ThetaFormatter>() {
        Spec::new(py, "theta")?
    } else if any.is::<CategoryFormatter>() {
        Spec::new(py, "category")?
    } else if any.is::<AutoDateFormatter>() {
        Spec::new(py, "date_auto")?
    } else if any.is::<ConciseDateFormatter>() {
        Spec::new(py, "date_concise")?
    } else if let Some(date) = any.downcast_ref::<DateFormatter>() {
        Spec::new(py, "date")?.with("fmt", date.format())?
    } else {
        return Ok(None);
    };
    Ok(Some(spec.0))
}

/// The spec of a `ScalarFormatter`.
fn scalar_formatter<'py>(py: Python<'py>, scalar: &ScalarFormatter) -> PyResult<Spec<'py>> {
    let use_offset = match scalar.use_offset() {
        OffsetMode::Auto => true.into_py(py)?,
        OffsetMode::Off => false.into_py(py)?,
        OffsetMode::Fixed(offset) => offset.into_py(py)?,
    };
    Spec::new(py, "scalar")?
        .with("use_offset", use_offset)?
        .with("use_mathtext", scalar.use_mathtext())?
        .with("usetex", scalar.usetex())?
        .with("scientific", scalar.scientific())?
        .with("powerlimits", scalar.powerlimits())
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::binding::dict_value;
    use crate::binding::testing::with_python;

    #[test]
    fn helpers_describe_as_their_python_specs() {
        with_python(|py| {
            let log = LogLocator::new(2.0)
                .expect("a valid base")
                .with_subs(LogSubs::Auto);
            let spec = locator(py, &log).unwrap().expect("a log spec");
            assert_eq!(dict_value::<String>(&spec, "kind").unwrap(), "log");
            assert_eq!(dict_value::<f64>(&spec, "base").unwrap(), 2.0);
            assert_eq!(dict_value::<String>(&spec, "subs").unwrap(), "auto");
            let theta = ThetaLocator::wrapping(Rc::new(AutoLocator::new()));
            let spec = locator(py, &theta).unwrap().expect("a theta spec");
            let base: Bound<'_> = dict_value(&spec, "base").unwrap();
            assert_eq!(dict_value::<String>(&base, "kind").unwrap(), "auto");
            let scalar = formatter(py, &ScalarFormatter::new())
                .unwrap()
                .expect("a spec");
            assert!(dict_value::<bool>(&scalar, "use_offset").unwrap());
            let opaque = pltrs::ticker::FuncFormatter::new(|_, _| String::new());
            assert!(formatter(py, &opaque).unwrap().is_none());
        });
    }
}
