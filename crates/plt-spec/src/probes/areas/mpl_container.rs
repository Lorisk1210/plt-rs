//! Probe operations for the `mpl_container` area: matplotlib's own tests
//! of removing containers and of the geometry bar containers report (see
//! `spec/reference/probes/test_mpl_container.py`).

use plt::axes::{Axes, BarOptions, ErrorbarOptions, PieLabelOptions, PieOptions, StemOptions};
use plt::figure::Figure;

use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};
use plt::ticker::LabelFormat;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("StemContainer.remove", stem_remove),
    Op::new("ErrorbarContainer.remove", errorbar_remove),
    Op::new("BarContainer", bar_container),
    Op::new("PieContainer.remove", pie_remove),
    Op::new("Axes.pie", pie_unpack),
];

/// `[lines, collections, patches, texts, containers]`: the lengths of the
/// axes' artist lists.
fn counts(ax: &Axes) -> Json {
    let lengths = [
        ax.lines().len(),
        ax.collections().len(),
        ax.patches().len(),
        ax.texts().len(),
        ax.containers().len(),
    ];
    Json::Array(lengths.into_iter().map(count).collect())
}

/// A length as a JSON number.
fn count(n: usize) -> Json {
    Json::from(n as f64)
}

/// Argument `key` as strings.
fn strings(args: Args<'_>, key: &str) -> Result<Vec<String>, ProbeError> {
    args.json(key)?
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .ok_or_else(|| ProbeError::BadArgs(format!("{key} is not a list of strings")))
}

fn stem_remove(args: Args<'_>) -> Outcome {
    let ax = Figure::new().subplot();
    let stems = ax
        .stem(
            args.f64_vec("locs")?,
            args.f64_vec("heads")?,
            StemOptions::default(),
        )
        .map_err(ProbeError::rejected)?;
    let before = counts(&ax);
    stems.remove();
    Ok(Json::Array(vec![before, counts(&ax)]))
}

fn errorbar_remove(args: Args<'_>) -> Outcome {
    let ax = Figure::new().subplot();
    let error = |key| -> Result<_, ProbeError> {
        Ok(match args.get(key) {
            None => None,
            Some(Json::Array(_)) => Some(args.f64_vec(key)?.into()),
            Some(_) => Some(args.f64(key)?.into()),
        })
    };
    let options = ErrorbarOptions {
        xerr: error("xerr")?,
        yerr: error("yerr")?,
        fmt: args
            .opt_str("fmt")?
            .unwrap_or_default()
            .parse()
            .map_err(ProbeError::rejected)?,
        ..ErrorbarOptions::default()
    };
    let bars = ax
        .errorbar([1.0], [1.0], options)
        .map_err(ProbeError::rejected)?;
    let before = counts(&ax);
    bars.remove();
    Ok(Json::Array(vec![before, counts(&ax)]))
}

fn bar_container(args: Args<'_>) -> Outcome {
    let ax = Figure::new().subplot();
    let options = BarOptions {
        base: args.f64_vec("base")?,
        ..BarOptions::default()
    };
    let (positions, values) = (args.f64_vec("positions")?, args.f64_vec("values")?);
    let bars = match args.str("orientation")? {
        "vertical" => ax.bar(positions, values, options),
        "horizontal" => ax.barh(positions, values, options),
        other => return Err(ProbeError::BadArgs(format!("unknown orientation {other}"))),
    }
    .map_err(ProbeError::rejected)?;
    let numbers = |values: Vec<f64>| Json::Array(values.into_iter().map(Json::from).collect());
    Ok(Json::Object(vec![
        ("position_centers".into(), numbers(bars.position_centers())),
        ("bottoms".into(), numbers(bars.bottoms())),
        ("tops".into(), numbers(bars.tops())),
    ]))
}

/// `pie(values, labels=labels, autopct=autopct, labeldistance=...)`.
fn pie_options(args: Args<'_>) -> Result<PieOptions, ProbeError> {
    let defaults = PieOptions::default();
    let labeldistance = if args.contains("labeldistance") {
        args.opt_f64("labeldistance")?
    } else {
        defaults.labeldistance
    };
    Ok(PieOptions {
        labels: strings(args, "labels")?,
        autopct: Some(LabelFormat::Template(args.str("autopct")?.to_owned())),
        labeldistance,
        ..defaults
    })
}

fn pie_remove(args: Args<'_>) -> Outcome {
    let ax = Figure::new().subplot();
    let mut pie = ax
        .pie(&args.f64_vec("values")?, pie_options(args)?)
        .map_err(ProbeError::rejected)?;
    let drawn = counts(&ax);
    ax.pie_label(
        &mut pie,
        strings(args, "added")?,
        PieLabelOptions::default(),
    )
    .map_err(ProbeError::rejected)?;
    let labelled = counts(&ax);
    pie.remove();
    Ok(Json::Array(vec![drawn, labelled, counts(&ax)]))
}

fn pie_unpack(args: Args<'_>) -> Outcome {
    let ax = Figure::new().subplot();
    let pie = ax
        .pie(&args.f64_vec("values")?, pie_options(args)?)
        .map_err(ProbeError::rejected)?;
    Ok(Json::Array(vec![
        count(pie.wedges.len()),
        count(pie.labels.len()),
        count(pie.pct_labels.len()),
    ]))
}
