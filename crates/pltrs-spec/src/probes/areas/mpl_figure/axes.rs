//! Axes of a figure: the current axes, adding axes of a projection,
//! shared log grids, removing shared axes, mosaics, reused grids and
//! picking.

use std::any::Any;

use pltrs::artist::{ArtistHandle, Picker};
use pltrs::axes::{Axes, ProjectionKind, ScatterOptions};
use pltrs::dates::{AutoDateFormatter, DateTime};
use pltrs::events::{Location, MouseEvent};
use pltrs::figure::{Figure, Mosaic, MosaicOptions, Share};
use pltrs::projections::ProjectionSpec;
use pltrs::ticker::ScalarFormatter;

use super::super::mosaic::nested;
use super::super::mpl_tests::in_test_style;
use crate::json::Json;
use crate::probes::encode::object;
use crate::probes::{Args, Outcome, ProbeError};

/// The axes `gca` names along `test_gca` (re-adding an axes made current
/// with `sca`).
pub(super) fn gca(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let ax0 = fig
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .map_err(ProbeError::rejected)?;
        let number = |ax: &Axes| Json::from(if *ax == ax0 { 0.0 } else { 1.0 });
        let after_add_axes = number(&fig.gca());
        let ax1 = fig.subplot();
        let after_add_subplot = number(&fig.gca());
        fig.sca(&ax0);
        let after_readd = number(&fig.gca());
        fig.sca(&ax0);
        let order = Json::Array(fig.axes().iter().map(number).collect());
        fig.sca(&ax1);
        Ok(object([
            ("after_add_axes", after_add_axes),
            ("after_add_subplot", after_add_subplot),
            ("after_readd", after_readd),
            ("order", order),
            ("last", number(&fig.gca())),
        ]))
    })
}

/// Two `add_subplot(1, 1, 1)` (or `add_axes((0, 0, 1, 1))`) with the given
/// projections.
pub(super) fn add_subplot(args: Args<'_>) -> Outcome {
    added(args, |fig, polar| {
        if polar {
            fig.add_polar_subplot(1, 1, 1).map(|ax| Axes::clone(&ax))
        } else {
            fig.add_subplot(1, 1, 1)
        }
        .map_err(ProbeError::rejected)
    })
}

/// [`add_subplot`] for `add_axes`.
pub(super) fn add_axes(args: Args<'_>) -> Outcome {
    added(args, |fig, polar| {
        let rect = [0.0, 0.0, 1.0, 1.0];
        let added = if polar {
            fig.add_polar_axes(rect).map(|ax| ax.axes().clone())
        } else {
            fig.add_axes(rect)
        };
        added.map_err(ProbeError::rejected)
    })
}

fn added(args: Args<'_>, add: impl Fn(&Figure, bool) -> Result<Axes, ProbeError>) -> Outcome {
    in_test_style(args, || {
        let projections = args
            .json("projections")?
            .as_array()
            .ok_or_else(|| ProbeError::BadArgs("projections is not a list".into()))?;
        let fig = Figure::new();
        let axes = projections
            .iter()
            .map(|projection| add(&fig, projection.as_str() == Some("polar")))
            .collect::<Result<Vec<_>, _>>()?;
        let names: Vec<&str> = fig.axes().iter().map(|ax| ax.projection().name()).collect();
        Ok(object([
            ("distinct", Json::from(axes.first() != axes.get(1))),
            ("names", Json::from(names)),
        ]))
    })
}

/// Whether each axes of a 2 x 2 grid sharing both axes shows x and y tick
/// labels once its last axes is made log-log.
pub(super) fn shared_log_labels(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let axs = fig
            .subplots_shared(2, 2, Share::All, Share::All)
            .map_err(ProbeError::rejected)?;
        for ax in axs.as_slice() {
            ax.plot(vec![10.0, 20.0, 30.0], vec![10.0, 20.0, 30.0])
                .map_err(ProbeError::rejected)?;
        }
        let last = &axs.as_slice()[3];
        last.set_yscale("log").map_err(ProbeError::rejected)?;
        last.set_xscale("log").map_err(ProbeError::rejected)?;
        let labelled = |axis: pltrs::axes::Axis| -> Result<bool, ProbeError> {
            let any = |minor| axis.ticklabels(minor).map(|labels| !labels.is_empty());
            Ok(any(false).map_err(ProbeError::rejected)?
                || any(true).map_err(ProbeError::rejected)?)
        };
        let rows = axs
            .as_slice()
            .iter()
            .map(|ax| {
                Ok(Json::from(vec![
                    labelled(ax.xaxis())?,
                    labelled(ax.yaxis())?,
                ]))
            })
            .collect::<Result<Vec<_>, ProbeError>>()?;
        Ok(Json::Array(rows))
    })
}

/// The axes left once the first of two axes sharing x is removed, after a
/// draw.
pub(super) fn remove_shared(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let axs = fig
            .subplots_shared(2, 1, Share::All, Share::None)
            .map_err(ProbeError::rejected)?;
        axs.as_slice()[0].remove();
        fig.canvas().draw().map_err(ProbeError::rejected)?;
        Ok(Json::from(fig.axes().len() as f64))
    })
}

/// The class of the x major formatter of the first of two axes sharing x
/// once the second is removed (after giving it a `ScalarFormatter` with
/// `set_scalar`) and dates are plotted on the first.
pub(super) fn formatter_after_removal(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let axs = fig
            .subplots_shared(1, 2, Share::All, Share::None)
            .map_err(ProbeError::rejected)?;
        let [first, second] = axs.as_slice() else {
            return Err(ProbeError::rejected("not two axes"));
        };
        if args.bool("set_scalar")? {
            second
                .xaxis()
                .set_major_formatter(ScalarFormatter::default());
        }
        second.remove();
        let day = |month| DateTime::new(2000, month, 1, 0, 0, 0, 0).map_err(ProbeError::rejected);
        first
            .plot(vec![day(1)?, day(2)?], vec![0.0, 1.0])
            .map_err(ProbeError::rejected)?;
        let formatter = first.xaxis().major_formatter();
        let any = formatter.as_ref() as &dyn Any;
        Ok(Json::from(if any.is::<AutoDateFormatter>() {
            "AutoDateFormatter"
        } else if any.is::<ScalarFormatter>() {
            "ScalarFormatter"
        } else {
            "other"
        }))
    })
}

/// The per-label projections of `per_subplot_kw`: a dict of label to
/// keywords, or `[key, keywords]` pairs whose key is a label or a list of
/// labels (a tuple key, one entry per label); keywords without a
/// `projection` keep the default one.
fn per_subplot(json: &Json) -> Result<Vec<(String, ProjectionSpec)>, ProbeError> {
    let bad = || ProbeError::BadArgs(format!("bad per_subplot_kw {json}"));
    let entries: Vec<(Json, &Json)> = match json {
        Json::Object(entries) => entries
            .iter()
            .map(|(label, kw)| (Json::String(label.clone()), kw))
            .collect(),
        Json::Array(pairs) => pairs
            .iter()
            .map(|pair| match pair.as_array() {
                Some([key, kw]) => Ok((key.clone(), kw)),
                _ => Err(bad()),
            })
            .collect::<Result<_, _>>()?,
        _ => return Err(bad()),
    };
    let mut out = Vec::new();
    for (key, kw) in entries {
        let projection = match kw.get("projection").and_then(Json::as_str) {
            None => ProjectionSpec::default(),
            Some(name) => ProjectionKind::from_name(name).ok_or_else(bad)?.into(),
        };
        let labels = match key {
            Json::String(label) => vec![label.clone()],
            Json::Array(labels) => labels
                .iter()
                .map(|label| label.as_str().map(str::to_owned))
                .collect::<Option<_>>()
                .ok_or_else(bad)?,
            _ => return Err(bad()),
        };
        out.extend(labels.into_iter().map(|label| (label, projection.clone())));
    }
    Ok(out)
}

/// A mosaic's labels, the order of the figure's axes, their grids' ratios
/// and, with `set_scales`, every axes' scales after setting them on `"A"`.
pub(super) fn subplot_mosaic(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let mosaic = match args.json("mosaic")? {
            Json::String(layout) => layout.parse::<Mosaic>().map_err(ProbeError::rejected)?,
            Json::Array(rows) => nested(rows, Some("."))?,
            other => return Err(ProbeError::BadArgs(format!("bad mosaic {other}"))),
        };
        let ratios = |key: &str| args.get(key).map(|_| args.f64_vec(key)).transpose();
        let options = MosaicOptions {
            width_ratios: ratios("width_ratios")?,
            height_ratios: ratios("height_ratios")?,
            sharex: args.opt_bool("sharex")?.unwrap_or(false),
            sharey: args.opt_bool("sharey")?.unwrap_or(false),
            per_subplot: args
                .get("per_subplot_kw")
                .map_or_else(|| Ok(Vec::new()), per_subplot)?,
            ..MosaicOptions::default()
        };
        let fig = Figure::new();
        let axd = fig
            .subplot_mosaic(&mosaic, options)
            .map_err(ProbeError::rejected)?;
        let label_of = |ax: &Axes| {
            axd.iter()
                .find(|(_, a)| *a == ax)
                .map_or_else(String::new, |(label, _)| label.to_owned())
        };
        let ratios_of = |pick: fn(&pltrs::gridspec::GridSpec) -> Vec<f64>| {
            Json::Object(
                axd.iter()
                    .map(|(label, ax)| {
                        let ratios = ax.subplot_spec().map(|spec| pick(spec.gridspec()));
                        (label.to_owned(), Json::from(ratios.unwrap_or_default()))
                    })
                    .collect(),
            )
        };
        let mut out = vec![
            (
                "keys".to_owned(),
                Json::from(axd.iter().map(|(l, _)| l).collect::<Vec<_>>()),
            ),
            (
                "axes".to_owned(),
                Json::from(fig.axes().iter().map(label_of).collect::<Vec<_>>()),
            ),
            (
                "width_ratios".to_owned(),
                ratios_of(|grid| grid.width_ratios().to_vec()),
            ),
            (
                "height_ratios".to_owned(),
                ratios_of(|grid| grid.height_ratios().to_vec()),
            ),
        ];
        if let Some(scales) = args.get("set_scales") {
            let [Json::String(x), Json::String(y)] = scales.as_array().unwrap_or(&[]) else {
                return Err(ProbeError::BadArgs(format!("bad scales {scales}")));
            };
            let first = axd
                .get("A")
                .ok_or_else(|| ProbeError::BadArgs("no axes A".into()))?;
            first.set_xscale(x.as_str()).map_err(ProbeError::rejected)?;
            first.set_yscale(y.as_str()).map_err(ProbeError::rejected)?;
            let scales = axd
                .iter()
                .map(|(label, ax)| {
                    (
                        label.to_owned(),
                        Json::from(vec![ax.xscale().to_string(), ax.yscale().to_string()]),
                    )
                })
                .collect();
            out.push(("scales".to_owned(), Json::Object(scales)));
        }
        Ok(Json::Object(out))
    })
}

/// Whether `add_subplot(3, 2, (3, 5))`, `add_subplot(3, 2, 4)` and
/// `subplot2grid((3, 2), (2, 1), colspan=2)` share one grid.
pub(super) fn reused_gridspec(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = pltrs::pyplot::figure();
        let ax1 = fig
            .add_subplot_span(3, 2, 3, 5)
            .map_err(ProbeError::rejected)?;
        let ax2 = fig.add_subplot(3, 2, 4).map_err(ProbeError::rejected)?;
        let ax3 =
            pltrs::pyplot::subplot2grid((3, 2), (2, 1), 1, 2).map_err(ProbeError::rejected)?;
        let grid = |ax: &Axes| {
            ax.subplot_spec()
                .map(|spec| spec.gridspec().clone())
                .ok_or_else(|| ProbeError::rejected("not a subplot"))
        };
        let (gs1, gs2, gs3) = (grid(&ax1)?, grid(&ax2)?, grid(&ax3)?);
        Ok(Json::from(vec![gs1.is_same(&gs2), gs1.is_same(&gs3)]))
    })
}

/// Whether the figure is stale after a draw and after a pick at the centre
/// of a pickable scatter.
pub(super) fn pick(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let ax = fig.subplot();
        let options = ScatterOptions {
            sizes: Some(vec![1000.0]),
            ..ScatterOptions::default()
        };
        let points = ax
            .scatter_with(vec![0.0], vec![0.0], options)
            .map_err(ProbeError::rejected)?;
        points.set_picker(Some(Picker::Default));
        fig.canvas().draw().map_err(ProbeError::rejected)?;
        let mut out = vec![fig.is_stale()];
        let extent = ax
            .window_extent()
            .ok_or_else(|| ProbeError::rejected("the axes has no extent"))?;
        let (x, y) = (
            extent.x0() + extent.width() / 2.0,
            extent.y0() + extent.height() / 2.0,
        );
        fig.pick(&MouseEvent {
            location: Location {
                x,
                y,
                inaxes: Some(ax.clone()),
                xdata: None,
                ydata: None,
            },
            button: None,
            key: None,
            step: 0.0,
            dblclick: false,
        });
        out.push(fig.is_stale());
        Ok(Json::from(out))
    })
}
