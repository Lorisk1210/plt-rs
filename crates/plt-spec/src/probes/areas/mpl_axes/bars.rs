//! Bars, grouped bars, and the legend of stack plots in subfigures.

use std::any::Any;

use plt::artist::{ArtistHandle, BaseProps};
use plt::axes::{
    Axes, BandAlign, BarContainer, BarOptions, ErrorSize, GroupedBarOptions, StackplotOptions,
};
use plt::container::Container;
use plt::dates::{AutoDateFormatter, TimeDelta};
use plt::figure::{Figure, SubfigureOptions};
use plt::legend::LegendOptions;
use plt::patches::PatchProps;
use plt::units::{AxisDelta, AxisValues};

use super::{
    color, count, iso_date, iso_dates, legend_label, made, number_list, numbers, on_axes, optional,
    pair, rgba_list, strings,
};
use crate::json::Json;
use crate::probes::areas::mpl_ticker::styled;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// Bar positions: a category, categories or numbers.
fn positions(value: &Json) -> Result<AxisValues, ProbeError> {
    match value {
        Json::String(name) => Ok(AxisValues::Categories(vec![name.clone()])),
        Json::Array(items)
            if items.iter().all(|item| item.as_str().is_some()) && !items.is_empty() =>
        {
            Ok(AxisValues::Categories(
                items
                    .iter()
                    .filter_map(Json::as_str)
                    .map(str::to_owned)
                    .collect(),
            ))
        }
        other => other
            .as_f64_vec()
            .map(AxisValues::Numbers)
            .ok_or_else(|| ProbeError::BadArgs(format!("bad bar positions {other}"))),
    }
}

/// `bar(x, height, ...)` options: colours, alpha and the label (one for
/// the container, or a list with one per bar).
fn bar_options(args: Args<'_>) -> Result<BarOptions, ProbeError> {
    let mut options = BarOptions {
        props: PatchProps {
            base: BaseProps {
                alpha: args.opt_f64("alpha")?,
                ..BaseProps::default()
            },
            ..PatchProps::default()
        },
        ..BarOptions::default()
    };
    if let Some(face) = args.get("color") {
        options.color = vec![color(face)?];
    }
    if let Some(edge) = args.get("edgecolor") {
        options.edgecolor = vec![color(edge)?];
    }
    match args.get("label") {
        None => {}
        Some(Json::String(label)) => options.label = Some(label.clone()),
        Some(labels) => options.bar_labels = strings(labels)?,
    }
    Ok(options)
}

/// `[x, y, width, height]` of every bar.
fn rects(bars: &BarContainer) -> Json {
    Json::Array(
        bars.patches
            .iter()
            .map(|patch| {
                let rect = patch.shape();
                numbers(&[rect.xy.x, rect.xy.y, rect.width, rect.height])
            })
            .collect(),
    )
}

pub(super) fn bar(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let heights = super::number_list(args.json("height")?)?;
        let bars = made(ax.bar(positions(args.json("x")?)?, heights, bar_options(args)?))?;
        let colors = bars
            .patches
            .iter()
            .map(|patch| made(patch.colors()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(encode::object([
            ("rects", rects(&bars)),
            (
                "faces",
                rgba_list(&colors.iter().map(|c| c.face).collect::<Vec<_>>()),
            ),
            (
                "edges",
                rgba_list(&colors.iter().map(|c| c.edge).collect::<Vec<_>>()),
            ),
            (
                "labels",
                Json::Array(
                    bars.patches
                        .iter()
                        .map(|patch| legend_label(Some(patch.label())))
                        .collect(),
                ),
            ),
            ("label", legend_label(bars.label.clone())),
            ("xlim", pair(ax.xlim())),
            ("ylim", pair(ax.ylim())),
        ]))
    })
}

/// A duration of `hours` hours.
fn duration(hours: i64) -> Result<TimeDelta, ProbeError> {
    made(TimeDelta::new(0, hours * 3600, 0))
}

/// A list of whole hours argument.
fn hour_list(args: Args<'_>, key: &str) -> Result<Vec<i64>, ProbeError> {
    args.json(key)?
        .as_array()
        .and_then(|items| items.iter().map(Json::as_i64).collect())
        .ok_or_else(|| ProbeError::BadArgs(format!("{key} is not a list of hours")))
}

pub(super) fn bar_timedelta(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        // Bar thicknesses are axis numbers: days on a date axis.
        let thickness = hour_list(args, "hours")?
            .into_iter()
            .map(|h| Ok(AxisDelta::from(duration(h)?).to_number()))
            .collect::<Result<Vec<_>, ProbeError>>()?;
        let error = args
            .opt_i64("err_hours")?
            .map(|h| duration(h).map(ErrorSize::from))
            .transpose()?;
        let vertical = args.str("orientation")? == "vertical";
        let mut options = BarOptions {
            thickness,
            ..BarOptions::default()
        };
        if vertical {
            options.errors.xerr = error;
        } else {
            options.errors.yerr = error;
        }
        let (positions, values) = (iso_dates(args.json("x")?)?, args.f64_vec("values")?);
        let bars = if vertical {
            made(ax.bar(positions, values, options))?
        } else {
            made(ax.barh(positions, values, options))?
        };
        Ok(encode::object([
            ("rects", rects(&bars)),
            ("xlim", pair(ax.xlim())),
            ("ylim", pair(ax.ylim())),
        ]))
    })
}

pub(super) fn broken_barh_dates(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let start = iso_date(args.str("start")?)?;
        let length = duration(args.i64("hours")?)?;
        let [y, height] = args.f64_vec("yrange")?[..] else {
            return Err(ProbeError::BadArgs("yrange is not [y, height]".into()));
        };
        ax.broken_barh(&[(start, length)], (y, height), BandAlign::Bottom);
        Ok(encode::bbox(&ax.data_lim().bbox))
    })
}

pub(super) fn bar_datetime_start(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let vertical = args.str("orientation")? == "vertical";
        let value_axis = if vertical { ax.yaxis() } else { ax.xaxis() };
        let days = |texts: [&str; 3]| {
            texts
                .map(iso_date)
                .into_iter()
                .collect::<Result<Vec<_>, _>>()
        };
        let starts = days(["2012-01-01", "2012-02-01", "2012-01-15"])?;
        let stops = days(["2012-02-07", "2012-02-13", "2012-02-12"])?;
        let base = value_axis.convert_units(starts);
        let lengths: Vec<f64> = value_axis
            .convert_units(stops)
            .iter()
            .zip(&base)
            .map(|(stop, start)| stop - start)
            .collect();
        let options = BarOptions {
            base,
            ..BarOptions::default()
        };
        let positions = vec![0.0, 1.0, 3.0];
        if vertical {
            made(ax.bar(positions, lengths, options))?;
        } else {
            made(ax.barh(positions, lengths, options))?;
        }
        let formatter = value_axis.major_formatter();
        Ok(Json::from(
            (formatter.as_ref() as &dyn Any).is::<AutoDateFormatter>(),
        ))
    })
}

/// How many of `containers` the axes records.
fn recorded(ax: &Axes, containers: &[BarContainer]) -> usize {
    let bars: Vec<BarContainer> = ax
        .containers()
        .into_iter()
        .filter_map(|container| match container {
            Container::Bar(bars) => Some(bars),
            _ => None,
        })
        .collect();
    containers
        .iter()
        .filter(|container| bars.iter().any(|bars| bars.patches == container.patches))
        .count()
}

pub(super) fn grouped_bar(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let options = GroupedBarOptions {
            positions: optional(args, "positions", number_list)?,
            tick_labels: optional(args, "tick_labels", strings)?.unwrap_or_default(),
            hatch: optional(args, "hatch", strings)?.unwrap_or_default(),
            ..GroupedBarOptions::default()
        };
        let containers = made(ax.grouped_bar(super::datasets(args.json("heights")?)?, options))?;
        let hatches: Vec<Json> = containers
            .iter()
            .map(|bars| {
                Json::Array(
                    bars.patches
                        .iter()
                        .map(|patch| patch.style().hatch.map_or(Json::Null, Json::from))
                        .collect(),
                )
            })
            .collect();
        let made_and_recorded = vec![count(containers.len()), count(recorded(ax, &containers))];
        for bars in &containers {
            bars.remove();
        }
        Ok(encode::object([
            ("containers", Json::Array(made_and_recorded)),
            ("hatches", Json::Array(hatches)),
            ("removed", count(recorded(ax, &containers))),
        ]))
    })
}

pub(super) fn subfigure_legend(args: Args<'_>) -> Outcome {
    styled(args, || {
        let fig = Figure::new();
        let subfigs = made(fig.subfigures(1, 2, SubfigureOptions::default()))?;
        for subfig in subfigs.as_slice() {
            let ax = subfig.subplot();
            let layers =
                made(ax.stackplot([3.0, 4.0], &[vec![1.0, 2.0]], StackplotOptions::default()))?;
            layers[0].set_label("a");
        }
        let legend = made(fig.legend(LegendOptions::default()))?;
        fig.layout();
        Ok(Json::Array(
            legend.labels().into_iter().map(Json::from).collect(),
        ))
    })
}
