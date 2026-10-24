//! Probe operations for the `colors` area.
//!
//! Colour arguments follow the reference module: JSON arrays are Python
//! tuples, so `[r, g, b(, a)]` is a colour and `[colour, alpha]` a
//! `(colour, alpha)` pair. The helpers here are shared with the colormap
//! and norm areas.

use plt::colors::{
    Alpha, Color, ColorTable, Colors, Extend, NamedColor, ScalarMappable, color_sequence,
    color_sequence_names, from_levels_and_colors, hsv_to_rgb, is_cycle_reference,
    named_color_table, rgb_to_hsv, same_color, to_hex, to_rgba_array,
};
use plt_core::Rgba;

use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("colors.to_rgba", to_rgba),
    Op::new("colors.to_rgba_array", to_rgba_array_op),
    Op::new("colors.to_rgb", to_rgb),
    Op::new("colors.to_hex", to_hex_op),
    Op::new("colors.is_color_like", is_color_like),
    Op::new("colors.same_color", same_color_op),
    Op::new("colors.rgb_to_hsv", rgb_to_hsv_op),
    Op::new("colors.hsv_to_rgb", hsv_to_rgb_op),
    Op::new("colors.BASE_COLORS", base_colors),
    Op::new("colors.TABLEAU_COLORS", |_| {
        Ok(hex_table(ColorTable::Tableau))
    }),
    Op::new("colors.CSS4_COLORS", |_| Ok(hex_table(ColorTable::Css4))),
    Op::new("colors.XKCD_COLORS", |_| Ok(hex_table(ColorTable::Xkcd))),
    Op::new("colors.get_named_colors_mapping", named_colors_mapping),
    Op::new("colors.color_sequences", color_sequences),
    Op::new("colors.color_sequence", color_sequence_op),
    Op::new("colors.from_levels_and_colors", from_levels_and_colors_op),
];

/// A Python colour argument: a colour, or a `(colour, alpha)` tuple.
#[derive(Clone, Debug)]
pub(crate) enum Spec {
    Color(Color),
    Pair(Box<Spec>, Option<f64>),
}

impl Spec {
    /// Decodes a JSON colour argument; anything else is not a colour.
    pub(crate) fn parse(value: &Json) -> Result<Self, ProbeError> {
        let not_a_color = || ProbeError::rejected(format!("{value} is not a colour"));
        match value {
            Json::String(text) => Ok(Self::Color(Color::from(text.as_str()))),
            Json::Array(items) => match items.as_slice() {
                [color, alpha] => {
                    let alpha = match alpha {
                        Json::Null => None,
                        other => Some(other.as_f64().ok_or_else(not_a_color)?),
                    };
                    Ok(Self::Pair(Box::new(Self::parse(color)?), alpha))
                }
                _ => match value.as_f64_vec().as_deref() {
                    Some(&[r, g, b]) => Ok(Self::Color(Color::from([r, g, b]))),
                    Some(&[r, g, b, a]) => Ok(Self::Color(Color::from([r, g, b, a]))),
                    _ => Err(not_a_color()),
                },
            },
            _ => Err(not_a_color()),
        }
    }

    /// matplotlib's `to_rgba(spec, alpha)`: a pair's alpha applies unless
    /// `alpha` is given; pairs do not nest.
    pub(crate) fn resolve(&self, alpha: Option<f64>) -> Result<Rgba, ProbeError> {
        let (color, alpha) = self.unpair(alpha)?;
        color.resolve_alpha(alpha).map_err(ProbeError::rejected)
    }

    /// The colour and effective alpha after `to_rgba`'s pair unpacking.
    fn unpair(&self, alpha: Option<f64>) -> Result<(&Color, Option<f64>), ProbeError> {
        match self {
            Self::Color(color) => Ok((color, alpha)),
            Self::Pair(inner, own) => match inner.as_ref() {
                Self::Color(color) => Ok((color, alpha.or(*own))),
                Self::Pair(..) => Err(ProbeError::rejected(
                    "RGBA sequence should have length 3 or 4",
                )),
            },
        }
    }

    /// The spec as a list element: pairs resolve to their colour.
    fn into_list_item(self) -> Result<Color, ProbeError> {
        match self {
            Self::Color(color) => Ok(color),
            pair @ Self::Pair(..) => pair.resolve(None).map(Color::from),
        }
    }
}

/// The colour argument `key`; `null` is Python's `None`, which is not a
/// colour.
fn color_value<'a>(args: Args<'a>, key: &str) -> &'a Json {
    args.get(key).unwrap_or(&Json::Null)
}

/// Resolves a JSON colour argument to RGBA.
pub(crate) fn rgba_arg(value: &Json) -> Result<Rgba, ProbeError> {
    Spec::parse(value)?.resolve(None)
}

/// `[r, g, b, a]`.
pub(crate) fn rgba_json(rgba: Rgba) -> Json {
    Json::from(rgba.to_array().to_vec())
}

/// `[[r, g, b, a], ...]`.
pub(crate) fn rgba_rows(rows: &[Rgba]) -> Json {
    Json::Array(rows.iter().map(|c| rgba_json(*c)).collect())
}

/// An alpha argument: absent, a number or a list of numbers.
pub(crate) fn alpha_arg(value: Option<&Json>) -> Result<Alpha, ProbeError> {
    match value {
        None => Ok(Alpha::Keep),
        Some(Json::Array(_)) => value
            .and_then(Json::as_f64_vec)
            .map(Alpha::Each)
            .ok_or_else(|| ProbeError::BadArgs("alpha list must hold numbers".into())),
        Some(other) => other
            .as_f64()
            .map(Alpha::Uniform)
            .ok_or_else(|| ProbeError::BadArgs(format!("bad alpha {other}"))),
    }
}

/// A Python list of colours (`[spec, ...]`), each element a colour.
pub(crate) fn color_list(items: &[Json]) -> Result<Colors, ProbeError> {
    items
        .iter()
        .map(|item| Spec::parse(item)?.into_list_item())
        .collect::<Result<Vec<_>, _>>()
        .map(Colors::Many)
}

pub(crate) fn to_rgba(args: Args<'_>) -> Outcome {
    let spec = Spec::parse(color_value(args, "c"))?;
    spec.resolve(args.opt_f64("alpha")?).map(rgba_json)
}

pub(crate) fn to_rgb(args: Args<'_>) -> Outcome {
    let c = rgba_arg(color_value(args, "c"))?;
    Ok(Json::from(vec![c.r, c.g, c.b]))
}

pub(crate) fn to_hex_op(args: Args<'_>) -> Outcome {
    let rgba = rgba_arg(color_value(args, "c"))?;
    let keep_alpha = args.opt_bool("keep_alpha")?.unwrap_or(false);
    to_hex(rgba, keep_alpha)
        .map(Json::from)
        .map_err(ProbeError::rejected)
}

pub(crate) fn is_color_like(args: Args<'_>) -> Outcome {
    let value = color_value(args, "c");
    let cycle = value.as_str().is_some_and(is_cycle_reference);
    let resolves = Spec::parse(value)
        .and_then(|spec| spec.resolve(None))
        .is_ok();
    Ok(Json::from(cycle || resolves))
}

/// Builds the `to_rgba_array` argument the reference module passes:
/// strings as-is, `ndarray` rows, tuples (`tuple: true`) or lists.
fn colors_arg(value: &Json, tuple: bool, alpha: &Alpha) -> Result<(Colors, Alpha), ProbeError> {
    let keep = |colors| Ok((colors, alpha.clone()));
    let Some(items) = value.as_array() else {
        return keep(Colors::One(Spec::parse(value)?.into_list_item()?));
    };
    if let Some(&[r, g, b]) = value.as_f64_vec().as_deref() {
        return keep(Colors::One(Color::from([r, g, b])));
    }
    if let Some(&[r, g, b, a]) = value.as_f64_vec().as_deref() {
        return keep(Colors::One(Color::from([r, g, b, a])));
    }
    match (tuple, items) {
        // A top-level `(colour, alpha)` tuple whose alpha is a number (or
        // None) is unpacked; the explicit alpha argument wins.
        (true, [color, own]) if own.is_null() || own.as_f64().is_some() => {
            let own = own.as_f64().map_or(Alpha::Keep, Alpha::Uniform);
            let alpha = if *alpha == Alpha::Keep {
                own
            } else {
                alpha.clone()
            };
            let spec = Spec::parse(color)?;
            let (color, inner_alpha) = spec.unpair(None)?;
            let alpha = match (alpha, inner_alpha) {
                (Alpha::Keep, Some(a)) => Alpha::Uniform(a),
                (alpha, _) => alpha,
            };
            Ok((Colors::One(color.clone()), alpha))
        }
        // `to_rgba((c0, c1), alpha)` treats the tuple as a pair and keeps c0.
        (true, [color, _]) if *alpha != Alpha::Keep => {
            keep(Colors::One(Spec::parse(color)?.into_list_item()?))
        }
        _ => keep(color_list(items)?),
    }
}

pub(crate) fn to_rgba_array_op(args: Args<'_>) -> Outcome {
    let alpha = alpha_arg(args.get("alpha"))?;
    let value = args.json("c")?;
    let (colors, alpha) = if args.opt_bool("ndarray")?.unwrap_or(false) {
        (ndarray_rows(value)?, alpha)
    } else {
        colors_arg(value, args.opt_bool("tuple")?.unwrap_or(false), &alpha)?
    };
    to_rgba_array(colors, alpha)
        .map(|rows| rgba_rows(&rows))
        .map_err(ProbeError::rejected)
}

/// An `(n, 3|4)` float array, as a list of numeric colours.
fn ndarray_rows(value: &Json) -> Result<Colors, ProbeError> {
    let bad = || ProbeError::BadArgs(format!("{value} is not an (n, 3|4) array"));
    let rows = value.as_array().ok_or_else(bad)?;
    rows.iter()
        .map(|row| match row.as_f64_vec().as_deref() {
            Some(&[r, g, b]) => Ok(Color::from([r, g, b])),
            Some(&[r, g, b, a]) => Ok(Color::from([r, g, b, a])),
            _ => Err(bad()),
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Colors::Many)
}

pub(crate) fn same_color_op(args: Args<'_>) -> Outcome {
    let side = |key| colors_arg(args.json(key)?, false, &Alpha::Keep).map(|(colors, _)| colors);
    same_color(side("c1")?, side("c2")?)
        .map(Json::from)
        .map_err(ProbeError::rejected)
}

/// Applies a `[x, y, z]` conversion over a nested array of triples.
fn map_triples(
    value: &Json,
    convert: &dyn Fn([f64; 3]) -> Result<[f64; 3], ProbeError>,
) -> Outcome {
    match value.as_f64_vec().as_deref() {
        Some(&[a, b, c]) => convert([a, b, c]).map(|out| Json::from(out.to_vec())),
        Some(_) => Err(ProbeError::rejected("last dimension must be 3")),
        None => value
            .as_array()
            .ok_or_else(|| ProbeError::BadArgs(format!("{value} is not an array")))?
            .iter()
            .map(|item| map_triples(item, convert))
            .collect::<Result<Vec<_>, _>>()
            .map(Json::Array),
    }
}

pub(crate) fn rgb_to_hsv_op(args: Args<'_>) -> Outcome {
    map_triples(args.json("arr")?, &|rgb| {
        rgb_to_hsv(rgb).map_err(ProbeError::rejected)
    })
}

pub(crate) fn hsv_to_rgb_op(args: Args<'_>) -> Outcome {
    map_triples(args.json("hsv")?, &|hsv| Ok(hsv_to_rgb(hsv)))
}

fn base_colors(_: Args<'_>) -> Outcome {
    Ok(Json::Object(
        ColorTable::Base
            .entries()
            .into_iter()
            .map(|(name, c)| (name.to_owned(), Json::from(vec![c.r, c.g, c.b])))
            .collect(),
    ))
}

fn hex_table(table: ColorTable) -> Json {
    Json::Object(
        table
            .hex_entries()
            .iter()
            .map(|&(name, hex)| (name.to_owned(), Json::from(hex)))
            .collect(),
    )
}

fn named_colors_mapping(_: Args<'_>) -> Outcome {
    Ok(Json::Object(
        named_color_table()
            .map(|(name, color)| {
                let value = match color {
                    NamedColor::Rgb(rgb) => Json::from(rgb.to_vec()),
                    NamedColor::Hex(hex) => Json::from(hex),
                };
                (name.to_owned(), value)
            })
            .collect(),
    ))
}

pub(crate) fn color_sequences(_: Args<'_>) -> Outcome {
    Ok(Json::from(color_sequence_names()))
}

pub(crate) fn color_sequence_op(args: Args<'_>) -> Outcome {
    let name = args.str("name")?;
    color_sequence(name)
        .map(|colors| rgba_rows(&colors))
        .ok_or_else(|| ProbeError::rejected(format!("unknown sequence {name:?}")))
}

/// `cmap(norm(values))` for the colormap and norm of
/// `from_levels_and_colors`, with the colormap's size, ends and extension.
pub(crate) fn from_levels_and_colors_op(args: Args<'_>) -> Outcome {
    let colors = args
        .json("colors")?
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs("colors is malformed".into()))?
        .iter()
        .map(|c| c.as_str().map(Color::from))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| ProbeError::BadArgs("colors is malformed".into()))?;
    let extend = match args.str("extend")? {
        "neither" => Extend::Neither,
        "min" => Extend::Min,
        "max" => Extend::Max,
        "both" => Extend::Both,
        other => return Err(ProbeError::BadArgs(format!("unknown extend {other:?}"))),
    };
    let (cmap, norm) = from_levels_and_colors(&args.f64_vec("levels")?, &colors, extend)
        .map_err(ProbeError::rejected)?;
    let (n, under, over) = (cmap.n(), cmap.under(), cmap.over());
    let mut mappable = ScalarMappable::with_norm(cmap, Box::new(norm));
    let values: Vec<Option<f64>> = args.f64_vec("values")?.into_iter().map(Some).collect();
    let rgba = mappable
        .to_rgba(&values, None)
        .map_err(ProbeError::rejected)?;
    let extend_name = match extend {
        Extend::Neither => "neither",
        Extend::Min => "min",
        Extend::Max => "max",
        Extend::Both => "both",
    };
    Ok(Json::Object(vec![
        (
            "rgba".to_owned(),
            Json::Array(rgba.into_iter().map(rgba_json).collect()),
        ),
        (
            "N".to_owned(),
            Json::Integer(i64::try_from(n).unwrap_or(i64::MAX)),
        ),
        ("under".to_owned(), rgba_json(under)),
        ("over".to_owned(), rgba_json(over)),
        ("extend".to_owned(), Json::String(extend_name.to_owned())),
    ]))
}
