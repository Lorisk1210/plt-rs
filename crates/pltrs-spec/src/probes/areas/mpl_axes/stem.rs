//! Stem plots: the formats of heads, stems and baseline, and the
//! baseline's interpolation on polar axes.

use std::f64::consts::FRAC_PI_2;

use pltrs::axes::StemOptions;
use pltrs::figure::Figure;

use super::{iso_dates, made, numbers, on_axes, rgba_list};
use crate::json::Json;
use crate::probes::areas::canon::Canon;
use crate::probes::areas::mpl_ticker::styled;
use crate::probes::encode::points;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// The stem options the probe's format strings give.
fn stem_options(args: Args<'_>) -> Result<StemOptions, ProbeError> {
    let mut options = StemOptions::default();
    if let Some(fmt) = args.opt_str("linefmt")? {
        options.linefmt = made(fmt.parse())?;
    }
    if let Some(fmt) = args.opt_str("markerfmt")? {
        // matplotlib reads `markerfmt=""` as `" "`, the hidden heads.
        let fmt = if fmt.is_empty() { " " } else { fmt };
        options.markerfmt = made(fmt.parse())?;
    }
    if let Some(fmt) = args.opt_str("basefmt")? {
        options.basefmt = Some(made(fmt.parse())?);
    }
    Ok(options)
}

pub(super) fn stem(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let stems = made(ax.stem(args.f64_vec("x")?, args.f64_vec("y")?, stem_options(args)?))?;
        let (x, y) = stems.markerline.data();
        let markers = &stems.markerline;
        Ok(encode::object([
            ("markers", Json::Array(vec![numbers(&x), numbers(&y)])),
            ("markercolor", encode::rgba(made(markers.rgba())?)),
            ("marker", markers.marker().canon()),
            ("markerstyle", markers.linestyle().canon()),
            (
                "linecolor",
                rgba_list(&made(stems.stemlines.colors())?.edges),
            ),
            ("basecolor", encode::rgba(made(stems.baseline.rgba())?)),
        ]))
    })
}

pub(super) fn dates(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let days = iso_dates(args.json("x")?)?;
        let stems = made(ax.stem(days, args.f64_vec("y")?, StemOptions::default()))?;
        Ok(encode::object([
            ("markers", points(&stems.markerline.xydata())),
            ("baseline", points(&stems.baseline.xydata())),
        ]))
    })
}

pub(super) fn baseline(args: Args<'_>) -> Outcome {
    styled(args, || {
        let projection = args.str("projection")?;
        if projection != "polar" {
            return Err(ProbeError::BadArgs(format!(
                "unknown projection {projection}"
            )));
        }
        let ax = Figure::new().polar_subplot();
        let x: Vec<f64> = (0..10)
            .map(|i| FRAC_PI_2 + FRAC_PI_2 * f64::from(i) / 9.0)
            .collect();
        let y: Vec<f64> = (0..10).map(|i| f64::from(i) / 9.0).collect();
        let options = StemOptions {
            bottom: 0.5,
            ..StemOptions::default()
        };
        let stems = made(ax.stem(x, y, options))?;
        Ok(Json::from(stems.baseline.interpolation_steps() > 100))
    })
}
