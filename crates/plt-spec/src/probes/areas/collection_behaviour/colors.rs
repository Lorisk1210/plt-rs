//! The `collection.colors` op: face and edge colours after defaults,
//! colour mapping and alpha.

use plt::artist::{ArtistHandle, BaseProps};
use plt::axes::{Axes, MappedColors, Mesh, PcolorOptions, ScatterColors, ScatterOptions};
use plt::collections::{Collection, CollectionProps, CollectionSpec, EdgeColors, HatchColors};
use plt::colors::{Alpha, Color, Normalize, colormaps};
use plt::figure::Figure;
use plt::image::ColorizerOptions;
use plt::rc;
use plt_core::Point;

use super::{grid, made};
use crate::json::Json;
use crate::probes::encode::{object, rgba};
use crate::probes::{Args, Outcome, ProbeError};

/// `x = arange(5)`.
const X5: [f64; 5] = [0.0, 1.0, 2.0, 3.0, 4.0];

/// `x / 5`.
fn fifths() -> Vec<f64> {
    X5.iter().map(|x| x / 5.0).collect()
}

fn colors_of(collection: &Collection) -> Result<Json, ProbeError> {
    let colors = made(collection.colors())?;
    let list = |colors: &[plt_core::Rgba]| Json::Array(colors.iter().map(|&c| rgba(c)).collect());
    Ok(object([
        ("faces", list(&colors.faces)),
        ("edges", list(&colors.edges)),
    ]))
}

fn scatter(ax: &Axes, colors: ScatterColors, alpha: Alpha) -> Result<Collection, ProbeError> {
    let options = ScatterOptions {
        colors,
        alpha,
        ..ScatterOptions::default()
    };
    made(ax.scatter_with(X5.to_vec(), X5.to_vec(), options))
}

fn mapped() -> ScatterColors {
    MappedColors::new(X5.to_vec()).into()
}

fn five_colors() -> ScatterColors {
    ScatterColors::PerPoint(["r", "g", "b", "c", "m"].map(Color::from).to_vec())
}

fn alpha_mapped(ax: &Axes) -> Result<Vec<Json>, ProbeError> {
    let first = scatter(ax, mapped(), Alpha::Each(fifths()))?;
    let second = scatter(ax, mapped(), Alpha::Keep)?;
    second.set_alphas(fifths());
    Ok(vec![colors_of(&first)?, colors_of(&second)?])
}

fn alpha_colors(ax: &Axes, set_later: bool) -> Result<Vec<Json>, ProbeError> {
    let alpha = || {
        if set_later {
            Alpha::Keep
        } else {
            Alpha::Each(fifths())
        }
    };
    let first = scatter(ax, five_colors(), alpha())?;
    let second = scatter(ax, ScatterColors::Single("r".into()), alpha())?;
    if set_later {
        first.set_alphas(fifths());
        second.set_alphas(fifths());
    }
    Ok(vec![colors_of(&first)?, colors_of(&second)?])
}

fn line_collection(ax: &Axes) -> Result<Vec<Json>, ProbeError> {
    let props = CollectionProps {
        base: BaseProps {
            zorder: Some(3.0),
            ..BaseProps::default()
        },
        linewidths: Some(vec![2.2]),
        edgecolors: Some(EdgeColors::Colors(vec!["r".into()])),
        facecolors: Some(vec![(0.0, 1.0, 0.0, 1.0).into()]),
        ..CollectionProps::default()
    };
    let styled = ax.add_collection(CollectionSpec::lines(Vec::new()).with_props(props));
    let plain = ax.add_collection(CollectionSpec::lines(Vec::new()));
    Ok(vec![colors_of(&styled)?, colors_of(&plain)?])
}

fn mesh_clim(ax: &Axes) -> Result<Vec<Json>, ProbeError> {
    let options = PcolorOptions {
        colors: ColorizerOptions {
            cmap: Some(made(colormaps::get("plasma"))?),
            norm: Some(Box::new(Normalize::new(0.0, 1.0))),
            ..ColorizerOptions::default()
        },
        ..PcolorOptions::default()
    };
    let mesh = made(ax.pcolormesh(Mesh::new(grid(1, 1, |_, _| 1.0)), options))?;
    let before = colors_of(&mesh)?;
    mesh.set_clim(Some(1.0), Some(2.0));
    Ok(vec![before, colors_of(&mesh)?])
}

fn mesh_alpha(ax: &Axes) -> Result<Vec<Json>, ProbeError> {
    let x = vec![0.0, 1.0, 2.0, 3.0];
    let alphas: Vec<f64> = (0..9).map(|v| f64::from(v) / 8.0).collect();
    let made_mesh = || {
        made(ax.pcolormesh(
            Mesh::with_coords(x.clone(), x.clone(), grid(3, 3, |r, c| 3.0 * r + c)),
            PcolorOptions::default(),
        ))
    };
    let (first, second) = (made_mesh()?, made_mesh()?);
    first.set_alphas(alphas.clone());
    second.set_alphas(alphas);
    Ok(vec![colors_of(&first)?, colors_of(&second)?])
}

fn pcolor_edges(ax: &Axes) -> Result<Vec<Json>, ProbeError> {
    let z = || Mesh::new(grid(3, 4, |r, c| 4.0 * r + c));
    let first = made(
        ax.pcolor(
            z(),
            PcolorOptions {
                props: CollectionProps::default()
                    .edgecolor("red")
                    .facecolor("none"),
                ..PcolorOptions::default()
            },
        ),
    )?;
    let recoloured = || -> Result<Collection, ProbeError> {
        let mesh = made(ax.pcolor(z(), PcolorOptions::default()))?;
        mesh.set_facecolor("none").set_edgecolor("red");
        Ok(mesh)
    };
    let second = recoloured()?;
    let third = recoloured()?;
    third.set_alpha(Some(0.5));
    Ok(vec![
        colors_of(&first)?,
        colors_of(&second)?,
        colors_of(&third)?,
    ])
}

/// A colour list, or `"edge"`.
fn hatch_colors(value: &Json) -> Result<HatchColors, ProbeError> {
    match value {
        Json::String(name) if name == "edge" => Ok(HatchColors::Edge),
        other => Ok(HatchColors::Colors(color_list(other)?)),
    }
}

fn color_list(value: &Json) -> Result<Vec<Color>, ProbeError> {
    value
        .as_array()
        .and_then(|items| items.iter().map(|c| c.as_str().map(Color::from)).collect())
        .ok_or_else(|| ProbeError::BadArgs(format!("not a list of colours: {value}")))
}

/// The hatch colours of a hatched square collection made under some
/// rcParams and changed by setters.
pub(super) fn hatchcolors(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    rc::context(|| {
        for (key, value) in args.json("rcparams")?.as_object().unwrap_or(&[]) {
            made(rc::set(key, value.as_str().unwrap_or_default()))?;
        }
        let mut props = CollectionProps::default().hatch("//");
        for (key, value) in args.json("kw")?.as_object().unwrap_or(&[]) {
            match key.as_str() {
                "edgecolor" => props.edgecolors = Some(EdgeColors::Colors(color_list(value)?)),
                "hatchcolor" => props.hatchcolors = Some(hatch_colors(value)?),
                other => return Err(ProbeError::BadArgs(format!("unknown keyword {other}"))),
            }
        }
        let square = vec![vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(0.0, 1.0),
        ]];
        let collection = ax.add_collection(CollectionSpec::polygons(square).with_props(props));
        for step in args.json("steps")?.as_array().unwrap_or(&[]) {
            match step.as_array().map(<[Json]>::as_ref) {
                Some([Json::String(name), value]) if name == "set_edgecolor" => {
                    collection.set_edgecolors(EdgeColors::Colors(color_list(value)?));
                }
                Some([Json::String(name), value]) if name == "set_hatchcolor" => {
                    collection.set_hatchcolors(hatch_colors(value)?);
                }
                _ => return Err(ProbeError::BadArgs(format!("bad step {step}"))),
            }
        }
        let hatches = made(collection.colors())?.hatches;
        Ok(Json::Array(hatches.into_iter().map(rgba).collect()))
    })
}

pub(super) fn colors(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let ax = fig.subplot();
    let colors = match args.str("scene")? {
        "alpha_mapped" => alpha_mapped(&ax)?,
        "alpha_colors" => alpha_colors(&ax, false)?,
        "alpha_colors_set" => alpha_colors(&ax, true)?,
        "line_collection" => line_collection(&ax)?,
        "mesh_clim" => mesh_clim(&ax)?,
        "mesh_alpha" => mesh_alpha(&ax)?,
        "pcolor_edges" => pcolor_edges(&ax)?,
        other => return Err(ProbeError::BadArgs(format!("unknown scene {other:?}"))),
    };
    Ok(Json::Array(colors))
}
