//! The file formats figures can be saved in, and how a save picks one
//! (matplotlib's `print_figure` format resolution).

use std::path::{Path as FsPath, PathBuf};
use std::str::FromStr;

use super::SaveError;
use crate::image::BitmapFormat;

/// The file formats figures can be saved in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SaveFormat {
    /// Portable Network Graphics (the Agg canvas).
    Png,
    /// Scalable Vector Graphics.
    Svg,
    /// Gzip-compressed SVG.
    Svgz,
    /// Portable Document Format.
    Pdf,
    /// PostScript, centred on the `ps.papersize` page.
    Ps,
    /// Encapsulated PostScript, sized to the figure.
    Eps,
    /// A LaTeX PGF picture.
    Pgf,
    /// JPEG, flattened onto white (`jpg` or `jpeg`).
    Jpeg,
    /// Uncompressed TIFF (`tif` or `tiff`).
    Tiff,
    /// A single-frame GIF.
    Gif,
    /// Lossless WebP.
    Webp,
    /// AVIF: AV1 images, coded by pltrs' own encoder.
    Avif,
    /// The bare RGBA bytes of the canvas (`raw` or `rgba`).
    Raw,
}

impl SaveFormat {
    /// The file extension (without a dot).
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Svg => "svg",
            Self::Svgz => "svgz",
            Self::Pdf => "pdf",
            Self::Ps => "ps",
            Self::Eps => "eps",
            Self::Pgf => "pgf",
            Self::Jpeg => "jpg",
            Self::Tiff => "tif",
            Self::Gif => "gif",
            Self::Webp => "webp",
            Self::Avif => "avif",
            Self::Raw => "raw",
        }
    }

    /// A file name in this format for a figure whose window is titled
    /// `title` (matplotlib's `get_default_filename`): the title with the
    /// characters file systems refuse, and spaces, replaced by `_`, or
    /// `image` for an empty title.
    ///
    /// ```
    /// use pltrs::figure::SaveFormat;
    /// assert_eq!(SaveFormat::Png.file_name_for("Figure 1"), "Figure_1.png");
    /// assert_eq!(SaveFormat::Pdf.file_name_for("0:1/2<3"), "0_1_2_3.pdf");
    /// assert_eq!(SaveFormat::Svg.file_name_for(""), "image.svg");
    /// ```
    pub fn file_name_for(self, title: &str) -> String {
        let base: String = title
            .chars()
            .map(|c| if "<>:\"/\\|?*\0 ".contains(c) { '_' } else { c })
            .collect();
        let base = if base.is_empty() { "image" } else { &base };
        format!("{base}.{}", self.extension())
    }

    /// Whether the format holds document metadata (matplotlib's
    /// `savefig(metadata=...)`): the vector formats and PNG.
    pub const fn takes_metadata(self) -> bool {
        !matches!(
            self,
            Self::Jpeg | Self::Tiff | Self::Gif | Self::Webp | Self::Avif | Self::Raw
        )
    }

    /// The bitmap encoding of the raster formats, `None` for the vector
    /// formats.
    pub const fn bitmap(self) -> Option<BitmapFormat> {
        match self {
            Self::Png => Some(BitmapFormat::Png),
            Self::Jpeg => Some(BitmapFormat::Jpeg),
            Self::Tiff => Some(BitmapFormat::Tiff),
            Self::Gif => Some(BitmapFormat::Gif),
            Self::Webp => Some(BitmapFormat::Webp),
            Self::Avif => Some(BitmapFormat::Avif),
            Self::Raw => Some(BitmapFormat::Raw),
            Self::Svg | Self::Svgz | Self::Pdf | Self::Ps | Self::Eps | Self::Pgf => None,
        }
    }
}

impl FromStr for SaveFormat {
    type Err = SaveError;

    /// Parses a format name or extension, ignoring case.
    fn from_str(name: &str) -> Result<Self, SaveError> {
        Ok(match name.to_ascii_lowercase().as_str() {
            "png" => Self::Png,
            "svg" => Self::Svg,
            "svgz" => Self::Svgz,
            "pdf" => Self::Pdf,
            "ps" => Self::Ps,
            "eps" => Self::Eps,
            "pgf" => Self::Pgf,
            "jpg" | "jpeg" => Self::Jpeg,
            "tif" | "tiff" => Self::Tiff,
            "gif" => Self::Gif,
            "webp" => Self::Webp,
            "avif" => Self::Avif,
            "raw" | "rgba" => Self::Raw,
            _ => return Err(SaveError::UnsupportedFormat(name.to_owned())),
        })
    }
}

/// matplotlib's `print_figure` format resolution: an explicit format, the
/// file name's extension, or `savefig.format` appended to the file name.
pub(super) fn resolve_format(
    path: &FsPath,
    format: Option<SaveFormat>,
) -> Result<(PathBuf, SaveFormat), SaveError> {
    if let Some(format) = format {
        return Ok((path.to_owned(), format));
    }
    match path.extension().and_then(|e| e.to_str()) {
        Some(extension) if !extension.is_empty() => Ok((path.to_owned(), extension.parse()?)),
        _ => {
            let format: SaveFormat = crate::rc::params().savefig_format.parse()?;
            let name = path.to_string_lossy();
            let stem = name.trim_end_matches('.');
            Ok((
                PathBuf::from(format!("{stem}.{}", format.extension())),
                format,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_come_from_extensions() {
        assert_eq!("SVG".parse::<SaveFormat>().unwrap(), SaveFormat::Svg);
        assert_eq!("jpeg".parse::<SaveFormat>().unwrap(), SaveFormat::Jpeg);
        assert_eq!("rgba".parse::<SaveFormat>().unwrap(), SaveFormat::Raw);
        assert_eq!("WebP".parse::<SaveFormat>().unwrap(), SaveFormat::Webp);
        assert_eq!("AVIF".parse::<SaveFormat>().unwrap(), SaveFormat::Avif);
        assert!(matches!(
            "heic".parse::<SaveFormat>(),
            Err(SaveError::UnsupportedFormat(_))
        ));
        let (path, format) = resolve_format(FsPath::new("out/plot.pdf"), None).unwrap();
        assert_eq!(
            (path, format),
            (PathBuf::from("out/plot.pdf"), SaveFormat::Pdf)
        );
        let (path, format) = resolve_format(FsPath::new("plot"), None).unwrap();
        assert_eq!((path, format), (PathBuf::from("plot.png"), SaveFormat::Png));
        let (_, format) = resolve_format(FsPath::new("plot.png"), Some(SaveFormat::Eps)).unwrap();
        assert_eq!(format, SaveFormat::Eps);
    }
}
