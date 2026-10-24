//! Probe operations for the `mpl_agg` area: matplotlib's own non-image
//! tests of the Agg canvas (`tests/test_agg.py`; see
//! `spec/reference/probes/test_mpl_agg.py`): saved colours, canvas sizes
//! and what the JPEG, GIF and WebP files record, read as Pillow reads
//! them.

use plt::figure::{Figure, SaveFormat, SaveOptions};
use plt_codec::raster::Pixels;
use plt_core::Rgba;

use crate::json::Json;
use crate::probes::areas::mpl_ticker::styled;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("Figure.savefig.pixel", pixel),
    Op::new("Figure.savefig.too_large", too_large),
    Op::new("FigureCanvasAgg.buffer_rgba", buffer_shape),
    Op::new("Figure.savefig.jpeg", jpeg),
    Op::new("Figure.savefig.gif", gif),
    Op::new("Figure.savefig.webp", webp),
];

fn bad(what: &str) -> ProbeError {
    ProbeError::BadArgs(format!("{what} is malformed"))
}

fn integers(values: &[u32]) -> Json {
    Json::Array(values.iter().map(|&v| Json::from(f64::from(v))).collect())
}

/// The probe's `figsize` and, when given, `dpi` as a figure.
fn figure(args: Args<'_>) -> Result<Figure, ProbeError> {
    let &[width, height] = args.f64_vec("figsize")?.as_slice() else {
        return Err(bad("figsize"));
    };
    let dpi = args
        .opt_f64("dpi")?
        .unwrap_or_else(|| plt::rc::params().figure_dpi);
    Ok(Figure::with_size(width, height, dpi))
}

/// The first pixel of a figure with a translucent face colour saved twice
/// as a PNG.
fn pixel(args: Args<'_>) -> Outcome {
    styled(args, || {
        let fig = figure(args)?;
        let face = match args.f64_vec("facecolor")?.as_slice() {
            &[r, g, b] => Rgba::new(r, g, b, args.f64("alpha")?),
            _ => return Err(bad("facecolor")),
        };
        fig.set_facecolor(face);
        let save = || {
            fig.to_png_with(SaveOptions {
                facecolor: Some(face),
                edgecolor: Some(Rgba::TRANSPARENT),
                ..SaveOptions::default()
            })
            .map_err(ProbeError::rejected)
        };
        save()?;
        let image = plt_codec::png::decode(&save()?).map_err(ProbeError::rejected)?;
        let pixels = image.to_rgba8();
        let first = pixels
            .get(..4)
            .ok_or_else(|| ProbeError::rejected("an empty image"))?;
        Ok(integers(
            &first.iter().map(|&b| u32::from(b)).collect::<Vec<_>>(),
        ))
    })
}

/// Saving a figure too large for the canvas.
fn too_large(args: Args<'_>) -> Outcome {
    styled(args, || {
        figure(args)?.to_png().map_err(ProbeError::rejected)?;
        Ok(Json::Null)
    })
}

/// `[height, width, 4]` of the drawn canvas.
fn buffer_shape(args: Args<'_>) -> Outcome {
    styled(args, || {
        let fig = figure(args)?;
        let canvas = fig.canvas();
        canvas.draw().map_err(ProbeError::rejected)?;
        let image = canvas.buffer_rgba().map_err(ProbeError::rejected)?;
        let side = |n: usize| u32::try_from(n).map_err(ProbeError::rejected);
        Ok(integers(&[side(image.height())?, side(image.width())?, 4]))
    })
}

/// A figure with `plot([0, 1, 2], [0, 1, 0])` saved as `format`.
fn plotted(format: SaveFormat, options: SaveOptions) -> Result<Vec<u8>, ProbeError> {
    let fig = Figure::new();
    fig.subplot()
        .plot(vec![0.0, 1.0, 2.0], vec![0.0, 1.0, 0.0])
        .map_err(ProbeError::rejected)?;
    fig.to_format(format, options).map_err(ProbeError::rejected)
}

/// The resolution in a JPEG's JFIF header, as Pillow's `info["dpi"]`.
fn jfif_dpi(jpeg: &[u8]) -> Result<[f64; 2], ProbeError> {
    let start = jpeg
        .windows(5)
        .position(|w| w == b"JFIF\0")
        .ok_or_else(|| ProbeError::rejected("no JFIF header"))?;
    let header = jpeg
        .get(start + 7..start + 12)
        .ok_or_else(|| ProbeError::rejected("a short JFIF header"))?;
    let density = |at: usize| f64::from(u16::from_be_bytes([header[at], header[at + 1]]));
    let scale = match header[0] {
        1 => 1.0,
        2 => 2.54,
        _ => return Err(ProbeError::rejected("a JFIF header without a resolution")),
    };
    Ok([density(1) * scale, density(3) * scale])
}

/// The resolution of the plot saved as a JPEG at `dpi`.
fn jpeg(args: Args<'_>) -> Outcome {
    styled(args, || {
        let options = SaveOptions {
            dpi: Some(args.f64("dpi")?),
            ..SaveOptions::default()
        };
        let [x, y] = jfif_dpi(&plotted(SaveFormat::Jpeg, options)?)?;
        Ok(Json::Array(vec![Json::from(x), Json::from(y)]))
    })
}

fn transparent(args: Args<'_>) -> Result<SaveOptions, ProbeError> {
    Ok(SaveOptions {
        transparent: Some(args.bool("transparent")?),
        ..SaveOptions::default()
    })
}

/// Pillow's mode of the plot saved as a GIF and the smallest alpha of the
/// image.
fn gif(args: Args<'_>) -> Outcome {
    styled(args, || {
        let data = plotted(SaveFormat::Gif, transparent(args)?)?;
        let image = plt_codec::gif::decode(&data).map_err(ProbeError::rejected)?;
        let mode = match image.pixels() {
            Pixels::Indexed { .. } => "P",
            Pixels::Gray(_) => "L",
            _ => return Err(ProbeError::rejected("not a palette image")),
        };
        let min_alpha = image
            .to_rgba8()
            .iter()
            .map(|pixel| pixel[3])
            .min()
            .ok_or_else(|| ProbeError::rejected("an empty image"))?;
        Ok(crate::probes::encode::object([
            ("mode", Json::from(mode)),
            ("min_alpha", Json::from(f64::from(min_alpha))),
        ]))
    })
}

/// Whether a WebP file says it has alpha, as libwebp's `has_alpha` (the
/// extended header's flag, or the lossless header's `alpha_is_used`).
fn webp_has_alpha(data: &[u8]) -> Result<bool, ProbeError> {
    const ALPHA_FLAG: u8 = 0x10;
    const LOSSLESS_ALPHA_BIT: u32 = 28;
    let malformed = || ProbeError::rejected("a malformed WebP file");
    if data.get(..4) != Some(b"RIFF") || data.get(8..12) != Some(b"WEBP") {
        return Err(malformed());
    }
    match data.get(12..16) {
        Some(b"VP8X") => Ok(data.get(20).ok_or_else(malformed)? & ALPHA_FLAG != 0),
        Some(b"VP8L") => {
            let header = data.get(21..25).ok_or_else(malformed)?;
            let bits = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
            Ok((bits >> LOSSLESS_ALPHA_BIT) & 1 == 1)
        }
        Some(b"VP8 ") => Ok(false),
        _ => Err(malformed()),
    }
}

/// Pillow's mode of the plot saved as a WebP.
fn webp(args: Args<'_>) -> Outcome {
    styled(args, || {
        let data = plotted(SaveFormat::Webp, transparent(args)?)?;
        Ok(Json::from(if webp_has_alpha(&data)? {
            "RGBA"
        } else {
            "RGB"
        }))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers_are_read_like_pillow() {
        let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0, 16];
        jpeg.extend_from_slice(b"JFIF\0\x01\x01\x02\x00\x50\x00\x28");
        assert_eq!(jfif_dpi(&jpeg).unwrap(), [80.0 * 2.54, 40.0 * 2.54]);
        assert!(jfif_dpi(b"no header").is_err());
        let mut lossless = b"RIFF\0\0\0\0WEBPVP8L\0\0\0\0\x2f".to_vec();
        lossless.extend_from_slice(&(1_u32 << 28).to_le_bytes());
        assert!(webp_has_alpha(&lossless).unwrap());
        assert!(webp_has_alpha(b"RIFF").is_err());
    }
}
