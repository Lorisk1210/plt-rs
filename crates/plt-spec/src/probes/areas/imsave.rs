//! Probe operations for the `imsave` area (`matplotlib.image.imsave` to
//! JPEG and TIFF).

use plt::colors::{ColorizerOptions, colormaps};
use plt::image::{
    ImageData, ImsaveOptions, Interpolation, Origin, ThumbnailOptions, imread, imsave_bytes,
    imsave_format, thumbnail,
};
use plt::numeric::Grid;
use plt_codec::base64;

use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError, malformed};
use crate::scratch::ScratchDir;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("image.imsave", imsave),
    Op::new("image.thumbnail", thumbnail_op),
];

/// A 2-D array of `cell`s from nested lists.
fn cells<T>(value: &Json, cell: impl Fn(&Json) -> Option<T>) -> Result<Grid<T>, ProbeError> {
    let rows = value
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .map(|row| row.as_array()?.iter().map(&cell).collect())
                .collect::<Option<Vec<Vec<T>>>>()
        })
        .ok_or_else(|| malformed("data"))?;
    Grid::from_rows(rows).map_err(|_| malformed("data"))
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
        Some(_) => return Err(malformed("dtype")),
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
        _ => return Err(malformed("data")),
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
        .map_err(|_| malformed("origin"))?;
    let jpeg_quality = args
        .opt_i64("quality")?
        .map(|q| u8::try_from(q).map_err(|_| malformed("quality")))
        .transpose()?;
    let options = ImsaveOptions {
        colorizer: ColorizerOptions {
            cmap,
            vmin: args.opt_f64("vmin")?,
            vmax: args.opt_f64("vmax")?,
            ..ColorizerOptions::default()
        },
        origin,
        dpi: args.opt_f64("dpi")?.unwrap_or(100.0),
        jpeg_quality,
    };
    let file = imsave_bytes(image_data(args)?, format, options).map_err(ProbeError::rejected)?;
    Ok(Json::from(base64::encode(&file)))
}

/// The colour image `data` saved as PNG, thumbnailed and read back: its
/// shape and RGBA bytes.
fn thumbnail_op(args: Args<'_>) -> Outcome {
    let data = cells(args.json("data")?, |cell| {
        pixel::<4, f64>(cell, Json::as_f64)
    })?;
    let interpolation: Interpolation = args
        .str("interpolation")?
        .parse()
        .map_err(ProbeError::rejected)?;
    let dir = ScratchDir::new("thumbnail").map_err(ProbeError::rejected)?;
    let (source, target) = (dir.path("image.png"), dir.path("thumb.png"));
    let png = imsave_bytes(
        ImageData::from(data),
        plt::image::BitmapFormat::Png,
        ImsaveOptions::default(),
    )
    .map_err(ProbeError::rejected)?;
    std::fs::write(&source, png).map_err(ProbeError::rejected)?;
    let options = ThumbnailOptions {
        scale: args.f64("scale")?,
        interpolation,
    };
    let made = thumbnail(&source, &target, options).map_err(ProbeError::rejected);
    let thumb = made.and_then(|_| imread(&target).map_err(ProbeError::rejected))?;
    let (rows, cols) = thumb.shape();
    // As the Python side reads it: imread's floats, times 255, rounded.
    let bytes: Vec<u8> = match thumb {
        ImageData::RgbaF32(pixels) => pixels
            .as_slice()
            .iter()
            .flatten()
            .map(|&v| (v * 255.0).round() as u8)
            .collect(),
        ImageData::RgbaBytes(pixels) => pixels.as_slice().iter().flatten().copied().collect(),
        _ => return Err(ProbeError::rejected("the thumbnail is not RGBA")),
    };
    Ok(Json::Object(vec![
        (
            "shape".to_owned(),
            Json::Array(vec![
                Json::Integer(i64::try_from(rows).unwrap_or(i64::MAX)),
                Json::Integer(i64::try_from(cols).unwrap_or(i64::MAX)),
            ]),
        ),
        ("rgba".to_owned(), Json::String(base64::encode(&bytes))),
    ]))
}
