//! Probe operations for the `text_behaviour` area: text rotation, wrapping
//! in subfigures, annotation coordinate names and the scenes ported from
//! matplotlib's `test_text.py` (see
//! `spec/reference/probes/test_text_behaviour.py`). Every op runs under
//! the reference configuration.

mod scenes;

use pltrs::artist::ArtistHandle;
use pltrs::coords::Coords;
use pltrs::figure::Figure;
use pltrs::gridspec::GridSpec;
use pltrs::text::AnnotateOptions;
use pltrs::transforms::Transform;
use pltrs_core::{Bbox, Point};

use super::text_metrics::{apply_text_kw, rotation, with_rc};
use crate::json::Json;
use crate::probes::encode::{bbox, parse_affine_ops};
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("text.get_rotation", |args| {
        with_rc(args, || get_rotation(args))
    }),
    Op::new("text.wrapped", |args| with_rc(args, || wrapped(args))),
    Op::new("text.annotate_coords", |args| {
        with_rc(args, || annotate_coords(args))
    }),
    Op::new("text.scene", |args| with_rc(args, || scenes::scene(args))),
];

/// An extent, or matplotlib's null bbox for an artist without one.
fn extent(extent: Option<Bbox>) -> Json {
    bbox(&extent.unwrap_or(Bbox::NULL))
}

/// `Text(rotation=...).get_rotation()`, or the rotation of a text whose
/// affine transform turns it.
fn get_rotation(args: Args<'_>) -> Outcome {
    let degrees = rotation(args.get("rotation").unwrap_or(&Json::Null))?;
    let fig = Figure::new();
    let text = match args.get("transform") {
        None => fig.text(0.0, 0.0, "foo"),
        Some(ops) => {
            let text = fig.subplot().text(0.0, 0.0, "test");
            text.set_transform(Transform::Affine(parse_affine_ops(ops)?))
                .set_transform_rotates_text(true);
            text
        }
    };
    text.set_rotation(degrees);
    let laid_out = text.layout().map_err(ProbeError::rejected)?;
    Ok(Json::from(laid_out.style.rotation))
}

/// `_get_wrapped_text()` of a wrapped figure or subfigure text.
fn wrapped(args: Args<'_>) -> Outcome {
    let size = args.f64_vec("figsize")?;
    let [width, height] = size[..] else {
        return Err(ProbeError::BadArgs(format!("bad figsize {size:?}")));
    };
    let fig = Figure::with_size(width, height, 100.0);
    let parent = if args.bool("subfigure")? {
        let grid = GridSpec::new(3, 3).map_err(ProbeError::rejected)?;
        fig.add_subfigure(&grid.cell(1, 1).map_err(ProbeError::rejected)?)
    } else {
        fig.clone()
    };
    let text = parent.text(args.f64("x")?, args.f64("y")?, args.str("text")?);
    text.set_wrap(true);
    apply_text_kw(&text, args.get("kw"))?;
    fig.layout();
    text.wrapped_text()
        .map(Json::from)
        .map_err(ProbeError::rejected)
}

/// Whether `xycoords` names a coordinate system an annotated point may be
/// given in.
fn annotate_coords(args: Args<'_>) -> Outcome {
    let xycoords: Coords = args
        .str("xycoords")?
        .parse()
        .map_err(ProbeError::rejected)?;
    let fig = Figure::new();
    let options = AnnotateOptions {
        xytext: Some(Point::new(0.5, 0.5).into()),
        xycoords,
        ..AnnotateOptions::default()
    };
    fig.subplot()
        .annotate("xy", (0.0, 0.0), options)
        .map_err(ProbeError::rejected)?;
    fig.render_raster().map_err(ProbeError::rejected)?;
    Ok(Json::Bool(true))
}
