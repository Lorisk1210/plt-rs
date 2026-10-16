//! Structural facts of a figure, mirroring `spec_support.figcases.figure_facts`.

use plt::axes::{Axes, AxisName, TitleLocation};
use plt::figure::Figure;

use crate::compare::{Tolerance, compare};
use crate::json::Json;

/// The facts plt-rs can report about a figure, in the golden layout.
pub fn figure_facts(fig: &Figure) -> Json {
    let (width, height) = fig.size_pixels();
    Json::Object(vec![
        (
            "size_px".into(),
            Json::from(vec![width.round(), height.round()]),
        ),
        ("suptitle".into(), fig.suptitle_text().into()),
        (
            "axes".into(),
            Json::Array(fig.axes().iter().map(axes_facts).collect()),
        ),
    ])
}

fn axes_facts(ax: &Axes) -> Json {
    let (x, y, w, h) = ax.position().bounds();
    let (x0, x1) = ax.xlim();
    let (y0, y1) = ax.ylim();
    Json::Object(vec![
        ("kind".into(), "rectilinear".into()),
        ("position".into(), Json::from(vec![x, y, w, h])),
        ("title".into(), ax.title(TitleLocation::Center).into()),
        ("xlim".into(), Json::from(vec![x0, x1])),
        ("ylim".into(), Json::from(vec![y0, y1])),
        ("xscale".into(), ax.xscale().into()),
        ("yscale".into(), ax.yscale().into()),
        ("xlabel".into(), ax.label(AxisName::X).into()),
        ("ylabel".into(), ax.label(AxisName::Y).into()),
        ("xticks".into(), ticks(ax, AxisName::X)),
        ("yticks".into(), ticks(ax, AxisName::Y)),
    ])
}

/// Drawn major ticks as `[[location, label], ...]`; a tick computation
/// failure is reported as its message so the diff shows it.
fn ticks(ax: &Axes, axis: AxisName) -> Json {
    match ax.ticks(axis) {
        Ok(set) => Json::Array(
            set.major
                .into_iter()
                .map(|tick| Json::Array(vec![tick.loc.into(), tick.label.into()]))
                .collect(),
        ),
        Err(message) => Json::String(message),
    }
}

/// Key-by-key differences between golden and actual facts: one message per
/// mismatched or missing fact, empty when they agree.
pub(crate) fn diff_facts(golden: &Json, actual: &Json) -> Vec<String> {
    let tolerance = Tolerance {
        rel: 0.0,
        abs: 1.5e-6,
    };
    let mut problems = Vec::new();
    diff_object(golden, actual, "", tolerance, &mut problems);
    problems
}

fn diff_object(
    golden: &Json,
    actual: &Json,
    path: &str,
    tol: Tolerance,
    problems: &mut Vec<String>,
) {
    let Some(entries) = golden.as_object() else {
        return;
    };
    for (key, expected) in entries {
        let here = format!("{path}.{key}");
        match (expected, actual.get(key)) {
            (_, None) => problems.push(format!("{here}: not reported by plt-rs")),
            (Json::Array(items), Some(Json::Array(got)))
                if items.iter().all(|i| i.as_object().is_some()) =>
            {
                if items.len() != got.len() {
                    problems.push(format!(
                        "{here}: expected {} entries, got {}",
                        items.len(),
                        got.len()
                    ));
                }
                for (i, (item, other)) in items.iter().zip(got).enumerate() {
                    diff_object(item, other, &format!("{here}[{i}]"), tol, problems);
                }
            }
            (_, Some(got)) => {
                if let Err(mismatch) = compare(expected, got, tol) {
                    problems.push(format!("{here}{}", strip_root(&mismatch.to_string())));
                }
            }
        }
    }
}

fn strip_root(message: &str) -> String {
    message
        .strip_prefix("at <root>")
        .map_or_else(|| format!(" {message}"), str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_reports_each_problem() {
        let golden =
            Json::parse(r#"{"size_px": [640, 480], "axes": [{"xlim": [0, 1], "title": "t"}]}"#)
                .unwrap();
        let actual = Json::parse(r#"{"size_px": [640, 480], "axes": [{"xlim": [0, 2]}]}"#).unwrap();
        let problems = diff_facts(&golden, &actual);
        assert_eq!(problems.len(), 2);
        assert!(problems[0].starts_with(".axes[0].xlim"));
        assert!(problems[1].contains("title: not reported"));
    }
}
