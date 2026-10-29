//! Probe operations for the `cursor` area (the status bar message: the
//! coordinates and the data under the pointer, and the per-variate
//! readout of multivariate data).

use pltrs::artist::ArtistHandle;
use pltrs::axes::{Axes, ImshowOptions, Mesh, NonUniformOptions, PcolorOptions, PcolorfastOptions};
use pltrs::colors::{BoundaryNorm, Colorize, ColorizerOptions, Extend, LogNorm, Norm, colormaps};
use pltrs::figure::Figure;
use pltrs::image::{Extent, ImageData, MultiField, Origin, ScalarField};
use pltrs::navigation::Navigation;
use pltrs::numeric::Grid;
use pltrs_core::Point;

use super::norms::multi::mappable;
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("cursor.message", message),
    Op::new("cursor.format_cursor_data", format_cursor_data),
];

const X_EDGES: [f64; 9] = [0.0, 1.0, 3.0, 4.0, 5.0, 7.0, 8.0, 9.0, 12.0];
const Y_EDGES: [f64; 7] = [0.0, 2.0, 3.0, 5.0, 6.0, 8.0, 9.0];

/// A `rows × columns` grid of `cell(row, column)`.
fn sized<T>(rows: u32, columns: u32, cell: impl Fn(f64, f64) -> T) -> Grid<T> {
    let rows: Vec<Vec<T>> = (0..rows)
        .map(|r| {
            (0..columns)
                .map(|c| cell(f64::from(r), f64::from(c)))
                .collect()
        })
        .collect();
    Grid::from_rows(rows).expect("rows of equal length")
}

/// The scenes' 6 × 8 grid of `cell(row, column)`.
fn grid<T>(cell: impl Fn(f64, f64) -> T) -> Grid<T> {
    sized(6, 8, cell)
}

/// `sin(0.7 r) cos(0.4 c) + 0.1 r`.
fn field(r: f64, c: f64) -> f64 {
    (0.7 * r).sin() * (0.4 * c).cos() + 0.1 * r
}

/// `FIELD` with NaN where `(r + c) % 5 == 0`.
fn masked_field(r: f64, c: f64) -> f64 {
    if (r + c) % 5.0 == 0.0 {
        f64::NAN
    } else {
        field(r, c)
    }
}

/// `value` as numpy's `astype(uint8)` converts it, wrapping modulo 256.
fn wrapping_byte(value: f64) -> u8 {
    (value as u32 % 256) as u8
}

fn imshow(ax: &Axes, data: impl Into<ImageData>, options: ImshowOptions) -> Result<(), ProbeError> {
    ax.imshow(data, options).map_err(ProbeError::rejected)?;
    Ok(())
}

fn normed(norm: impl Norm + 'static) -> ImshowOptions {
    ImshowOptions {
        colorizer: ColorizerOptions {
            norm: Some(norm.into()),
            ..ColorizerOptions::default()
        },
        ..ImshowOptions::default()
    }
}

fn extent(left: f64, right: f64, bottom: f64, top: f64) -> Option<Extent> {
    Some(Extent::new(left, right, bottom, top))
}

/// The data and options of the scenes made of one `imshow`.
fn single_image(scene: &str) -> Result<Option<(ImageData, ImshowOptions)>, ProbeError> {
    let plain = ImshowOptions::default;
    Ok(Some(match scene {
        "scalar" => (grid(field).into(), plain()),
        "lognorm" => (
            grid(|r, c| 10f64.powf((8.0 * r + c) / 12.0 - 1.0)).into(),
            normed(LogNorm::default()),
        ),
        "boundary" => {
            let bounds = vec![-1.0, -0.5, 0.0, 0.25, 0.5, 1.0, 1.5];
            let norm =
                BoundaryNorm::new(bounds, 256, Extend::Neither).map_err(ProbeError::rejected)?;
            (grid(field).into(), normed(norm))
        }
        "integers" => {
            let values = grid(|r, c| 7 * (8 * r as i64 + c as i64) - 100);
            (ScalarField::integers(&values).into(), plain())
        }
        "masked" => (grid(masked_field).into(), plain()),
        "rgb" => {
            let rgb = |r: f64, c: f64| {
                [
                    (0.13 * r + 0.07 * c) % 1.0,
                    (0.29 * r + 0.3) % 1.0,
                    (0.21 * c + 0.6) % 1.0,
                ]
            };
            (grid(rgb).into(), plain())
        }
        "rgba_bytes" => {
            let rgba = |r: f64, c: f64| {
                [
                    37.0 * r + 11.0 * c,
                    255.0 - 23.0 * c,
                    40.0 * r,
                    255.0 - 30.0 * r,
                ]
                .map(wrapping_byte)
            };
            (grid(rgba).into(), plain())
        }
        "lower" => (
            grid(field).into(),
            ImshowOptions {
                origin: Some(Origin::Lower),
                extent: extent(0.0, 4.0, 0.0, 3.0),
                ..plain()
            },
        ),
        "singular" => (Grid::filled(3, 4, 2.5).into(), plain()),
        _ => return Ok(None),
    }))
}

/// Builds `scene` in `ax`.
fn build(ax: &Axes, scene: &str) -> Result<(), ProbeError> {
    if let Some((data, options)) = single_image(scene)? {
        return imshow(ax, data, options);
    }
    let plain = ImshowOptions::default;
    match scene {
        "overlap" => {
            imshow(ax, grid(field), plain())?;
            let gray = colormaps::get("gray").map_err(ProbeError::rejected)?;
            let options = ImshowOptions {
                colorizer: ColorizerOptions::default().cmap(gray),
                extent: extent(1.0, 4.0, 3.0, 1.0),
                ..plain()
            };
            imshow(ax, sized(3, 4, field), options)?;
            ax.plot([0.0, 7.0], [0.0, 5.0])
                .map_err(ProbeError::rejected)?;
        }
        "hidden" => {
            imshow(ax, grid(field), plain())?;
            let options = ImshowOptions {
                extent: extent(2.0, 6.0, 4.0, 1.0),
                ..plain()
            };
            ax.imshow(Grid::filled(2, 2, 9.0), options)
                .map_err(ProbeError::rejected)?
                .set_visible(false);
        }
        "nomouseover" => {
            ax.imshow(grid(field), plain())
                .map_err(ProbeError::rejected)?
                .set_mouseover(false);
        }
        "mesh" => {
            let mesh = Mesh::from_coords(X_EDGES.to_vec(), Y_EDGES.to_vec(), grid(field));
            ax.pcolormesh(mesh, PcolorOptions::default())
                .map_err(ProbeError::rejected)?
                .set_mouseover(true);
        }
        "pcolorfast" => {
            let coords = (X_EDGES.to_vec().into(), Y_EDGES.to_vec().into());
            ax.pcolorfast(Some(coords), grid(field), PcolorfastOptions::default())
                .map_err(ProbeError::rejected)?;
        }
        "nonuniform" => {
            let (x, y) = (&X_EDGES[..8], &Y_EDGES[..6]);
            ax.nonuniform_image(x, y, grid(field), NonUniformOptions::default())
                .map_err(ProbeError::rejected)?;
            ax.set_xlim(-1.0, 10.0)
                .and_then(|ax| ax.set_ylim(-1.0, 9.0))
                .map_err(ProbeError::rejected)?;
        }
        _ => return Err(ProbeError::BadArgs(format!("unknown scene {scene:?}"))),
    }
    Ok(())
}

fn message(args: Args<'_>) -> Outcome {
    let fig = Figure::with_size(4.0, 3.0, 100.0);
    build(&fig.subplot(), args.str("scene")?)?;
    fig.layout();
    let pixels = args
        .json("pixels")?
        .as_array()
        .and_then(|pixels| {
            pixels
                .iter()
                .map(|pixel| match pixel.as_array()? {
                    [x, y] => Some(Point::new(x.as_f64()?, y.as_f64()?)),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
        })
        .ok_or_else(|| ProbeError::BadArgs("pixels is malformed".into()))?;
    let navigation = Navigation::new();
    Ok(Json::Array(
        pixels
            .into_iter()
            .map(|at| Json::String(navigation.readout(&fig, at)))
            .collect(),
    ))
}

/// The variate of the multivariate readout named `name`.
fn variate(name: &str) -> Result<Grid<f64>, ProbeError> {
    Ok(match name {
        "field" => grid(field),
        "log" => grid(|r, c| 10f64.powf((8.0 * r + c) / 12.0 - 1.0)),
        "masked" => grid(masked_field),
        "ramp" => grid(|r, c| (r + 2.0 * c) / 20.0),
        other => return Err(ProbeError::BadArgs(format!("unknown variate {other:?}"))),
    })
}

/// The readout of every element of a multivariate mesh, row by row.
fn format_cursor_data(args: Args<'_>) -> Outcome {
    let names = args
        .json("variates")?
        .as_array()
        .and_then(|names| names.iter().map(Json::as_str).collect::<Option<Vec<_>>>())
        .ok_or_else(|| ProbeError::BadArgs("variates must be names".into()))?;
    let variates = names
        .into_iter()
        .map(variate)
        .collect::<Result<Vec<_>, _>>()?;
    let data = MultiField::new(variates).map_err(ProbeError::rejected)?;
    let mappable = mappable(args, &data.masked_values())?;
    let (rows, cols) = data.shape();
    let readouts = (0..rows)
        .flat_map(|r| (0..cols).map(move |c| (r, c)))
        .filter_map(|cell| data.at(cell))
        .map(|values| Json::String(mappable.format_cursor_data(&values)))
        .collect();
    Ok(Json::Array(readouts))
}
