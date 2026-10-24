//! Probe operations for the `mpl_category` area: matplotlib's own tests of
//! categorical axes (see `spec/reference/probes/test_mpl_category.py`).

use plt::axes::{Axes, BarOptions, HistPlotOptions};
use plt::figure::Figure;
use plt::ticker::{Formatter, Locator, TickContext};
use plt::units::{AxisValues, Categories, CategoryFormatter, CategoryLocator};

use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError, encode};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("category.UnitData", unit_data),
    Op::new("category.StrCategoryConverter.convert", convert),
    Op::new("category.StrCategoryLocator.tick_values", locator_ticks),
    Op::new("category.StrCategoryFormatter.__call__", formatter_labels),
    Op::new("axes.Axes.plot_units", plot_units),
    Op::new("axes.Axes.hist", hist),
    Op::new("axes.Axes.set_xlim", set_xlim),
];

fn rejected(err: impl std::fmt::Display) -> ProbeError {
    ProbeError::rejected(err)
}

/// A list of category names.
fn names(value: &Json) -> Result<Vec<String>, ProbeError> {
    value
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs(format!("{value} is not a list")))?
        .iter()
        .map(|name| {
            name.as_str()
                .map(str::to_owned)
                .ok_or_else(|| ProbeError::BadArgs(format!("{name} is not a name")))
        })
        .collect()
}

/// Plotting values: category names, or numbers (an empty list included).
fn axis_values(value: &Json) -> Result<AxisValues, ProbeError> {
    let items = value
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs(format!("{value} is not a list")))?;
    if !items.is_empty() && items.iter().all(|item| item.as_str().is_some()) {
        return names(value).map(AxisValues::Categories);
    }
    value
        .as_f64_vec()
        .map(AxisValues::Numbers)
        .ok_or_else(|| ProbeError::BadArgs(format!("{value} mixes names and numbers")))
}

/// The categories of `data`.
fn categories_of(value: &Json) -> Result<Categories, ProbeError> {
    let mut categories = Categories::default();
    categories.convert(&names(value)?);
    Ok(categories)
}

/// `[keys, values]` of a category mapping.
fn mapping(categories: &Categories) -> Json {
    let names = categories.names();
    Json::Array(vec![
        Json::from(names.to_vec()),
        Json::from((0..names.len()).map(|i| i as f64).collect::<Vec<_>>()),
    ])
}

fn unit_data(args: Args<'_>) -> Outcome {
    let mut categories = categories_of(args.json("data")?)?;
    if let Some(update) = args.get("update") {
        categories.convert(&names(update)?);
    }
    Ok(mapping(&categories))
}

/// A position per name of a list, or one position for a name.
fn convert(args: Args<'_>) -> Outcome {
    let value = args.json("value")?;
    let mut categories = Categories::default();
    if let Some(name) = value.as_str() {
        return Ok(Json::from(categories.convert(&[name.to_owned()])[0]));
    }
    Ok(Json::from(categories.convert(&names(value)?)))
}

fn locator_ticks(args: Args<'_>) -> Outcome {
    let locator = CategoryLocator::new(&categories_of(args.json("data")?)?);
    let ticks = locator
        .tick_values(f64::NAN, f64::NAN, &TickContext::default())
        .map_err(rejected)?;
    Ok(Json::from(ticks))
}

/// The label `formatter` gives position `value`.
fn label(formatter: &dyn Formatter, value: f64) -> Result<Json, ProbeError> {
    formatter
        .format_data(value, &TickContext::default())
        .map(Json::from)
        .map_err(rejected)
}

fn formatter_labels(args: Args<'_>) -> Outcome {
    let categories = categories_of(args.json("data")?)?;
    let formatter = CategoryFormatter::new(&categories);
    let labels = (0..categories.names().len())
        .map(|i| label(&formatter, i as f64))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json::Array(vec![
        Json::Array(labels.clone()),
        Json::Array(labels),
    ]))
}

/// What the probe records of a categorical axis, or `null`.
fn axis_summary(axis: &plt::axes::Axis) -> Outcome {
    let Some(categories) = axis.categories() else {
        return Ok(Json::Null);
    };
    let locs = axis.ticklocs(false).map_err(rejected)?;
    let formatter = axis.major_formatter();
    let labels = locs
        .iter()
        .map(|&loc| label(formatter.as_ref(), loc))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(encode::object([
        ("labels", Json::Array(labels)),
        ("after", label(formatter.as_ref(), locs.len() as f64)?),
        ("units", mapping(&categories)),
        ("locs", Json::from(locs)),
    ]))
}

/// One call of the `calls` argument.
fn call(ax: &Axes, call: &Json) -> Result<(), ProbeError> {
    let bad = || ProbeError::BadArgs(format!("bad call {call}"));
    let items = call.as_array().ok_or_else(bad)?;
    let (method, values) = items.split_first().ok_or_else(bad)?;
    match (method.as_str().ok_or_else(bad)?, values) {
        ("xaxis.update_units", [values]) => {
            ax.xaxis().convert_units(axis_values(values)?);
        }
        ("scatter", [x, y]) => {
            ax.scatter(axis_values(x)?, axis_values(y)?)
                .map_err(rejected)?;
        }
        ("plot", [x, y]) => {
            ax.plot(axis_values(x)?, axis_values(y)?)
                .map_err(rejected)?;
        }
        ("bar", [x, y]) => {
            // Heights given as names are positions of the y axis.
            let heights = ax.yaxis().convert_units(axis_values(y)?);
            ax.bar(axis_values(x)?, heights, BarOptions::default())
                .map_err(rejected)?;
        }
        _ => return Err(bad()),
    }
    Ok(())
}

fn plot_units(args: Args<'_>) -> Outcome {
    let ax = Figure::new().subplot();
    for each in args.json("calls")?.as_array().unwrap_or(&[]) {
        call(&ax, each)?;
    }
    Ok(encode::object([
        ("x", axis_summary(&ax.xaxis())?),
        ("y", axis_summary(&ax.yaxis())?),
    ]))
}

fn hist(args: Args<'_>) -> Outcome {
    let ax = Figure::new().subplot();
    let positions = ax.xaxis().convert_units(axis_values(args.json("x")?)?);
    let plot = ax
        .hist(&[positions], HistPlotOptions::default())
        .map_err(rejected)?;
    let counts = plot
        .tops
        .first()
        .ok_or_else(|| ProbeError::rejected("no histogram"))?;
    Ok(encode::object([
        ("n", Json::from(counts.clone())),
        ("bins", Json::from(plot.edges)),
    ]))
}

fn set_xlim(args: Args<'_>) -> Outcome {
    let ax = Figure::new().subplot();
    ax.plot(axis_values(args.json("x")?)?, axis_values(args.json("y")?)?)
        .map_err(rejected)?;
    ax.set_xlim(args.str("left")?, args.str("right")?)
        .map_err(rejected)?;
    let (left, right) = ax.xlim();
    Ok(Json::from(vec![left, right]))
}
