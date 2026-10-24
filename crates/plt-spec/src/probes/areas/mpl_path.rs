//! Probe operations for the `mpl_path` area: matplotlib's own tests of
//! paths (see `spec/reference/probes/test_mpl_path.py`). Most operations
//! are the `path` area's.

use plt::figure::Figure;
use plt::transforms::Point;
use plt_core::Path;
use plt_core::path::PolygonOptions;

use super::path::{
    arc, cleaned, compound, contains_path, contains_points, get_extents, interpolated,
    intersects_path, path_arg, point_list, to_polygons,
};
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError, encode};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("path.Path", reshaped_path),
    Op::new("path.Path.get_extents", get_extents),
    Op::new("path.Path.contains_points", contains_points),
    Op::new("path.Path.contains_path", contains_path),
    Op::new("path.Path.make_compound_path", compound),
    Op::new("path.Path.clip_to_bbox.to_polygons", clipped_polygons),
    Op::new("path.Path.to_polygons", to_polygons),
    Op::new("path.Path.intersects_path", intersects_path),
    Op::new("path.Path.arc", arc),
    Op::new("path.Path.cleaned", cleaned),
    Op::new("path.Path.interpolated", interpolated),
    Op::new("axes.Axes.axvspan.contains_point", span_contains_point),
];

/// `Path(np.asarray(vertices, float).reshape(-1, 2), closed=closed)`.
fn reshaped_path(args: Args<'_>) -> Outcome {
    let vertices = args.json("vertices")?;
    let vertices = if vertices.as_array().is_some_and(<[Json]>::is_empty) {
        Vec::new()
    } else {
        encode::parse_points(vertices)?
    };
    let path = if args.bool("closed")? {
        Path::closed(vertices)
    } else {
        Path::polyline(vertices)
    };
    Ok(encode::path(&path))
}

/// `path.clip_to_bbox(bbox).to_polygons()` with the default options.
fn clipped_polygons(args: Args<'_>) -> Outcome {
    let bbox = encode::parse_bbox(args.json("bbox")?)?;
    let clipped = path_arg(args, "path")?.clip_to_bbox(&bbox, true);
    Ok(Json::Array(
        clipped
            .to_polygons(&PolygonOptions::default())
            .iter()
            .map(|polygon| point_list(polygon))
            .collect(),
    ))
}

/// Whether `ax.axvspan(*span)` contains the display point of a data point,
/// in an axes of the given x scale and y limits.
fn span_contains_point(args: Args<'_>) -> Outcome {
    let pair = |key| match args.f64_vec(key)?[..] {
        [a, b] => Ok((a, b)),
        _ => Err(ProbeError::BadArgs(format!("{key} needs two values"))),
    };
    let (ymin, ymax) = pair("ylim")?;
    let (xmin, xmax) = pair("span")?;
    let (x, y) = pair("point")?;
    let figure = Figure::new();
    let ax = figure.subplot();
    ax.set_xscale(args.str("xscale")?)
        .map_err(ProbeError::rejected)?;
    ax.set_ylim(ymin, ymax);
    let span = ax.axvspan(xmin, xmax);
    let data = figure
        .resolve_transform(&ax.trans_data())
        .map_err(ProbeError::rejected)?;
    Ok(span
        .contains_display_point(data.transform_point(Point::new(x, y)))
        .into())
}
