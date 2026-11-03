//! Writing images as PNG, JPEG, TIFF, GIF, WebP or raw files (matplotlib.image's
//! `imsave`), and the errors of reading and writing them.

use std::fmt;
use std::path::Path;

use pltrs_codec::png::DecodeError;
use pltrs_codec::{gif, jpeg, tiff, webp};
use pltrs_core::Rgba;
use pltrs_render::PngMetadata;

use super::color_image::color_image_bytes;
use super::normalize::normalize_in;
use super::{BitmapError, BitmapFormat, BitmapSettings, ImageData, Origin};
use crate::colors::{ColorizerError, ColorizerOptions, NormError, impl_colorize};
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
    /// Metadata was given for a format that cannot hold it (any but PNG).
    MetadataUnsupported(BitmapFormat),
    /// The default colormap `image.cmap` is unknown, or a norm was given
    /// with limits.
    Colorizer(ColorizerError),
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
            Self::MetadataUnsupported(format) => {
                write!(f, "metadata not supported for format {:?}", format.name())
            }
            Self::Colorizer(err) => err.fmt(f),
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
            Self::Colorizer(err) => Some(err),
            Self::Norm(err) => Some(err),
            Self::UnidentifiedImage
            | Self::UnsupportedFormat(_)
            | Self::MetadataUnsupported(_)
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

impl From<ColorizerError> for ImageIoError {
    fn from(err: ColorizerError) -> Self {
        Self::Colorizer(err)
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
    /// The colormap and the norm or limits of scalar data (matplotlib's
    /// `cmap`, `vmin` and `vmax`; a norm excludes limits).
    pub colorizer: ColorizerOptions,
    /// Where row 0 of the array goes (`image.origin` when `None`).
    pub origin: Option<Origin>,
    /// The resolution recorded in the file.
    pub dpi: f64,
    /// JPEG quality from 1 to 100 (Pillow's `quality` in matplotlib's
    /// `pil_kwargs`; 75 when `None`).
    pub jpeg_quality: Option<u8>,
    /// AVIF quality from 0 to 100, 100 being lossless (Pillow's `quality`
    /// in matplotlib's `pil_kwargs`; 75 when `None`).
    pub avif_quality: Option<u8>,
    /// The text entries of a PNG (matplotlib's `metadata`): `None` writes
    /// the `Software` entry naming pltrs, as `savefig` does; build on
    /// [`PngMetadata::default`] to add or drop entries. Only PNG files
    /// hold metadata.
    pub metadata: Option<PngMetadata>,
}

impl_colorize!(ImsaveOptions);

impl Default for ImsaveOptions {
    fn default() -> Self {
        Self {
            colorizer: ColorizerOptions::default(),
            origin: None,
            dpi: 100.0,
            jpeg_quality: None,
            avif_quality: None,
            metadata: None,
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
    options: ImsaveOptions,
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
/// `tiff`, `gif`, `webp` and `avif`), so `tif`, `raw` and `rgba` are
/// refused as matplotlib refuses them.
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
        "avif" => Ok(BitmapFormat::Avif),
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
    options: ImsaveOptions,
) -> Result<Vec<u8>, ImageIoError> {
    imsave_bytes(data, BitmapFormat::Png, options)
}

/// Encodes an array the way matplotlib's `imsave` does: scalars through
/// the colormap (masked values, NaN included, in its "bad" colour, as for
/// a numpy masked array), float colours truncated to bytes, rows flipped
/// for [`Origin::Lower`], JPEG flattened onto `savefig.facecolor`, and
/// PNG text entries from the metadata.
///
/// ```
/// use pltrs::figure::PngMetadata;
/// use pltrs::image::{BitmapFormat, ImsaveOptions, imsave_bytes};
/// use pltrs::numeric::Grid;
///
/// let pixels = Grid::from_vec(1, 2, vec![[0_u8, 128, 255], [255, 0, 0]])?;
/// // matplotlib's `imsave(..., metadata={"Title": "Ramp"})`.
/// let options = ImsaveOptions {
///     metadata: Some(PngMetadata::default().with("Title", "Ramp")),
///     ..ImsaveOptions::default()
/// };
/// let png = imsave_bytes(pixels, BitmapFormat::Png, options)?;
/// let decoded = pltrs_codec::png::decode(&png)?;
/// let keys: Vec<&str> =
///     decoded.metadata().text.iter().map(|entry| entry.keyword.as_str()).collect();
/// assert_eq!(keys, ["Software", "Title"]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When the colormap is unknown, the limits cannot normalise the data,
/// float colours leave `[0, 1]`, the image is empty or too large for the
/// format, metadata is given for a format other than PNG, or a metadata
/// keyword is not a valid PNG keyword.
pub fn imsave_bytes(
    data: impl Into<ImageData>,
    format: BitmapFormat,
    options: ImsaveOptions,
) -> Result<Vec<u8>, ImageIoError> {
    if options.metadata.is_some() && format != BitmapFormat::Png {
        return Err(ImageIoError::MetadataUnsupported(format));
    }
    let data = data.into();
    let (rows, cols) = data.shape();
    let mut pixels = rgba_bytes(&data, &options)?;
    let origin = options
        .origin
        .unwrap_or_else(|| crate::rc::params().image_origin);
    if origin == Origin::Lower {
        let mut flipped = Vec::with_capacity(pixels.len());
        for row in pixels.chunks_exact(cols.max(1)).rev() {
            flipped.extend_from_slice(row);
        }
        pixels = flipped;
    }
    let settings = BitmapSettings {
        dpi: options.dpi,
        jpeg_quality: options
            .jpeg_quality
            .unwrap_or(BitmapSettings::default().jpeg_quality),
        avif_quality: options
            .avif_quality
            .unwrap_or(BitmapSettings::default().avif_quality),
        background: jpeg_background(),
        png_text: options.metadata.unwrap_or_default(),
    };
    Ok(format.encode(cols, rows, &pixels.concat(), settings)?)
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
    let byte = |v: f64| (v * 255.0) as u8;
    [byte(rgba.r), byte(rgba.g), byte(rgba.b)]
}

/// The 8-bit RGBA pixels of `data`, row-major (matplotlib's
/// `Colorizer.to_rgba(arr, bytes=True)`).
fn rgba_bytes(data: &ImageData, options: &ImsaveOptions) -> Result<Vec<[u8; 4]>, ImageIoError> {
    Ok(match data {
        ImageData::Scalar(field) => {
            let mut mappable = options
                .colorizer
                .clone()
                .into_mappable(&crate::rc::params())?;
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
    use pltrs_codec::png;

    use crate::colors::Colorize;
    use crate::numeric::Grid;

    fn decode(png: &[u8]) -> Vec<u8> {
        png::Decoder::new().decode(png).unwrap().to_rgba8()
    }

    #[test]
    fn scalars_are_colour_mapped_between_the_limits() {
        let data = Grid::from_vec(1, 3, vec![0.0, 5.0, f64::NAN]).unwrap();
        let options = ImsaveOptions {
            colorizer: ColorizerOptions::default()
                .cmap(crate::colors::colormaps::get("gray").unwrap())
                .limits(None, Some(10.0)),
            ..ImsaveOptions::default()
        };
        let pixels = decode(&imsave_png(data, options).unwrap());
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
        let pixels = decode(&imsave_png(data, options).unwrap());
        assert_eq!(pixels, [0, 0, 255, 255, 255, 127, 0, 255]);
        let out_of_range = Grid::from_vec(1, 1, vec![[1.5, 0.0, 0.0]]).unwrap();
        assert!(matches!(
            imsave_png(out_of_range, ImsaveOptions::default()),
            Err(ImageIoError::ColorRange)
        ));
    }

    #[test]
    fn png_metadata_names_pltrs_unless_replaced() {
        let data = Grid::from_vec(1, 1, vec![[0_u8, 128, 255]]).unwrap();
        let text = |metadata: Option<PngMetadata>| {
            let options = ImsaveOptions {
                metadata,
                ..ImsaveOptions::default()
            };
            let png = imsave_png(data.clone(), options).unwrap();
            png::decode(&png)
                .unwrap()
                .metadata()
                .text
                .iter()
                .map(|entry| (entry.keyword.clone(), entry.text.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            text(None),
            [("Software".to_owned(), pltrs_render::PNG_SOFTWARE.to_owned())]
        );
        assert_eq!(text(Some(PngMetadata::none())), []);
        let options = ImsaveOptions {
            metadata: Some(PngMetadata::default()),
            ..ImsaveOptions::default()
        };
        assert!(matches!(
            imsave_bytes(data, BitmapFormat::Jpeg, options),
            Err(ImageIoError::MetadataUnsupported(BitmapFormat::Jpeg))
        ));
    }

    #[test]
    fn files_are_written_by_extension() {
        let dir = std::env::temp_dir().join(format!("pltrs-imsave-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let data = Grid::from_vec(1, 1, vec![[0_u8, 128, 255]]).unwrap();
        for (name, signature) in [
            ("tiny.png", &b"\x89PNG"[..]),
            ("tiny.jpeg", b"\xff\xd8\xff"),
            ("tiny.TIFF", b"II*\0"),
            ("tiny.gif", b"GIF89a"),
        ] {
            imsave(dir.join(name), data.clone(), ImsaveOptions::default()).unwrap();
            assert!(
                std::fs::read(dir.join(name))
                    .unwrap()
                    .starts_with(signature),
                "{name}"
            );
        }
        for refused in ["tif", "rgba", "pdf"] {
            assert!(matches!(
                imsave(dir.join(format!("tiny.{refused}")), data.clone(), ImsaveOptions::default()),
                Err(ImageIoError::UnsupportedFormat(ext)) if ext == refused
            ));
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
