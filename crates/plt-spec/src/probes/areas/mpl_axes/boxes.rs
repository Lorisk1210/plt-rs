//! Box plots and violin plots: argument checks, styling and the ticks
//! they manage.

use plt::artist::ArtistHandle;
use plt::axes::{
    Axes, BoxArtist, BoxplotOptions, BxpOptions, ViolinDrawOptions, ViolinplotOptions,
};
use plt::collections::Collection;
use plt::lines::Line2D;
use plt::stats::{BoxplotOptions as StatsOptions, ViolinOptions, ViolinStats, boxplot_stats};

use super::{colors, count, datasets, made, number_list, numbers, on_axes, optional, rgba_list};
use crate::json::Json;
use crate::probes::areas::canon::Canon;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// `[xdata, ydata, marker, zorder]` of a line.
fn line_desc(line: &Line2D) -> Json {
    let (x, y) = line.data();
    Json::Array(vec![
        numbers(&x),
        numbers(&y),
        line.marker().canon(),
        Json::from(line.zorder()),
    ])
}

/// The descriptions of `lines`.
fn lines_desc<'a>(lines: impl IntoIterator<Item = &'a Line2D>) -> Json {
    Json::Array(lines.into_iter().map(line_desc).collect())
}

/// Whether argument `key` lists whole numbers written as integers.
fn integers(args: Args<'_>, key: &str) -> bool {
    args.get(key)
        .and_then(Json::as_array)
        .is_some_and(|items| items.iter().all(|item| matches!(item, Json::Integer(_))))
}

/// The drawing options of `boxplot`/`bxp`: positions, widths, cap widths
/// and the switches the probe sets.
fn draw_options(args: Args<'_>) -> Result<BxpOptions, ProbeError> {
    Ok(BxpOptions {
        positions: optional(args, "positions", number_list)?,
        integer_positions: integers(args, "positions"),
        widths: optional(args, "widths", number_list)?.unwrap_or_default(),
        capwidths: optional(args, "capwidths", number_list)?.unwrap_or_default(),
        showmeans: args.opt_bool("showmeans")?,
        manage_ticks: args.opt_bool("manage_ticks")?.unwrap_or(true),
        zorder: args.opt_f64("zorder")?,
        ..BxpOptions::default()
    })
}

/// A notch interval `(low, high)`; unset ends keep the computed ones.
type Interval = (Option<f64>, Option<f64>);

/// `conf_intervals` as `(low, high)` pairs.
fn intervals(value: &Json) -> Result<Vec<Interval>, ProbeError> {
    datasets(value)?
        .into_iter()
        .map(|pair| match pair[..] {
            [low, high] => Ok((Some(low), Some(high))),
            _ => Err(ProbeError::BadArgs(format!("bad interval {pair:?}"))),
        })
        .collect()
}

pub(super) fn boxplot(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        if let Some(lim) = optional(args, "xlim", number_list)? {
            let [left, right] = lim[..] else {
                return Err(ProbeError::BadArgs("xlim is not [left, right]".into()));
            };
            ax.set_xlim(left, right);
        }
        if let Some(before) = optional(args, "before", datasets)? {
            let draw = BxpOptions {
                positions: optional(args, "before_positions", number_list)?,
                integer_positions: integers(args, "before_positions"),
                ..BxpOptions::default()
            };
            let options = BoxplotOptions {
                draw,
                ..BoxplotOptions::default()
            };
            made(ax.boxplot(&before, options))?;
        }
        let options = BoxplotOptions {
            usermedians: optional(args, "usermedians", number_list)?
                .map(|medians| medians.into_iter().map(Some).collect()),
            conf_intervals: optional(args, "conf_intervals", intervals)?,
            tick_labels: optional(args, "tick_labels", super::strings)?,
            draw: draw_options(args)?,
            ..BoxplotOptions::default()
        };
        let parts = made(ax.boxplot(&datasets(args.json("data")?)?, options))?;
        let boxes = parts.boxes.iter().filter_map(|part| match part {
            BoxArtist::Line(line) => Some(line),
            BoxArtist::Patch(_) => None,
        });
        let axis = ax.xaxis();
        Ok(encode::object([
            ("boxes", lines_desc(boxes)),
            ("whiskers", lines_desc(&parts.whiskers)),
            ("caps", lines_desc(&parts.caps)),
            ("medians", lines_desc(&parts.medians)),
            ("fliers", lines_desc(&parts.fliers)),
            ("means", lines_desc(&parts.means)),
            ("xlim", super::pair(ax.xlim())),
            ("xticks", numbers(&made(axis.ticklocs(false))?)),
            (
                "xticklabels",
                Json::Array(
                    made(axis.ticklabels(false))?
                        .into_iter()
                        .map(Json::from)
                        .collect(),
                ),
            ),
        ]))
    })
}

pub(super) fn bxp(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        made(ax.set_yscale("log"))?;
        let stats = made(boxplot_stats(
            &datasets(args.json("data")?)?,
            StatsOptions::default(),
        ))?;
        let parts = made(ax.bxp(&stats, draw_options(args)?))?;
        Ok(count(parts.boxes.len()))
    })
}

/// The edge colours of a collection.
fn edge_colors(collection: &Collection) -> Result<Json, ProbeError> {
    Ok(rgba_list(&made(collection.colors())?.edges))
}

/// The face colours of a collection.
fn face_colors(collection: &Collection) -> Result<Json, ProbeError> {
    Ok(rgba_list(&made(collection.colors())?.faces))
}

/// How `violinplot` draws: positions, widths, colours and switches.
fn violin_draw(args: Args<'_>) -> Result<ViolinDrawOptions, ProbeError> {
    let defaults = ViolinDrawOptions::default();
    Ok(ViolinDrawOptions {
        positions: optional(args, "positions", number_list)?,
        widths: optional(args, "widths", number_list)?.unwrap_or(defaults.widths),
        showmeans: args.opt_bool("showmeans")?.unwrap_or(defaults.showmeans),
        showextrema: args
            .opt_bool("showextrema")?
            .unwrap_or(defaults.showextrema),
        showmedians: args
            .opt_bool("showmedians")?
            .unwrap_or(defaults.showmedians),
        facecolors: optional(args, "facecolor", colors)?.unwrap_or_default(),
        linecolors: optional(args, "linecolor", colors)?.unwrap_or_default(),
        ..defaults
    })
}

/// `violinplot(data, ...)`.
fn violinplot_from(ax: &Axes, args: Args<'_>) -> Result<plt::axes::ViolinParts, ProbeError> {
    let options = ViolinplotOptions {
        stats: ViolinOptions {
            quantiles: optional(args, "quantiles", datasets)?,
            ..ViolinOptions::default()
        },
        draw: violin_draw(args)?,
    };
    made(ax.violinplot(&datasets(args.json("data")?)?, options))
}

pub(super) fn violinplot(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let parts = violinplot_from(ax, args)?;
        let bodies = parts
            .bodies
            .iter()
            .map(face_colors)
            .collect::<Result<Vec<_>, _>>()?;
        let alphas = parts.bodies.iter().map(|body| Json::from(body.alpha()));
        let lines = [
            ("cbars", &parts.cbars),
            ("cmins", &parts.cmins),
            ("cmaxes", &parts.cmaxes),
            ("cmeans", &parts.cmeans),
            ("cmedians", &parts.cmedians),
        ]
        .into_iter()
        .filter_map(|(name, part)| part.as_ref().map(|lines| (name, lines)))
        .map(|(name, lines)| Ok((name.to_owned(), edge_colors(lines)?)))
        .collect::<Result<Vec<_>, ProbeError>>()?;
        Ok(encode::object([
            ("bodies", Json::Array(bodies)),
            ("alphas", Json::Array(alphas.collect())),
            ("lines", Json::Object(lines)),
        ]))
    })
}

pub(super) fn violin_alpha(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let parts = violinplot_from(ax, args)?;
        let body = &parts.bodies[0];
        let (face, alpha) = (face_colors(body)?, body.alpha());
        body.set_facecolor("red");
        Ok(encode::object([
            ("face", face),
            ("alpha", Json::from(alpha)),
            ("after", face_colors(body)?),
        ]))
    })
}

/// One `{coords, vals, mean, median, min, max, quantiles}` object.
fn violin_stats(value: &Json) -> Result<ViolinStats, ProbeError> {
    let number = |key: &str| {
        value
            .get(key)
            .and_then(Json::as_f64)
            .ok_or_else(|| ProbeError::BadArgs(format!("stats without {key}")))
    };
    let list = |key: &str| {
        value
            .get(key)
            .and_then(Json::as_f64_vec)
            .ok_or_else(|| ProbeError::BadArgs(format!("stats without {key}")))
    };
    Ok(ViolinStats {
        coords: list("coords")?,
        vals: list("vals")?,
        mean: number("mean")?,
        median: number("median")?,
        min: number("min")?,
        max: number("max")?,
        quantiles: list("quantiles")?,
    })
}

pub(super) fn violin(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let stats = args
            .json("stats")?
            .as_array()
            .ok_or_else(|| ProbeError::BadArgs("stats is not a list".into()))?
            .iter()
            .map(violin_stats)
            .collect::<Result<Vec<_>, _>>()?;
        let options = ViolinDrawOptions {
            positions: Some(args.f64_vec("positions")?),
            widths: args.f64_vec("widths")?,
            ..ViolinDrawOptions::default()
        };
        made(ax.violin(&stats, options))?;
        Ok(encode::bbox(&ax.data_lim().bbox))
    })
}
