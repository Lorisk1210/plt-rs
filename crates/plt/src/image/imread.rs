//! Reading image files into arrays (matplotlib.image's `imread`): PNG as
//! matplotlib's own reader does, JPEG, TIFF and GIF as Pillow opens them
//! (`pil_to_array`).

use std::path::Path;

use plt_codec::png::{self, ColorType};
use plt_codec::raster::{Pixels, Raster};
use plt_codec::{gif, jpeg, tiff};

use super::{ImageData, ImageIoError, ScalarField};
use crate::numeric::Grid;

/// A file format `imread` reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImreadFormat {
    /// Portable Network Graphics.
    Png,
    /// JPEG (JFIF or Adobe).
    Jpeg,
    /// TIFF, classic or `BigTIFF`.
    Tiff,
    /// GIF.
    Gif,
}

impl ImreadFormat {
    /// The format of a file's first bytes, by the signatures Pillow
    /// recognises, or `None` for anything else.
    pub fn sniff(data: &[u8]) -> Option<Self> {
        const TIFF: [&[u8]; 6] = [b"MM\0*", b"II*\0", b"MM*\0", b"II\0*", b"MM\0+", b"II+\0"];
        if data.starts_with(&png::SIGNATURE) {
            Some(Self::Png)
        } else if data.starts_with(b"\xFF\xD8\xFF") {
            Some(Self::Jpeg)
        } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
            Some(Self::Gif)
        } else if TIFF.iter().any(|prefix| data.starts_with(prefix)) {
            Some(Self::Tiff)
        } else {
            None
        }
    }
}

/// Reads an image file into an array (matplotlib's `imread`). As in
/// matplotlib, a `.png` file must be a PNG and is read by
/// [`imread_png`]; any other file is read by [`imread_bytes`], whatever
/// its extension says.
///
/// # Errors
///
/// When the file cannot be read, is not an image of a supported format,
/// or is malformed.
pub fn imread(path: impl AsRef<Path>) -> Result<ImageData, ImageIoError> {
    let path = path.as_ref();
    let data = std::fs::read(path)?;
    let png = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("png"));
    if png {
        imread_png(&data)
    } else {
        imread_bytes(&data)
    }
}

/// Decodes an image file held in memory, recognising its format by its
/// contents as Pillow's `Image.open` does: PNG as [`imread_png`] does, and
/// JPEG, TIFF and GIF into byte arrays as matplotlib's `pil_to_array`
/// returns them — greyscale (8- or 16-bit) as integer scalars, RGB as RGB
/// bytes, and every other mode converted to RGBA bytes (palettes with
/// their transparent index, CMYK, bilevel and grey with alpha).
///
/// ```
/// use plt::image::{ImageData, imread_bytes};
/// use plt_codec::jpeg::{JpegOptions, encode_rgb8};
/// let jpeg = encode_rgb8(2, 2, &[90; 12], &JpegOptions::default())?;
/// let ImageData::RgbBytes(pixels) = imread_bytes(&jpeg)? else { unreachable!() };
/// assert_eq!(pixels.shape(), (2, 2));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When the data is not an image of a supported format, or is malformed.
pub fn imread_bytes(data: &[u8]) -> Result<ImageData, ImageIoError> {
    let raster = match ImreadFormat::sniff(data) {
        Some(ImreadFormat::Png) => return imread_png(data),
        Some(ImreadFormat::Jpeg) => jpeg::decode(data)?,
        Some(ImreadFormat::Tiff) => tiff::decode(data)?,
        Some(ImreadFormat::Gif) => gif::decode(data)?,
        None => return Err(ImageIoError::UnidentifiedImage),
    };
    Ok(pil_to_array(raster))
}

/// matplotlib's `pil_to_array`: modes `L`, `I;16`, `RGB` and `RGBA` as
/// they are, anything else converted to RGBA.
fn pil_to_array(raster: Raster) -> ImageData {
    let shape = (raster.height() as usize, raster.width() as usize);
    match raster.into_pixels() {
        Pixels::Gray(levels) => integers(shape, levels.into_iter().map(i64::from).collect()),
        Pixels::Gray16(levels) => integers(shape, levels.into_iter().map(i64::from).collect()),
        Pixels::Rgb(pixels) => ImageData::RgbBytes(grid(shape, pixels)),
        Pixels::Rgba(pixels) => ImageData::RgbaBytes(grid(shape, pixels)),
        other => ImageData::RgbaBytes(grid(shape, other.to_rgba8())),
    }
}

/// The pixels of a raster as a grid of `(rows, columns)`.
fn grid<T>((rows, cols): (usize, usize), pixels: Vec<T>) -> Grid<T> {
    Grid::from_vec(rows, cols, pixels).expect("a raster fills its grid")
}

/// Integer scalars (a uint8 or uint16 array).
fn integers(shape: (usize, usize), values: Vec<i64>) -> ImageData {
    ImageData::Scalar(ScalarField::integers(&grid(shape, values)))
}

/// Decodes PNG bytes into an image array the way matplotlib's `imread`
/// does: greyscale becomes single-precision scalars in `[0, 1]`, RGB stays
/// RGB and every other format (palette, grey with alpha, RGBA) becomes
/// RGBA, with single-precision channels in `[0, 1]` (numpy's `float32`);
/// 16-bit colour channels keep their high byte, as Pillow does.
///
/// ```
/// use plt::image::{ImageData, ImsaveOptions, imread_png, imsave_png};
/// use plt::numeric::Grid;
/// let gray = Grid::from_vec(1, 2, vec![0.0, 1.0])?;
/// let png = imsave_png(gray, &ImsaveOptions::default())?;
/// let ImageData::RgbaF32(pixels) = imread_png(&png)? else { unreachable!() };
/// assert_eq!(pixels.shape(), (1, 2));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When the bytes are not a valid PNG.
pub fn imread_png(png: &[u8]) -> Result<ImageData, ImageIoError> {
    let image = png::Decoder::new().decode(png)?;
    let (rows, cols) = (image.height() as usize, image.width() as usize);
    let rgba = image.to_rgba16();
    let pixel = |row: usize, col: usize| -> [u16; 4] {
        let start = (row * cols + col) * 4;
        [
            rgba[start],
            rgba[start + 1],
            rgba[start + 2],
            rgba[start + 3],
        ]
    };
    let byte_unit = |value: u16| unit(value >> 8, 255.0);
    Ok(match image.color_type() {
        ColorType::Gray => {
            let levels = Grid::from_fn(rows, cols, |row, col| unit(pixel(row, col)[0], 65535.0));
            ImageData::Scalar(ScalarField::single(&levels))
        }
        ColorType::Rgb => Grid::from_fn(rows, cols, |row, col| {
            let [r, g, b, _] = pixel(row, col);
            [r, g, b].map(byte_unit)
        })
        .into(),
        ColorType::Rgba | ColorType::GrayAlpha | ColorType::Indexed => {
            Grid::from_fn(rows, cols, |row, col| pixel(row, col).map(byte_unit)).into()
        }
    })
}

/// `value / max` in single precision, as matplotlib's float32 arrays.
fn unit(value: u16, max: f32) -> f32 {
    f32::from(value) / max
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::{ImsaveOptions, imsave};
    use plt_codec::png::{BitDepth, Encoder, ImageRef};

    #[test]
    fn png_formats_read_like_matplotlib() {
        let gray = Encoder::new()
            .encode(ImageRef::new(2, 1, ColorType::Gray, BitDepth::Eight, &[0, 51]).unwrap())
            .unwrap();
        let ImageData::Scalar(field) = imread_png(&gray).unwrap() else {
            panic!("greyscale reads as scalars");
        };
        assert_eq!(
            field.values().as_slice(),
            [0.0, f64::from(51.0_f32 / 255.0)]
        );
        assert_eq!(field.kind(), crate::image::ScalarKind::Float32);
        let rgba = [255, 0, 0, 255, 0, 0, 255, 51];
        let png = Encoder::new()
            .encode(ImageRef::rgba8(2, 1, &rgba).unwrap())
            .unwrap();
        let ImageData::RgbaF32(pixels) = imread_png(&png).unwrap() else {
            panic!("RGBA reads as RGBA");
        };
        assert_eq!(pixels.as_slice()[1][3], 51.0_f32 / 255.0);
        assert!(imread_png(b"not a png").is_err());
        assert!(matches!(imread_bytes(&png).unwrap(), ImageData::RgbaF32(_)));
    }

    #[test]
    fn formats_are_recognised_by_their_contents() {
        assert_eq!(
            ImreadFormat::sniff(b"\xff\xd8\xff\xe0"),
            Some(ImreadFormat::Jpeg)
        );
        assert_eq!(
            ImreadFormat::sniff(b"MM\0*\0\0\0\x08"),
            Some(ImreadFormat::Tiff)
        );
        assert_eq!(ImreadFormat::sniff(b"GIF87a"), Some(ImreadFormat::Gif));
        assert_eq!(ImreadFormat::sniff(b"BM"), None);
        assert!(matches!(
            imread_bytes(b"BM\0\0"),
            Err(ImageIoError::UnidentifiedImage)
        ));
        assert!(matches!(
            imread_bytes(b"\xff\xd8\xff\xd9"),
            Err(ImageIoError::Jpeg(_))
        ));
    }

    #[test]
    fn modes_become_arrays_like_pil_to_array() {
        let gif = plt_codec::gif::GifEncoder::new()
            .encode(
                2,
                1,
                &[plt_codec::gif::GifFrame {
                    pixels: &[0, 0, 0, 0, 10, 20, 30, 255],
                    delay: 1,
                }],
            )
            .unwrap();
        let ImageData::RgbaBytes(pixels) = imread_bytes(&gif).unwrap() else {
            panic!("palette images become RGBA");
        };
        assert_eq!(pixels.as_slice()[0][3], 0, "the transparent index");
        assert_eq!(pixels.as_slice()[1], [10, 20, 30, 255]);
        let tiff = plt_codec::tiff::encode_rgba8(1, 1, &[1, 2, 3, 4], 72.0).unwrap();
        let ImageData::RgbaBytes(pixels) = imread_bytes(&tiff).unwrap() else {
            panic!("RGBA stays RGBA");
        };
        assert_eq!(pixels.as_slice(), [[1, 2, 3, 4]]);
    }

    #[test]
    fn files_are_read_by_extension_then_contents() {
        let dir = std::env::temp_dir().join(format!("plt-imread-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let data = Grid::from_vec(1, 1, vec![[0_u8, 128, 255]]).unwrap();
        imsave(
            dir.join("tiny.png"),
            data.clone(),
            &ImsaveOptions::default(),
        )
        .unwrap();
        let ImageData::RgbaF32(pixels) = imread(dir.join("tiny.png")).unwrap() else {
            panic!("imsave writes RGBA");
        };
        assert_eq!(pixels.as_slice()[0], [0.0, 128.0_f32 / 255.0, 1.0, 1.0]);
        imsave(
            dir.join("tiny.jpeg"),
            data.clone(),
            &ImsaveOptions::default(),
        )
        .unwrap();
        assert!(matches!(
            imread(dir.join("tiny.jpeg")).unwrap(),
            ImageData::RgbBytes(_)
        ));
        // A PNG under another name is still a PNG, but a ".png" must be one.
        std::fs::copy(dir.join("tiny.png"), dir.join("renamed.gif")).unwrap();
        assert!(matches!(
            imread(dir.join("renamed.gif")).unwrap(),
            ImageData::RgbaF32(_)
        ));
        std::fs::copy(dir.join("tiny.jpeg"), dir.join("renamed.png")).unwrap();
        assert!(matches!(
            imread(dir.join("renamed.png")),
            Err(ImageIoError::Decode(_))
        ));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
