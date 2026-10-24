//! Norms named by their scale, with limits, in every plot mapping values
//! to colours.

use plt::axes::{
    Axes, HexbinOptions, ImshowOptions, MappedColors, Mesh, PcolorOptions, ScatterOptions,
};
use plt::colors::{Colorize, ColorizerOptions, Norm, NormName, Normalized};
use plt::numeric::Grid;

use super::{made, on_axes};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// The norm of what `plot` makes of the four `values` with `colorizer`.
fn mapped_norm(
    ax: &Axes,
    plot: &str,
    values: &[f64],
    colorizer: ColorizerOptions,
) -> Result<Box<dyn Norm>, ProbeError> {
    let grid = || made(Grid::from_vec(2, 2, values.to_vec()));
    let norm = match plot {
        "hexbin" => {
            let options = HexbinOptions {
                values: Some(values.to_vec()),
                colorizer,
                ..HexbinOptions::default()
            };
            made(ax.hexbin(&[0.0, 1.0, 0.0, 1.0], &[0.0, 0.0, 1.0, 1.0], options))?.norm()
        }
        "scatter" => {
            let options = ScatterOptions {
                colors: MappedColors::new(values).colorizer(colorizer).into(),
                ..ScatterOptions::default()
            };
            let points = vec![0.0, 1.0, 2.0, 3.0];
            made(ax.scatter_with(points.clone(), points, options))?.norm()
        }
        "imshow" => {
            let options = ImshowOptions {
                colorizer,
                ..ImshowOptions::default()
            };
            Some(made(ax.imshow(grid()?, options))?.norm())
        }
        "pcolormesh" => {
            let options = PcolorOptions {
                colorizer,
                ..PcolorOptions::default()
            };
            made(ax.pcolormesh(Mesh::new(grid()?), options))?.norm()
        }
        other => return Err(ProbeError::BadArgs(format!("unknown plot {other:?}"))),
    };
    norm.ok_or_else(|| ProbeError::BadArgs(format!("{plot} maps no values")))
}

pub(super) fn norm_by_name(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let name: NormName = made(args.str("norm")?.parse())?;
        let colorizer = ColorizerOptions::default()
            .norm(name)
            .limits(args.opt_f64("vmin")?, args.opt_f64("vmax")?);
        let values = args.f64_vec("values")?;
        let mut norm = mapped_norm(ax, args.str("plot")?, &values, colorizer)?;
        let normalized = made(norm.normalize(&args.f64_vec("samples")?))?
            .into_iter()
            .map(|value| match value {
                Normalized::Fraction(fraction) => Ok(Json::from(fraction)),
                Normalized::Masked => Ok(Json::Null),
                Normalized::Index(_) => Err(ProbeError::BadArgs(
                    "a norm made from a scale indexes no colours".to_owned(),
                )),
            })
            .collect::<Result<_, _>>()?;
        Ok(encode::object([
            ("vmin", norm.vmin().map_or(Json::Null, Json::from)),
            ("vmax", norm.vmax().map_or(Json::Null, Json::from)),
            ("normalized", Json::Array(normalized)),
        ]))
    })
}
