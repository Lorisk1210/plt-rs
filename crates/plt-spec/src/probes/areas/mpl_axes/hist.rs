//! Histograms, 2-D histograms and stairs.

use plt::artist::ArtistHandle;
use plt::axes::{Hist2dOptions, HistArtists, HistPlotOptions, StairsOptions};
use plt::colors::Rgba;
use plt::figure::Figure;
use plt::patches::{Patch, PatchShape};
use plt::stats::{Bins, HistBottom, HistType, Histogram2dOptions};

use super::{color, iso_dates, made, numbers, on_axes, optional, pair, rgba_list};
use crate::json::Json;
use crate::probes::areas::mpl_ticker::styled;
use crate::probes::encode::points;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// `x` as datasets: a flat list is one dataset, a list of lists one per
/// inner list.
fn hist_datasets(value: &Json) -> Result<Vec<Vec<f64>>, ProbeError> {
    match value.as_f64_vec() {
        Some(single) => Ok(vec![single]),
        None => super::datasets(value),
    }
}

/// `histtype=...`.
fn histtype(args: Args<'_>) -> Result<HistType, ProbeError> {
    Ok(match args.opt_str("histtype")?.unwrap_or("bar") {
        "bar" => HistType::Bar,
        "barstacked" => HistType::BarStacked,
        "step" => HistType::Step,
        "stepfilled" => HistType::StepFilled,
        other => return Err(ProbeError::BadArgs(format!("bad histtype {other}"))),
    })
}

/// The histogram options of the probe's keywords.
fn hist_options(args: Args<'_>) -> Result<HistPlotOptions<'static>, ProbeError> {
    let bins = match args.get("bins") {
        None => None,
        Some(Json::Integer(n)) => Some(Bins::Count(made(usize::try_from(*n))?)),
        Some(Json::String(estimator)) => Some(Bins::Estimator(made(estimator.parse())?)),
        Some(edges) => Some(Bins::Edges(super::number_list(edges)?)),
    };
    let range = optional(
        args,
        "range",
        |range| match super::number_list(range)?[..] {
            [low, high] => Ok((low, high)),
            _ => Err(ProbeError::BadArgs(format!("bad range {range}"))),
        },
    )?;
    let bottom = match args.get("bottom") {
        None => HistBottom::default(),
        Some(b @ Json::Array(_)) => HistBottom::PerBin(super::number_list(b)?),
        Some(b) => HistBottom::Uniform(
            b.as_f64()
                .ok_or_else(|| ProbeError::BadArgs("bad bottom".into()))?,
        ),
    };
    let label = match args.get("label") {
        None | Some(Json::Null) => Vec::new(),
        Some(Json::String(label)) => vec![label.clone()],
        Some(labels) => super::strings(labels)?,
    };
    let mut options = HistPlotOptions {
        bins,
        range,
        density: args.opt_bool("density")?.unwrap_or(false),
        stacked: args.opt_bool("stacked")?.unwrap_or(false),
        histtype: histtype(args)?,
        bottom,
        color: optional(args, "color", color)?.into_iter().collect(),
        label,
        ..HistPlotOptions::default()
    };
    options.props.facecolor = optional(args, "facecolor", color)?;
    options.props.edgecolor = optional(args, "edgecolor", color)?;
    Ok(options)
}

/// `{faces, edges, zorders}` entries of patches.
fn patch_summary<S: PatchShape>(
    patches: &[Patch<S>],
    faces: &mut Vec<Rgba>,
    edges: &mut Vec<Rgba>,
    zorders: &mut Vec<Json>,
) -> Result<(), ProbeError> {
    for patch in patches {
        let colors = made(patch.colors())?;
        faces.push(colors.face);
        edges.push(colors.edge);
        zorders.push(Json::from(patch.zorder()));
    }
    Ok(())
}

pub(super) fn hist(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let datasets = hist_datasets(args.json("x")?)?;
        let plot = made(ax.hist(&datasets, hist_options(args)?))?;
        let (mut faces, mut edges, mut zorders) = (Vec::new(), Vec::new(), Vec::new());
        let mut xy = Vec::new();
        let labels: Vec<Json> = match &plot.artists {
            HistArtists::Bars(containers) => {
                for bars in containers {
                    patch_summary(&bars.patches, &mut faces, &mut edges, &mut zorders)?;
                }
                containers
                    .iter()
                    .filter_map(|bars| bars.patches.first())
                    .map(|patch| super::legend_label(Some(patch.label())))
                    .collect()
            }
            HistArtists::Steps(outlines) => {
                patch_summary(outlines, &mut faces, &mut edges, &mut zorders)?;
                xy.extend(
                    outlines
                        .iter()
                        .map(|outline| points(outline.shape().vertices())),
                );
                outlines
                    .iter()
                    .map(|outline| super::legend_label(Some(outline.label())))
                    .collect()
            }
        };
        Ok(encode::object([
            ("n", counts(&plot.tops)),
            ("xy", Json::Array(xy)),
            ("faces", rgba_list(&faces)),
            ("edges", rgba_list(&edges)),
            ("zorders", Json::Array(zorders)),
            ("labels", Json::Array(labels)),
        ]))
    })
}

/// The counts of a histogram: flat for one dataset, one list per dataset
/// otherwise.
fn counts(tops: &[Vec<f64>]) -> Json {
    match tops {
        [single] => numbers(single),
        several => Json::Array(several.iter().map(numbers).collect()),
    }
}

pub(super) fn hist_bins(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let datasets = hist_datasets(args.json("x")?)?;
        let plot = made(ax.hist(&datasets, hist_options(args)?))?;
        Ok(encode::object([
            ("n", counts(&plot.tops)),
            ("bins", numbers(&plot.edges)),
        ]))
    })
}

pub(super) fn hist_dates(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let axis = ax.xaxis();
        let datasets = args
            .json("data")?
            .as_array()
            .ok_or_else(|| ProbeError::BadArgs("data is not a list of datasets".into()))?
            .iter()
            .map(|days| Ok(axis.convert_units(iso_dates(days)?)))
            .collect::<Result<Vec<_>, ProbeError>>()?;
        let bins = optional(args, "bins", |edges| {
            Ok(Bins::Edges(axis.convert_units(iso_dates(edges)?)))
        })?;
        let options = HistPlotOptions {
            bins,
            stacked: args.bool("stacked")?,
            ..HistPlotOptions::default()
        };
        let plot = made(ax.hist(&datasets, options))?;
        Ok(encode::object([
            ("n", Json::Array(plot.tops.iter().map(numbers).collect())),
            ("bins", numbers(&plot.edges)),
        ]))
    })
}

pub(super) fn legend_labels(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        made(ax.hist(&hist_datasets(args.json("x")?)?, hist_options(args)?))?;
        let (_, labels) = ax.legend_handles_labels();
        Ok(Json::Array(labels.into_iter().map(Json::from).collect()))
    })
}

pub(super) fn log_barstacked(args: Args<'_>) -> Outcome {
    styled(args, || {
        let fig = Figure::new();
        let axs = made(fig.subplots(2, 1))?;
        let barstacked = || HistPlotOptions {
            bins: Some(Bins::Count(2)),
            histtype: HistType::BarStacked,
            ..HistPlotOptions::default()
        };
        let data = [vec![vec![0.0], vec![0.0, 1.0]], vec![vec![0.0, 0.0, 1.0]]];
        for (ax, datasets) in axs.as_slice().iter().zip(&data) {
            made(ax.hist(datasets, barstacked()))?;
            made(ax.set_yscale("log"))?;
        }
        fig.layout();
        Ok(Json::Array(
            axs.as_slice().iter().map(|ax| pair(ax.ylim())).collect(),
        ))
    })
}

pub(super) fn hist2d(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let options = Hist2dOptions {
            histogram: Histogram2dOptions {
                density: args.bool("density")?,
                ..Histogram2dOptions::default()
            },
            ..Hist2dOptions::default()
        };
        let hist = made(ax.hist2d(&args.f64_vec("x")?, &args.f64_vec("y")?, options))?;
        Ok(Json::Array(hist.counts.iter_rows().map(numbers).collect()))
    })
}

pub(super) fn stairs(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let options = StairsOptions {
            edges: Some(args.f64_vec("edges")?),
            ..StairsOptions::default()
        };
        made(ax.stairs(args.f64_vec("values")?, options))?;
        Ok(encode::object([
            ("xlim", pair(ax.xlim())),
            ("ylim", pair(ax.ylim())),
        ]))
    })
}

pub(super) fn stairs_update(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let options = StairsOptions {
            edges: Some(args.f64_vec("edges")?),
            ..StairsOptions::default()
        };
        let patch = made(ax.stairs(args.f64_vec("values")?, options))?;
        let shape = made(
            patch
                .shape()
                .with_data(None, Some(args.f64_vec("new_edges")?)),
        )?;
        patch.set_shape(shape);
        Ok(Json::Null)
    })
}
