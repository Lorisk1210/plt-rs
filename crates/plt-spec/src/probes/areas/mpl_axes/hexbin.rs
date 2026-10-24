//! Hexagonal binning: offsets, colour limits, extents and picking.

use plt::artist::{ArtistHandle, Picker};
use plt::axes::{Axes, HexBins, HexGridSize, HexScale, HexbinOptions};
use plt::collections::Collection;
use plt::image::{ColorizerOptions, Extent};
use plt_core::Point;

use super::{made, on_axes};
use crate::json::Json;
use crate::probes::encode::points;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// A binning scale argument.
fn scale(args: Args<'_>, key: &str) -> Result<HexScale, ProbeError> {
    match args.opt_str(key)? {
        None | Some("linear") => Ok(HexScale::Linear),
        Some("log") => Ok(HexScale::Log),
        Some(other) => Err(ProbeError::BadArgs(format!("unknown scale {other}"))),
    }
}

/// `extent` as `[xmin, xmax, ymin, ymax]`.
fn extent(args: Args<'_>) -> Result<Option<Extent>, ProbeError> {
    if !args.contains("extent") {
        return Ok(None);
    }
    match args.f64_vec("extent")?.as_slice() {
        &[left, right, bottom, top] => Ok(Some(Extent {
            left,
            right,
            bottom,
            top,
        })),
        other => Err(ProbeError::BadArgs(format!("bad extent {other:?}"))),
    }
}

/// `hexbin(x, y, ...)` with the probe's keywords.
fn hexbin_from(ax: &Axes, args: Args<'_>) -> Result<Collection, ProbeError> {
    let gridsize = match args.opt_i64("gridsize")? {
        Some(n) => HexGridSize::Count(made(usize::try_from(n))?),
        None => HexGridSize::default(),
    };
    let bins = match args.opt_str("bins")? {
        None => None,
        Some("log") => Some(HexBins::Log),
        Some(other) => return Err(ProbeError::BadArgs(format!("unknown bins {other}"))),
    };
    let options = HexbinOptions {
        gridsize,
        bins,
        xscale: scale(args, "xscale")?,
        yscale: scale(args, "yscale")?,
        extent: extent(args)?,
        colors: ColorizerOptions::default().limits(args.opt_f64("vmin")?, args.opt_f64("vmax")?),
        ..HexbinOptions::default()
    };
    made(ax.hexbin(&args.f64_vec("x")?, &args.f64_vec("y")?, options))
}

pub(super) fn hexbin(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let hexes = hexbin_from(ax, args)?;
        let (vmin, vmax) = hexes.clim();
        Ok(encode::object([
            ("offsets", points(&hexes.offsets())),
            ("clim", Json::from(vec![Json::from(vmin), Json::from(vmax)])),
        ]))
    })
}

pub(super) fn contains(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let hexes = hexbin_from(ax, args)?;
        hexes.set_picker(Some(Picker::Radius(-1.0)));
        let [x, y] = args.f64_vec("point")?[..] else {
            return Err(ProbeError::BadArgs("point is not [x, y]".into()));
        };
        Ok(Json::from(hexes.contains(Point::new(x, y)).is_some()))
    })
}
