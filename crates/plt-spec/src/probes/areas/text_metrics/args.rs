//! Decoding the shared arguments of `text_metrics` probes: the reference
//! rc configuration with per-probe overrides, font dicts and the keyword
//! arguments of `fig.text`.

use plt::fonts::{
    FontProperties, FontSize, FontStretch, FontWeight, parse_math_fontfamily, parse_style,
    parse_variant,
};
use plt::rc::{self, RcParams};
use plt::text::{HorizontalAlignment, LineSpacing, Text, TextRotationMode, VerticalAlignment};

use crate::json::Json;
use crate::probes::{Args, ProbeError};

/// Runs `f` under the reference configuration (`matplotlib.testing.setup()`
/// plus `text.hinting = none`) with the probe's `rc` overrides applied.
pub(crate) fn with_rc<T>(
    args: Args<'_>,
    f: impl FnOnce() -> Result<T, ProbeError>,
) -> Result<T, ProbeError> {
    rc::context(|| {
        rc::replace(RcParams::matplotlib());
        rc::set("font.family", "DejaVu Sans").map_err(ProbeError::rejected)?;
        rc::set("text.hinting", "none").map_err(ProbeError::rejected)?;
        for (key, value) in args.get("rc").and_then(Json::as_object).unwrap_or(&[]) {
            rc::set(key, &rc_text(value)?).map_err(ProbeError::rejected)?;
        }
        f()
    })
}

/// The rc-file spelling of a JSON rc value: booleans as `True`/`False`,
/// `null` as `None` and lists comma-separated.
pub(crate) fn rc_text(value: &Json) -> Result<String, ProbeError> {
    match value {
        Json::Null => Ok("None".to_owned()),
        Json::Bool(true) => Ok("True".to_owned()),
        Json::Bool(false) => Ok("False".to_owned()),
        Json::String(text) => Ok(text.clone()),
        Json::Integer(n) => Ok(n.to_string()),
        Json::Number(n) => Ok(n.to_string()),
        Json::Array(items) => items
            .iter()
            .map(rc_text)
            .collect::<Result<Vec<_>, _>>()
            .map(|items| items.join(", ")),
        other => Err(ProbeError::BadArgs(format!("unsupported rc value {other}"))),
    }
}

/// A family given as a string or a list of strings.
pub(super) fn families(value: &Json) -> Result<Vec<String>, ProbeError> {
    match value {
        Json::String(name) => Ok(vec![name.clone()]),
        Json::Array(items) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| ProbeError::BadArgs(format!("bad family {value}")))
            })
            .collect(),
        other => Err(ProbeError::BadArgs(format!("bad family {other}"))),
    }
}

/// A font size: points or a size string (`set_size`).
pub(super) fn size(value: &Json) -> Result<FontSize, ProbeError> {
    match value {
        Json::String(text) => text.parse().map_err(ProbeError::rejected),
        other => other
            .as_f64()
            .map(FontSize::Points)
            .ok_or_else(|| ProbeError::BadArgs(format!("bad size {other}"))),
    }
}

/// An integer property given as a number: Python's `int(value)` truncates.
fn integer(value: f64) -> Result<i64, ProbeError> {
    if value.is_finite() {
        Ok(value.trunc() as i64)
    } else {
        Err(ProbeError::rejected(format!("{value} is not an integer")))
    }
}

/// A font weight: a keyword or a number (`set_weight`).
pub(crate) fn weight(value: &Json) -> Result<FontWeight, ProbeError> {
    match value {
        Json::String(text) => text.parse().map_err(ProbeError::rejected),
        other => {
            let number = other
                .as_f64()
                .ok_or_else(|| ProbeError::BadArgs(format!("bad weight {other}")))?;
            FontWeight::from_number(integer(number)?).map_err(ProbeError::rejected)
        }
    }
}

/// A font stretch: a keyword or a number (`set_stretch`).
pub(super) fn stretch(value: &Json) -> Result<FontStretch, ProbeError> {
    match value {
        Json::String(text) => text.parse().map_err(ProbeError::rejected),
        other => {
            let number = other
                .as_f64()
                .ok_or_else(|| ProbeError::BadArgs(format!("bad stretch {other}")))?;
            FontStretch::from_number(integer(number)?).map_err(ProbeError::rejected)
        }
    }
}

/// A string argument.
fn string(value: &Json) -> Result<&str, ProbeError> {
    value
        .as_str()
        .ok_or_else(|| ProbeError::BadArgs(format!("expected a string, got {value}")))
}

/// Applies one `FontProperties` setter; `null` restores the rc default.
fn set_font_property(
    font: &mut FontProperties,
    key: &str,
    value: &Json,
    rc: &RcParams,
) -> Result<(), ProbeError> {
    let default = FontProperties::from_rc(rc);
    if value.is_null() {
        match key {
            "family" => font.families = default.families,
            "style" => font.style = default.style,
            "variant" => font.variant = default.variant,
            "weight" => font.weight = default.weight,
            "stretch" => font.stretch = default.stretch,
            "size" => font.size = default.size,
            "math_fontfamily" => font.math_fontfamily = default.math_fontfamily,
            _ => return Err(ProbeError::BadArgs(format!("unknown font key {key}"))),
        }
        return Ok(());
    }
    match key {
        "family" => font.families = families(value)?,
        "style" => font.style = parse_style(string(value)?).map_err(ProbeError::rejected)?,
        "variant" => font.variant = parse_variant(string(value)?).map_err(ProbeError::rejected)?,
        "weight" => font.weight = weight(value)?,
        "stretch" => font.stretch = stretch(value)?,
        "size" => font.set_size(size(value)?, rc.font_size),
        "math_fontfamily" => {
            font.math_fontfamily =
                parse_math_fontfamily(string(value)?).map_err(ProbeError::rejected)?;
        }
        _ => return Err(ProbeError::BadArgs(format!("unknown font key {key}"))),
    }
    Ok(())
}

/// The order in which `FontProperties.__init__` (and the probes' font
/// dicts) apply the setters.
const FONT_KEYS: [&str; 7] = [
    "family",
    "style",
    "variant",
    "weight",
    "stretch",
    "size",
    "math_fontfamily",
];

/// A "font dict": `FontProperties()` followed by the setters of the keys
/// present, in [`FONT_KEYS`] order (`null` means the defaults).
pub(crate) fn font_dict(value: Option<&Json>) -> Result<FontProperties, ProbeError> {
    let rc = rc::params();
    let mut font = FontProperties::from_rc(&rc);
    let Some(dict) = value else {
        return Ok(font);
    };
    for key in FONT_KEYS {
        if let Some(item) = dict.get(key) {
            set_font_property(&mut font, key, item, &rc)?;
        }
    }
    Ok(font)
}

/// `FontProperties(**kw)`: like a font dict, except that a lone string
/// `family` is a fontconfig pattern.
pub(crate) fn font_properties(kw: &Json) -> Result<FontProperties, ProbeError> {
    let rc = rc::params();
    let mut font = font_dict(Some(kw))?;
    let is_set = |key: &str| kw.get(key).is_some_and(|value| !value.is_null());
    if let Some(Json::String(pattern)) = kw.get("family")
        && !["style", "variant", "weight", "stretch", "size"]
            .iter()
            .any(|key| is_set(key))
    {
        font.set_fontconfig_pattern(pattern, rc.font_size)
            .map_err(ProbeError::rejected)?;
    }
    Ok(font)
}

/// `rotation`: degrees, `"vertical"`, `"horizontal"` or `null`.
pub(crate) fn rotation(value: &Json) -> Result<f64, ProbeError> {
    match value {
        Json::Null => Ok(0.0),
        Json::String(name) => match name.as_str() {
            "horizontal" => Ok(0.0),
            "vertical" => Ok(90.0),
            other => Err(ProbeError::rejected(format!("bad rotation {other:?}"))),
        },
        other => other
            .as_f64()
            .ok_or_else(|| ProbeError::rejected(format!("bad rotation {other}"))),
    }
}

/// `linespacing`: `"normal"` or a number.
fn linespacing(value: &Json) -> Result<LineSpacing, ProbeError> {
    match value {
        Json::String(name) if name == "normal" => Ok(LineSpacing::Normal),
        other => other
            .as_f64()
            .map(LineSpacing::Factor)
            .ok_or_else(|| ProbeError::rejected(format!("bad linespacing {other}"))),
    }
}

fn bool_arg(value: &Json) -> Result<bool, ProbeError> {
    value
        .as_bool()
        .ok_or_else(|| ProbeError::BadArgs(format!("expected a boolean, got {value}")))
}

fn halign(value: &Json) -> Result<HorizontalAlignment, ProbeError> {
    let name = string(value)?;
    HorizontalAlignment::parse(name)
        .ok_or_else(|| ProbeError::rejected(format!("bad alignment {name:?}")))
}

/// Applies the keyword arguments of `fig.text(x, y, s, **kw)` in order.
pub(crate) fn apply_text_kw(text: &Text, kw: Option<&Json>) -> Result<(), ProbeError> {
    let rc = rc::params();
    for (key, value) in kw.and_then(Json::as_object).unwrap_or(&[]) {
        match key.as_str() {
            "family" | "style" | "variant" | "weight" | "stretch" | "size" | "math_fontfamily" => {
                let mut font = text.font();
                set_font_property(&mut font, key, value, &rc)?;
                text.set_font(font);
            }
            "rotation" => {
                text.set_rotation(rotation(value)?);
            }
            "ha" => {
                text.set_halign(halign(value)?);
            }
            "ma" => {
                text.set_multialign(Some(halign(value)?));
            }
            "va" => {
                let name = string(value)?;
                let valign = VerticalAlignment::parse(name)
                    .ok_or_else(|| ProbeError::rejected(format!("bad alignment {name:?}")))?;
                text.set_valign(valign);
            }
            "rotation_mode" => {
                let mode = match value {
                    Json::Null => TextRotationMode::Default,
                    other => {
                        let name = string(other)?;
                        TextRotationMode::parse(name).ok_or_else(|| {
                            ProbeError::rejected(format!("bad rotation mode {name:?}"))
                        })?
                    }
                };
                text.set_rotation_mode(mode);
            }
            "linespacing" => {
                text.set_linespacing(linespacing(value)?);
            }
            "wrap" => {
                text.set_wrap(bool_arg(value)?);
            }
            "parse_math" => {
                text.set_parse_math(bool_arg(value)?);
            }
            other => return Err(ProbeError::BadArgs(format!("unknown text keyword {other}"))),
        }
    }
    Ok(())
}
