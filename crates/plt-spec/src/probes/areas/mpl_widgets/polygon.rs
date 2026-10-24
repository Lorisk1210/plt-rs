//! The `PolygonSelector` op.

use std::cell::RefCell;
use std::rc::Rc;

use plt::artist::ArtistHandle;
use plt::axes::Axes;
use plt::lines::Line2D;
use plt::widgets::{PolygonSelector, PolygonSelectorOptions};

use super::region::{alpha, per_line};
use super::steps::{Attempt, Tool, replay, unknown};
use super::{bad, get_ax, markerfacecolor, props, rgba};
use crate::json::Json;
use crate::probes::areas::canon::Canon;
use crate::probes::areas::mpl_ticker::styled;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// Vertices as `[[x, y], ...]`.
fn vertices(verts: &[(f64, f64)]) -> Json {
    Json::Array(
        verts
            .iter()
            .map(|&(x, y)| Json::Array(vec![Json::from(x), Json::from(y)]))
            .collect(),
    )
}

/// What `_handles_artists.<getter>` records for the marker lines `lines`.
fn handle_query(lines: &[Line2D], getter: &str) -> Result<Json, ProbeError> {
    match getter {
        "get_color" => per_line(lines, |line| rgba(&line.color())),
        "get_alpha" => per_line(lines, |line| Ok(alpha(line))),
        "get_marker" => per_line(lines, |line| Ok(line.marker().canon())),
        "get_markerfacecolor" => per_line(lines, markerfacecolor),
        "get_markeredgecolor" => per_line(lines, |line| rgba(&line.effective_markeredgecolor())),
        other => Err(unknown("handle query", other)),
    }
}

impl Tool for PolygonSelector {
    fn ax(&self) -> Axes {
        PolygonSelector::ax(self)
    }

    fn call(&self, method: &str, args: &[Json]) -> Result<Attempt, ProbeError> {
        let props = || props::line(args.first().ok_or_else(|| bad(method))?);
        match method {
            "set_props" => self.set_props(props()?),
            "set_handle_props" => self.set_handle_props(props()?),
            "clear" => self.clear(),
            other => return Err(unknown("method", other)),
        }
        Ok(Attempt::Done)
    }

    fn set(&self, attribute: &str, value: &Json) -> Result<Attempt, ProbeError> {
        match attribute {
            "verts" => {
                let verts = value
                    .as_array()
                    .ok_or_else(|| bad("verts"))?
                    .iter()
                    .map(|xy| match xy.as_f64_vec().as_deref() {
                        Some(&[x, y]) => Ok((x, y)),
                        _ => Err(bad("verts")),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(self.set_verts(&verts).into())
            }
            other => Err(unknown("attribute", other)),
        }
    }

    fn get(&self, query: &str) -> Result<Json, ProbeError> {
        let line = self.artist();
        if let Some(getter) = query.strip_prefix("_handles_artists.") {
            return handle_query(&[self.handles()], getter);
        }
        if let Some(getter) = query.strip_prefix("_box._handles_artists.") {
            return handle_query(&self.box_handles(), getter);
        }
        Ok(match query {
            "verts" => vertices(&self.verts()),
            "_box.extents" => self.box_extents().map_or(Json::Null, |(x0, x1, y0, y1)| {
                super::numbers(&[x0, x1, y0, y1])
            }),
            "_selection_completed" => Json::from(self.selection_completed()),
            "get_visible" => Json::from(self.visible()),
            "_selection_artist.get_visible" => Json::from(line.visible()),
            "_selection_artist.get_xydata" => {
                let points: Vec<(f64, f64)> = line.xydata().iter().map(|p| (p.x, p.y)).collect();
                vertices(&points)
            }
            "_selection_artist.get_color" => rgba(&line.color())?,
            "_selection_artist.get_alpha" => alpha(&line),
            "_box._selection_artist.get_linestyle"
            | "_box._selection_artist.get_facecolor"
            | "_box._selection_artist.get_edgecolor" => {
                let patch = self.box_artist().ok_or_else(|| bad("no bounding box"))?;
                match query.rsplit('.').next() {
                    Some("get_linestyle") => patch.style().linestyle.canon(),
                    Some("get_facecolor") => {
                        encode::rgba(patch.colors().map_err(ProbeError::rejected)?.face)
                    }
                    _ => encode::rgba(patch.colors().map_err(ProbeError::rejected)?.edge),
                }
            }
            other => return Err(unknown("query", other)),
        })
    }
}

/// The selector options the probe gives.
fn options(given: &Json) -> Result<PolygonSelectorOptions, ProbeError> {
    let mut options = PolygonSelectorOptions::default();
    let entries = given.as_object().ok_or_else(|| bad("options"))?;
    for (name, value) in entries {
        match name.as_str() {
            "draw_bounding_box" => {
                options.draw_bounding_box = value.as_bool().ok_or_else(|| bad(name))?;
            }
            // matplotlib replaces its default line look with `props`.
            "props" => options.props = props::line(value)?,
            "handle_props" => options.handle_props = Some(props::line(value)?),
            "box_props" => options.box_props = Some(props::patch(value)?),
            "box_handle_props" => options.box_handle_props = props::line(value)?,
            other => return Err(unknown("option", other)),
        }
    }
    Ok(options)
}

/// `PolygonSelector(ax, onselect, **options)` driven by `steps`, on axes
/// limited to `xlim` and `ylim` when given.
pub(super) fn polygon(args: Args<'_>) -> Outcome {
    styled(args, || {
        let ax = get_ax()?;
        if let (Some(xlim), Some(ylim)) = (args.get("xlim"), args.get("ylim")) {
            match (xlim.as_f64_vec().as_deref(), ylim.as_f64_vec().as_deref()) {
                (Some(&[x0, x1]), Some(&[y0, y1])) => {
                    ax.set_xlim(x0, x1).set_ylim(y0, y1);
                }
                _ => return Err(bad("limits")),
            }
        }
        let tool = PolygonSelector::new(&ax, options(args.json("options")?)?);
        let selects: Rc<RefCell<Vec<Json>>> = Rc::default();
        let sink = Rc::clone(&selects);
        tool.on_select(move |verts| sink.borrow_mut().push(vertices(verts)));
        let steps = replay(&tool, args.json("steps")?)?;
        let selects = Json::Array(selects.borrow().clone());
        Ok(encode::object([("steps", steps), ("selects", selects)]))
    })
}
