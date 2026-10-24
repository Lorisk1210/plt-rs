//! The legend labels of plotting calls drawing several datasets: `plot`
//! of the columns of a 2-D array and `boxplot`.

use plt::artist::{ArtistHandle, is_hidden_label};
use plt::axes::{BoxArtist, BoxLabel, BoxProps, BoxplotOptions};
use plt::figure::Figure;
use plt::legend::{LegendHandle, LegendOptions};
use plt::lines::LineProps;
use plt::patches::PatchProps;

use super::super::mpl_tests::in_test_style;
use super::texts;
use crate::json::Json;
use crate::probes::encode::object;
use crate::probes::{Args, Outcome, ProbeError};
use crate::rng::SpecRng;

/// The datasets of a `y` argument: the columns of a list of rows, or a
/// flat list as one dataset.
fn columns(y: &Json) -> Result<Vec<Vec<f64>>, ProbeError> {
    let bad = || ProbeError::BadArgs(format!("bad y {y}"));
    if let Some(values) = y.as_f64_vec() {
        return Ok(vec![values]);
    }
    let rows = y
        .as_array()
        .ok_or_else(bad)?
        .iter()
        .map(|row| row.as_f64_vec().ok_or_else(bad))
        .collect::<Result<Vec<_>, _>>()?;
    let width = rows.first().map_or(0, Vec::len);
    Ok((0..width)
        .map(|j| rows.iter().map(|row| row[j]).collect())
        .collect())
}

/// A label given as a string or a number (written as Python's `str`).
fn scalar_label(value: &Json) -> Option<String> {
    match value {
        Json::String(text) => Some(text.clone()),
        Json::Integer(n) => Some(n.to_string()),
        _ => None,
    }
}

/// A list of labels.
fn label_list(value: &Json) -> Result<Vec<String>, ProbeError> {
    value
        .as_array()
        .and_then(|items| items.iter().map(scalar_label).collect())
        .ok_or_else(|| ProbeError::BadArgs(format!("bad labels {value}")))
}

/// `test_plot_*`: the labels of the lines of one `plot` call with
/// `label` and of its legend.
pub(super) fn plot(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let columns = columns(args.json("y")?)?;
        let x = match args.get("x") {
            Some(x) => x
                .as_f64_vec()
                .ok_or_else(|| ProbeError::BadArgs(format!("bad x {x}")))?,
            None => (0..columns.first().map_or(0, Vec::len))
                .map(|i| i as f64)
                .collect(),
        };
        let label = args.json("label")?;
        let (props, labels) = if let Some(label) = scalar_label(label) {
            (LineProps::default().label(label), None)
        } else {
            (LineProps::default(), Some(label_list(label)?))
        };
        let ax = Figure::new().subplot();
        let lines = ax
            .plot_columns(x, &columns, &props, labels.as_deref())
            .map_err(ProbeError::rejected)?;
        let legend = ax
            .legend(LegendOptions::default())
            .map_err(ProbeError::rejected)?;
        Ok(object([
            (
                "lines",
                Json::from(lines.iter().map(ArtistHandle::label).collect::<Vec<_>>()),
            ),
            ("texts", texts(&legend)),
        ]))
    })
}

/// A legend label, `null` for a hidden one.
fn shown(label: String) -> Json {
    if is_hidden_label(&label) {
        Json::Null
    } else {
        Json::from(label)
    }
}

/// `test_boxplot_legend_labels`: the labels of the boxes and medians of
/// a box plot with `label`, and the kinds and labels of its legend
/// entries.
pub(super) fn boxplot(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let values = SpecRng::new(0).uniform(40, 0.0, 1.0);
        let datasets: Vec<Vec<f64>> = (0..4)
            .map(|j| values.iter().skip(j).step_by(4).copied().collect())
            .collect();
        let label = args.json("label")?;
        let mut options = BoxplotOptions::default();
        options.draw.label = Some(if let Some(label) = scalar_label(label) {
            BoxLabel::First(label)
        } else {
            BoxLabel::Each(label_list(label)?)
        });
        if args.opt_bool("patch_artist")?.unwrap_or(false) {
            options.draw.boxes = Some(BoxProps::Patch(PatchProps::default()));
        }
        options.draw.showbox = args.opt_bool("showbox")?;
        let ax = Figure::new().subplot();
        let parts = ax
            .boxplot(&datasets, options)
            .map_err(ProbeError::rejected)?;
        let (handles, labels) = ax.legend_handles_labels();
        let kind = |handle: &LegendHandle| {
            let patch = parts.boxes.iter().any(|drawn| match drawn {
                BoxArtist::Patch(patch) => LegendHandle::from(patch) == *handle,
                BoxArtist::Line(_) => false,
            });
            let line = (parts.medians.iter()).any(|median| LegendHandle::from(median) == *handle);
            Json::from(match (patch, line) {
                (true, _) => "patch",
                (_, true) => "line",
                _ => "other",
            })
        };
        let box_label = |drawn: &BoxArtist| match drawn {
            BoxArtist::Line(line) => shown(line.label()),
            BoxArtist::Patch(patch) => shown(patch.label()),
        };
        Ok(object([
            (
                "boxes",
                Json::Array(parts.boxes.iter().map(box_label).collect()),
            ),
            (
                "medians",
                Json::Array(parts.medians.iter().map(|m| shown(m.label())).collect()),
            ),
            ("handles", Json::Array(handles.iter().map(kind).collect())),
            ("labels", Json::from(labels)),
        ]))
    })
}
