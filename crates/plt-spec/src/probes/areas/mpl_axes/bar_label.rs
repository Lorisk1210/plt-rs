//! Bar labels: placement, alignment, formats and padding.

use plt::axes::{BarLabelFormat, BarLabelOptions, BarLabelType, BarOptions, ErrorSize};

use super::{made, number_list, numbers, on_axes, optional, pair, strings};
use crate::json::Json;
use crate::probes::areas::canon::Canon;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// The labelling options of the probe's keywords.
fn label_options(args: Args<'_>) -> Result<BarLabelOptions, ProbeError> {
    let mut options = BarLabelOptions {
        labels: optional(args, "labels", strings)?.unwrap_or_default(),
        padding: optional(args, "padding", number_list)?.unwrap_or_default(),
        ..BarLabelOptions::default()
    };
    if let Some(fmt) = args.opt_str("fmt")? {
        options.fmt = BarLabelFormat::Template(fmt.to_owned());
    }
    options.label_type = match args.opt_str("label_type")?.unwrap_or("edge") {
        "edge" => BarLabelType::Edge,
        "center" => BarLabelType::Center,
        other => return Err(ProbeError::BadArgs(format!("bad label_type {other}"))),
    };
    Ok(options)
}

pub(super) fn bar_label(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        for axis in optional(args, "invert", strings)?.unwrap_or_default() {
            match axis.as_str() {
                "x" => ax.xaxis().set_inverted(true),
                "y" => ax.yaxis().set_inverted(true),
                other => return Err(ProbeError::BadArgs(format!("bad axis {other}"))),
            };
        }
        let mut options = BarOptions::default();
        options.errors.yerr = optional(args, "yerr", |yerr| {
            Ok(match yerr.as_f64() {
                Some(error) => ErrorSize::from(error),
                None => ErrorSize::from(number_list(yerr)?),
            })
        })?;
        let positions = super::bars::positions(args.json("x")?)?;
        let values = args.f64_vec("values")?;
        let bars = if args.opt_str("orientation")? == Some("horizontal") {
            made(ax.barh(positions, values, options))?
        } else {
            made(ax.bar(positions, values, options))?
        };
        let labels = made(ax.bar_label(&bars, label_options(args)?))?;
        let labels = labels.iter().map(|label| {
            let (xy, xyann) = (label.xy(), label.position());
            encode::object([
                ("text", Json::from(label.text())),
                ("xy", numbers(&[xy.x, xy.y])),
                ("xyann", numbers(&[xyann.x, xyann.y])),
                ("ha", label.halign().canon()),
                ("va", label.valign().canon()),
            ])
        });
        Ok(encode::object([
            ("labels", Json::Array(labels.collect())),
            ("ylim", pair(ax.ylim())),
        ]))
    })
}
