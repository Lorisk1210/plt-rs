//! Probe operations for the `collection_behaviour` area: data limits,
//! resolved colours, hit tests, legend elements and cursor data of
//! collections, ported from matplotlib's `test_collections.py` (see
//! `spec/reference/probes/test_collection_behaviour.py`). Every op runs
//! under the reference configuration.

mod colors;
mod legend;

use pltrs::axes::{Axes, Mesh, MeshCoords, PcolorOptions, ScatterOptions};
use pltrs::collections::{Collection, CollectionSpec};
use pltrs::figure::Figure;
use pltrs::navigation::Navigation;
use pltrs::numeric::Grid;
use pltrs::transforms::Transform;
use pltrs_core::Point;

use super::text_metrics::with_rc;
use crate::json::Json;
use crate::probes::encode::{bbox, object, parse_points};
use crate::probes::{Args, Op, Outcome, ProbeError, made};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("collection.limits", |args| with_rc(args, || limits(args))),
    Op::new("collection.colors", |args| {
        with_rc(args, || colors::colors(args))
    }),
    Op::new("collection.contains", |args| {
        with_rc(args, || contains(args))
    }),
    Op::new("collection.legend_elements", |args| {
        with_rc(args, || legend::legend_elements(args))
    }),
    Op::new("collection.cursor", |args| with_rc(args, || cursor(args))),
    Op::new("collection.hatchcolors", |args| {
        with_rc(args, || colors::hatchcolors(args))
    }),
];

/// A grid of `cell(row, column)`.
fn grid(rows: usize, columns: usize, cell: impl Fn(f64, f64) -> f64) -> Grid<f64> {
    Grid::from_fn(rows, columns, |r, c| cell(r as f64, c as f64))
}

fn scatter(ax: &Axes, x: &[f64], y: &[f64]) -> Result<Collection, ProbeError> {
    made(ax.scatter(x.to_vec(), y.to_vec()))
}

fn blended(ax: &Axes) -> Result<(), ProbeError> {
    made(ax.plot([2.0, 3.0, 4.0], [0.4, 0.6, 0.5]))?;
    let lines = (1..=5)
        .map(|x| {
            let x = f64::from(x);
            vec![Point::new(x, 0.0), Point::new(x, 1.0)]
        })
        .collect();
    let transform = Transform::blended(ax.trans_data(), ax.trans_axes());
    ax.add_collection(CollectionSpec::lines(lines).with_transform(transform));
    Ok(())
}

fn zeros(ax: &Axes, transform: Transform) -> Result<(), ProbeError> {
    let options = ScatterOptions {
        offset_transform: Some(transform),
        ..ScatterOptions::default()
    };
    made(ax.scatter_with(vec![0.0], vec![0.0], options))?;
    scatter(ax, &[3.0], &[3.0])?;
    Ok(())
}

/// `ax.dataLim` and the view limits of a scene.
fn limits(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    match args.str("scene")? {
        "scatter" => {
            scatter(&ax, &[0.0, 1.0], &[0.0, 1.0])?;
        }
        "empty_scatter" => {
            scatter(&ax, &[0.0, 1.0], &[0.0, 1.0])?;
            scatter(&ax, &[], &[])?;
        }
        "blended" => blended(&ax)?,
        "singleton" => {
            scatter(&ax, &[0.0], &[0.0])?;
        }
        "zeros_data" => zeros(&ax, ax.trans_data())?,
        "zeros_axes" => zeros(&ax, ax.trans_axes())?,
        "paths_no_offsets" => {
            let p = Point::new;
            ax.add_collection(CollectionSpec::polygons(vec![
                vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 2.0)],
                vec![p(-1.0, -1.0), p(0.0, -3.0), p(0.5, -1.0)],
            ]));
        }
        other => return Err(ProbeError::BadArgs(format!("unknown scene {other:?}"))),
    }
    let (x0, x1) = ax.xlim();
    let (y0, y1) = ax.ylim();
    Ok(object([
        ("datalim", bbox(&ax.data_lim().bbox)),
        ("xlim", Json::from(vec![x0, x1])),
        ("ylim", Json::from(vec![y0, y1])),
    ]))
}

/// `ax.pcolormesh(X)` with `X = x[:, None] * x[None, :]`, `x = arange(4)`.
fn mesh(ax: &Axes) -> Result<Collection, ProbeError> {
    made(ax.pcolormesh(
        Mesh::new(grid(4, 4, |r, c| r * c)),
        PcolorOptions::default(),
    ))
}

/// A V-shaped cell.
fn mesh_concave(ax: &Axes) -> Result<Collection, ProbeError> {
    let x = Grid::from_rows(vec![vec![0.0, -1.0], vec![1.0, 0.0]]).map_err(ProbeError::rejected)?;
    let y = Grid::from_rows(vec![vec![0.0, 1.0], vec![1.0, -1.0]]).map_err(ProbeError::rejected)?;
    made(ax.pcolormesh(
        Mesh::from_coords(
            MeshCoords::Grid(x),
            MeshCoords::Grid(y),
            grid(1, 1, |_, _| 0.0),
        ),
        PcolorOptions::default(),
    ))
}

/// `coll.contains(event)` at each data point.
fn contains(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let collection = match args.str("scene")? {
        "scatter" => made(ax.scatter_with(
            vec![0.0],
            vec![0.0],
            ScatterOptions {
                sizes: Some(vec![1000.0]),
                ..ScatterOptions::default()
            },
        ))?,
        "mesh" => mesh(&ax)?,
        "mesh_concave" => mesh_concave(&ax)?,
        other => return Err(ProbeError::BadArgs(format!("unknown scene {other:?}"))),
    };
    fig.layout();
    let data = made(fig.resolve_transform(&ax.trans_data()))?;
    let hits = parse_points(args.json("points")?)?
        .into_iter()
        .map(|p| {
            pltrs::artist::ArtistHandle::contains(&collection, data.transform_point(p)).map_or(
                Json::Null,
                |indices| {
                    Json::Array(
                        indices
                            .into_iter()
                            .map(|i| Json::Integer(i64::try_from(i).expect("indices fit in i64")))
                            .collect(),
                    )
                },
            )
        })
        .collect();
    Ok(Json::Array(hits))
}

/// The status bar message over quads that all cover the same square.
fn cursor(args: Args<'_>) -> Outcome {
    let pixel = args.f64_vec("pixel")?;
    let [x, y] = pixel[..] else {
        return Err(ProbeError::BadArgs(format!("bad pixel {pixel:?}")));
    };
    let fig = Figure::new();
    let ax = fig.subplot();
    made(ax.pcolormesh(
        Mesh::from_coords(
            vec![1.0, 2.0, 1.0, 2.0],
            vec![1.0, 2.0, 1.0, 2.0],
            grid(3, 3, |_, _| 1.0),
        ),
        PcolorOptions::default(),
    ))?;
    fig.layout();
    Ok(Navigation::new().readout(&fig, Point::new(x, y)).into())
}
