//! The data limits of one artist placed through a transform
//! (`TestTransformPlotInterface` and the `*_pre_transform_limits` tests):
//! only the directions in which the transform ends in the axes' data
//! coordinates count, mapped back into them.

use plt::axes::{Axes, Mesh, PcolorOptions};
use plt::collections::CollectionProps;
use plt::contour::{ContourOptions, QuadCoords, QuadField};
use plt::figure::Figure;
use plt::lines::LineProps;
use plt::numeric::{Grid, linspace, meshgrid};
use plt::patches::{PatchProps, PathShape};
use plt::transforms::Transform;
use plt_core::Path;

use super::NonAffine;
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// The transform a tree describes on `ax`: `"data"`, `"axes"`,
/// `["affine", ops]`, `["nonaffine", ["affine", ops]]`, `["+", a, b]` or
/// `["blend", x, y]`.
fn tree(ax: &Axes, spec: &Json) -> Result<Transform, ProbeError> {
    let bad = || ProbeError::BadArgs(format!("not a transform tree: {spec}"));
    match spec.as_str() {
        Some("data") => return Ok(ax.trans_data()),
        Some("axes") => return Ok(ax.trans_axes()),
        Some(_) => return Err(bad()),
        None => {}
    }
    match spec.as_array().ok_or_else(bad)? {
        [kind, ops] if kind.as_str() == Some("affine") => {
            Ok(Transform::Affine(encode::parse_affine_ops(ops)?))
        }
        [kind, inner] if kind.as_str() == Some("nonaffine") => match tree(ax, inner)? {
            Transform::Affine(affine) => Ok(Transform::custom(NonAffine(affine))),
            _ => Err(bad()),
        },
        [kind, a, b] => {
            let (a, b) = (tree(ax, a)?, tree(ax, b)?);
            match kind.as_str() {
                Some("+") => Ok(a.then(b)),
                Some("blend") => Ok(Transform::blended(a, b)),
                _ => Err(bad()),
            }
        }
        _ => Err(bad()),
    }
}

/// Adds an artist with `add(ax, transform)` to a fresh axes (polar with
/// `projection: "polar"`) and returns its data limits.
fn data_lim(
    args: Args<'_>,
    add: impl FnOnce(&Axes, Transform) -> Result<(), ProbeError>,
) -> Outcome {
    let figure = Figure::new();
    let ax = match args.opt_str("projection")? {
        None => figure.subplot(),
        Some("polar") => figure.polar_subplot().axes().clone(),
        Some(other) => return Err(ProbeError::BadArgs(format!("projection {other}"))),
    };
    add(&ax, tree(&ax, args.json("transform")?)?)?;
    Ok(encode::bbox(&ax.data_lim().bbox))
}

/// `plot(x, y, transform=...)`, or `plot(y, transform=...)` without `x`.
pub(super) fn plot(args: Args<'_>) -> Outcome {
    let y = args.f64_vec("y")?;
    let x = match args.get("x") {
        Some(_) => args.f64_vec("x")?,
        None => (0..y.len()).map(|i| i as f64).collect(),
    };
    data_lim(args, |ax, transform| {
        ax.plot_with(x, y, LineProps::default().transform(transform))
            .map(drop)
            .map_err(ProbeError::rejected)
    })
}

/// `add_patch(PathPatch(Path(vertices), transform=...))`.
pub(super) fn add_patch(args: Args<'_>) -> Outcome {
    let path = Path::polyline(encode::parse_points(args.json("vertices")?)?);
    data_lim(args, |ax, transform| {
        ax.add_patch_with(PathShape(path), PatchProps::default().transform(transform));
        Ok(())
    })
}

/// The tests' grid: `xs, ys = np.meshgrid(np.linspace(15, 20, 15),
/// np.linspace(12.4, 12.5, 20))` and `z = np.log(xs * ys)`.
fn grid() -> (Grid<f64>, Grid<f64>, Grid<f64>) {
    let (xs, ys) = meshgrid(&linspace(15.0, 20.0, 15), &linspace(12.4, 12.5, 20));
    let z = Grid::from_fn(20, 15, |r, c| (xs[(r, c)] * ys[(r, c)]).ln());
    (xs, ys, z)
}

/// `contourf(xs, ys, z, transform=...)`.
pub(super) fn contourf(args: Args<'_>) -> Outcome {
    let (x, y, z) = grid();
    data_lim(args, |ax, transform| {
        let field = QuadField::new(z).with_coords(QuadCoords::Mesh { x, y });
        let options = ContourOptions {
            transform: Some(transform),
            ..ContourOptions::default()
        };
        ax.contourf(&field, options)
            .map(drop)
            .map_err(ProbeError::rejected)
    })
}

/// `xs, ys, z[:-1, :-1]` as a pseudocolour mesh.
fn mesh() -> Mesh {
    let (x, y, z) = grid();
    Mesh::with_coords(x, y, Grid::from_fn(19, 14, |r, c| z[(r, c)]))
}

/// The pcolor options placing the mesh through `transform`.
fn placed(transform: Transform) -> PcolorOptions {
    PcolorOptions {
        props: CollectionProps::default().transform(transform),
        ..PcolorOptions::default()
    }
}

/// `pcolor(xs, ys, z[:-1, :-1], transform=...)`.
pub(super) fn pcolor(args: Args<'_>) -> Outcome {
    data_lim(args, |ax, transform| {
        ax.pcolor(mesh(), placed(transform))
            .map(drop)
            .map_err(ProbeError::rejected)
    })
}

/// `pcolormesh(xs, ys, z[:-1, :-1], transform=...)`.
pub(super) fn pcolormesh(args: Args<'_>) -> Outcome {
    data_lim(args, |ax, transform| {
        ax.pcolormesh(mesh(), placed(transform))
            .map(drop)
            .map_err(ProbeError::rejected)
    })
}
