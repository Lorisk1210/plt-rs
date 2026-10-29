//! Probe operations for the `mpl_collections` area: matplotlib's own
//! non-image tests of collections that the `collection_behaviour` area
//! leaves out (see `spec/reference/probes/test_mpl_collections.py`).

mod events;
mod legend;
mod meshes;

use pltrs::artist::ArtistHandle;
use pltrs::collections::{
    Collection, CollectionProps, CollectionSpec, ColorMapping, EllipseCollection, EllipseUnits,
};
use pltrs::colors::{Color, ScalarMappable, colormaps};
use pltrs::figure::Figure;
use pltrs::lines::LineStyle;
use pltrs::numeric::linspace;
use pltrs::rc;
use pltrs::transforms::Transform;
use pltrs_core::{Affine2D, Path, Point};
use pltrs_render::{CapStyle, Dashes, JoinStyle};

use super::mpl_tests::in_test_style;
use crate::json::Json;
use crate::probes::encode::{self, numbers, object, rgba};
use crate::probes::{Args, Op, Outcome, ProbeError, made};
use crate::rng::SpecRng;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("EventCollection.get", |args| {
        in_test_style(args, || events::get(args))
    }),
    Op::new("LineCollection.norm", |args| in_test_style(args, norm)),
    Op::new("Collection.get_datalim", |args| {
        in_test_style(args, || datalim(args))
    }),
    Op::new("PathCollection.get_linestyle", |args| {
        in_test_style(args, broadcast_styles)
    }),
    Op::new("Collection.set_linestyle", |args| {
        in_test_style(args, || set_linestyle(args))
    }),
    Op::new("Axes.scatter", |args| in_test_style(args, || scatter(args))),
    Op::new("PathCollection.get_capstyle", |args| {
        in_test_style(args, || capstyle(args))
    }),
    Op::new("PathCollection.get_joinstyle", |args| {
        in_test_style(args, || joinstyle(args))
    }),
    Op::new("LineCollection.get", |args| {
        in_test_style(args, line_collection)
    }),
    Op::new("LineCollection.get_segments", |args| {
        in_test_style(args, segments)
    }),
    Op::new("Collection.get_offset_transform", |args| {
        in_test_style(args, offset_transform)
    }),
    Op::new("EllipseCollection.get", |args| {
        in_test_style(args, ellipses)
    }),
    Op::new("QuadMesh.get_cursor_data", |args| {
        in_test_style(args, || meshes::cursor_data(args))
    }),
    Op::new("QuadMesh.get_coordinates", |args| {
        in_test_style(args, meshes::coordinates)
    }),
    Op::new("QuadMesh.set_array", |args| {
        in_test_style(args, || meshes::set_array(args))
    }),
    Op::new("Collection.get_facecolors", |args| {
        in_test_style(args, || meshes::facecolors(args))
    }),
    Op::new("Collection.color_logic", |args| {
        in_test_style(args, || meshes::color_logic(args))
    }),
    Op::new("PolyQuadMesh.get_paths", |args| {
        in_test_style(args, || meshes::masked_vertices(args))
    }),
    Op::new("Collection.get_hatch_linewidth", |args| {
        in_test_style(args, || hatch_linewidth(args))
    }),
    Op::new("FillBetweenPolyCollection", |args| {
        in_test_style(args, || fill_between(args))
    }),
    Op::new("PathCollection.legend_elements", |args| {
        in_test_style(args, || legend::legend_elements(args))
    }),
];

fn unknown(what: &str) -> ProbeError {
    ProbeError::BadArgs(format!("unknown scene {what:?}"))
}

/// `[[offset, pattern | null], ...]`, as `get_linestyle` gives them.
fn dashes(patterns: Vec<Dashes>) -> Json {
    Json::Array(
        patterns
            .into_iter()
            .map(|d| {
                let pattern = d.pattern.filter(|p| !p.is_empty()).map(numbers);
                Json::Array(vec![Json::from(d.offset), Json::from(pattern)])
            })
            .collect(),
    )
}

/// RGBA colours.
fn colors(colors: Vec<pltrs_core::Rgba>) -> Json {
    Json::Array(colors.into_iter().map(rgba).collect())
}

/// A collection added to a fresh `plt.subplots()` axes.
fn added(spec: CollectionSpec) -> Collection {
    Figure::new().subplot().add_collection(spec)
}

/// `test_collection_norm_autoscale`: the norm of a mapped line collection
/// at 2, before and after new values.
fn norm() -> Outcome {
    let segments = (0..4)
        .map(|i| {
            (0..3)
                .map(|j| Point::new(f64::from(6 * i + 2 * j), f64::from(6 * i + 2 * j + 1)))
                .collect()
        })
        .collect();
    let mapping = ColorMapping {
        values: (0..4).map(|i| Some(f64::from(i))).collect(),
        mappable: ScalarMappable::new(made(colormaps::get("viridis"))?),
    };
    let lines = added(CollectionSpec::lines(segments).with_mapping(mapping));
    let at_two = |lines: &Collection| -> Result<f64, ProbeError> {
        let mut norm = lines
            .norm()
            .ok_or_else(|| ProbeError::rejected("no norm"))?;
        match made(norm.normalize(&[2.0]))?.as_slice() {
            [pltrs::colors::Normalized::Fraction(x)] => Ok(*x),
            other => Err(ProbeError::rejected(format!("normalised to {other:?}"))),
        }
    };
    let before = at_two(&lines)?;
    made(lines.set_array((5..9).map(|i| Some(f64::from(i))).collect()))?;
    Ok(numbers([before, at_two(&lines)?]))
}

/// `get_datalim(ax.transData)` of the scenes of `test_null_collection_datalim`,
/// `test_no_offsets_datalim` and `test_set_offsets_late`.
fn datalim(args: Args<'_>) -> Outcome {
    match args.str("scene")? {
        "null" => Ok(encode::bbox(
            &added(CollectionSpec::paths(Vec::new())).datalim(),
        )),
        "no_offsets" => {
            let path = Path::polyline(vec![Point::ORIGIN, Point::new(1.0, 0.0)]);
            Ok(encode::bbox(
                &added(CollectionSpec::paths(vec![path])).datalim(),
            ))
        }
        "offsets_late" => {
            let ax = Figure::new().subplot();
            let null = ax.add_collection(CollectionSpec::circles(vec![2.0]));
            let init = ax.add_collection(
                CollectionSpec::circles(vec![2.0])
                    .with_offsets(vec![Point::new(10.0, 10.0)], Transform::Display),
            );
            let late = ax.add_collection(CollectionSpec::circles(vec![2.0]));
            late.set_offsets(vec![Point::new(10.0, 10.0)]);
            Ok(object([
                ("null_differs", Json::Bool(null.datalim() != init.datalim())),
                ("late_matches", Json::Bool(init.datalim() == late.datalim())),
            ]))
        }
        other => Err(unknown(other)),
    }
}

/// `test_lslw_bcast`: widths and styles cycled to a common length.
fn broadcast_styles() -> Outcome {
    let paths = added(CollectionSpec::paths(Vec::new()));
    paths
        .set_linestyles(vec![LineStyle::Solid; 2])
        .set_linewidths(vec![1.0, 2.0, 3.0]);
    let (styles, widths) = (paths.dash_patterns(), paths.broadcast_linewidths());
    paths.set_linestyles(vec![LineStyle::Solid; 3]);
    Ok(object([
        ("linestyles", dashes(styles)),
        ("linewidths", numbers(widths)),
        ("linestyles_3", dashes(paths.dash_patterns())),
        ("linewidths_3", numbers(paths.broadcast_linewidths())),
    ]))
}

/// A line style given by name or as `[offset, [on, off, ...]]`.
fn linestyle(value: &Json) -> Result<LineStyle, ProbeError> {
    match value {
        Json::String(name) => made(name.parse()),
        Json::Array(parts) => match parts.as_slice() {
            [offset, pattern] => Ok(LineStyle::dashes(
                offset
                    .as_f64()
                    .ok_or_else(|| ProbeError::BadArgs("bad offset".into()))?,
                pattern
                    .as_f64_vec()
                    .ok_or_else(|| ProbeError::BadArgs("bad pattern".into()))?,
            )),
            _ => Err(ProbeError::BadArgs(format!("bad line style {value}"))),
        },
        other => Err(ProbeError::BadArgs(format!("bad line style {other}"))),
    }
}

/// `test_set_wrong_linestyle`: the scaled dashes of a style, or the error
/// of an unknown one.
fn set_linestyle(args: Args<'_>) -> Outcome {
    let style = linestyle(args.json("ls")?)?;
    let collection = added(CollectionSpec::paths(Vec::new()));
    collection.set_linestyles(vec![style]);
    Ok(dashes(collection.dash_patterns()))
}

/// `test_scatter_empty_linestyle_pdf` and `test_linestyle_single_dashes`:
/// the dashes of a scatter's edges once saved as PDF.
fn scatter(args: Args<'_>) -> Outcome {
    let style = linestyle(args.json("ls")?)?;
    let fig = Figure::new();
    let points = made(
        fig.subplot()
            .scatter(vec![0.0, 1.0, 2.0], vec![0.0, 1.0, 2.0]),
    )?;
    points.set_linestyles(vec![style]);
    made(fig.to_pdf())?;
    Ok(dashes(points.dash_patterns()))
}

/// A cap or join style given by name, or none.
fn style_arg<T>(
    args: Args<'_>,
    key: &str,
    parse: fn(&str) -> Option<T>,
) -> Result<Option<T>, ProbeError> {
    args.opt_str(key)?
        .map(|name| parse(name).ok_or_else(|| ProbeError::BadArgs(format!("bad {key} {name:?}"))))
        .transpose()
}

fn cap_from(name: &str) -> Option<CapStyle> {
    Some(match name {
        "butt" => CapStyle::Butt,
        "round" => CapStyle::Round,
        "projecting" => CapStyle::Projecting,
        _ => return None,
    })
}

fn cap_name(style: CapStyle) -> &'static str {
    match style {
        CapStyle::Butt => "butt",
        CapStyle::Round => "round",
        CapStyle::Projecting => "projecting",
    }
}

fn join_from(name: &str) -> Option<JoinStyle> {
    Some(match name {
        "miter" => JoinStyle::Miter,
        "round" => JoinStyle::Round,
        "bevel" => JoinStyle::Bevel,
        _ => return None,
    })
}

fn join_name(style: JoinStyle) -> &'static str {
    match style {
        JoinStyle::Miter => "miter",
        JoinStyle::Round => "round",
        JoinStyle::Bevel => "bevel",
    }
}

/// `test_capstyle`: the cap style given when made, then set.
fn capstyle(args: Args<'_>) -> Outcome {
    let props = CollectionProps {
        capstyle: style_arg(args, "init", cap_from)?,
        ..CollectionProps::default()
    };
    let paths = added(CollectionSpec::paths(Vec::new()).with_props(props));
    if let Some(style) = style_arg(args, "set", cap_from)? {
        paths.set_capstyle(style);
    }
    Ok(Json::from(paths.style().capstyle.map(cap_name)))
}

/// `test_joinstyle`: the join style given when made, then set.
fn joinstyle(args: Args<'_>) -> Outcome {
    let props = CollectionProps {
        joinstyle: style_arg(args, "init", join_from)?,
        ..CollectionProps::default()
    };
    let paths = added(CollectionSpec::paths(Vec::new()).with_props(props));
    if let Some(style) = style_arg(args, "set", join_from)? {
        paths.set_joinstyle(style);
    }
    Ok(Json::from(paths.style().joinstyle.map(join_name)))
}

/// `test_LineCollection_args`: what line collections without segments
/// keep of their keywords.
fn line_collection() -> Outcome {
    let ax = Figure::new().subplot();
    let mut props = CollectionProps::default()
        .linewidth(2.2)
        .edgecolor("r")
        .zorder(3.0);
    props.facecolors = Some(vec![Color::from((0.0, 1.0, 0.0, 1.0))]);
    let lines = ax.add_collection(CollectionSpec::lines(Vec::new()).with_props(props));
    let plain = ax.add_collection(CollectionSpec::lines(Vec::new()));
    let resolved = made(lines.colors())?;
    Ok(object([
        ("linewidth", Json::from(lines.broadcast_linewidths()[0])),
        ("edgecolor", colors(resolved.edges)),
        ("zorder", Json::from(lines.zorder())),
        ("facecolor", colors(resolved.faces)),
        ("facecolor_none", colors(made(plain.colors())?.faces)),
    ]))
}

/// `test_get_segments`: a segment of 256 points reads back unchanged.
fn segments() -> Outcome {
    let t = linspace(0.0, 1.0, 256);
    let segment: Vec<Point> = t.iter().map(|&v| Point::new(v, v)).collect();
    let lines = added(CollectionSpec::lines(vec![segment.clone()]));
    Ok(Json::Bool(lines.segments() == [segment]))
}

/// `test_set_offset_transform`: a skew given when made reads back as one
/// set afterwards.
fn offset_transform() -> Outcome {
    let ax = Figure::new().subplot();
    let skew = Transform::Affine(Affine2D::identity().skew(2.0, 2.0));
    let spec = || CollectionSpec::paths(vec![Path::unit_circle()]);
    let init = ax.add_collection(spec().with_offsets(vec![Point::ORIGIN], skew.clone()));
    let late = ax.add_collection(spec().with_offsets(vec![Point::ORIGIN], Transform::Display));
    late.set_offset_transform(Some(skew.clone()));
    Ok(Json::Bool(
        init.offset_transform().as_ref() == Some(&skew)
            && late.offset_transform() == init.offset_transform(),
    ))
}

/// `test_EllipseCollection_setter_getter`: the axes and angles as made,
/// then as set.
fn ellipses() -> Outcome {
    let mut stream = SpecRng::new(0);
    let ax = Figure::new().subplot();
    let offsets = stream
        .uniform(20, 0.0, 1.0)
        .chunks(2)
        .map(|xy| Point::new(xy[0] * 10.0, xy[1] * 10.0))
        .collect();
    let spec = CollectionSpec::ellipses(&[2.0], &[3.0], &[45.0], EllipseUnits::X)
        .with_offsets(offsets, ax.trans_data());
    let ellipses: EllipseCollection = ax
        .add_collection(spec)
        .raw()
        .downcast()
        .ok_or_else(|| ProbeError::rejected("not an ellipse collection"))?;
    let made_as = (ellipses.widths(), ellipses.heights(), ellipses.angles());
    let scaled = |stream: &mut SpecRng, by: f64| -> Vec<f64> {
        stream
            .uniform(20, 0.0, 1.0)
            .into_iter()
            .map(|u| u * by)
            .collect()
    };
    let (widths, heights) = (scaled(&mut stream, 2.0), scaled(&mut stream, 3.0));
    let angles = scaled(&mut stream, 180.0);
    ellipses
        .set_widths(&widths)
        .set_heights(&heights)
        .set_angles(&angles);
    Ok(object([
        ("widths", numbers(made_as.0)),
        ("heights", numbers(made_as.1)),
        ("angles", numbers(made_as.2)),
        ("set_widths", numbers(ellipses.widths())),
        ("set_heights", numbers(ellipses.heights())),
        ("set_angles", numbers(ellipses.angles())),
    ]))
}

/// `test_hatch_linewidth`: a hatch width taken from the rc when made, or
/// set afterwards.
fn hatch_linewidth(args: Args<'_>) -> Outcome {
    let square = |x0: f64, y0: f64| {
        vec![
            Point::new(x0, y0),
            Point::new(x0, y0 + 0.3),
            Point::new(x0 + 0.3, y0 + 0.3),
            Point::new(x0 + 0.3, y0),
        ]
    };
    let set = args.bool("set")?;
    if !set {
        made(rc::set("hatch.linewidth", "2.0"))?;
    }
    let spec = CollectionSpec::polygons(vec![square(0.1, 0.1), square(0.6, 0.6)])
        .with_props(CollectionProps::default().hatch("x"));
    let polygons = added(spec);
    if set {
        polygons.set_hatch_linewidth(2.0);
    }
    Ok(Json::from(polygons.style().hatch_linewidth))
}

/// `test_fill_between_poly_collection_raise`: a mask of the wrong size,
/// and curves of unequal sizes.
fn fill_between(args: Args<'_>) -> Outcome {
    let ax = Figure::new().subplot();
    let t = linspace(0.0, 16.0, 50);
    let filled = match args.str("case")? {
        "where" => {
            let f1: Vec<f64> = t.iter().map(|v| v.sin()).collect();
            let f2: Vec<f64> = f1.iter().map(|v| v + 0.2).collect();
            let options = pltrs::axes::FillBetweenOptions {
                mask: Some(vec![false; 3]),
                ..pltrs::axes::FillBetweenOptions::default()
            };
            ax.fill_between(t, f1, f2, options)
        }
        "sizes" => ax.fill_betweenx(
            t,
            vec![1.0, 2.0],
            vec![1.2, 2.2],
            pltrs::axes::FillBetweenOptions::default(),
        ),
        other => return Err(unknown(other)),
    };
    made(filled)?;
    Ok(Json::Null)
}
