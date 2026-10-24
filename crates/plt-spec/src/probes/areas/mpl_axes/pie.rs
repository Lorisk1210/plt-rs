//! Pie charts: argument checks, label properties and `pie_label`.

use plt::axes::{PieContainer, PieLabelAlignment, PieLabelOptions, PieOptions};
use plt::fonts::FontSize;
use plt::text::{Text, TextProps};

use super::{color, made, number_list, numbers, on_axes, optional, strings};
use crate::json::Json;
use crate::probes::areas::canon::Canon;
use crate::probes::{Args, Outcome, ProbeError, encode};
use plt::ticker::LabelFormat;

/// `{text, ha, va, rotation, rotation_mode, size, color}` of a label.
fn text_desc(text: &Text) -> Result<Json, ProbeError> {
    Ok(encode::object([
        ("text", Json::from(text.text())),
        ("ha", text.halign().canon()),
        ("va", text.valign().canon()),
        ("rotation", Json::from(text.rotation())),
        ("rotation_mode", text.rotation_mode().canon()),
        ("size", Json::from(text.font().size)),
        ("color", encode::rgba(made(text.color().resolve())?)),
    ]))
}

/// The descriptions of `texts`.
fn texts_desc(texts: &[Text]) -> Result<Json, ProbeError> {
    texts
        .iter()
        .map(text_desc)
        .collect::<Result<Vec<_>, _>>()
        .map(Json::Array)
}

/// `textprops`: alignment, rotation and rotation mode, size and colour
/// (written into `options`).
fn text_props(value: &Json, options: &mut PieOptions) -> Result<(), ProbeError> {
    let field = |key: &str| value.get(key).filter(|v| !v.is_null());
    let name = |key: &str| field(key).and_then(Json::as_str);
    let mut props = TextProps::default();
    if let Some(ha) = name("horizontalalignment") {
        props = props.halign(made(ha.parse::<plt::text::HAlign>())?);
    }
    if let Some(va) = name("verticalalignment") {
        props = props.valign(made(va.parse::<plt::text::VAlign>())?);
    }
    if let Some(mode) = name("rotation_mode") {
        props = props.rotation_mode(made(mode.parse::<plt::text::TextRotationMode>())?);
    }
    if let Some(rotation) = field("rotation").and_then(Json::as_f64) {
        props = props.rotation(rotation);
    }
    options.textprops = props;
    options.text_size = field("size").and_then(Json::as_f64).map(FontSize::Points);
    options.text_color = field("color").map(color).transpose()?;
    Ok(())
}

/// The pie options of the probe's keywords.
fn pie_options(args: Args<'_>) -> Result<PieOptions, ProbeError> {
    let mut options = PieOptions {
        explode: optional(args, "explode", number_list)?.unwrap_or_default(),
        labels: optional(args, "labels", strings)?.unwrap_or_default(),
        autopct: args
            .opt_str("autopct")?
            .map(|format| LabelFormat::Template(format.to_owned())),
        startangle: args.opt_f64("startangle")?.unwrap_or_default(),
        ..PieOptions::default()
    };
    if let Some(radius) = args.opt_f64("radius")? {
        options.radius = radius;
    }
    if let Some(normalize) = args.opt_bool("normalize")? {
        options.normalize = normalize;
    }
    if let Some(props) = args.get("textprops") {
        text_props(props, &mut options)?;
    }
    Ok(options)
}

pub(super) fn pie(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let pie = made(ax.pie(&args.f64_vec("x")?, pie_options(args)?))?;
        let wedges = pie.wedges.iter().map(|wedge| {
            let shape = wedge.shape();
            numbers([shape.theta1, shape.theta2])
        });
        Ok(encode::object([
            ("wedges", Json::Array(wedges.collect())),
            ("texts", texts_desc(&pie.labels)?),
            ("pct", texts_desc(&pie.pct_labels)?),
        ]))
    })
}

/// `alignment` of `pie_label`.
fn alignment(args: Args<'_>) -> Result<PieLabelAlignment, ProbeError> {
    match args.opt_str("alignment")?.unwrap_or("auto") {
        "auto" => Ok(PieLabelAlignment::Auto),
        "center" => Ok(PieLabelAlignment::Center),
        "outer" => Ok(PieLabelAlignment::Outer),
        other => Err(ProbeError::BadArgs(format!("bad alignment {other}"))),
    }
}

pub(super) fn pie_label(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let options = PieOptions {
            startangle: args.opt_f64("startangle")?.unwrap_or_default(),
            ..PieOptions::default()
        };
        let mut pie: PieContainer = made(ax.pie(&args.f64_vec("x")?, options))?;
        let defaults = PieLabelOptions::default();
        let options = PieLabelOptions {
            distance: args.opt_f64("distance")?.unwrap_or(defaults.distance),
            rotate: args.opt_bool("rotate")?.unwrap_or(defaults.rotate),
            alignment: alignment(args)?,
            ..defaults
        };
        let texts = match args.json("labels")? {
            Json::String(template) => ax.pie_label(&mut pie, template.as_str(), options),
            labels => ax.pie_label(&mut pie, strings(labels)?, options),
        };
        texts_desc(&made(texts)?)
    })
}
