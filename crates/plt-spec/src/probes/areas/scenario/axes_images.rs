//! Axes steps making colour-mapped artists (`imshow`, `pcolor`,
//! `pcolormesh`, `pcolorfast`, `hist2d`, `contour`, `contourf`), the
//! mappables colorbars show, and the methods changing their norm and
//! limits.

use plt::axes::{
    Axes, Hist2dOptions, ImshowOptions, Mesh, MeshCoords, PcolorOptions, PcolorfastOptions,
};
use plt::colors::{Color, Extend, LogNorm, Norm, Normalize, colormaps};
use plt::contour::{ContourNorm, ContourOptions, Levels, QuadCoords, QuadField};
use plt::image::{ColorizerOptions, Origin};
use plt::numeric::Grid;

use super::call::{Call, KEYWORD_ONLY};
use super::scene::{Handled, Object, Produced};
use crate::json::Json;
use crate::probes::ProbeError;

/// Runs a colour-mapping method of `ax`.
pub(super) fn call(ax: &Axes, call: &Call<'_>) -> Handled {
    Some(match call.method {
        "imshow" => imshow(ax, call),
        "pcolor" | "pcolormesh" => pcolor(ax, call),
        "pcolorfast" => grid(call, 0).and_then(|z| {
            ax.pcolorfast(None, z, PcolorfastOptions::default())
                .map(|_| Produced::Nothing)
                .map_err(ProbeError::rejected)
        }),
        "hist2d" => call.f64_vec(0, "x").and_then(|x| {
            ax.hist2d(&x, &call.f64_vec(1, "y")?, Hist2dOptions::default())
                .map(|_| Produced::Nothing)
                .map_err(ProbeError::rejected)
        }),
        "contour" | "contourf" => contour(ax, call),
        _ => return None,
    })
}

/// Runs a method changing the colour mapping of a mappable.
pub(super) fn mappable_call(object: &Object, call: &Call<'_>) -> Handled {
    let change = match call.method {
        "set_norm" => match norm(call.required(0, "norm")) {
            Ok(norm) => Change::Norm(norm),
            Err(err) => return Some(Err(err)),
        },
        "set_clim" => match clim(call) {
            Ok((vmin, vmax)) => Change::Clim(vmin, vmax),
            Err(err) => return Some(Err(err)),
        },
        _ => return None,
    };
    match (object, change) {
        (Object::Collection(c), Change::Norm(norm)) => {
            c.set_norm(norm);
        }
        (Object::Collection(c), Change::Clim(vmin, vmax)) => {
            c.set_clim(vmin, vmax);
        }
        (Object::Image(image), Change::Norm(norm)) => {
            image.set_norm(norm);
        }
        (Object::Image(image), Change::Clim(vmin, vmax)) => {
            image.set_clim(vmin, vmax);
        }
        (Object::Contours(set), Change::Norm(norm)) => {
            set.set_norm(norm);
        }
        (Object::Contours(set), Change::Clim(vmin, vmax)) => {
            set.set_clim(vmin, vmax);
        }
        _ => return Some(Err(call.bad("not a mappable"))),
    }
    Some(Ok(Produced::Nothing))
}

/// A change of a mappable's colour mapping.
enum Change {
    Norm(Box<dyn Norm>),
    Clim(Option<f64>, Option<f64>),
}

/// matplotlib's `set_clim(vmin, vmax)`, `set_clim((vmin, vmax))` and
/// `set_clim(vmin=..., vmax=...)`.
fn clim(call: &Call<'_>) -> Result<(Option<f64>, Option<f64>), ProbeError> {
    if let Some(Json::Array(pair)) = call.arg(0, "vmin") {
        return match pair.as_slice() {
            [lo, hi] => Ok((lo.as_f64(), hi.as_f64())),
            _ => Err(ProbeError::rejected("clim needs two values")),
        };
    }
    Ok((call.opt_f64(0, "vmin")?, call.opt_f64(1, "vmax")?))
}

/// `{"new": "LogNorm" | "Normalize", "args": [vmin, vmax]}`.
fn norm(value: Result<&Json, ProbeError>) -> Result<Box<dyn Norm>, ProbeError> {
    let value = value?;
    let name = value.get("new").and_then(Json::as_str);
    let limits = value
        .get("args")
        .map_or(Some(Vec::new()), Json::as_f64_vec)
        .ok_or_else(|| ProbeError::BadArgs(format!("bad norm {value}")))?;
    Ok(match (name, limits.as_slice()) {
        (Some("LogNorm"), []) => Box::new(LogNorm::default()),
        (Some("LogNorm"), &[vmin, vmax]) => Box::new(LogNorm::new(vmin, vmax)),
        (Some("Normalize"), &[vmin, vmax]) => Box::new(Normalize::new(vmin, vmax)),
        _ => return Err(ProbeError::BadArgs(format!("unsupported norm {value}"))),
    })
}

/// The `norm`, `vmin`, `vmax` and `cmap` keywords.
fn colorizer(call: &Call<'_>) -> Result<ColorizerOptions, ProbeError> {
    let cmap = call
        .opt_str(KEYWORD_ONLY, "cmap")?
        .map(|name| colormaps::get_cmap(Some(name)).map_err(ProbeError::rejected))
        .transpose()?;
    Ok(ColorizerOptions {
        cmap,
        norm: call.keyword("norm").map(|n| norm(Ok(n))).transpose()?,
        vmin: call.opt_f64(KEYWORD_ONLY, "vmin")?,
        vmax: call.opt_f64(KEYWORD_ONLY, "vmax")?,
        ..ColorizerOptions::default()
    })
}

/// A 2-D array argument as a grid.
fn grid(call: &Call<'_>, index: usize) -> Result<Grid<f64>, ProbeError> {
    Grid::from_rows(call.rows(index, "z")?).map_err(ProbeError::rejected)
}

/// Coordinates of a mesh: one per edge (1-D) or one per vertex (2-D).
fn coords(call: &Call<'_>, index: usize, keyword: &str) -> Result<MeshCoords, ProbeError> {
    match call.required(index, keyword)?.as_array() {
        Some([Json::Array(_), ..]) => grid(call, index).map(MeshCoords::from),
        _ => call.f64_vec(index, keyword).map(MeshCoords::from),
    }
}

fn imshow(ax: &Axes, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let origin = match call.opt_str(KEYWORD_ONLY, "origin")? {
        None => None,
        Some("upper") => Some(Origin::Upper),
        Some("lower") => Some(Origin::Lower),
        Some(other) => return Err(ProbeError::rejected(format!("bad origin {other:?}"))),
    };
    let options = ImshowOptions {
        origin,
        colors: colorizer(call)?,
        ..ImshowOptions::default()
    };
    ax.imshow(grid(call, 0)?, options)
        .map(|image| Produced::Object(Object::Image(image)))
        .map_err(ProbeError::rejected)
}

/// matplotlib's `pcolor`/`pcolormesh([x, y,] c, norm=..., ...)`.
fn pcolor(ax: &Axes, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let mesh = match call.arity() {
        1 => Mesh::new(grid(call, 0)?),
        3 => Mesh::with_coords(coords(call, 0, "x")?, coords(call, 1, "y")?, grid(call, 2)?),
        _ => return Err(call.bad("unsupported mesh arguments")),
    };
    let options = PcolorOptions {
        colors: colorizer(call)?,
        ..PcolorOptions::default()
    };
    let made = if call.method == "pcolor" {
        ax.pcolor(mesh, options)
    } else {
        ax.pcolormesh(mesh, options)
    };
    made.map(|c| Produced::Object(Object::Collection(c)))
        .map_err(ProbeError::rejected)
}

/// matplotlib's `contour`/`contourf([x, y,] z, [levels], levels=...,
/// colors=..., extend=...)`, `x` and `y` one per column and row or one
/// per point.
fn contour(ax: &Axes, call: &Call<'_>) -> Result<Produced, ProbeError> {
    let (field, levels_at) = match call.arity() {
        1 | 2 => (QuadField::new(grid(call, 0)?), 1),
        3 | 4 => {
            let z = grid(call, 2)?;
            let coords = match (coords(call, 0, "x")?, coords(call, 1, "y")?) {
                (MeshCoords::Grid(x), MeshCoords::Grid(y)) => QuadCoords::Mesh { x, y },
                (MeshCoords::Vector(x), MeshCoords::Vector(y)) => QuadCoords::Axes { x, y },
                _ => return Err(call.bad("x and y must have the same shape")),
            };
            (QuadField::new(z).with_coords(coords), 3)
        }
        _ => return Err(call.bad("unsupported contour arguments")),
    };
    let mut options = ContourOptions::default();
    if let Some(levels) = call.arg(levels_at, "levels") {
        options.levels = match levels {
            Json::Array(_) => Levels::Values(call.f64_vec(levels_at, "levels")?),
            count => Levels::Count(count.as_i64().ok_or_else(|| call.bad("bad levels"))?),
        };
    }
    if let Some(colors) = call.keyword("colors") {
        let colors = colors
            .as_array()
            .and_then(|items| items.iter().map(|c| c.as_str().map(Color::from)).collect())
            .ok_or_else(|| call.bad("colors are names"))?;
        options.colors = Some(colors);
    }
    options.extend = match call.opt_str(KEYWORD_ONLY, "extend")? {
        None | Some("neither") => Extend::Neither,
        Some("both") => Extend::Both,
        Some("min") => Extend::Min,
        Some("max") => Extend::Max,
        Some(other) => return Err(ProbeError::rejected(format!("bad extend {other:?}"))),
    };
    if let Some(value) = call.keyword("norm") {
        options.norm = ContourNorm::Custom(norm(Ok(value))?);
    }
    let made = if call.method == "contourf" {
        ax.contourf(&field, options)
    } else {
        ax.contour(&field, options)
    };
    made.map(|set| Produced::Object(Object::Contours(set)))
        .map_err(ProbeError::rejected)
}
