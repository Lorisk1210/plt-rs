//! Writing figures: matplotlib's `savefig` with the Agg canvas (PNG) and
//! the SVG, PDF and PostScript backends (the latter in `postscript.rs`),
//! chosen from the file extension.

use std::fmt;
use std::path::{Path as FsPath, PathBuf};
use std::str::FromStr;

use plt_core::path::convert::SimplifySettings;
use plt_render::{
    Metadata, PdfOptions, PdfRenderer, RasterBackendError, RasterRenderer, Renderer, VectorError,
};

use super::print::SaveOptions;
use super::{DrawErrors, Figure};
use crate::image::{BitmapError, BitmapFormat, BitmapSettings};
use crate::rc::FontType;

/// Why a figure could not be rendered or saved.
#[derive(Debug)]
pub enum SaveError {
    /// The canvas could not be created or encoded.
    Backend(RasterBackendError),
    /// A vector document could not be written.
    Vector(VectorError),
    /// The canvas could not be encoded as a bitmap file.
    Bitmap(BitmapError),
    /// Some artists could not be drawn.
    Draw(DrawErrors),
    /// The file format is not one plt-rs can write.
    UnsupportedFormat(String),
    /// Writing the file failed.
    Io(std::io::Error),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(err) => write!(f, "cannot render the figure: {err}"),
            Self::Vector(err) => write!(f, "cannot write the document: {err}"),
            Self::Bitmap(err) => write!(f, "cannot write the image: {err}"),
            Self::Draw(err) => err.fmt(f),
            Self::UnsupportedFormat(format) => write!(
                f,
                "format {format:?} is not supported (supported formats: eps, gif, jpeg, jpg, pdf, pgf, png, ps, raw, rgba, svg, svgz, tif, tiff, webp)"
            ),
            Self::Io(err) => write!(f, "cannot write the figure: {err}"),
        }
    }
}

impl std::error::Error for SaveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Backend(err) => Some(err),
            Self::Vector(err) => Some(err),
            Self::Bitmap(err) => Some(err),
            Self::Draw(err) => Some(err),
            Self::Io(err) => Some(err),
            Self::UnsupportedFormat(_) => None,
        }
    }
}

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
            Self::Raw => "raw",
        }
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
            "raw" | "rgba" => Self::Raw,
            _ => return Err(SaveError::UnsupportedFormat(name.to_owned())),
        })
    }
}

/// The settings of a PDF document from the `pdf.*` rcParams, with
/// `metadata`.
pub(super) fn pdf_options(metadata: Metadata) -> PdfOptions {
    let rc = crate::rc::params();
    PdfOptions {
        compression: u8::try_from(rc.pdf_compression.clamp(0, 9)).unwrap_or(6),
        inherit_color: rc.pdf_inheritcolor,
        metadata,
        simplify: simplify_settings(),
        composite_images: rc.image_composite_image,
        fonttype: embedding(rc.pdf_fonttype),
        core_fonts: rc.pdf_use14corefonts,
    }
}

/// How a `pdf.fonttype` or `ps.fonttype` setting embeds fonts.
pub(super) const fn embedding(fonttype: FontType) -> plt_render::FontType {
    match fonttype {
        FontType::Type3 => plt_render::FontType::Type3,
        FontType::TrueType => plt_render::FontType::Type42,
    }
}

/// matplotlib's `path.simplify` settings from the rcParams.
pub(super) fn simplify_settings() -> SimplifySettings {
    let rc = crate::rc::params();
    SimplifySettings {
        enabled: rc.path_simplify,
        threshold: rc.path_simplify_threshold,
    }
}

/// The whole pixels a canvas `size` pixels long holds: truncated, but
/// rounded up within 1e-8 of the next pixel to undo floating-point error
/// (matplotlib's `FigureCanvasBase.get_width_height`).
pub(super) fn whole_pixels(size: f64) -> u32 {
    // The conversion saturates: negative and NaN sizes give 0.
    (size + 1e-8) as u32
}

/// A raster canvas of `width × height` whole pixels at `dpi`, with the
/// current `path.simplify` and `agg.path.chunksize` settings.
pub(super) fn raster_renderer(
    width: u32,
    height: u32,
    dpi: f64,
) -> Result<RasterRenderer, RasterBackendError> {
    // Negative sizes never split, as matplotlib's `npts > nmax > 100`.
    let chunksize = usize::try_from(crate::rc::params().agg_path_chunksize).unwrap_or(0);
    RasterRenderer::new(f64::from(width), f64::from(height), dpi).map(|canvas| {
        canvas
            .with_simplify(simplify_settings())
            .with_path_chunksize(chunksize)
    })
}

impl Figure {
    /// A raster canvas for this figure — [`Figure::width_height`] pixels,
    /// as `FigureCanvasAgg` makes — with the current `path.simplify` and
    /// `agg.path.chunksize` settings, ready to be drawn into.
    pub fn raster_canvas(&self) -> Result<RasterRenderer, SaveError> {
        let (width, height) = self.size_inches();
        Self::raster_canvas_sized(width, height, self.dpi())
    }

    /// A raster canvas of `width × height` inches at `dpi`, in whole
    /// pixels, with the current `path.simplify` and `agg.path.chunksize`
    /// settings.
    pub(crate) fn raster_canvas_sized(
        width: f64,
        height: f64,
        dpi: f64,
    ) -> Result<RasterRenderer, SaveError> {
        raster_renderer(whole_pixels(width * dpi), whole_pixels(height * dpi), dpi)
            .map_err(SaveError::Backend)
    }

    /// Renders the figure with the Agg-compatible raster backend, as
    /// `savefig` does.
    pub fn render_raster(&self) -> Result<RasterRenderer, SaveError> {
        self.render_raster_with(&super::SaveOptions::default())
    }

    /// The figure as PNG bytes (matplotlib's `savefig(format="png")`).
    pub fn to_png(&self) -> Result<Vec<u8>, SaveError> {
        self.render_raster()?.to_png().map_err(SaveError::Backend)
    }

    /// The figure as PNG bytes with save options.
    ///
    /// # Errors
    ///
    /// As [`Figure::render_raster_with`], or when encoding fails.
    pub fn to_png_with(&self, options: &SaveOptions) -> Result<Vec<u8>, SaveError> {
        self.render_raster_with(options)?
            .to_png()
            .map_err(SaveError::Backend)
    }

    /// The figure as a PDF document (matplotlib's `savefig(format="pdf")`).
    pub fn to_pdf(&self) -> Result<Vec<u8>, SaveError> {
        self.to_pdf_with(&SaveOptions::default())
    }

    /// The figure as a PDF document with save options; the `pdf.*`
    /// rcParams configure the document.
    ///
    /// # Errors
    ///
    /// When some artists cannot be drawn or the document cannot be written.
    pub fn to_pdf_with(&self, options: &SaveOptions) -> Result<Vec<u8>, SaveError> {
        let pdf = pdf_options(options.metadata.clone().unwrap_or_default());
        // Text the backend draws in core fonts is laid out with their metrics.
        crate::text::with_afm_metrics(pdf.afm_directory(), || {
            self.render_vector_with(options, |width, height, dpi| {
                PdfRenderer::new(width, height, dpi, pdf)
            })
        })?
        .finish()
        .map_err(SaveError::Vector)
    }

    /// The figure in `format` with save options (matplotlib's
    /// `savefig(buffer, format=...)`).
    ///
    /// # Errors
    ///
    /// When some artists cannot be drawn or the output cannot be encoded.
    pub fn to_format(
        &self,
        format: SaveFormat,
        options: &SaveOptions,
    ) -> Result<Vec<u8>, SaveError> {
        match format {
            SaveFormat::Png => self.to_png_with(options),
            SaveFormat::Svg => self.to_svg_with(options).map(String::into_bytes),
            SaveFormat::Svgz => self
                .to_svg_with(options)
                .map(|svg| plt_render::gzip(svg.as_bytes())),
            SaveFormat::Pdf => self.to_pdf_with(options),
            SaveFormat::Ps => self.to_ps_with(options).map(String::into_bytes),
            SaveFormat::Eps => self.to_eps_with(options).map(String::into_bytes),
            SaveFormat::Pgf => self.to_pgf_with(options).map(String::into_bytes),
            SaveFormat::Jpeg => self.to_bitmap(BitmapFormat::Jpeg, options),
            SaveFormat::Tiff => self.to_bitmap(BitmapFormat::Tiff, options),
            SaveFormat::Gif => self.to_bitmap(BitmapFormat::Gif, options),
            SaveFormat::Webp => self.to_bitmap(BitmapFormat::Webp, options),
            SaveFormat::Raw => self.to_bitmap(BitmapFormat::Raw, options),
        }
    }

    /// The canvas rendered with save options and encoded as `format`
    /// (Agg's `print_raw`, and `print_jpg`, `print_tif` and `print_gif`
    /// through `imsave`): the save resolution recorded, JPEG flattened onto
    /// white.
    fn to_bitmap(&self, format: BitmapFormat, options: &SaveOptions) -> Result<Vec<u8>, SaveError> {
        let canvas = self.render_raster_with(options)?;
        let defaults = BitmapSettings::default();
        let settings = BitmapSettings {
            dpi: canvas.dpi(),
            jpeg_quality: options.jpeg_quality.unwrap_or(defaults.jpeg_quality),
            ..defaults
        };
        let image = canvas.to_image();
        format
            .encode(image.width(), image.height(), image.data(), &settings)
            .map_err(SaveError::Bitmap)
    }

    /// Writes the figure to `path` in the format its extension names
    /// (PNG, JPEG, TIFF, GIF, WebP, raw RGBA, SVG, SVGZ, PDF, PS, EPS or
    /// PGF;
    /// `savefig.format` and an appended extension without one).
    pub fn savefig(&self, path: impl AsRef<FsPath>) -> Result<(), SaveError> {
        self.savefig_with(path, &SaveOptions::default())
    }

    /// Writes the figure to `path` with save options (matplotlib's
    /// `savefig` keywords).
    ///
    /// # Errors
    ///
    /// When the format is unsupported, the figure cannot be rendered or
    /// the file cannot be written.
    pub fn savefig_with(
        &self,
        path: impl AsRef<FsPath>,
        options: &SaveOptions,
    ) -> Result<(), SaveError> {
        let (path, format) = resolve_format(path.as_ref(), options.format)?;
        match format {
            SaveFormat::Svg | SaveFormat::Svgz => {
                return self.save_svg(&path, options, format == SaveFormat::Svgz);
            }
            SaveFormat::Pgf => return self.save_pgf(&path, options),
            _ => {}
        }
        let document = if matches!(format, SaveFormat::Ps | SaveFormat::Eps) {
            let titled = SaveOptions {
                metadata: Some(file_metadata(options, &path)),
                ..options.clone()
            };
            self.to_format(format, &titled)?
        } else {
            self.to_format(format, options)?
        };
        std::fs::write(path, document).map_err(SaveError::Io)
    }
}

/// matplotlib's `print_figure` format resolution: an explicit format, the
/// file name's extension, or `savefig.format` appended to the file name.
fn resolve_format(
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

/// Metadata for a document saved to `path`: the file name as title (as
/// matplotlib's PostScript backend writes it) unless metadata is given.
fn file_metadata(options: &SaveOptions, path: &FsPath) -> Metadata {
    options.metadata.clone().unwrap_or_else(|| Metadata {
        title: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned()),
        ..Metadata::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::BboxInches;
    use plt_core::Bbox;

    #[test]
    fn renders_the_figure_background() {
        let fig = Figure::with_size(1.0, 0.5, 20.0);
        let canvas = fig.render_raster().unwrap();
        let pixmap = canvas.pixmap();
        assert_eq!((pixmap.width(), pixmap.height()), (20, 10));
        assert_eq!(&pixmap.data()[(3 * 20 + 3) * 4..][..4], &[255; 4]);
        let png = fig.to_png().unwrap();
        assert_eq!(&png[1..4], b"PNG");
    }

    #[test]
    fn canvases_round_up_sizes_a_rounding_error_short_of_a_pixel() {
        assert_eq!(whole_pixels(9.2 * 100.0), 920);
        assert_eq!(whole_pixels(255.5), 255);
        assert_eq!((whole_pixels(-1.0), whole_pixels(f64::NAN)), (0, 0));
        let pixmap = Figure::with_size(9.2, 0.1, 100.0)
            .render_raster()
            .unwrap()
            .into_pixmap();
        assert_eq!((pixmap.width(), pixmap.height()), (920, 10));
    }

    #[test]
    fn saving_uses_the_savefig_colours_and_restores_the_figure_ones() {
        crate::rc::context(|| {
            crate::rc::set("figure.facecolor", "0.75").unwrap();
            crate::rc::set("savefig.facecolor", "black").unwrap();
            let fig = Figure::with_size(1.0, 0.5, 20.0);
            let canvas = fig.render_raster().unwrap();
            assert_eq!(&canvas.pixmap().data()[..4], &[0, 0, 0, 255]);
            assert!((fig.facecolor().r - 0.75).abs() < 1e-12);
        });
    }

    #[test]
    fn savefig_reports_io_errors() {
        let fig = Figure::with_size(0.5, 0.5, 10.0);
        let missing = std::env::temp_dir().join("plt-rs-missing-dir/x/figure.png");
        assert!(matches!(fig.savefig(missing), Err(SaveError::Io(_))));
    }

    #[test]
    fn bitmap_formats_record_the_save_resolution() {
        let fig = Figure::with_size(1.0, 0.5, 50.0);
        let options = SaveOptions {
            dpi: Some(30.0),
            ..SaveOptions::default()
        };
        let raw = fig.to_format(SaveFormat::Raw, &options).unwrap();
        assert_eq!(raw.len(), 30 * 15 * 4);
        let jpeg = fig.to_format(SaveFormat::Jpeg, &options).unwrap();
        assert_eq!(&jpeg[6..11], b"JFIF\0");
        assert_eq!(&jpeg[14..18], [0, 30, 0, 30], "JFIF density");
        let tiff = fig.to_format(SaveFormat::Tiff, &options).unwrap();
        assert_eq!(&tiff[190..198], [30, 0, 0, 0, 1, 0, 0, 0], "x resolution");
        let gif = fig.to_format(SaveFormat::Gif, &options).unwrap();
        assert_eq!(&gif[..10], b"GIF89a\x1e\0\x0f\0");
        // WebP is lossless: it decodes to the canvas.
        let webp = fig.to_format(SaveFormat::Webp, &options).unwrap();
        let decoded = plt_codec::webp::decode(&webp).unwrap().to_rgba8().concat();
        assert_eq!(decoded, raw);
    }

    #[test]
    fn formats_come_from_extensions() {
        assert_eq!("SVG".parse::<SaveFormat>().unwrap(), SaveFormat::Svg);
        assert_eq!("jpeg".parse::<SaveFormat>().unwrap(), SaveFormat::Jpeg);
        assert_eq!("rgba".parse::<SaveFormat>().unwrap(), SaveFormat::Raw);
        assert_eq!("WebP".parse::<SaveFormat>().unwrap(), SaveFormat::Webp);
        assert!(matches!(
            "avif".parse::<SaveFormat>(),
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

    #[test]
    fn svg_images_can_go_beside_the_file() {
        let dir = std::env::temp_dir().join(format!("plt-rs-svg-images-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("figure.svg");
        crate::rc::context(|| {
            crate::rc::set("svg.image_inline", "False").unwrap();
            let fig = Figure::new();
            let values = crate::numeric::Grid::from_rows(vec![vec![0.0, 1.0]]).unwrap();
            fig.subplot()
                .imshow(values, crate::axes::ImshowOptions::default())
                .unwrap();
            fig.savefig(&path).unwrap();
        });
        let image = dir.join("figure.svg.image0.png");
        let svg = std::fs::read_to_string(&path).unwrap();
        assert!(svg.contains(&format!("xlink:href=\"{}\"", image.display())));
        assert!(std::fs::read(&image).unwrap().starts_with(b"\x89PNG"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn svg_documents() {
        let fig = Figure::with_size(2.0, 1.0, 50.0);
        let svg = fig.to_svg().unwrap();
        assert!(svg.contains("width=\"144pt\" height=\"72pt\""));
        let svgz = fig
            .to_format(SaveFormat::Svgz, &SaveOptions::default())
            .unwrap();
        assert_eq!(&svgz[..2], &[0x1f, 0x8b]);
        let metadata = file_metadata(&SaveOptions::default(), FsPath::new("dir/a.ps"));
        assert_eq!(metadata.title.as_deref(), Some("a.ps"));
    }

    #[test]
    fn afm_fonts_measure_and_draw_text_when_the_rc_asks() {
        crate::rc::context(|| {
            crate::rc::set("pdf.compression", "0").unwrap();
            let fig = Figure::with_size(2.0, 1.0, 50.0);
            fig.text(0.5, 0.5, "AVATAR")
                .set_halign(crate::text::HorizontalAlignment::Center);
            let embedded = String::from_utf8_lossy(&fig.to_pdf().unwrap()).into_owned();
            assert!(!embedded.contains("/Type1"));
            crate::rc::set("pdf.use14corefonts", "True").unwrap();
            crate::rc::set("ps.useafm", "True").unwrap();
            let pdf = String::from_utf8_lossy(&fig.to_pdf().unwrap()).into_owned();
            // Centred by Helvetica's AFM width: 72 - 36.11 / 2.
            assert!(pdf.contains("53.945 36 Td\n(AVATAR) Tj ET"), "{pdf}");
            assert!(pdf.contains("/Subtype /Type1 /BaseFont /Helvetica"));
            let ps = fig.to_ps().unwrap();
            assert!(ps.contains("/Helvetica 10.000 selectfont"));
            assert!(!ps.contains("/FontName"), "nothing is embedded");
        });
    }

    #[test]
    fn pdf_documents() {
        let fig = Figure::with_size(2.0, 1.0, 50.0);
        let ax = fig.subplot();
        ax.plot(vec![0.0, 1.0], vec![0.0, 1.0]).unwrap();
        let pdf = fig.to_pdf().unwrap();
        assert!(pdf.starts_with(b"%PDF-1.4"));
        let tight = SaveOptions {
            bbox_inches: Some(BboxInches::Inches(Bbox::from_bounds(0.0, 0.0, 1.0, 0.5))),
            ..SaveOptions::default()
        };
        let cropped = String::from_utf8_lossy(&fig.to_pdf_with(&tight).unwrap()).into_owned();
        assert!(cropped.contains("/MediaBox [ 0 0 72 36 ]"));
        let dir = std::env::temp_dir().join("plt-rs-save-formats");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(matches!(
            fig.savefig(dir.join("figure.avif")),
            Err(SaveError::UnsupportedFormat(_))
        ));
    }
}
