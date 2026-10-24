//! `cbook.boxplot_stats`, `cbook.violin_stats` and `mlab.GaussianKDE`.

use plt::numeric::Grid;
use plt::stats::{self, BandwidthMethod, BoxplotOptions, GaussianKde, ViolinOptions, Whis};

use super::{datasets, floats, grid_json, object};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};

pub(crate) fn boxplot_stats(args: Args<'_>) -> Outcome {
    let as_ndarray = args.opt_bool("X_ndarray")?.unwrap_or(false);
    let (sets, _) = datasets(args.json("X")?, as_ndarray)?;
    let whis = match args.get("whis") {
        None => Whis::default(),
        Some(Json::String(_)) => {
            return Err(ProbeError::rejected(
                "whis must be a float or list of percentiles",
            ));
        }
        Some(w @ Json::Array(_)) => match floats(w)?.as_slice() {
            [lo, hi] => Whis::Percentiles(*lo, *hi),
            _ => return Err(ProbeError::rejected("whis needs two percentiles")),
        },
        Some(w) => Whis::Factor(super::number(w)?),
    };
    let labels = args
        .get("labels")
        .map(|labels| {
            labels
                .as_array()
                .and_then(|items| {
                    items
                        .iter()
                        .map(|l| l.as_str().map(str::to_owned))
                        .collect()
                })
                .ok_or_else(|| ProbeError::BadArgs(format!("bad labels {labels}")))
        })
        .transpose()?;
    let opts = BoxplotOptions {
        whis,
        labels,
        autorange: args.opt_bool("autorange")?.unwrap_or(false),
    };
    let all = stats::boxplot_stats(&sets, opts).map_err(ProbeError::rejected)?;
    Ok(Json::Array(
        all.into_iter()
            .map(|s| {
                let mut entries = vec![
                    ("mean".to_owned(), Json::from(s.mean)),
                    ("iqr".to_owned(), Json::from(s.iqr)),
                    ("cilo".to_owned(), Json::from(s.cilo)),
                    ("cihi".to_owned(), Json::from(s.cihi)),
                    ("whishi".to_owned(), Json::from(s.whishi)),
                    ("whislo".to_owned(), Json::from(s.whislo)),
                    ("fliers".to_owned(), Json::from(s.fliers)),
                    ("q1".to_owned(), Json::from(s.q1)),
                    ("med".to_owned(), Json::from(s.med)),
                    ("q3".to_owned(), Json::from(s.q3)),
                ];
                if let Some(label) = s.label {
                    entries.push(("label".to_owned(), Json::from(label)));
                }
                Json::Object(entries)
            })
            .collect(),
    ))
}

/// `bw_method`: absent or null for Scott, a rule name, or a number.
fn parse_bandwidth(value: Option<&Json>) -> Result<BandwidthMethod, ProbeError> {
    match value {
        None => Ok(BandwidthMethod::Scott),
        Some(Json::String(name)) => match name.as_str() {
            "scott" => Ok(BandwidthMethod::Scott),
            "silverman" => Ok(BandwidthMethod::Silverman),
            _ => Err(ProbeError::rejected(format!("unknown bw_method {name:?}"))),
        },
        Some(v) => super::number(v).map(BandwidthMethod::Factor),
    }
}

pub(super) fn violin_stats(args: Args<'_>) -> Outcome {
    if let Some(name) = args.opt_str("method_name")?
        && name != "GaussianKDE"
    {
        return Err(ProbeError::rejected(format!("unknown KDE method {name:?}")));
    }
    let as_ndarray = args.opt_bool("X_ndarray")?.unwrap_or(false);
    let (sets, _) = datasets(args.json("X")?, as_ndarray)?;
    let quantiles = match args.get("quantiles") {
        Some(q) if q.as_array().is_some_and(|items| !items.is_empty()) => {
            Some(datasets(q, false)?.0)
        }
        _ => None,
    };
    let points = args.opt_i64("points")?.unwrap_or(100);
    let opts = ViolinOptions {
        bw_method: parse_bandwidth(args.get("bw_method"))?,
        points: usize::try_from(points).map_err(ProbeError::rejected)?,
        quantiles,
    };
    let all = stats::violin_stats(&sets, opts).map_err(ProbeError::rejected)?;
    Ok(Json::Array(
        all.into_iter()
            .map(|s| {
                object([
                    ("vals", Json::from(s.vals)),
                    ("coords", Json::from(s.coords)),
                    ("mean", Json::from(s.mean)),
                    ("median", Json::from(s.median)),
                    ("min", Json::from(s.min)),
                    ("max", Json::from(s.max)),
                    ("quantiles", Json::from(s.quantiles)),
                ])
            })
            .collect(),
    ))
}

/// `np.atleast_2d` of a scalar, flat list or nested list.
fn atleast_2d(value: &Json) -> Result<Grid<f64>, ProbeError> {
    match value {
        Json::Array(items) if items.iter().any(|v| v.as_array().is_some()) => {
            let rows = items.iter().map(floats).collect::<Result<Vec<_>, _>>()?;
            Grid::from_rows(rows).map_err(ProbeError::rejected)
        }
        Json::Array(_) => {
            let row = floats(value)?;
            Ok(Grid::from_fn(1, row.len(), |_, c| row[c]))
        }
        v => Ok(Grid::filled(1, 1, super::number(v)?)),
    }
}

pub(crate) fn gaussian_kde(args: Args<'_>) -> Outcome {
    let dataset = atleast_2d(args.json("dataset")?)?;
    let kde = GaussianKde::from_grid(dataset, parse_bandwidth(args.get("bw_method"))?)
        .map_err(ProbeError::rejected)?;
    let points = atleast_2d(args.json("points")?)?;
    let values = kde.evaluate_grid(&points).map_err(ProbeError::rejected)?;
    Ok(object([
        ("dim", Json::from(kde.dim() as f64)),
        ("num_dp", Json::from(kde.num_points() as f64)),
        ("factor", Json::from(kde.factor())),
        ("covariance", grid_json(kde.covariance())),
        ("inv_cov", grid_json(kde.inv_cov())),
        ("norm_factor", Json::from(kde.norm_factor())),
        ("values", Json::from(values)),
    ]))
}
