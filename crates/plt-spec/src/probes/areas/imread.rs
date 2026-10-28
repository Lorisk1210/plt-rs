//! Probe operations for the `imread` and `imread_webp` areas
//! (`matplotlib.image.imread` of JPEG, TIFF, GIF and WebP files, and of
//! the truncated PNG files both libraries reject).

use plt::image::{ImageData, imread_bytes};
use plt_codec::base64;

use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[Op::new("image.imread", imread)];

/// A row-major grid of `cell`s as nested lists.
fn nested<T>(shape: (usize, usize), cells: &[T], cell: impl Fn(&T) -> Json) -> Json {
    let (_, cols) = shape;
    Json::Array(
        cells
            .chunks(cols.max(1))
            .map(|row| Json::Array(row.iter().map(&cell).collect()))
            .collect(),
    )
}

/// A list of integer channels.
fn channels(values: &[u8]) -> Json {
    Json::Array(
        values
            .iter()
            .map(|&v| Json::Integer(i64::from(v)))
            .collect(),
    )
}

fn imread(args: Args<'_>) -> Outcome {
    // `format` only tells matplotlib to let Pillow recognise the file.
    args.str("format")?;
    let data = base64::decode(args.str("data")?)
        .map_err(|err| ProbeError::BadArgs(format!("data is not base64: {err}")))?;
    let image = imread_bytes(&data).map_err(ProbeError::rejected)?;
    let shape = image.shape();
    Ok(match &image {
        ImageData::Scalar(field) => nested(shape, field.values().as_slice(), |&v| {
            // Integer arrays hold whole numbers.
            Json::Integer(v as i64)
        }),
        ImageData::RgbBytes(grid) => nested(shape, grid.as_slice(), |pixel| channels(pixel)),
        ImageData::RgbaBytes(grid) => nested(shape, grid.as_slice(), |pixel| channels(pixel)),
        ImageData::Rgb(_) | ImageData::Rgba(_) | ImageData::RgbF32(_) | ImageData::RgbaF32(_) => {
            return Err(ProbeError::BadArgs(
                "a PNG file: imread returns floats".to_owned(),
            ));
        }
    })
}
