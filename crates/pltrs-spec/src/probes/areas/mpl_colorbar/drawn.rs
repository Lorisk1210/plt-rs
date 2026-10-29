//! What a colorbar draws: the extension patches of an inverted colorbar,
//! the outline, the dividers, and the blocks of colorbars without a
//! mappable.

use pltrs::artist::ArtistHandle;
use pltrs::axes::{ImshowOptions, Mesh, PcolorOptions};
use pltrs::colorbar::{ColorbarAxes, ColorbarOptions};
use pltrs::colors::{
    Color, Colorize, ColorizerOptions, LogNorm, Norm, Normalize, PowerNorm, ScalarMappable,
    colormaps, from_levels_and_colors,
};
use pltrs::figure::Figure;
use pltrs::numeric::{Grid, linspace};
use pltrs::orientation::Orientation;
use pltrs_core::Rgba;

use super::super::mpl_tests::in_test_style;
use super::{bump, extend, orientation, rgba};
use crate::json::Json;
use crate::probes::encode::{numbers, object};
use crate::probes::{Args, Outcome, ProbeError};

/// `test_colorbar_extension_inverted_axis`: the face colours of the
/// extension patches of a colorbar of `imshow(arange(12))` in viridis with
/// black under and white over colours, once its long axis is inverted.
pub(super) fn extend_patches(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let cmap = colormaps::get("viridis")
            .and_then(|cmap| cmap.with_under(Rgba::BLACK))
            .and_then(|cmap| cmap.with_over(Rgba::WHITE))
            .map_err(ProbeError::rejected)?;
        let fig = Figure::new();
        let ax = fig.subplot();
        let options = ImshowOptions {
            colorizer: ColorizerOptions::default().cmap(cmap),
            ..ImshowOptions::default()
        };
        let image = ax
            .imshow(Grid::from_fn(3, 4, |r, c| (r * 4 + c) as f64), options)
            .map_err(ProbeError::rejected)?;
        let options = ColorbarOptions {
            orientation: Some(orientation(args)?),
            extend: Some(extend(args)?),
            ..ColorbarOptions::default()
        };
        let colorbar = fig
            .colorbar(&image, options)
            .map_err(ProbeError::rejected)?;
        let axis = colorbar.long_axis();
        axis.set_inverted(!axis.inverted());
        colorbar
            .extend_patches()
            .iter()
            .map(|patch| patch.colors().map(|colors| rgba(colors.face)))
            .collect::<Result<_, _>>()
            .map(Json::Array)
            .map_err(ProbeError::rejected)
    })
}

/// `test_colorbar_scale_reset`: the edge colour of a colorbar's outline
/// set to red, after the mesh took a `LogNorm` and then a `Normalize`.
pub(super) fn outline(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let fig = Figure::new();
        let ax = fig.subplot();
        let cmap = colormaps::get("RdBu_r").map_err(ProbeError::rejected)?;
        let options = PcolorOptions {
            colorizer: ColorizerOptions::default().cmap(cmap),
            ..PcolorOptions::default()
        };
        let mesh = ax
            .pcolormesh(Mesh::new(bump()), options)
            .map_err(ProbeError::rejected)?;
        mesh.set_rasterized(true);
        let colorbar = fig
            .colorbar(&mesh, ColorbarOptions::beside(&ax))
            .map_err(ProbeError::rejected)?;
        colorbar.outline().set_edgecolor("red");
        mesh.set_norm(Box::new(LogNorm::new(1.0, 100.0)));
        mesh.set_norm(Box::new(Normalize::new(-20.0, 20.0)));
        let colors = colorbar.outline().colors().map_err(ProbeError::rejected)?;
        Ok(rgba(colors.edge))
    })
}

/// `test_colorbar_extend_drawedges`: the dividers of a colorbar of
/// `from_levels_and_colors(arange(5), colors, extend)` with `drawedges`,
/// once its view is narrowed to 1.1 .. 2.9.
pub(super) fn dividers(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let orientation = orientation(args)?;
        let offset = args.i64("coloroffset")?;
        let count = usize::try_from(5 + offset)
            .map_err(|_| ProbeError::BadArgs(format!("bad coloroffset {offset}")))?;
        let viridis = colormaps::get("viridis").map_err(ProbeError::rejected)?;
        // `cmap(np.linspace(100, 255, n).astype(int))` indexes the table.
        let colors: Vec<Color> = linspace(100.0, 255.0, count)
            .into_iter()
            .map(|index| Color::from(viridis.lut()[index as usize]))
            .collect();
        let bounds: Vec<f64> = (0..5).map(f64::from).collect();
        let (cmap, norm) = from_levels_and_colors(&bounds, &colors, extend(args)?)
            .map_err(ProbeError::rejected)?;
        let fig = Figure::new();
        let ax = fig.subplot();
        let options = ColorbarOptions {
            axes: ColorbarAxes::Cax(ax.clone()),
            orientation: Some(orientation),
            drawedges: true,
            ..ColorbarOptions::default()
        };
        let mappable = ScalarMappable::with_norm(cmap, Box::new(norm));
        let colorbar = fig
            .colorbar(&mappable, options)
            .map_err(ProbeError::rejected)?;
        match orientation {
            Orientation::Horizontal => ax.set_xlim(1.1, 2.9).map_err(ProbeError::rejected)?,
            Orientation::Vertical => ax.set_ylim(1.1, 2.9).map_err(ProbeError::rejected)?,
        };
        Ok(Json::Array(
            colorbar
                .divider_segments()
                .iter()
                .map(|segment| {
                    Json::Array(
                        segment
                            .vertices()
                            .iter()
                            .map(|point| numbers([point.x, point.y]))
                            .collect(),
                    )
                })
                .collect(),
        ))
    })
}

/// `test_colorbarbase`, `test_colorbar_lognorm_extension` and
/// `test_colorbar_powernorm_extension`: the blocks and limits of
/// `Colorbar(ax, cmap=..., norm=..., extend=...)`, which has no mappable.
pub(super) fn without_mappable(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let cmap = colormaps::get_cmap(args.opt_str("cmap")?).map_err(ProbeError::rejected)?;
        let mappable = match args.get("norm") {
            None => ScalarMappable::new(cmap),
            Some(spec) => ScalarMappable::with_norm(cmap, norm(spec)?),
        };
        let fig = Figure::new();
        let ax = fig.subplot();
        let options = ColorbarOptions {
            axes: ColorbarAxes::Cax(ax.clone()),
            orientation: Some(Orientation::Vertical),
            extend: args.opt_str("extend")?.map(|_| extend(args)).transpose()?,
            ..ColorbarOptions::default()
        };
        let colorbar = fig
            .colorbar(&mappable, options)
            .map_err(ProbeError::rejected)?;
        let (bottom, top) = ax.ylim();
        Ok(object([
            ("values", numbers(colorbar.values())),
            ("boundaries", numbers(colorbar.boundaries())),
            ("ylim", numbers([bottom, top])),
        ]))
    })
}

/// `[name, *args]`: `LogNorm(vmin, vmax)` or `PowerNorm(gamma, vmin,
/// vmax)`.
fn norm(spec: &Json) -> Result<Box<dyn Norm>, ProbeError> {
    let bad = || ProbeError::BadArgs(format!("bad norm {spec}"));
    let items = spec.as_array().ok_or_else(bad)?;
    let (name, numbers) = items.split_first().ok_or_else(bad)?;
    let numbers: Vec<f64> = numbers.iter().filter_map(Json::as_f64).collect();
    Ok(match (name.as_str(), numbers.as_slice()) {
        (Some("LogNorm"), &[vmin, vmax]) => Box::new(LogNorm::new(vmin, vmax)),
        (Some("PowerNorm"), &[gamma, vmin, vmax]) => {
            Box::new(PowerNorm::new(gamma).with_vmin(vmin).with_vmax(vmax))
        }
        _ => return Err(bad()),
    })
}
