//! Probe operations for the `api_audit_3d` area: the 3-D API the API audit
//! found missing and added (see `spec/reference/probes/test_api_audit_3d.py`).

use pltrs::artist::ArtistHandle;
use pltrs::collections::{Collection, CollectionPatch, CollectionProps, CollectionSpec};
use pltrs::dates::DateTime;
use pltrs::figure::Figure;
use pltrs::lines::LineProps;
use pltrs::mplot3d::{Axes3D, Axis3};
use pltrs::patches::{Circle, PatchProps};
use pltrs::path::PathCode;
use pltrs::scale::ScaleName;
use pltrs::transforms::Transform;
use pltrs::units::AxisValues;
use pltrs_core::Point;

use crate::json::Json;
use crate::probes::encode::{count, numbers, pair, point, rgba};
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("Axes3D.convert_zunits", convert_zunits),
    Op::new("Axis3D.update_units", update_units),
    Op::new("Axes3D.plot", plot_units),
    Op::new("Axes3D.get_zgridlines", zgridlines),
    Op::new("Axes3D.get_zticklines", zticklines),
    Op::new("Axes3D.set_lim3d", set_lim3d),
    Op::new("Collection3D.set_sort_zpos", sort_zpos),
    Op::new("Line3DCollection.set_segments", set_segments),
    Op::new("Patch3DCollection", patch_collection),
    Op::new("Patch3DCollection.get_offsets", patch_offsets),
    Op::new("Poly3DCollection.set_verts", set_verts),
    Op::new("Poly3DCollection.set_3d_properties", set_3d_properties),
];

/// A figure with one 3-D axes.
fn axes3d() -> (Figure, Axes3D) {
    let fig = Figure::new();
    let ax = fig.subplot_3d();
    (fig, ax)
}

/// Midnight of the given days of January 2020.
fn days(days: &[u32]) -> Result<Vec<DateTime>, ProbeError> {
    days.iter()
        .map(|&day| DateTime::new(2020, 1, day, 0, 0, 0, 0).map_err(ProbeError::rejected))
        .collect()
}

/// The labels of the major ticks of `axis` of `ax`.
fn labels(ax: &Axes3D, axis: Axis3) -> Outcome {
    let labels = ax
        .axis(axis)
        .ticklabels(false)
        .map_err(ProbeError::rejected)?;
    Ok(Json::Array(labels.into_iter().map(Json::from).collect()))
}

fn convert_zunits(args: Args<'_>) -> Outcome {
    let (_fig, ax) = axes3d();
    let values = match args.str("kind")? {
        "numbers" => AxisValues::from([1.5, 2.5]),
        "dates" => AxisValues::from(days(&[1, 3])?),
        other => return Err(ProbeError::BadArgs(format!("unknown kind {other}"))),
    };
    let converted = ax.zaxis().convert_units(values);
    Ok(Json::Array(vec![
        numbers(converted),
        Json::Bool(ax.zaxis().converter().is_some()),
    ]))
}

fn update_units(args: Args<'_>) -> Outcome {
    let (_fig, ax) = axes3d();
    let (axis, updated, converted) = match args.str("kind")? {
        "dates" => {
            let axis = ax.zaxis();
            let updated = axis.update_units(days(&[1, 2])?);
            let converted = axis.convert_units(days(&[3, 10])?);
            ax.set_zlim(converted[0], converted[1])
                .map_err(ProbeError::rejected)?;
            (Axis3::Z, updated, converted)
        }
        "names" => {
            let axis = ax.xaxis();
            let updated = axis.update_units(["a", "b", "c"]);
            let converted = axis.convert_units(["c", "d"]);
            ax.set_xlim(-0.5, 3.5).map_err(ProbeError::rejected)?;
            (Axis3::X, updated, converted)
        }
        other => return Err(ProbeError::BadArgs(format!("unknown kind {other}"))),
    };
    Ok(Json::Array(vec![
        Json::Bool(updated),
        numbers(converted),
        labels(&ax, axis)?,
    ]))
}

/// The axis named `name`.
fn axis_named(name: &str) -> Result<Axis3, ProbeError> {
    match name {
        "x" => Ok(Axis3::X),
        "y" => Ok(Axis3::Y),
        "z" => Ok(Axis3::Z),
        other => Err(ProbeError::BadArgs(format!("unknown axis {other}"))),
    }
}

fn plot_units(args: Args<'_>) -> Outcome {
    let zdir = axis_named(args.str("zdir")?)?;
    let (_fig, ax) = axes3d();
    ax.plot_with(
        ["a", "b", "c", "a"],
        [0.0, 1.0, 2.0, 3.0],
        days(&[1, 2, 4, 8])?,
        zdir,
        LineProps::default(),
    )
    .map_err(ProbeError::rejected)?;
    Ok(Json::Array(vec![
        pair(ax.xlim()),
        pair(ax.ylim()),
        pair(ax.zlim()),
        labels(&ax, Axis3::X)?,
        labels(&ax, Axis3::Z)?,
    ]))
}

fn zgridlines(args: Args<'_>) -> Outcome {
    let (fig, ax) = axes3d();
    let [low, high] = limits(args.f64_vec("zlim")?)?;
    ax.set_zlim(low, high).map_err(ProbeError::rejected)?;
    fig.layout();
    let lines = ax.zaxis().gridlines().map_err(ProbeError::rejected)?;
    let drawn = (lines.iter())
        .filter(|line| line.in_view)
        .map(|line| Json::Array(line.points.iter().map(|&p| point(p)).collect()))
        .collect();
    Ok(Json::Array(vec![count(lines.len()), Json::Array(drawn)]))
}

fn zticklines(args: Args<'_>) -> Outcome {
    let (fig, ax) = axes3d();
    let [low, high] = limits(args.f64_vec("zlim")?)?;
    ax.set_zlim(low, high).map_err(ProbeError::rejected)?;
    fig.layout();
    let marks = ax.zaxis().ticklines(false).map_err(ProbeError::rejected)?;
    let drawn = (marks.iter())
        .filter(|mark| mark.in_view)
        .filter_map(|mark| mark.marks.first())
        .map(|[a, b]| Json::Array(vec![numbers([a.x, b.x]), numbers([a.y, b.y])]))
        .collect();
    Ok(Json::Array(vec![count(marks.len()), Json::Array(drawn)]))
}

/// `[low, high]` from two numbers.
fn limits(values: Vec<f64>) -> Result<[f64; 2], ProbeError> {
    values
        .try_into()
        .map_err(|_| ProbeError::BadArgs("limits are two numbers".into()))
}

fn set_lim3d(args: Args<'_>) -> Outcome {
    let [low, high] = limits(args.f64_vec("lim")?)?;
    let margin = args.opt_f64("view_margin")?;
    let scale = ScaleName::from(args.str("scale")?);
    let setter = args.str("setter")?;
    let automargin = if args.bool("automargin")? {
        "True"
    } else {
        "False"
    };
    pltrs::rc::context(|| {
        pltrs::rc::set("axes3d.automargin", automargin).map_err(ProbeError::rejected)?;
        let (_fig, ax) = axes3d();
        ax.set_zscale(scale).map_err(ProbeError::rejected)?;
        match setter {
            "set_zlim" => ax.set_limits_with_margin(Axis3::Z, low, high, margin),
            "set_zbound" => ax.set_bound_with_margin(Axis3::Z, Some(low), Some(high), margin),
            other => return Err(ProbeError::BadArgs(format!("unknown setter {other}"))),
        }
        .map_err(ProbeError::rejected)?;
        Ok(pair(ax.zlim()))
    })
}

/// A triangle in the plane at height `z`.
fn triangle(z: f64) -> Vec<[f64; 3]> {
    vec![[0.0, 0.0, z], [1.0, 0.0, z], [0.0, 1.0, z]]
}

/// A collection of `kind` at height `z`.
fn collection(ax: &Axes3D, kind: &str, z: f64) -> Result<Collection, ProbeError> {
    match kind {
        "poly" => ax.add_poly_collection3d(vec![triangle(z)], CollectionProps::default(), true),
        "line" => ax.add_line_collection3d(vec![triangle(z)], CollectionProps::default(), true),
        "scatter" => ax.scatter([0.0, 1.0], [0.0, 1.0], [z, z]),
        other => return Err(ProbeError::BadArgs(format!("unknown collection {other}"))),
    }
    .map_err(ProbeError::rejected)
}

fn sort_zpos(args: Args<'_>) -> Outcome {
    let kind = args.str("kind")?;
    let (fig, ax) = axes3d();
    let (first, second) = (collection(&ax, kind, 0.0)?, collection(&ax, kind, 1.0)?);
    fig.layout();
    let before = pair((first.zorder(), second.zorder()));
    ax.set_sort_zpos(&second, Some(-5.0))
        .map_err(ProbeError::rejected)?;
    fig.layout();
    Ok(Json::Array(vec![
        before,
        pair((first.zorder(), second.zorder())),
    ]))
}

/// A list of polylines through 3-D points.
fn segments(value: &Json) -> Result<Vec<Vec<[f64; 3]>>, ProbeError> {
    let malformed = || ProbeError::BadArgs("segments are lists of [x, y, z]".into());
    let lines = value.as_array().ok_or_else(malformed)?;
    lines
        .iter()
        .map(|line| {
            let points = line.as_array().ok_or_else(malformed)?;
            points
                .iter()
                .map(|p| {
                    let coords = p.as_f64_vec().ok_or_else(malformed)?;
                    coords.try_into().map_err(|_| malformed())
                })
                .collect()
        })
        .collect()
}

fn set_segments(args: Args<'_>) -> Outcome {
    let (fig, ax) = axes3d();
    let lines = ax
        .add_line_collection3d(
            vec![vec![[0.0; 3], [1.0; 3]]],
            CollectionProps::default(),
            true,
        )
        .map_err(ProbeError::rejected)?;
    ax.set_segments(&lines, segments(args.json("segments")?)?)
        .map_err(ProbeError::rejected)?;
    fig.layout();
    Ok(Json::Array(
        (lines.paths().iter())
            .map(|path| Json::Array(path.vertices().iter().map(|&p| point(p)).collect()))
            .collect(),
    ))
}

/// The patch collection of the area's ops: a small circle at two display
/// offsets placed at 0.1 and 0.9 along y.
fn patches(ax: &Axes3D) -> Result<Collection, ProbeError> {
    let circle = CollectionPatch::new(&Circle::new((0.0, 0.0), 0.01), PatchProps::default());
    let spec = CollectionSpec::patches(&[circle], false)
        .map_err(ProbeError::rejected)?
        .with_offsets(
            vec![Point::new(0.2, 0.3), Point::new(0.6, 0.7)],
            Transform::Display,
        );
    ax.add_collection3d(spec, &[0.1, 0.9], Axis3::Y, true)
        .map_err(ProbeError::rejected)
}

fn patch_collection(args: Args<'_>) -> Outcome {
    let (_fig, ax) = axes3d();
    let col = patches(&ax)?;
    if let Some(shade) = args.opt_bool("depthshade")? {
        ax.set_depthshade(&col, shade)
            .map_err(ProbeError::rejected)?;
    }
    let shading = ax.depthshade(&col).map_err(ProbeError::rejected)?;
    ax.set_depthshade(&col, !shading)
        .map_err(ProbeError::rejected)?;
    let offsets = |ax: &Axes3D| -> Outcome {
        let columns = ax.scatter_offsets(&col).map_err(ProbeError::rejected)?;
        Ok(Json::Array(columns.iter().map(numbers).collect()))
    };
    let lifted = offsets(&ax)?;
    // matplotlib's `set_3d_properties([0.5, 0.25], "x")` on the 2-D offsets.
    let [x, y, z] = Axis3::X.juggle([vec![0.2, 0.6], vec![0.3, 0.7], vec![0.5, 0.25]]);
    ax.set_scatter_offsets(&col, x, y, z)
        .map_err(ProbeError::rejected)?;
    Ok(Json::Array(vec![
        Json::Bool(shading),
        Json::Bool(ax.depthshade(&col).map_err(ProbeError::rejected)?),
        lifted,
        offsets(&ax)?,
    ]))
}

fn patch_offsets(_args: Args<'_>) -> Outcome {
    let (fig, ax) = axes3d();
    let col = patches(&ax)?;
    fig.layout();
    Ok(Json::Array(col.offsets().into_iter().map(point).collect()))
}

/// `[[vertex count, codes or null], ...]` of the paths of `col`.
fn path_summaries(col: &Collection) -> Json {
    Json::Array(
        (col.paths().iter())
            .map(|path| {
                let codes = path.codes().map_or(Json::Null, |codes| {
                    numbers(codes.iter().map(|&code| f64::from(code as u8)))
                });
                Json::Array(vec![count(path.len()), codes])
            })
            .collect(),
    )
}

/// The face colours `col` draws with.
fn face_colors(col: &Collection) -> Outcome {
    let colors = col.colors().map_err(ProbeError::rejected)?;
    Ok(Json::Array(colors.faces.into_iter().map(rgba).collect()))
}

fn set_verts(args: Args<'_>) -> Outcome {
    let (closed, codes) = (args.bool("closed")?, args.bool("codes")?);
    let (fig, ax) = axes3d();
    let props = CollectionProps {
        facecolors: Some(vec!["r".into(), "b".into()]),
        ..CollectionProps::default()
    };
    let col = ax
        .add_poly_collection3d(vec![triangle(0.0)], props, true)
        .map_err(ProbeError::rejected)?;
    let square = vec![
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
        [0.0, 1.0, 1.0],
    ];
    let faces = vec![triangle(2.0), square];
    if codes {
        let lines = (faces.iter())
            .map(|face| {
                let mut codes = vec![PathCode::LineTo; face.len()];
                codes[0] = PathCode::MoveTo;
                codes
            })
            .collect();
        ax.set_poly_verts_and_codes(&col, faces, lines)
    } else {
        ax.set_poly_verts(&col, faces, closed)
    }
    .map_err(ProbeError::rejected)?;
    fig.layout();
    Ok(Json::Array(vec![path_summaries(&col), face_colors(&col)?]))
}

fn set_3d_properties(_args: Args<'_>) -> Outcome {
    let (fig, ax) = axes3d();
    let (first, second) = (collection(&ax, "poly", 0.0)?, collection(&ax, "poly", 1.0)?);
    ax.set_sort_zpos(&second, Some(-5.0))
        .map_err(ProbeError::rejected)?;
    second.set_facecolor("red");
    ax.set_poly_3d_properties(&second)
        .map_err(ProbeError::rejected)?;
    fig.layout();
    Ok(Json::Array(vec![
        pair((first.zorder(), second.zorder())),
        face_colors(&second)?,
    ]))
}
