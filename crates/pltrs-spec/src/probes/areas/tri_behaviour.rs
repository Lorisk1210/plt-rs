//! Probe operations for the `tri_behaviour` area: `tripcolor` values and
//! limits, Delaunay triangulations far from the origin and `triplot`
//! legend entries, ported from matplotlib's `test_triangulation.py` (see
//! `spec/reference/probes/test_tri_behaviour.py`). Every op runs under the
//! reference configuration.

use pltrs::axes::{TriShading, TriValues, Tripcolor, TripcolorOptions};
use pltrs::figure::Figure;
use pltrs::legend::LegendHandle;
use pltrs::lines::LineProps;
use pltrs::tri::Triangulation;

use super::text_metrics::with_rc;
use crate::json::Json;
use crate::probes::encode::{object, rgba};
use crate::probes::{Args, Op, Outcome, ProbeError, made};
use crate::rng::SpecRng;
use pltrs::colors::ColorizerOptions;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("tri.tripcolor", |args| with_rc(args, || tripcolor(args))),
    Op::new("tri.delaunay_offset", |args| {
        with_rc(args, || delaunay_offset(args))
    }),
    Op::new("tri.triplot_legend", |args| with_rc(args, triplot_legend)),
];

fn points(name: &str) -> Result<(Vec<f64>, Vec<f64>), ProbeError> {
    match name {
        "diamond" => Ok((vec![-1.0, 0.0, 1.0, 0.0], vec![0.0, -1.0, 0.0, 1.0])),
        "random" => {
            let mut generator = SpecRng::new(5);
            Ok((
                generator.uniform(10, 0.0, 1.0),
                generator.uniform(10, 0.0, 1.0),
            ))
        }
        other => Err(ProbeError::BadArgs(format!("unknown points {other:?}"))),
    }
}

fn limits(clim: (Option<f64>, Option<f64>)) -> Json {
    Json::from(vec![clim.0, clim.1])
}

/// A `tripcolor` plot: its shading, norm limits and flat face colours.
fn tripcolor(args: Args<'_>) -> Outcome {
    let (x, y) = points(args.str("points")?)?;
    let triangulation = made(Triangulation::delaunay(&x, &y))?;
    let values = match (args.get("c"), args.get("facecolors")) {
        (_, Some(_)) => TriValues::Faces(args.f64_vec("facecolors")?),
        (Some(_), None) => TriValues::Auto(args.f64_vec("c")?),
        (None, None) => return Err(ProbeError::BadArgs("no values".into())),
    };
    let shading = match args.str("shading")? {
        "flat" => TriShading::Flat,
        "gouraud" => TriShading::Gouraud,
        other => return Err(ProbeError::BadArgs(format!("unknown shading {other:?}"))),
    };
    let (vmin, vmax) = match args.get("clim").map(|_| args.f64_vec("clim")).transpose()? {
        Some(clim) => (clim.first().copied(), clim.get(1).copied()),
        None => (None, None),
    };
    let options = TripcolorOptions {
        colorizer: ColorizerOptions {
            vmin,
            vmax,
            ..ColorizerOptions::default()
        },
        shading,
        ..TripcolorOptions::default()
    };
    let ax = Figure::new().subplot();
    Ok(match made(ax.tripcolor(&triangulation, values, options))? {
        Tripcolor::Flat(collection) => {
            let faces = made(collection.colors())?.faces;
            object([
                ("kind", "flat".into()),
                ("clim", limits(collection.clim())),
                ("faces", Json::Array(faces.into_iter().map(rgba).collect())),
            ])
        }
        Tripcolor::Gouraud(mesh) => object([
            ("kind", "gouraud".into()),
            ("clim", limits(mesh.clim())),
            ("faces", Json::Null),
        ]),
    })
}

/// The number of Delaunay triangles of the shifted unit square.
fn delaunay_offset(args: Args<'_>) -> Outcome {
    let offset = args.f64("offset")?;
    let x = [0.0, 1.0, 0.0, 1.0, 0.5].map(|v| v + offset);
    let y = [0.0, 0.0, 1.0, 1.0, 0.5].map(|v| v + offset);
    let triangulation = made(Triangulation::delaunay(&x, &y))?;
    Ok(Json::Integer(
        i64::try_from(triangulation.triangles().len()).expect("few triangles"),
    ))
}

/// The legend entries a labelled `triplot` leaves.
fn triplot_legend() -> Outcome {
    let ax = Figure::new().subplot();
    let triangulation = made(Triangulation::new(
        &[0.0, 2.0, 1.0],
        &[0.0, 0.0, 1.0],
        vec![[0, 1, 2]],
    ))?;
    let lines = made(ax.triplot(&triangulation, "", LineProps::default().label("label")))?;
    let (handles, labels) = ax.legend_handles_labels();
    let same = handles.first() == Some(&LegendHandle::from(&lines.edges));
    Ok(object([
        ("labels", Json::from(labels)),
        (
            "handles",
            Json::Integer(i64::try_from(handles.len()).expect("few handles")),
        ),
        ("same_artist", same.into()),
    ]))
}
