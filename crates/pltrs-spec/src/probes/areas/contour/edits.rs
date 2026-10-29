//! `ContourSet.find_nearest_contour` and `ContourSet.label_edits` probes
//! (`nearest_op` and `label_edits_op` in the spec): the contour point
//! nearest to given points, and labels added and removed after `clabel`.

use pltrs::artist::ArtistHandle;
use pltrs::contour::{ContourSet, NearestIn};
use pltrs::transforms::Transform;
use pltrs_core::Point;

use super::clabel::{clabel_options, contour_in};
use super::input;
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// Answers each `[x, y, indices, pixel]` query with `[level, line, index,
/// x, y, d2]`, after an optional `clabel`.
pub(super) fn find_nearest_contour(args: Args<'_>) -> Outcome {
    input::with_rc(args, || {
        let cs = contour_in(args, args.opt_bool("filled")?.unwrap_or(false))?;
        if let Some(clabel) = args.get("clabel") {
            cs.clabel(clabel_options(Args::new(clabel))?)
                .map_err(ProbeError::rejected)?;
        }
        let queries = list(args.json("queries")?, "queries")?;
        let answers = queries
            .iter()
            .map(|query| nearest(&cs, query))
            .collect::<Result<_, _>>()?;
        Ok(Json::Array(answers))
    })
}

/// One `find_nearest_contour` query.
fn nearest(cs: &ContourSet, query: &Json) -> Result<Json, ProbeError> {
    let bad = || ProbeError::BadArgs(format!("bad query {query}"));
    let [x, y, indices, pixel] = list(query, "query")? else {
        return Err(bad());
    };
    let point = Point::new(x.as_f64().ok_or_else(bad)?, y.as_f64().ok_or_else(bad)?);
    let indices = match indices {
        Json::Null => None,
        Json::Array(items) => Some(
            items
                .iter()
                .map(index)
                .collect::<Option<Vec<_>>>()
                .ok_or_else(bad)?,
        ),
        _ => return Err(bad()),
    };
    let space = match pixel {
        Json::Bool(true) => NearestIn::Display,
        Json::Bool(false) => NearestIn::Coordinates,
        _ => return Err(bad()),
    };
    let near = cs
        .find_nearest_contour(point, indices.as_deref(), space)
        .map_err(ProbeError::rejected)?
        .ok_or_else(|| ProbeError::rejected("no contour lines"))?;
    Ok(Json::Array(vec![
        integer(near.level),
        integer(near.line),
        integer(near.vertex),
        Json::from(near.point.x),
        Json::from(near.point.y),
        Json::from(near.distance_squared),
    ]))
}

/// Labels with `clabel`, applies the `["add", x, y, rotation, level]` and
/// `["pop", index]` edits in order, and reports the labels left.
pub(super) fn label_edits(args: Args<'_>) -> Outcome {
    input::with_rc(args, || {
        let cs = contour_in(args, false)?;
        cs.clabel(clabel_options(Args::new(args.json("clabel")?))?)
            .map_err(ProbeError::rejected)?;
        for edit in list(args.json("edits")?, "edits")? {
            apply(&cs, edit)?;
        }
        let data = cs
            .axes()
            .figure()
            .resolve_transform(&cs.axes().trans_data())
            .map_err(ProbeError::rejected)?;
        let labels = cs
            .labels()
            .iter()
            .map(|label| {
                let text = &label.artist;
                // matplotlib's `Text.get_rotation()` turns the data angle of
                // a text rotating with the data back into the display.
                let rotation = if text.transform_rotates_text() {
                    data.transform_angle(text.rotation(), text.position())
                } else {
                    label.rotation
                };
                Json::Object(vec![
                    ("level".into(), Json::from(label.level)),
                    ("text".into(), Json::from(label.text.clone())),
                    (
                        "display".into(),
                        Json::from(vec![label.position.x, label.position.y]),
                    ),
                    ("rotation".into(), Json::from(rotation)),
                    ("color".into(), encode::rgba(label.color)),
                ])
            })
            .collect();
        Ok(Json::Array(labels))
    })
}

/// One label edit.
fn apply(cs: &ContourSet, edit: &Json) -> Result<(), ProbeError> {
    let bad = || ProbeError::BadArgs(format!("bad edit {edit}"));
    match list(edit, "edit")? {
        [Json::String(add), x, y, rotation, level] if add == "add" => {
            let point = Point::new(x.as_f64().ok_or_else(bad)?, y.as_f64().ok_or_else(bad)?);
            let rotation = rotation.as_f64().ok_or_else(bad)?;
            let level = index(level).ok_or_else(bad)?;
            cs.add_label(point, &Transform::Display, rotation, level)
                .map_err(ProbeError::rejected)?;
        }
        [Json::String(pop), at] if pop == "pop" => {
            let at = match at {
                Json::Null => None,
                at => Some(index(at).ok_or_else(bad)?),
            };
            cs.pop_label(at).map_err(ProbeError::rejected)?;
        }
        _ => return Err(bad()),
    }
    Ok(())
}

/// The items of the JSON list `value`, named `what` in errors.
fn list<'a>(value: &'a Json, what: &str) -> Result<&'a [Json], ProbeError> {
    value
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs(format!("{what} is not a list")))
}

/// A non-negative integer.
fn index(value: &Json) -> Option<usize> {
    value.as_i64().and_then(|v| usize::try_from(v).ok())
}

/// An index as a JSON integer.
fn integer(index: usize) -> Json {
    Json::Integer(i64::try_from(index).expect("indices fit in i64"))
}
