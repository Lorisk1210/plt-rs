//! The `collection.legend_elements` op: the legend entries summarising
//! the colours or sizes of a scatter plot.

use plt::axes::{MappedColors, ScatterOptions};
use plt::collections::{ElementCount, LegendElementsOptions, LegendProperty};
use plt::colors::{Color, Colorize, colormaps};
use plt::figure::Figure;
use plt::markers::{FillStyle, Marker, MarkerStyle};
use plt::ticker::StrMethodFormatter;

use super::made;
use crate::json::Json;
use crate::probes::encode::object;
use crate::probes::{Args, Outcome, ProbeError};
use crate::rng::SpecRng;

fn bad(what: impl std::fmt::Display) -> ProbeError {
    ProbeError::BadArgs(what.to_string())
}

/// `num`: a target count, a list of values or `"auto"`.
fn count(value: &Json) -> Result<ElementCount, ProbeError> {
    match value {
        Json::String(name) if name == "auto" => Ok(ElementCount::Auto),
        Json::Integer(n) => usize::try_from(*n)
            .map(ElementCount::Target)
            .map_err(|_| bad(format!("bad num {n}"))),
        other => other
            .as_f64_vec()
            .map(ElementCount::Values)
            .ok_or_else(|| bad(format!("bad num {other}"))),
    }
}

/// The functions `func` names.
fn func(name: &str) -> Result<Box<dyn Fn(f64) -> f64>, ProbeError> {
    match name {
        "double" => Ok(Box::new(|x| 2.0 * x)),
        "inverse5" => Ok(Box::new(|x| 5.0 / x)),
        other => Err(bad(format!("unknown func {other:?}"))),
    }
}

fn options(value: &Json) -> Result<LegendElementsOptions, ProbeError> {
    let mut options = LegendElementsOptions::default();
    for (key, item) in value.as_object().unwrap_or(&[]) {
        let text = || {
            item.as_str()
                .ok_or_else(|| bad(format!("bad {key} {item}")))
        };
        match key.as_str() {
            "prop" => {
                options.prop = match text()? {
                    "colors" => LegendProperty::Colors,
                    "sizes" => LegendProperty::Sizes,
                    other => return Err(bad(format!("unknown prop {other:?}"))),
                };
            }
            "num" => options.num = count(item)?,
            "fmt" => options.fmt = Some(Box::new(made(StrMethodFormatter::new(text()?))?)),
            "func" => options.func = Some(func(text()?)?),
            "alpha" => options.alpha = item.as_f64(),
            "color" => options.color = Some(Color::from(text()?)),
            other => return Err(bad(format!("unknown option {other}"))),
        }
    }
    Ok(options)
}

pub(super) fn legend_elements(args: Args<'_>) -> Outcome {
    let mut generator = SpecRng::new(1);
    let (x, y) = (
        generator.uniform(10, 0.0, 1.0),
        generator.uniform(10, 0.0, 1.0),
    );
    let fig = Figure::new();
    let ax = fig.subplot();
    let scatter_options = ScatterOptions {
        sizes: Some(args.f64_vec("sizes")?),
        colors: MappedColors::new(args.f64_vec("values")?)
            .cmap(made(colormaps::get("jet"))?)
            .into(),
        marker: Some(made(MarkerStyle::new(
            made("o".parse::<Marker>())?,
            FillStyle::Full,
        ))?),
        linewidths: Some(vec![0.0]),
        ..ScatterOptions::default()
    };
    let scatter = made(ax.scatter_with(x, y, scatter_options))?;
    let (handles, labels) = made(scatter.legend_elements(options(args.json("options")?)?))?;
    Ok(object([
        ("labels", Json::from(labels)),
        (
            "handles",
            Json::Integer(i64::try_from(handles.len()).expect("few handles")),
        ),
    ]))
}
