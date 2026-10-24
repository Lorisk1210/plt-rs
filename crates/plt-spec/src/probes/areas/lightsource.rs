//! Probe operations for the `lightsource` area (`matplotlib.colors.LightSource`).

use plt::colors::{
    BlendMode, LightSource, Relief, ShadeOptions, blend_overlay, blend_soft_light, colormaps,
};
use plt::numeric::Grid;

use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("lightsource.direction", |args| {
        Ok(Json::from(light(args)?.direction().to_vec()))
    }),
    Op::new("lightsource.hillshade", hillshade),
    Op::new("lightsource.shade_normals", shade_normals),
    Op::new("lightsource.blend", blend),
    Op::new("lightsource.shade_rgb", shade_rgb),
    Op::new("lightsource.shade", shade),
];

fn bad(what: &str) -> ProbeError {
    ProbeError::BadArgs(format!("{what} is malformed"))
}

/// The light described by the `light` argument.
fn light(args: Args<'_>) -> Result<LightSource, ProbeError> {
    let light = Args::new(args.json("light")?);
    let mut source = LightSource::new(light.f64("azdeg")?, light.f64("altdeg")?);
    for (key, field) in [
        ("hsv_min_val", &mut source.hsv_min_val),
        ("hsv_max_val", &mut source.hsv_max_val),
        ("hsv_min_sat", &mut source.hsv_min_sat),
        ("hsv_max_sat", &mut source.hsv_max_sat),
    ] {
        if let Some(value) = light.opt_f64(key)? {
            *field = value;
        }
    }
    Ok(source)
}

/// The `relief` argument.
fn relief(args: Args<'_>) -> Result<Relief, ProbeError> {
    let relief = Args::new(args.json("relief")?);
    Ok(Relief {
        vert_exag: relief.f64("vert_exag")?,
        dx: relief.f64("dx")?,
        dy: relief.f64("dy")?,
        fraction: relief.f64("fraction")?,
    })
}

/// A 2-D array of numbers.
fn grid(value: &Json, what: &str) -> Result<Grid<f64>, ProbeError> {
    let rows = value
        .as_array()
        .ok_or_else(|| bad(what))?
        .iter()
        .map(|row| row.as_f64_vec().ok_or_else(|| bad(what)))
        .collect::<Result<Vec<_>, _>>()?;
    Grid::from_rows(rows).map_err(|_| bad(what))
}

/// An `(M, N, 3)` array.
fn triples(value: &Json, what: &str) -> Result<Grid<[f64; 3]>, ProbeError> {
    let rows = value
        .as_array()
        .ok_or_else(|| bad(what))?
        .iter()
        .map(|row| {
            row.as_array()
                .ok_or_else(|| bad(what))?
                .iter()
                .map(|cell| match cell.as_f64_vec().as_deref() {
                    Some(&[a, b, c]) => Ok([a, b, c]),
                    _ => Err(bad(what)),
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    Grid::from_rows(rows).map_err(|_| bad(what))
}

fn blend_mode(name: &str) -> Result<BlendMode, ProbeError> {
    match name {
        "hsv" => Ok(BlendMode::Hsv),
        "soft" => Ok(BlendMode::SoftLight),
        "overlay" => Ok(BlendMode::Overlay),
        _ => Err(bad("blend mode")),
    }
}

/// A grid as nested lists, each cell turned into JSON by `cell`.
fn rows_json<T>(grid: &Grid<T>, cell: impl Fn(&T) -> Json) -> Json {
    Json::Array(
        grid.iter_rows()
            .map(|row| Json::Array(row.iter().map(&cell).collect()))
            .collect(),
    )
}

pub(crate) fn hillshade(args: Args<'_>) -> Outcome {
    let elevation = grid(args.json("elevation")?, "elevation")?;
    let shade = light(args)?
        .hillshade(&elevation, &relief(args)?)
        .map_err(ProbeError::rejected)?;
    Ok(rows_json(&shade, |&v| Json::from(v)))
}

fn shade_normals(args: Args<'_>) -> Outcome {
    let normals = triples(args.json("normals")?, "normals")?;
    let shade = light(args)?.shade_normals(&normals, args.f64("fraction")?);
    Ok(rows_json(&shade, |&v| Json::from(v)))
}

fn blend(args: Args<'_>) -> Outcome {
    let source = light(args)?;
    let rgb = triples(args.json("rgb")?, "rgb")?;
    let intensity = grid(args.json("intensity")?, "intensity")?;
    let mode = args.str("mode")?;
    let blended = rgb
        .as_slice()
        .iter()
        .zip(intensity.as_slice())
        .map(|(&color, &light)| match mode {
            "hsv" => source.blend_hsv(color, light).map_err(ProbeError::rejected),
            "soft" => Ok(blend_soft_light(color, light)),
            "overlay" => Ok(blend_overlay(color, light)),
            _ => Err(bad("mode")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let blended = Grid::from_vec(rgb.rows(), rgb.cols(), blended).map_err(|_| bad("rgb"))?;
    Ok(rows_json(&blended, |c| Json::from(c.to_vec())))
}

fn shade_rgb(args: Args<'_>) -> Outcome {
    let rgb = triples(args.json("rgb")?, "rgb")?;
    let elevation = grid(args.json("elevation")?, "elevation")?;
    let shaded = light(args)?
        .shade_rgb(
            &rgb,
            &elevation,
            &blend_mode(args.str("blend_mode")?)?,
            &relief(args)?,
        )
        .map_err(ProbeError::rejected)?;
    Ok(rows_json(&shaded, |c| Json::from(c.to_vec())))
}

pub(crate) fn shade(args: Args<'_>) -> Outcome {
    let data = grid(args.json("data")?, "data")?;
    let cmap = colormaps::get(args.str("cmap")?).map_err(ProbeError::rejected)?;
    let options = ShadeOptions {
        blend: blend_mode(args.str("blend_mode")?)?,
        vmin: args.opt_f64("vmin")?,
        vmax: args.opt_f64("vmax")?,
        relief: relief(args)?,
        ..ShadeOptions::default()
    };
    let shaded = light(args)?
        .shade(&data, &cmap, &options)
        .map_err(ProbeError::rejected)?;
    Ok(rows_json(&shaded, |c| Json::from(vec![c.r, c.g, c.b, c.a])))
}
