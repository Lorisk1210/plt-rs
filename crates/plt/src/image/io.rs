//! Writing images as PNG, JPEG, TIFF, GIF, WebP or raw files (matplotlib.image's
//! `imsave`), and the errors of reading and writing them.

use std::fmt;
use std::path::Path;

use plt_codec::png::DecodeError;
use plt_codec::{gif, jpeg, tiff, webp};
use plt_core::Rgba;

use super::color_image::color_image_bytes;
use super::normalize::normalize_in;
use super::{BitmapError, BitmapFormat, BitmapSettings, ImageData, Origin};
use crate::colors::{Colormap, ColormapError, NormError, ScalarMappable, colormaps};
use crate::rc::ColorOrAuto;

/// Why an image cannot be read or written.
#[derive(Debug)]
pub enum ImageIoError {
    /// The file could not be read or written.
    Io(std::io::Error),
    /// The file is not a valid PNG.
    Decode(DecodeError),
    /// The file is not a valid JPEG.
    Jpeg(jpeg::DecodeError),
    /// The file is not a valid TIFF.
    Tiff(tiff::DecodeError),
    /// The file is not a valid GIF.
    Gif(gif::DecodeError),
    /// The file is not a valid WebP.
    Webp(webp::DecodeError),
    /// The data is not in any format that can be read (Pillow's
    /// `UnidentifiedImageError`).
    UnidentifiedImage,
    /// The pixels could not be encoded.
    Bitmap(BitmapError),
    /// The file extension names a format that cannot be written (anything
    /// but a bitmap format).
    UnsupportedFormat(String),
    /// The default colormap `image.cmap` is unknown.
    Colormap(ColormapError),
    /// The colour limits cannot normalise the data.
    Norm(NormError),
    /// Float colours outside `[0, 1]`.
    ColorRange,
    /// Scalars where colours were expected (they need a colormap).
    ScalarImage,
}

impl fmt::Display for ImageIoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "cannot access the image file: {err}"),
            Self::Decode(err) => write!(f, "cannot read the PNG: {err}"),
            Self::Jpeg(err) => write!(f, "cannot read the JPEG: {err}"),
            Self::Tiff(err) => write!(f, "cannot read the TIFF: {err}"),
            Self::Gif(err) => write!(f, "cannot read the GIF: {err}"),
            Self::Webp(err) => write!(f, "cannot read the WebP: {err}"),
            Self::UnidentifiedImage => f.write_str("cannot identify the image file"),
            Self::Bitmap(err) => write!(f, "cannot write the image: {err}"),
            Self::UnsupportedFormat(format) => write!(
                f,
                "format {format:?} is not supported (imsave writes gif, jpeg, jpg, png, tiff and webp)"
            ),
            Self::Colormap(err) => err.fmt(f),
            Self::Norm(err) => err.fmt(f),
            Self::ColorRange => {
                f.write_str("Floating point image RGB values must be in the 0..1 range.")
            }
            Self::ScalarImage => f.write_str("the image holds scalars, not colours"),
        }
    }
}

impl std::error::Error for ImageIoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Decode(err) => Some(err),
            Self::Jpeg(err) => Some(err),
            Self::Tiff(err) => Some(err),
            Self::Gif(err) => Some(err),
            Self::Webp(err) => Some(err),
            Self::Bitmap(err) => Some(err),
            Self::Colormap(err) => Some(err),
            Self::Norm(err) => Some(err),
            Self::UnidentifiedImage
            | Self::UnsupportedFormat(_)
            | Self::ColorRange
            | Self::ScalarImage => None,
        }
    }
}

impl From<std::io::Error> for ImageIoError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<DecodeError> for ImageIoError {
    fn from(err: DecodeError) -> Self {
        Self::Decode(err)
    }
}

impl From<jpeg::DecodeError> for ImageIoError {
    fn from(err: jpeg::DecodeError) -> Self {
        Self::Jpeg(err)
    }
}

impl From<tiff::DecodeError> for ImageIoError {
    fn from(err: tiff::DecodeError) -> Self {
        Self::Tiff(err)
    }
}

impl From<gif::DecodeError> for ImageIoError {
    fn from(err: gif::DecodeError) -> Self {
        Self::Gif(err)
    }
}

impl From<webp::DecodeError> for ImageIoError {
    fn from(err: webp::DecodeError) -> Self {
        Self::Webp(err)
    }
}

impl From<BitmapError> for ImageIoError {
    fn from(err: BitmapError) -> Self {
        Self::Bitmap(err)
    }
}

impl From<ColormapError> for ImageIoError {
    fn from(err: ColormapError) -> Self {
        Self::Colormap(err)
    }
}

impl From<NormError> for ImageIoError {
    fn from(err: NormError) -> Self {
        Self::Norm(err)
    }
}

/// How [`imsave`] turns an array into pixels.
#[derive(Clone, Debug)]
pub struct ImsaveOptions {
    /// The value mapped to the bottom of the colormap (scalar data; the
    /// data minimum when `None`).
    pub vmin: Option<f64>,
    /// The value mapped to the top of the colormap (scalar data; the data
    /// maximum when `None`).
    pub vmax: Option<f64>,
    /// The colormap of scalar data (`image.cmap` when `None`).
    pub cmap: Option<Colormap>,
    /// Where row 0 of the array goes (`image.origin` when `None`).
    pub origin: Option<Origin>,
    /// The resolution recorded in the file.
    pub dpi: f64,
    /// JPEG quality from 1 to 100 (Pillow's `quality` in matplotlib's
    /// `pil_kwargs`; 75 when `None`).
    pub jpeg_quality: Option<u8>,
}

impl Default for ImsaveOptions {
    fn default() -> Self {
        Self {
            vmin: None,
            vmax: None,
            cmap: None,
            origin: None,
            dpi: 100.0,
            jpeg_quality: None,
        }
    }
}

/// Saves an array as an image file in the format its extension names
/// (matplotlib's `imsave`; see [`imsave_format`]), PNG without one.
/// Scalar data is colour-mapped through `cmap` between `vmin` and `vmax`.
///
/// # Errors
///
/// As [`imsave_bytes`], and when the file cannot be written or its
/// extension names another format.
pub fn imsave(
    path: impl AsRef<Path>,
    data: impl Into<ImageData>,
    options: &ImsaveOptions,
) -> Result<(), ImageIoError> {
    let path = path.as_ref();
    let format = match path.extension() {
        None => BitmapFormat::Png,
        Some(ext) => imsave_format(&ext.to_string_lossy())?,
    };
    std::fs::write(path, imsave_bytes(data, format, options)?)?;
    Ok(())
}

/// The format [`imsave`] writes for a format name or file extension,
/// ignoring case: the names Pillow saves under (`png`, `jpg`, `jpeg`,
/// `tiff`, `gif` and `webp`), so `tif`, `raw` and `rgba` are refused as
/// matplotlib refuses them.
///
/// # Errors
///
/// When the name is none of those.
pub fn imsave_format(name: &str) -> Result<BitmapFormat, ImageIoError> {
    match name.to_lowercase().as_str() {
        "png" => Ok(BitmapFormat::Png),
        "jpg" | "jpeg" => Ok(BitmapFormat::Jpeg),
        "tiff" => Ok(BitmapFormat::Tiff),
        "gif" => Ok(BitmapFormat::Gif),
        "webp" => Ok(BitmapFormat::Webp),
        other => Err(ImageIoError::UnsupportedFormat(other.to_owned())),
    }
}

/// [`imsave_bytes`] as PNG.
///
/// # Errors
///
/// As [`imsave_bytes`].
pub fn imsave_png(
    data: impl Into<ImageData>,
    options: &ImsaveOptions,
) -> Result<Vec<u8>, ImageIoError> {
    imsave_bytes(data, BitmapFormat::Png, options)
}

/// Encodes an array the way matplotlib's `imsave` does: scalars through
/// the colormap (masked values, NaN included, in its "bad" colour, as for
/// a numpy masked array), float colours truncated to bytes, rows flipped
/// for [`Origin::Lower`], and JPEG flattened onto `savefig.facecolor`.
///
/// # Errors
///
/// When the colormap is unknown, the limits cannot normalise the data,
/// float colours leave `[0, 1]`, or the image is empty or too large for
/// the format.
pub fn imsave_bytes(
    data: impl Into<ImageData>,
    format: BitmapFormat,
    options: &ImsaveOptions,
) -> Result<Vec<u8>, ImageIoError> {
    let data = data.into();
    let (rows, cols) = data.shape();
    let mut pixels = rgba_bytes(&data, options)?;
    let origin = options
        .origin
        .unwrap_or_else(|| crate::rc::params().image_origin.parse().unwrap_or_default());
    if origin == Origin::Lower {
        let mut flipped = Vec::with_capacity(pixels.len());
        for row in pixels.chunks_exact(cols.max(1)).rev() {
            flipped.extend_from_slice(row);
        }
        pixels = flipped;
    }
    let defaults = BitmapSettings::default();
    let settings = BitmapSettings {
        dpi: options.dpi,
        jpeg_quality: options.jpeg_quality.unwrap_or(defaults.jpeg_quality),
        background: jpeg_background(),
    };
    Ok(format.encode(cols, rows, &pixels.concat(), &settings)?)
}

/// The colour `imsave` flattens JPEG pixels onto: `savefig.facecolor`, or
/// `figure.facecolor` when that is `"auto"`, each channel truncated to a
/// byte.
fn jpeg_background() -> [u8; 3] {
    let rc = crate::rc::params();
    let color = match &rc.savefig_facecolor {
        ColorOrAuto::Color(color) => color.resolve(),
        ColorOrAuto::Auto => rc.figure_facecolor.resolve(),
    };
    let rgba = color.unwrap_or(Rgba::WHITE);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "colour channels are in [0, 1]"
    )]
    let byte = |v: f64| (v * 255.0) as u8;
    [byte(rgba.r), byte(rgba.g), byte(rgba.b)]
}

/// The 8-bit RGBA pixels of `data`, row-major (matplotlib's
/// `Colorizer.to_rgba(arr, bytes=True)`).
fn rgba_bytes(data: &ImageData, options: &ImsaveOptions) -> Result<Vec<[u8; 4]>, ImageIoError> {
    Ok(match data {
        ImageData::Scalar(field) => {
            let cmap = match &options.cmap {
                Some(cmap) => cmap.clone(),
                None => colormaps::get_cmap(None)?,
            };
            let mut mappable = ScalarMappable::new(cmap);
            mappable.set_clim(options.vmin, options.vmax);
            let values = field.masked_values();
            let normalized = normalize_in(&mut mappable, &values, field.kind(), field.precision())?;
            let cmap = mappable.cmap();
            normalized
                .into_iter()
                .map(|value| cmap.lookup_bytes(value, None))
                .collect()
        }
        colors => color_image_bytes(colors)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use plt_codec::png;

    use crate::numeric::Grid;

    fn decode(png: &[u8]) -> Vec<u8> {
        png::Decoder::new().decode(png).unwrap().to_rgba8()
    }

    #[test]
    fn scalars_are_colour_mapped_between_the_limits() {
        let data = Grid::from_vec(1, 3, vec![0.0, 5.0, f64::NAN]).unwrap();
        let options = ImsaveOptions {
            cmap: Some(colormaps::get("gray").unwrap()),
            vmax: Some(10.0),
            ..ImsaveOptions::default()
        };
        let pixels = decode(&imsave_png(data, &options).unwrap());
        assert_eq!(&pixels[..4], [0, 0, 0, 255]);
        assert_eq!(&pixels[4..8], [128, 128, 128, 255], "5 of 0..10");
        assert_eq!(
            &pixels[8..],
            [0, 0, 0, 0],
            "masked: the transparent bad colour"
        );
    }

    #[test]
    fn colours_are_truncated_and_lower_origin_flips_rows() {
        let data = Grid::from_vec(2, 1, vec![[1.0, 0.5, 0.0], [0.0, 0.0, 1.0]]).unwrap();
        let options = ImsaveOptions {
            origin: Some(Origin::Lower),
            ..ImsaveOptions::default()
        };
        let pixels = decode(&imsave_png(data, &options).unwrap());
        assert_eq!(pixels, [0, 0, 255, 255, 255, 127, 0, 255]);
        let out_of_range = Grid::from_vec(1, 1, vec![[1.5, 0.0, 0.0]]).unwrap();
        assert!(matches!(
            imsave_png(out_of_range, &ImsaveOptions::default()),
            Err(ImageIoError::ColorRange)
        ));
    }

    #[test]
    fn files_are_written_by_extension() {
        let dir = std::env::temp_dir().join(format!("plt-imsave-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let data = Grid::from_vec(1, 1, vec![[0_u8, 128, 255]]).unwrap();
        for (name, signature) in [
            ("tiny.png", &b"\x89PNG"[..]),
            ("tiny.jpeg", b"\xff\xd8\xff"),
            ("tiny.TIFF", b"II*\0"),
            ("tiny.gif", b"GIF89a"),
        ] {
            imsave(dir.join(name), data.clone(), &ImsaveOptions::default()).unwrap();
            assert!(
                std::fs::read(dir.join(name))
                    .unwrap()
                    .starts_with(signature),
                "{name}"
            );
        }
        for refused in ["tif", "rgba", "pdf"] {
            assert!(matches!(
                imsave(dir.join(format!("tiny.{refused}")), data.clone(), &ImsaveOptions::default()),
                Err(ImageIoError::UnsupportedFormat(ext)) if ext == refused
            ));
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
