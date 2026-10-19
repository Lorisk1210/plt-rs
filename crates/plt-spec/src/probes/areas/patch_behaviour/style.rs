//! The `patch.style` op: the colours and edge properties of a rectangle
//! made with keyword arguments under some rcParams, then changed by
//! setters.

use plt::colors::Color;
use plt::figure::Figure;
use plt::lines::LineStyle;
use plt::patches::{Patch, PatchProps, Rectangle};
use plt::rc;

use crate::json::Json;
use crate::probes::encode::{object, rgba};
use crate::probes::{Args, Outcome, ProbeError};

fn color(value: &Json) -> Result<Color, ProbeError> {
    value
        .as_str()
        .map(Color::from)
        .ok_or_else(|| ProbeError::BadArgs(format!("expected a colour name, got {value}")))
}

/// A line style name or `[offset, [on, off, ...]]`.
fn linestyle(value: &Json) -> Result<LineStyle, ProbeError> {
    match value {
        Json::String(name) => name.parse().map_err(ProbeError::rejected),
        Json::Array(parts) => match parts.as_slice() {
            [offset, pattern] => Ok(LineStyle::dashes(
                offset.as_f64().unwrap_or_default(),
                pattern.as_f64_vec().unwrap_or_default(),
            )),
            _ => Err(ProbeError::BadArgs(format!("bad line style {value}"))),
        },
        other => Err(ProbeError::BadArgs(format!("bad line style {other}"))),
    }
}

/// `Rectangle((0, 0), 1, 1, **kw)` as properties.
fn props(kw: &Json) -> Result<PatchProps, ProbeError> {
    let mut props = PatchProps::default();
    for (key, value) in kw.as_object().unwrap_or(&[]) {
        match key.as_str() {
            "facecolor" => props.facecolor = Some(color(value)?),
            "edgecolor" | "ec" => props.edgecolor = Some(color(value)?),
            "hatchcolor" => props.hatchcolor = Some(color(value)?),
            "hatch" => props.hatch = value.as_str().map(str::to_owned),
            "fill" => props.fill = value.as_bool(),
            "alpha" => props.base.alpha = value.as_f64(),
            "edgegapcolor" => {}
            other => return Err(ProbeError::BadArgs(format!("unknown keyword {other}"))),
        }
    }
    Ok(props)
}

/// The rectangle, made while the probe's rcParams are in effect.
fn rectangle(fig: &Figure, kw: &Json, rc_overrides: &Json) -> Result<Patch<Rectangle>, ProbeError> {
    rc::context(|| {
        for (key, value) in rc_overrides.as_object().unwrap_or(&[]) {
            let text = match value {
                Json::Bool(flag) => if *flag { "True" } else { "False" }.to_owned(),
                Json::String(text) => text.clone(),
                other => other.to_string(),
            };
            rc::set(key, &text).map_err(ProbeError::rejected)?;
        }
        let rect = fig.add_patch(Rectangle::new((0.0, 0.0), 1.0, 1.0));
        rect.update_props(props(kw)?);
        if let Some(gap) = kw.get("edgegapcolor") {
            rect.set_edgegapcolor(color(gap)?);
        }
        Ok(rect)
    })
}

pub(super) fn style(args: Args<'_>) -> Outcome {
    let fig = Figure::new();
    let rect = rectangle(&fig, args.json("kw")?, args.json("rcparams")?)?;
    for step in args.json("steps")?.as_array().unwrap_or(&[]) {
        match step.as_array().map(<[Json]>::as_ref) {
            Some([Json::String(name), value]) => match name.as_str() {
                "set_edgecolor" => rect.set_edgecolor(color(value)?),
                "set_facecolor" => rect.set_facecolor(color(value)?),
                "set_hatchcolor" => rect.set_hatchcolor(color(value)?),
                "set_edgegapcolor" => rect.set_edgegapcolor(color(value)?),
                "set_linestyle" => rect.set_linestyle(linestyle(value)?),
                other => return Err(ProbeError::BadArgs(format!("unknown setter {other}"))),
            },
            _ => return Err(ProbeError::BadArgs(format!("bad step {step}"))),
        };
    }
    let colors = rect.colors().map_err(ProbeError::rejected)?;
    let style = rect.style();
    let gap = if style.edgegapcolor.is_none() {
        Json::Null
    } else {
        rgba(style.edgegapcolor.resolve().map_err(ProbeError::rejected)?)
    };
    Ok(object([
        ("facecolor", rgba(colors.face)),
        ("edgecolor", rgba(colors.edge)),
        ("hatchcolor", rgba(colors.hatch)),
        ("capstyle", style.capstyle.name().into()),
        ("joinstyle", style.joinstyle.name().into()),
        ("edgegapcolor", gap),
        ("dashed", style.linestyle.is_dashed().into()),
    ]))
}
