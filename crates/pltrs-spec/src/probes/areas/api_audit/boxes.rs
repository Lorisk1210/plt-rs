//! Probe operations of the `api_audit` area for boxes the audit found
//! missing: a box with locked edges and bootstrapped notches.

use pltrs::axes::{BoxArtist, BoxplotOptions, BxpOptions};
use pltrs::figure::Figure;
use pltrs::stats::{self, Bootstrap, BoxplotStatsOptions};
use pltrs::transforms::{Bbox, LockableBbox};

use crate::json::Json;
use crate::probes::encode::count;
use crate::probes::{Args, Outcome, ProbeError};

/// The corners of `bbox`, `[[x0, y0], [x1, y1]]`.
fn corners(bbox: &Bbox) -> Json {
    let corner = |x: f64, y: f64| Json::Array(vec![Json::from(x), Json::from(y)]);
    Json::Array(vec![
        corner(bbox.x0(), bbox.y0()),
        corner(bbox.x1(), bbox.y1()),
    ])
}

/// A lock: its value, or null.
fn lock_value(lock: Option<f64>) -> Json {
    lock.map_or(Json::Null, Json::from)
}

/// The lock of edge `edge` (`"x0"`, ...) of `locked`.
fn lock_of<'a>(
    locked: &'a mut LockableBbox,
    edge: &str,
) -> Result<&'a mut Option<f64>, ProbeError> {
    match edge {
        "x0" => Ok(&mut locked.locked_x0),
        "y0" => Ok(&mut locked.locked_y0),
        "x1" => Ok(&mut locked.locked_x1),
        "y1" => Ok(&mut locked.locked_y1),
        other => Err(ProbeError::BadArgs(format!("unknown edge {other}"))),
    }
}

pub(super) fn lockable_bbox(args: Args<'_>) -> Outcome {
    let edge = args.str("lock")?;
    let mut locked = LockableBbox::new(Bbox::UNIT);
    *lock_of(&mut locked, edge)? = Some(2.0);
    let mut steps = vec![
        corners(&locked.bbox()),
        lock_value(*lock_of(&mut locked, edge)?),
    ];
    locked.child = Bbox::from_extents(10.0, 10.0, 11.0, 11.0);
    steps.push(corners(&locked.bbox()));
    *lock_of(&mut locked, edge)? = None;
    steps.push(corners(&locked.bbox()));
    steps.push(lock_value(*lock_of(&mut locked, edge)?));
    *lock_of(&mut locked, edge)? = Some(3.0);
    steps.push(corners(&locked.bbox()));
    steps.push(Json::Array(
        [
            locked.locked_x0,
            locked.locked_y0,
            locked.locked_x1,
            locked.locked_y1,
        ]
        .map(lock_value)
        .to_vec(),
    ));
    Ok(Json::Array(steps))
}

pub(super) fn boxplot_bootstrap(args: Args<'_>) -> Outcome {
    let data = args.f64_vec("data")?;
    let bootstrap = Bootstrap {
        resamples: args.usize("bootstrap")?,
        seed: 0,
    };
    let bootstrapped = BoxplotStatsOptions {
        bootstrap: Some(bootstrap),
        ..BoxplotStatsOptions::default()
    };
    let stats = &stats::boxplot_stats(&[&data], bootstrapped).map_err(ProbeError::rejected)?[0];
    let plain = &stats::boxplot_stats(&[&data], BoxplotStatsOptions::default())
        .map_err(ProbeError::rejected)?[0];
    let (low, high) = data
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), &v| {
            (low.min(v), high.max(v))
        });
    let within = |a: f64, b: f64| low <= a && a <= b && b <= high;
    let options = BoxplotOptions {
        bootstrap: Some(bootstrap),
        draw: BxpOptions {
            shownotches: Some(true),
            ..BxpOptions::default()
        },
        ..BoxplotOptions::default()
    };
    let parts = Figure::new()
        .subplot()
        .boxplot(&[&data], options)
        .map_err(ProbeError::rejected)?;
    let Some(BoxArtist::Line(outline)) = parts.boxes.first() else {
        return Err(ProbeError::Rejected("no box outline".into()));
    };
    let outline = outline.ydata();
    let same = [
        (stats.q1, plain.q1),
        (stats.med, plain.med),
        (stats.q3, plain.q3),
        (stats.whislo, plain.whislo),
        (stats.whishi, plain.whishi),
    ]
    .map(|(a, b)| Json::Bool(a.to_bits() == b.to_bits()));
    Ok(Json::Array(vec![
        Json::Bool(within(stats.cilo, stats.cihi)),
        Json::Array(same.to_vec()),
        Json::Bool(within(outline[2], outline[4])),
        count(outline.len()),
    ]))
}
