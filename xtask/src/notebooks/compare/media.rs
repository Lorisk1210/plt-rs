//! The media outputs and messages carry: images, compared by their pixels
//! with matplotlib's RMS, and SVG and PDF documents, compared by their
//! drawing items as the spec's vector comparison compares them.

use pltrs_codec::{base64, jpeg, png};
use pltrs_spec::figures::compare_rgba;
use pltrs_spec::figures::vector::{VectorFormat, compare_items};

/// The largest RMS difference between two images that are the same (the
/// spec's tolerance, as `cargo xtask diff`'s).
const MAX_RMS: f64 = 1.0;

/// The start of the base64 text of every PNG file (its signature).
const PNG_BASE64: &str = "iVBORw0KGgo";

/// The images a text embeds, with their formats.
type Images = Vec<(ImageFormat, Vec<u8>)>;

/// The raster formats the kernel sends images in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ImageFormat {
    /// PNG.
    Png,
    /// JPEG.
    Jpeg,
}

impl ImageFormat {
    /// The format of the mime type `mime` (`image/png`), if it is one.
    pub(super) fn of_mime(mime: &str) -> Option<Self> {
        match mime {
            "image/png" => Some(Self::Png),
            "image/jpeg" => Some(Self::Jpeg),
            _ => None,
        }
    }

    /// The format of the file `bytes` hold, by its signature.
    pub(super) fn of_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(&png::SIGNATURE) {
            Some(Self::Png)
        } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
            Some(Self::Jpeg)
        } else {
            None
        }
    }

    /// The width, height and RGBA pixels of the image `bytes`.
    fn decode(self, bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
        match self {
            Self::Png => {
                let image = png::decode(bytes).map_err(|err| format!("PNG: {err}"))?;
                Ok((image.width(), image.height(), image.to_rgba8()))
            }
            Self::Jpeg => {
                let image = jpeg::decode(bytes).map_err(|err| format!("JPEG: {err}"))?;
                Ok((
                    image.width(),
                    image.height(),
                    image.to_rgba8().into_iter().flatten().collect(),
                ))
            }
        }
    }
}

/// Whether the images `expected` and `actual` of `format` are the same:
/// of one size, and within [`MAX_RMS`] of each other.
pub(super) fn compare_images(
    format: ImageFormat,
    expected: &[u8],
    actual: &[u8],
) -> Result<(), String> {
    let (width, height, expected) = format.decode(expected)?;
    let (actual_width, actual_height, actual) = format.decode(actual)?;
    if (width, height) != (actual_width, actual_height) {
        return Err(format!(
            "image of {actual_width}x{actual_height} pixels, not {width}x{height}"
        ));
    }
    let report = compare_rgba(&actual, &expected);
    if report.rms > MAX_RMS {
        return Err(format!("image {report}"));
    }
    Ok(())
}

/// Whether the documents `expected` and `actual` of `format` (SVG, PDF)
/// draw the same items, numbers within their formatting precision.
pub(super) fn compare_documents(
    format: VectorFormat,
    expected: &[u8],
    actual: &[u8],
) -> Result<(), String> {
    let expected = format
        .items(expected)
        .map_err(|err| format!("matplotlib's {format}: {err}"))?;
    let actual = format
        .items(actual)
        .map_err(|err| format!("pltrs' {format}: {err}"))?;
    let report = compare_items(&expected, &actual);
    if report.is_structurally_equal() {
        return Ok(());
    }
    let details = report.details(2);
    Err(format!(
        "{format} {report}: {}",
        details.split_whitespace().collect::<Vec<_>>().join(" ")
    ))
}

/// The bytes of the base64 text `text`, whose lines may be broken (with
/// a backslash before the line break, in a JavaScript string).
pub(super) fn decoded_base64(text: &str) -> Result<Vec<u8>, String> {
    let joined: String = text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '\\')
        .collect();
    base64::decode(&joined).map_err(|err| format!("base64: {err}"))
}

/// The raster images embedded in `text`: its data URLs of PNG and JPEG
/// images, or the whole text if it is the base64 of a PNG file (ipympl's
/// `_data_url`); and `text` with each replaced by `<image>`.
pub(super) fn embedded_images(text: &str) -> Result<(String, Images), String> {
    if text.starts_with(PNG_BASE64) && text.chars().all(is_base64) {
        return Ok((
            "<image>".to_owned(),
            vec![(ImageFormat::Png, decoded_base64(text)?)],
        ));
    }
    let mut images = Vec::new();
    let mut rest_of_text = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("data:image/") {
        let (before, url) = rest.split_at(start);
        rest_of_text.push_str(before);
        let format = [("png", ImageFormat::Png), ("jpeg", ImageFormat::Jpeg)]
            .into_iter()
            .find_map(|(name, format)| {
                url.strip_prefix("data:image/")?
                    .strip_prefix(name)?
                    .strip_prefix(";base64,")
                    .map(|data| (format, data))
            });
        let Some((format, data)) = format else {
            rest_of_text.push_str("data:image/");
            rest = &url["data:image/".len()..];
            continue;
        };
        let length = data
            .char_indices()
            .find(|(_, c)| !is_base64(*c) && !matches!(c, '\\' | '\n'))
            .map_or(data.len(), |(at, _)| at);
        images.push((format, decoded_base64(&data[..length])?));
        rest_of_text.push_str("<image>");
        rest = &data[length..];
    }
    rest_of_text.push_str(rest);
    Ok((rest_of_text, images))
}

/// Whether `c` belongs to base64's alphabet.
fn is_base64(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=')
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A PNG of `width` x 1 pixels of the grey `level`.
    fn grey_png(width: u32, level: u8) -> Vec<u8> {
        let pixels = vec![level; width as usize * 4];
        png::encode(png::ImageRef::rgba8(width, 1, &pixels).unwrap()).unwrap()
    }

    #[test]
    fn images_compare_by_size_and_rms() {
        let image = grey_png(3, 100);
        assert_eq!(ImageFormat::of_bytes(&image), Some(ImageFormat::Png));
        assert!(compare_images(ImageFormat::Png, &image, &grey_png(3, 100)).is_ok());
        assert!(compare_images(ImageFormat::Png, &image, &grey_png(3, 101)).is_ok());
        let far = compare_images(ImageFormat::Png, &image, &grey_png(3, 110)).unwrap_err();
        assert!(far.starts_with("image rms"), "{far}");
        let wide = compare_images(ImageFormat::Png, &image, &grey_png(4, 100)).unwrap_err();
        assert_eq!(wide, "image of 4x1 pixels, not 3x1");
    }

    #[test]
    fn embedded_images_are_taken_out_of_text() {
        let image = base64::encode(&grey_png(2, 7));
        let broken = format!("{}\\\n{}", &image[..10], &image[10..]);
        let text = format!(
            "<img src='data:image/png;base64,{image}'/> f = \"data:image/png;base64,{broken}\" data:image/gif;"
        );
        let (rest, images) = embedded_images(&text).unwrap();
        assert_eq!(rest, "<img src='<image>'/> f = \"<image>\" data:image/gif;");
        assert_eq!(images.len(), 2);
        assert_eq!(images[0], images[1]);
        let (rest, images) = embedded_images(&image).unwrap();
        assert_eq!((rest.as_str(), images.len()), ("<image>", 1));
        let (rest, images) = embedded_images("plain").unwrap();
        assert_eq!((rest.as_str(), images.len()), ("plain", 0));
    }

    #[test]
    fn documents_compare_by_items() {
        let svg = |width: &str| {
            format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}pt\" height=\"10pt\"><path d=\"M 0 0 L 1 1\"/></svg>"
            )
        };
        let same = compare_documents(
            VectorFormat::Svg,
            svg("10").as_bytes(),
            svg("10").as_bytes(),
        );
        assert!(same.is_ok(), "{same:?}");
    }
}
