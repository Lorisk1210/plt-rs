//! Probe operations for the `imsave` area (`matplotlib.image.imsave` to
//! JPEG and TIFF).

use plt::colors::colormaps;
use plt::image::{ImageData, ImsaveOptions, Origin, imsave_bytes, imsave_format};
use plt::numeric::Grid;
use plt_codec::base64;

use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[Op::new("image.imsave", imsave)];

fn bad(what: &str) -> ProbeError {
    ProbeError::BadArgs(format!("{what} is malformed"))
}

/// A 2-D array of `cell`s from nested lists.
fn cells<T>(value: &Json, cell: impl Fn(&Json) -> Option<T>) -> Result<Grid<T>, ProbeError> {
    let rows = value
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .map(|row| row.as_array()?.iter().map(&cell).collect())
                .collect::<Option<Vec<Vec<T>>>>()
        })
        .ok_or_else(|| bad("data"))?;
    Grid::from_rows(rows).map_err(|_| bad("data"))
}

/// A pixel of `N` channels.
fn pixel<const N: usize, C>(value: &Json, channel: impl Fn(&Json) -> Option<C>) -> Option<[C; N]> {
    let channels: Vec<C> = value
        .as_array()?
        .iter()
        .map(channel)
        .collect::<Option<_>>()?;
    channels.try_into().ok()
}

/// A `uint8` channel.
fn byte(value: &Json) -> Option<u8> {
    value.as_i64().and_then(|n| u8::try_from(n).ok())
}

/// The `data` argument as an image array of `dtype`.
fn image_data(args: Args<'_>) -> Result<ImageData, ProbeError> {
    let data = args.json("data")?;
    let bytes = match args.opt_str("dtype")? {
        None | Some("float") => false,
        Some("uint8") => true,
        Some(_) => return Err(bad("dtype")),
    };
    let channels = data
        .as_array()
        .and_then(|rows| rows.first()?.as_array()?.first()?.as_array())
        .map_or(0, <[Json]>::len);
    Ok(match (channels, bytes) {
        (0, false) => ImageData::from(cells(data, Json::as_f64)?),
        (3, false) => ImageData::from(cells(data, |c| pixel::<3, _>(c, Json::as_f64))?),
        (4, false) => ImageData::from(cells(data, |c| pixel::<4, _>(c, Json::as_f64))?),
        (3, true) => ImageData::from(cells(data, |c| pixel::<3, _>(c, byte))?),
        (4, true) => ImageData::from(cells(data, |c| pixel::<4, _>(c, byte))?),
        _ => return Err(bad("data")),
    })
}

fn imsave(args: Args<'_>) -> Outcome {
    let format = imsave_format(args.str("format")?).map_err(ProbeError::rejected)?;
    let cmap = args
        .opt_str("cmap")?
        .map(colormaps::get)
        .transpose()
        .map_err(ProbeError::rejected)?;
    let origin = args
        .opt_str("origin")?
        .map(str::parse::<Origin>)
        .transpose()
        .map_err(|_| bad("origin"))?;
    let jpeg_quality = args
        .opt_i64("quality")?
        .map(|q| u8::try_from(q).map_err(|_| bad("quality")))
        .transpose()?;
    let options = ImsaveOptions {
        vmin: args.opt_f64("vmin")?,
        vmax: args.opt_f64("vmax")?,
        cmap,
        origin,
        dpi: args.opt_f64("dpi")?.unwrap_or(100.0),
        jpeg_quality,
    };
    let file = imsave_bytes(image_data(args)?, format, &options).map_err(ProbeError::rejected)?;
    Ok(Json::from(base64::encode(&file)))
}
