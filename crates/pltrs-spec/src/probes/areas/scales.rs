//! Probe operations for the `scales` area.
//!
//! Scales are created through [`scale_factory`] from the probe's `params`
//! (matplotlib constructor keywords); `functions` names one of the
//! reference module's forward/inverse pairs.

use std::any::Any;

use pltrs::scale::{
    AsinhScale, FuncScale, FuncScaleLog, FuncTransform, LinearScale, LogScale, LogitScale,
    NonPositive, Scale, ScaleOptions, SymmetricalLogScale, scale_factory, scale_names,
};
use pltrs::ticker::{
    AsinhLocator, AutoLocator, AutoMinorLocator, AxisName, Formatter, Locator, LogFormatter,
    LogLabelStyle, LogLocator, LogSubs, LogitFormatter, LogitLocator, MaxNLocator, MinorDivisions,
    Nbins, NullFormatter, NullLocator, NumTicks, Prune, ScalarFormatter, StrMethodFormatter,
    SymmetricalLogLocator,
};

use super::ticker::{Params, settings};
use crate::json::Json;
use crate::probes::encode::count;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("scale.get_scale_names", get_scale_names),
    Op::new("scale.scale_factory", factory),
    Op::new("scale.LinearScale.transform", linear::transform),
    Op::new("scale.LinearScale.inverted.transform", linear::inverse),
    Op::new(
        "scale.LinearScale.limit_range_for_scale",
        linear::limit_range,
    ),
    Op::new("scale.LinearScale.val_in_range", linear::val_in_range),
    Op::new(
        "scale.LinearScale.default_locators_and_formatters",
        linear::defaults,
    ),
    Op::new("scale.LogScale.transform", log::transform),
    Op::new("scale.LogScale.inverted.transform", log::inverse),
    Op::new("scale.LogScale.limit_range_for_scale", log::limit_range),
    Op::new("scale.LogScale.val_in_range", log::val_in_range),
    Op::new(
        "scale.LogScale.default_locators_and_formatters",
        log::defaults,
    ),
    Op::new("scale.SymmetricalLogScale.transform", symlog::transform),
    Op::new(
        "scale.SymmetricalLogScale.inverted.transform",
        symlog::inverse,
    ),
    Op::new(
        "scale.SymmetricalLogScale.limit_range_for_scale",
        symlog::limit_range,
    ),
    Op::new(
        "scale.SymmetricalLogScale.val_in_range",
        symlog::val_in_range,
    ),
    Op::new(
        "scale.SymmetricalLogScale.default_locators_and_formatters",
        symlog::defaults,
    ),
    Op::new("scale.LogitScale.transform", logit::transform),
    Op::new("scale.LogitScale.inverted.transform", logit::inverse),
    Op::new("scale.LogitScale.limit_range_for_scale", logit::limit_range),
    Op::new("scale.LogitScale.val_in_range", logit::val_in_range),
    Op::new(
        "scale.LogitScale.default_locators_and_formatters",
        logit::defaults,
    ),
    Op::new("scale.AsinhScale.transform", asinh::transform),
    Op::new("scale.AsinhScale.inverted.transform", asinh::inverse),
    Op::new("scale.AsinhScale.limit_range_for_scale", asinh::limit_range),
    Op::new("scale.AsinhScale.val_in_range", asinh::val_in_range),
    Op::new(
        "scale.AsinhScale.default_locators_and_formatters",
        asinh::defaults,
    ),
    Op::new("scale.FuncScale.transform", function::transform),
    Op::new("scale.FuncScale.inverted.transform", function::inverse),
    Op::new(
        "scale.FuncScale.limit_range_for_scale",
        function::limit_range,
    ),
    Op::new("scale.FuncScale.val_in_range", function::val_in_range),
    Op::new(
        "scale.FuncScale.default_locators_and_formatters",
        function::defaults,
    ),
    Op::new("scale.FuncScaleLog.transform", functionlog::transform),
    Op::new(
        "scale.FuncScaleLog.inverted.transform",
        functionlog::inverse,
    ),
    Op::new(
        "scale.FuncScaleLog.limit_range_for_scale",
        functionlog::limit_range,
    ),
    Op::new("scale.FuncScaleLog.val_in_range", functionlog::val_in_range),
    Op::new(
        "scale.FuncScaleLog.default_locators_and_formatters",
        functionlog::defaults,
    ),
];

/// The per-scale operations, parameterised by the scale's registered name.
macro_rules! scale_ops {
    ($($module:ident => $name:literal),* $(,)?) => {$(
        pub(crate) mod $module {
            use super::{Args, Outcome};

            pub(crate) fn transform(args: Args<'_>) -> Outcome {
                super::transform($name, args, false)
            }

            pub(super) fn inverse(args: Args<'_>) -> Outcome {
                super::transform($name, args, true)
            }

            pub(super) fn limit_range(args: Args<'_>) -> Outcome {
                super::limit_range($name, args)
            }

            pub(super) fn val_in_range(args: Args<'_>) -> Outcome {
                super::val_in_range($name, args)
            }

            pub(crate) fn defaults(args: Args<'_>) -> Outcome {
                super::defaults($name, args)
            }
        }
    )*};
}

scale_ops! {
    linear => "linear",
    log => "log",
    symlog => "symlog",
    logit => "logit",
    asinh => "asinh",
    function => "function",
    functionlog => "functionlog",
}

// ---------------------------------------------------------------------------
// Scale construction
// ---------------------------------------------------------------------------

/// The reference module's named forward/inverse pairs.
fn functions(name: &str) -> Result<FuncTransform, ProbeError> {
    Ok(match name {
        "square" => FuncTransform::new(|x| x * x, f64::sqrt),
        "cube" => FuncTransform::new(|x| x.powf(3.0), f64::cbrt),
        "log1p" => FuncTransform::new(f64::ln_1p, f64::exp_m1),
        "affine" => FuncTransform::new(|x| 2.0 * x + 1.0, |y| (y - 1.0) / 2.0),
        other => return Err(ProbeError::BadArgs(format!("unknown functions {other:?}"))),
    })
}

pub(crate) fn options(args: Args<'_>) -> Result<ScaleOptions, ProbeError> {
    let p = Params::of(
        args,
        &[
            "base",
            "subs",
            "nonpositive",
            "linthresh",
            "linscale",
            "linear_width",
            "one_half",
            "use_overline",
            "functions",
        ],
    )?
    .args();
    let nonpositive = match p.opt_str("nonpositive")? {
        None => None,
        Some("clip") => Some(NonPositive::Clip),
        Some("mask") => Some(NonPositive::Mask),
        Some(other) => {
            return Err(ProbeError::Rejected(format!(
                "invalid nonpositive {other:?}"
            )));
        }
    };
    let subs = p
        .get("subs")
        .map(|s| {
            s.as_f64_vec()
                .ok_or_else(|| ProbeError::BadArgs("subs".into()))
        })
        .transpose()?;
    Ok(ScaleOptions {
        base: p.opt_f64("base")?,
        subs,
        nonpositive,
        linthresh: p.opt_f64("linthresh")?,
        linscale: p.opt_f64("linscale")?,
        linear_width: p.opt_f64("linear_width")?,
        one_half: p.opt_str("one_half")?.map(str::to_owned),
        use_overline: p.opt_bool("use_overline")?,
        functions: p.opt_str("functions")?.map(functions).transpose()?,
    })
}

pub(crate) fn scale(name: &str, args: Args<'_>) -> Result<Box<dyn Scale>, ProbeError> {
    scale_factory(name, options(args)?).map_err(ProbeError::rejected)
}

/// The matplotlib class name of a scale.
fn class_name(scale: &dyn Scale) -> &'static str {
    let any: &dyn Any = scale;
    if any.is::<LinearScale>() {
        "LinearScale"
    } else if any.is::<LogScale>() {
        "LogScale"
    } else if any.is::<SymmetricalLogScale>() {
        "SymmetricalLogScale"
    } else if any.is::<LogitScale>() {
        "LogitScale"
    } else if any.is::<AsinhScale>() {
        "AsinhScale"
    } else if any.is::<FuncScale>() {
        "FuncScale"
    } else if any.is::<FuncScaleLog>() {
        "FuncScaleLog"
    } else {
        "ScaleBase"
    }
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn get_scale_names(_: Args<'_>) -> Outcome {
    Ok(Json::from(
        scale_names()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
    ))
}

pub(crate) fn factory(args: Args<'_>) -> Outcome {
    let scale = scale(args.str("scale")?, args)?;
    Ok(Json::from(class_name(scale.as_ref())))
}

fn transform(name: &str, args: Args<'_>, inverse: bool) -> Outcome {
    let scale = scale(name, args)?;
    let values = args.f64_vec("values")?;
    Ok(Json::from(if inverse {
        scale.inverse(&values)
    } else {
        scale.forward(&values)
    }))
}

fn limit_range(name: &str, args: Args<'_>) -> Outcome {
    let (lo, hi) = scale(name, args)?.limit_range_for_scale(
        args.f64("vmin")?,
        args.f64("vmax")?,
        args.f64("minpos")?,
    );
    Ok(Json::from(vec![lo, hi]))
}

fn val_in_range(name: &str, args: Args<'_>) -> Outcome {
    let scale = scale(name, args)?;
    let values = args.f64_vec("values")?;
    Ok(Json::from(
        values
            .iter()
            .map(|&v| scale.val_in_range(v))
            .collect::<Vec<_>>(),
    ))
}

fn defaults(name: &str, args: Args<'_>) -> Outcome {
    let scale = scale(name, args)?;
    let axis = match args.str("axis")? {
        "x" => AxisName::X,
        "y" => AxisName::Y,
        other => return Err(ProbeError::BadArgs(format!("unknown axis {other:?}"))),
    };
    let tickers = scale.default_tickers(axis, &settings(args)?);
    Ok(object(vec![
        (
            "major_locator",
            describe_locator(tickers.major_locator.as_ref()),
        ),
        (
            "minor_locator",
            describe_locator(tickers.minor_locator.as_ref()),
        ),
        (
            "major_formatter",
            describe_formatter(tickers.major_formatter.as_ref()),
        ),
        (
            "minor_formatter",
            describe_formatter(tickers.minor_formatter.as_ref()),
        ),
    ]))
}

// ---------------------------------------------------------------------------
// Describing tick helpers like the reference module's `describe`
// ---------------------------------------------------------------------------

fn object(fields: Vec<(&str, Json)>) -> Json {
    Json::Object(
        fields
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

fn auto_or(n: Option<usize>) -> Json {
    n.map_or_else(|| Json::from("auto"), count)
}

fn max_n_fields(class: &str, locator: &MaxNLocator) -> Vec<(&'static str, Json)> {
    let nbins = match locator.nbins() {
        Nbins::Auto => None,
        Nbins::Fixed(n) => Some(n),
    };
    let prune = locator.prune().map(|prune| match prune {
        Prune::Lower => "lower",
        Prune::Upper => "upper",
        Prune::Both => "both",
    });
    vec![
        ("class", Json::from(class.to_owned())),
        ("nbins", auto_or(nbins)),
        ("steps", Json::from(locator.steps().to_vec())),
        ("integer", Json::from(locator.integer())),
        ("symmetric", Json::from(locator.symmetric())),
        ("prune", Json::from(prune)),
        ("min_n_ticks", count(locator.min_n_ticks())),
    ]
}

fn describe_locator(locator: &dyn Locator) -> Json {
    let any: &dyn Any = locator;
    let fields = if let Some(auto) = any.downcast_ref::<AutoLocator>() {
        max_n_fields("AutoLocator", auto.as_max_n())
    } else if let Some(logit) = any.downcast_ref::<LogitLocator>() {
        let mut fields = max_n_fields("LogitLocator", logit.as_max_n());
        fields.push(("minor", Json::from(logit.minor())));
        fields
    } else if let Some(max_n) = any.downcast_ref::<MaxNLocator>() {
        max_n_fields("MaxNLocator", max_n)
    } else if let Some(log) = any.downcast_ref::<LogLocator>() {
        let subs = match log.subs() {
            LogSubs::Auto => Json::from("auto"),
            LogSubs::All => Json::from("all"),
            LogSubs::Explicit(subs) => Json::from(subs.clone()),
        };
        let numticks = match log.numticks() {
            NumTicks::Auto => None,
            NumTicks::Fixed(n) => Some(n),
        };
        vec![
            ("class", Json::from("LogLocator")),
            ("base", Json::from(log.base())),
            ("subs", subs),
            ("numticks", auto_or(numticks)),
        ]
    } else if let Some(symlog) = any.downcast_ref::<SymmetricalLogLocator>() {
        vec![
            ("class", Json::from("SymmetricalLogLocator")),
            ("base", Json::from(symlog.base())),
            ("linthresh", Json::from(symlog.linthresh())),
            ("subs", Json::from(symlog.subs().to_vec())),
            ("numticks", count(symlog.numticks())),
        ]
    } else if let Some(asinh) = any.downcast_ref::<AsinhLocator>() {
        vec![
            ("class", Json::from("AsinhLocator")),
            ("linear_width", Json::from(asinh.linear_width())),
            ("numticks", count(asinh.numticks())),
            ("symthresh", Json::from(asinh.symthresh())),
            ("base", Json::from(asinh.base())),
            ("subs", Json::from(asinh.subs().map(<[f64]>::to_vec))),
        ]
    } else if let Some(minor) = any.downcast_ref::<AutoMinorLocator>() {
        let ndivs = minor.ndivs().map(|ndivs| match ndivs {
            MinorDivisions::Auto => Json::from("auto"),
            MinorDivisions::Count(n) => Json::from(f64::from(n)),
        });
        vec![
            ("class", Json::from("AutoMinorLocator")),
            ("ndivs", ndivs.unwrap_or(Json::Null)),
        ]
    } else if any.is::<NullLocator>() {
        vec![("class", Json::from("NullLocator"))]
    } else {
        vec![("class", Json::from("Locator"))]
    };
    object(fields)
}

fn describe_formatter(formatter: &dyn Formatter) -> Json {
    let any: &dyn Any = formatter;
    let fields = if let Some(scalar) = any.downcast_ref::<ScalarFormatter>() {
        let (lo, hi) = scalar.powerlimits();
        vec![
            ("class", Json::from("ScalarFormatter")),
            ("useOffset", Json::from(scalar.use_offset().is_auto())),
            ("offset", Json::from(scalar.use_offset().fixed_offset())),
            ("useMathText", Json::from(scalar.use_mathtext())),
            ("useLocale", Json::from(false)),
            ("usetex", Json::from(scalar.usetex())),
            ("scientific", Json::from(scalar.scientific())),
            (
                "powerlimits",
                Json::from(vec![f64::from(lo), f64::from(hi)]),
            ),
            (
                "offset_threshold",
                Json::from(f64::from(scalar.offset_threshold())),
            ),
        ]
    } else if let Some(log) = any.downcast_ref::<LogFormatter>() {
        let class = match log.style() {
            LogLabelStyle::Plain => "LogFormatter",
            LogLabelStyle::Exponent => "LogFormatterExponent",
            LogLabelStyle::Mathtext => "LogFormatterMathtext",
            LogLabelStyle::SciNotation => "LogFormatterSciNotation",
        };
        let (subset, all) = log.minor_thresholds();
        vec![
            ("class", Json::from(class)),
            ("base", Json::from(log.base())),
            ("labelOnlyBase", Json::from(log.label_only_base())),
            ("minor_thresholds", Json::from(vec![subset, all])),
            ("linthresh", Json::from(log.linthresh())),
        ]
    } else if let Some(logit) = any.downcast_ref::<LogitFormatter>() {
        vec![
            ("class", Json::from("LogitFormatter")),
            ("use_overline", Json::from(logit.use_overline())),
            ("one_half", Json::from(logit.one_half())),
            ("minor", Json::from(logit.minor())),
            ("minor_threshold", count(logit.minor_threshold())),
            ("minor_number", count(logit.minor_number())),
        ]
    } else if let Some(method) = any.downcast_ref::<StrMethodFormatter>() {
        vec![
            ("class", Json::from("StrMethodFormatter")),
            ("fmt", Json::from(method.fmt())),
        ]
    } else if any.is::<NullFormatter>() {
        vec![("class", Json::from("NullFormatter"))]
    } else {
        vec![("class", Json::from("Formatter"))]
    };
    object(fields)
}
