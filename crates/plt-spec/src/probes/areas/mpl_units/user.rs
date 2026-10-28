//! The `mpl_units` probes of user unit classes: the units tests'
//! `Quantity` with its converter, `jpl_units` epochs and an array-like
//! class (the unit classes are in `crate::user_units`).

use plt::axes::Axes;
use plt::dates::{DateTime, get_epoch};
use plt::figure::{Figure, Share};
use plt::lines::LineProps;
use plt::numeric::linspace;
use plt::units::{ConversionInterface, Convertible, Unit};

use super::{axes, texts};
use crate::json::Json;
use crate::probes::areas::mpl_ticker::styled;
use crate::probes::encode::{self, numbers, pair};
use crate::probes::{Args, Outcome, ProbeError, malformed};
use crate::user_units::jpl::Epoch;
use crate::user_units::quantity::{Quantity, QuantityUnit, quantities};

/// The unit named `name`.
fn quantity_unit(name: &Json) -> Result<QuantityUnit, ProbeError> {
    name.as_str()
        .and_then(QuantityUnit::parse)
        .ok_or_else(|| malformed("unit"))
}

/// The test's name of an axis unit, or null.
fn unit_name(unit: Option<&Unit>) -> Json {
    unit.and_then(Unit::downcast_ref::<QuantityUnit>)
        .map_or(Json::Null, |unit| Json::from(unit.name()))
}

/// The probe's limits `name`: `[low, high, unit]`.
fn limits(args: Args<'_>, name: &str) -> Result<(Quantity, Quantity), ProbeError> {
    match args.json(name)?.as_array() {
        Some([low, high, unit]) => {
            let unit = quantity_unit(unit)?;
            let magnitude = |value: &Json| value.as_f64().ok_or_else(|| malformed(name));
            Ok((
                Quantity::new(magnitude(low)?, unit),
                Quantity::new(magnitude(high)?, unit),
            ))
        }
        _ => Err(malformed(name)),
    }
}

/// `test_empty_set_limits_with_units`: limits in quantities on a new axes.
pub(super) fn lim_quantity(args: Args<'_>) -> Outcome {
    styled(args, || {
        let (fig, ax) = axes();
        let (left, right) = limits(args, "x")?;
        ax.set_xlim(left, right).map_err(ProbeError::rejected)?;
        let (bottom, top) = limits(args, "y")?;
        ax.set_ylim(bottom, top).map_err(ProbeError::rejected)?;
        fig.layout();
        Ok(encode::object([
            ("xlim", pair(ax.xlim())),
            ("ylim", pair(ax.ylim())),
            (
                "units",
                Json::Array(vec![
                    unit_name(ax.xaxis().units().as_ref()),
                    unit_name(ax.yaxis().units().as_ref()),
                ]),
            ),
            ("labels", labels(&ax)),
        ]))
    })
}

/// The x and y labels of `ax`.
fn labels(ax: &Axes) -> Json {
    texts([
        ax.xaxis().label_text().as_str(),
        ax.yaxis().label_text().as_str(),
    ])
}

/// The units of the x and y axis of each of `axs`.
fn axes_units(axs: &[Axes]) -> Json {
    Json::Array(
        axs.iter()
            .map(|ax| {
                Json::Array(vec![
                    unit_name(ax.xaxis().units().as_ref()),
                    unit_name(ax.yaxis().units().as_ref()),
                ])
            })
            .collect(),
    )
}

/// The x and y labels of each of `axs`.
fn axes_labels(axs: &[Axes]) -> Json {
    Json::Array(axs.iter().map(labels).collect())
}

/// `test_shared_axis_quantity`: units set on axes sharing both axes.
pub(super) fn shared_quantity(args: Args<'_>) -> Outcome {
    styled(args, || {
        let fig = Figure::new();
        let grid = fig
            .subplots_shared(2, 1, Share::All, Share::All)
            .map_err(ProbeError::rejected)?;
        let axs = [grid[(0, 0)].clone(), grid[(1, 0)].clone()];
        let x = quantities(linspace(0.0, 1.0, 10), QuantityUnit::Hours);
        let lines = [(&axs[0], 1.0), (&axs[1], 3.0)]
            .into_iter()
            .map(|(ax, low)| {
                let y = quantities(linspace(low, low + 1.0, 10), QuantityUnit::Feet);
                ax.plot(x.clone(), y).map_err(ProbeError::rejected)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let plotted = encode::object([("units", axes_units(&axs)), ("labels", axes_labels(&axs))]);
        axs[0].xaxis().set_units(QuantityUnit::Seconds);
        axs[1].yaxis().set_units(QuantityUnit::Inches);
        fig.layout();
        let each =
            |f: fn(&Axes) -> (f64, f64)| Json::Array(axs.iter().map(|ax| pair(f(ax))).collect());
        Ok(encode::object([
            ("plotted", plotted),
            ("units", axes_units(&axs)),
            ("labels", axes_labels(&axs)),
            (
                "xy",
                Json::Array(
                    lines
                        .iter()
                        .map(|line| encode::points(&line.xydata()))
                        .collect(),
                ),
            ),
            ("xlim", each(Axes::xlim)),
            ("ylim", each(Axes::ylim)),
        ]))
    })
}

/// One step of `test_empty_default_limits` on `ax` in `fig`: the limits
/// after a draw, null after anything else.
fn default_limits_step(fig: &Figure, ax: &Axes, step: &Json) -> Result<Json, ProbeError> {
    let parts = step.as_array().ok_or_else(|| malformed("step"))?;
    let number = |value: &Json| value.as_f64().ok_or_else(|| malformed("step"));
    match parts {
        [kind, name, magnitude, unit] if kind.as_str() == Some("update_units") => {
            let axis = match name.as_str() {
                Some("x") => ax.xaxis(),
                Some("y") => ax.yaxis(),
                _ => return Err(malformed("axis")),
            };
            axis.update_units(vec![Quantity::new(
                number(magnitude)?,
                quantity_unit(unit)?,
            )]);
        }
        [kind, at] if kind.as_str() == Some("axhline") => {
            ax.axhline(number(at)?, LineProps::default());
        }
        [kind, at] if kind.as_str() == Some("axvline") => {
            ax.axvline(number(at)?, LineProps::default());
        }
        [kind] if kind.as_str() == Some("plot") => {
            let x = quantities([0.0, 1.0, 2.0], QuantityUnit::Miles);
            let y = quantities([0.0, 2.0, 4.0], QuantityUnit::Feet);
            ax.plot(x, y).map_err(ProbeError::rejected)?;
        }
        [kind] if kind.as_str() == Some("draw") => {
            fig.layout();
            return Ok(Json::Array(vec![pair(ax.xlim()), pair(ax.ylim())]));
        }
        _ => return Err(malformed("step")),
    }
    Ok(Json::Null)
}

/// `test_empty_default_limits`: the default limits of the quantity
/// converter against data and lines.
pub(super) fn default_limits(args: Args<'_>) -> Outcome {
    styled(args, || {
        let (fig, ax) = axes();
        let steps = args
            .json("steps")?
            .as_array()
            .ok_or_else(|| malformed("steps"))?;
        let recorded = steps
            .iter()
            .map(|step| default_limits_step(&fig, &ax, step))
            .collect::<Result<_, _>>()?;
        Ok(Json::Array(recorded))
    })
}

/// `test_jpl_datetime_units_consistent`: a date and the `jpl_units` epoch
/// of it convert to the same number.
pub(super) fn jpl_consistent(args: Args<'_>) -> Outcome {
    styled(args, || {
        let date = match args.f64_vec("date")?.as_slice() {
            &[year, month, day] => DateTime::new(year as i32, month as u32, day as u32, 0, 0, 0, 0)
                .map_err(ProbeError::rejected)?,
            _ => return Err(malformed("date")),
        };
        let (_fig, ax) = axes();
        let converted = Epoch::converter()
            .convert(&[Epoch::from_date(date)], None, &ax.xaxis().into())
            .map_err(ProbeError::rejected)?;
        let &[epoch] = converted.as_slice() else {
            return Err(ProbeError::rejected("one number per epoch"));
        };
        Ok(encode::object([
            ("datetime", Json::from(get_epoch().date2num(date))),
            ("epoch", Json::from(epoch)),
        ]))
    })
}

/// `test_units.Kernel`: an array-like that only hands out its values
/// (`__array__`).
struct Kernel(Vec<f64>);

impl IntoIterator for Kernel {
    type Item = f64;
    type IntoIter = std::vec::IntoIter<f64>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// `test_plot_kernel`: an array-like plotted against its index.
pub(super) fn array_like(args: Args<'_>) -> Outcome {
    styled(args, || {
        let (_fig, ax) = axes();
        let line = ax.plot_y(Kernel(args.f64_vec("values")?));
        Ok(encode::object([
            ("xdata", numbers(line.xdata())),
            ("ydata", numbers(line.ydata())),
        ]))
    })
}
