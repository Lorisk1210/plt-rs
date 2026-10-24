//! Norm calls of `test_colors.py` the `norms` area has no op for: the
//! setters of `CenteredNorm`, `FuncNorm`, a colorizer's limits, the
//! colorbars of norms, and `ScalarMappable.to_rgba` of colour images.

use plt::axes::ImshowOptions;
use plt::colorbar::{ColorbarAxes, ColorbarOptions};
use plt::colors::{
    CenteredNorm, ColorizerOptions, FuncNorm, Norm, Normalize, ScalarMappable, colormaps,
};
use plt::figure::Figure;
use plt::image::{ImageData, ScalarField, color_image_bytes, color_image_rgba};
use plt::numeric::Grid;

use super::norms::{self, make_norm, normalized_json};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};

/// The `{"class", "params"}` norm under `key`.
fn norm_arg(args: Args<'_>, key: &str) -> Result<Box<dyn Norm>, ProbeError> {
    let spec = Args::new(args.json(key)?);
    make_norm(spec.str("class")?, spec.json("params")?)
}

/// `{vmin, vmax, halfrange, vcenter}` of a centred norm.
fn centered_state(norm: &CenteredNorm) -> Json {
    Json::Object(vec![
        ("vmin".into(), Json::from(norm.vmin())),
        ("vmax".into(), Json::from(norm.vmax())),
        ("halfrange".into(), Json::from(norm.halfrange())),
        ("vcenter".into(), Json::from(norm.vcenter())),
    ])
}

/// The state of `CenteredNorm(**params)` after each step: `["call",
/// values]` or `[attribute, value]` for `vcenter`, `halfrange`, `vmin` or
/// `vmax`.
pub(super) fn centered_steps(args: Args<'_>) -> Outcome {
    let params = Args::new(args.json("params")?);
    let mut norm = CenteredNorm::new(
        params.opt_f64("vcenter")?.unwrap_or(0.0),
        params.opt_f64("halfrange")?,
    );
    let steps = args
        .json("steps")?
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs("steps is not a list".into()))?;
    let mut states = Vec::new();
    for step in steps {
        let bad = || ProbeError::BadArgs(format!("bad step {step}"));
        let Some([name, value]) = step.as_array() else {
            return Err(bad());
        };
        match name.as_str().ok_or_else(bad)? {
            "call" => {
                let values = value.as_f64_vec().ok_or_else(bad)?;
                norm.normalize(&values).map_err(ProbeError::rejected)?;
            }
            attribute => {
                let value = value.as_f64().ok_or_else(bad)?;
                match attribute {
                    "vcenter" => norm.set_vcenter(value),
                    "halfrange" => norm.set_halfrange(Some(value)),
                    "vmin" => norm.set_vmin(Some(value)),
                    "vmax" => norm.set_vmax(Some(value)),
                    _ => return Err(bad()),
                }
            }
        }
        states.push(centered_state(&norm));
    }
    Ok(Json::Array(states))
}

/// `FuncNorm(functions, vmin=vmin, vmax=vmax)` for the named function
/// pair: `"square"` (`x**2`, `sqrt`) or `"log10"` (`log10`, `10**x`).
fn func_norm(args: Args<'_>) -> Result<FuncNorm, ProbeError> {
    let mut norm = match args.str("functions")? {
        "square" => FuncNorm::new(|x| x * x, f64::sqrt),
        "log10" => FuncNorm::new(f64::log10, |x| 10_f64.powf(x)),
        other => return Err(ProbeError::BadArgs(format!("functions {other}"))),
    };
    norm.set_vmin(Some(args.f64("vmin")?));
    norm.set_vmax(Some(args.f64("vmax")?));
    Ok(norm)
}

/// `FuncNorm(...)(values)`.
pub(super) fn func_call(args: Args<'_>) -> Outcome {
    let result = func_norm(args)?
        .normalize(&args.f64_vec("values")?)
        .map_err(ProbeError::rejected)?;
    Ok(Json::Array(
        result.into_iter().map(normalized_json).collect(),
    ))
}

/// `FuncNorm(...).inverse(values)`.
pub(super) fn func_inverse(args: Args<'_>) -> Outcome {
    let result = func_norm(args)?
        .inverse(&args.f64_vec("values")?)
        .map_err(ProbeError::rejected)?;
    Ok(Json::from(result))
}

/// `ScalarMappable().to_rgba(values, bytes=bytes)`: the `norms` area's op
/// for scalar values, the pass-through of an RGB or RGBA image (rows of
/// pixels of three or four floats) otherwise.
pub(super) fn scalar_mappable_to_rgba(args: Args<'_>) -> Outcome {
    let Some(image) = color_image(args.json("values")?)? else {
        return norms::scalar_mappable_to_rgba(args);
    };
    let pixels: Vec<Json> = if args.opt_bool("bytes")?.unwrap_or(false) {
        color_image_bytes(&image)
            .map_err(ProbeError::rejected)?
            .into_iter()
            .map(|pixel| Json::Array(pixel.map(|v| Json::Integer(v.into())).to_vec()))
            .collect()
    } else {
        color_image_rgba(&image)
            .map_err(ProbeError::rejected)?
            .into_iter()
            .map(|pixel| Json::from(pixel.to_vec()))
            .collect()
    };
    let (_, cols) = image.shape();
    Ok(Json::Array(
        pixels
            .chunks(cols.max(1))
            .map(|row| Json::Array(row.to_vec()))
            .collect(),
    ))
}

/// Rows of pixels as an image of `N`-channel pixels.
fn pixel_grid<const N: usize>(rows: &[Vec<Vec<f64>>]) -> Result<Grid<[f64; N]>, ProbeError> {
    let pixel = |p: &Vec<f64>| {
        <[f64; N]>::try_from(p.as_slice())
            .map_err(|_| ProbeError::BadArgs("pixels of mixed channel counts".into()))
    };
    let rows = rows
        .iter()
        .map(|row| row.iter().map(pixel).collect::<Result<Vec<_>, _>>())
        .collect::<Result<Vec<_>, _>>()?;
    Grid::from_rows(rows).map_err(ProbeError::rejected)
}

/// The probe's values as an RGB or RGBA image, when they are rows of
/// pixels of three or four numbers; `None` for scalar values.
fn color_image(values: &Json) -> Result<Option<ImageData>, ProbeError> {
    let rows: Option<Vec<Vec<Vec<f64>>>> = values.as_array().and_then(|rows| {
        rows.iter()
            .map(|row| row.as_array()?.iter().map(Json::as_f64_vec).collect())
            .collect()
    });
    let Some(rows) = rows else {
        return Ok(None);
    };
    match rows.first().and_then(|row| row.first()).map(Vec::len) {
        None => Ok(None),
        Some(3) => pixel_grid(&rows).map(|grid| Some(ImageData::Rgb(grid))),
        Some(4) => pixel_grid(&rows).map(|grid| Some(ImageData::Rgba(grid))),
        Some(n) => Err(ProbeError::BadArgs(format!("pixels of {n} channels"))),
    }
}

/// `{vmin, vmax, norm_vmin, norm_vmax}` of a default colorizer after
/// setting the limits given (a colorizer's limits are its norm's).
pub(super) fn colorizer_limits(args: Args<'_>) -> Outcome {
    let cmap = colormaps::get_cmap(None).map_err(ProbeError::rejected)?;
    let mut colorizer = ScalarMappable::with_norm(cmap, Box::new(Normalize::default()));
    if let Some(vmin) = args.opt_f64("vmin")? {
        colorizer.set_clim(Some(vmin), None);
    }
    if let Some(vmax) = args.opt_f64("vmax")? {
        colorizer.set_clim(None, Some(vmax));
    }
    let (vmin, vmax) = (colorizer.norm().vmin(), colorizer.norm().vmax());
    Ok(Json::Object(vec![
        ("vmin".into(), Json::from(vmin)),
        ("vmax".into(), Json::from(vmax)),
        ("norm_vmin".into(), Json::from(vmin)),
        ("norm_vmax".into(), Json::from(vmax)),
    ]))
}

/// The ticks of `ColorbarBase(Figure().add_subplot(), norm=norm)`.
pub(super) fn colorbar_ticks(args: Args<'_>) -> Outcome {
    let figure = Figure::new();
    let ax = figure.subplot();
    let cmap = colormaps::get_cmap(None).map_err(ProbeError::rejected)?;
    let mappable = ScalarMappable::with_norm(cmap, norm_arg(args, "norm")?);
    let options = ColorbarOptions {
        axes: ColorbarAxes::Cax(ax),
        ..ColorbarOptions::default()
    };
    let colorbar = figure
        .colorbar(&mappable, options)
        .map_err(ProbeError::rejected)?;
    let ticks = colorbar
        .long_axis()
        .ticklocs(false)
        .map_err(ProbeError::rejected)?;
    Ok(Json::from(ticks))
}

/// The y limits of the colorbar of `imshow(data, norm=norm)`.
pub(super) fn colorbar_ylim(args: Args<'_>) -> Outcome {
    let rows = args
        .json("data")?
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .map(Json::as_f64_vec)
                .collect::<Option<Vec<_>>>()
        })
        .ok_or_else(|| ProbeError::BadArgs("data is not a grid".into()))?;
    let cols = rows.first().map_or(0, Vec::len);
    let grid = Grid::from_vec(rows.len(), cols, rows.concat()).map_err(ProbeError::rejected)?;
    let figure = Figure::new();
    let ax = figure.subplot();
    let options = ImshowOptions {
        colorizer: ColorizerOptions {
            norm: Some(norm_arg(args, "norm")?.into()),
            ..ColorizerOptions::default()
        },
        ..ImshowOptions::default()
    };
    let image = ax
        .imshow(ScalarField::new(grid), options)
        .map_err(ProbeError::rejected)?;
    let colorbar = figure
        .colorbar(&image, ColorbarOptions::default())
        .map_err(ProbeError::rejected)?;
    let (bottom, top) = colorbar.ax().ylim();
    Ok(Json::from(vec![bottom, top]))
}
