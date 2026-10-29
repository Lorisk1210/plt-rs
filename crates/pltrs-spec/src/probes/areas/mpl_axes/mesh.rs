//! Images and pseudocolour meshes: colour limits, argument checks and
//! the artist `pcolorfast` picks.

use pltrs::axes::{
    Axes, ImshowOptions, MappedColors, Mesh, MeshCoords, PcolorFast, PcolorOptions,
    PcolorfastOptions, ScatterOptions, Shading,
};
use pltrs::colors::{Colorize, ColorizerOptions, Normalize};
use pltrs::image::ImageData;
use pltrs::numeric::Grid;

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
            colorizer: colors,
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

/// `C`: rows of numbers, or rows of RGB triples.
fn fast_values(value: &Json) -> Result<ImageData, ProbeError> {
    if let Ok(rows) = datasets(value) {
        return Ok(made(Grid::from_rows(rows))?.into());
    }
    let rows = value
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs(format!("bad C {value}")))?
        .iter()
        .map(|row| {
            datasets(row)?
                .into_iter()
                .map(|rgb| match rgb[..] {
                    [r, g, b] => Ok([r, g, b]),
                    _ => Err(ProbeError::BadArgs(format!("not RGB: {rgb:?}"))),
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(made(Grid::from_rows(rows))?.into())
}

pub(super) fn pcolorfast(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let xy = match (args.get("x"), args.get("y")) {
            (Some(x), Some(y)) => Some((coords(x)?, coords(y)?)),
            _ => None,
        };
        let drawn = made(ax.pcolorfast(
            xy,
            fast_values(args.json("C")?)?,
            PcolorfastOptions::default(),
        ))?;
        Ok(Json::from(match drawn {
            PcolorFast::Image(_) => "AxesImage",
            PcolorFast::PcolorImage(_) => "PcolorImage",
            PcolorFast::QuadMesh(_) => "QuadMesh",
        }))
    })
}

/// `clim=(7, 8)`.
fn clim_colors() -> ColorizerOptions {
    ColorizerOptions::default().limits(Some(7.0), Some(8.0))
}

/// The colour limits `method` sets from `clim=(7, 8)`.
fn clim_of(ax: &Axes, method: &str) -> Result<(Option<f64>, Option<f64>), ProbeError> {
    let data = || made(Grid::from_rows(vec![vec![0.0, 1.0], vec![2.0, 3.0]]));
    let pcolor_options = || PcolorOptions {
        colorizer: clim_colors(),
        ..PcolorOptions::default()
    };
    Ok(match method {
        "scatter" => {
            let colors = MappedColors::new(vec![0.0, 1.0, 2.0]).limits(Some(7.0), Some(8.0));
            let options = ScatterOptions {
                colors: colors.into(),
                ..ScatterOptions::default()
            };
            made(ax.scatter_with(vec![0.0, 1.0, 2.0], vec![0.0, 1.0, 2.0], options))?.clim()
        }
        "imshow" => {
            let options = ImshowOptions {
                colorizer: clim_colors(),
                ..ImshowOptions::default()
            };
            made(ax.imshow(data()?, options))?.clim()
        }
        "pcolor" => made(ax.pcolor(Mesh::new(data()?), pcolor_options()))?.clim(),
        "pcolormesh" => made(ax.pcolormesh(Mesh::new(data()?), pcolor_options()))?.clim(),
        "pcolorfast" => {
            let options = PcolorfastOptions {
                colorizer: clim_colors(),
                ..PcolorfastOptions::default()
            };
            match made(ax.pcolorfast(None, data()?, options))? {
                PcolorFast::Image(image) => image.clim(),
                PcolorFast::QuadMesh(mesh) => mesh.clim(),
                PcolorFast::PcolorImage(_) => {
                    return Err(ProbeError::BadArgs("unexpected PcolorImage".into()));
                }
            }
        }
        other => return Err(ProbeError::BadArgs(format!("unknown method {other}"))),
    })
}

pub(super) fn clim(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let (vmin, vmax) = clim_of(ax, args.str("method")?)?;
        Ok(Json::Array(vec![Json::from(vmin), Json::from(vmax)]))
    })
}
