//! Images and pseudocolour meshes: colour limits and argument checks.

use plt::axes::{ImshowOptions, Mesh, MeshCoords, PcolorOptions, Shading};
use plt::colors::Normalize;
use plt::image::ColorizerOptions;
use plt::numeric::Grid;

use super::{count, datasets, made, on_axes};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// A grid of rows of numbers.
fn grid(value: &Json) -> Result<Grid<f64>, ProbeError> {
    made(Grid::from_rows(datasets(value)?))
}

/// Coordinates given as a list (1-D) or a list of rows (2-D).
fn coords(value: &Json) -> Result<MeshCoords, ProbeError> {
    match value.as_f64_vec() {
        Some(vector) => Ok(MeshCoords::Vector(vector)),
        None => Ok(MeshCoords::Grid(grid(value)?)),
    }
}

/// `[x, y,] C` as a mesh.
fn mesh(args: Args<'_>) -> Result<Mesh, ProbeError> {
    let values = grid(args.json("C")?)?;
    match (args.get("x"), args.get("y")) {
        (Some(x), Some(y)) => Ok(Mesh::from_coords(coords(x)?, coords(y)?, values)),
        _ => Ok(Mesh::new(values)),
    }
}

/// `shading=...`, absent for the rcParam.
fn shading(args: Args<'_>) -> Result<Option<Shading>, ProbeError> {
    args.opt_str("shading")?
        .map(|name| name.parse().map_err(ProbeError::rejected))
        .transpose()
}

pub(super) fn imshow(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let [low, high] = args.f64_vec("norm")?[..] else {
            return Err(ProbeError::BadArgs("norm is not [vmin, vmax]".into()));
        };
        let colors = ColorizerOptions::default()
            .norm(Normalize::new(low, high))
            .limits(args.opt_f64("vmin")?, args.opt_f64("vmax")?);
        let options = ImshowOptions {
            colors,
            ..ImshowOptions::default()
        };
        let image = made(ax.imshow(grid(args.json("data")?)?, options))?;
        let (vmin, vmax) = image.clim();
        Ok(Json::from(vec![Json::from(vmin), Json::from(vmax)]))
    })
}

pub(super) fn pcolor(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let cells = made(ax.pcolor(mesh(args)?, PcolorOptions::default()))?;
        Ok(encode::object([
            ("datalim", encode::bbox(&ax.data_lim().bbox)),
            (
                "count",
                count(cells.array().map_or(0, |values| values.len())),
            ),
        ]))
    })
}

pub(super) fn pcolormesh(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let options = PcolorOptions {
            shading: shading(args)?,
            ..PcolorOptions::default()
        };
        made(ax.pcolormesh(mesh(args)?, options))?;
        Ok(encode::bbox(&ax.data_lim().bbox))
    })
}
