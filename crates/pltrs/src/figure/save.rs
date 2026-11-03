//! Writing figures: matplotlib's `savefig` with the Agg canvas (PNG, and
//! JPEG, TIFF, GIF, WebP, AVIF and raw RGBA through the bitmap encoders)
//! and the
//! SVG, PDF, PostScript and PGF backends (the latter two in `postscript`
//! and `pgf.rs`), chosen from the file extension. The formats are in
//! `format`, the renderer settings taken from the rcParams in `settings`.

mod error;
mod format;
mod settings;

pub use error::SaveError;
pub use format::SaveFormat;
pub(super) use settings::{
    embedding, pdf_options, raster_renderer, simplify_settings, whole_pixels,
};

use std::path::Path as FsPath;

use pltrs_render::{Metadata, PdfRenderer, PngMetadata, RasterRenderer, Renderer};

use super::Figure;
use super::print::SaveOptions;
use crate::image::{BitmapFormat, BitmapSettings};
use format::resolve_format;

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
        self.render_raster_with(super::SaveOptions::default())
    }

    /// The figure as PNG bytes (matplotlib's `savefig(format="png")`),
    /// with matplotlib's `Software` text entry.
    pub fn to_png(&self) -> Result<Vec<u8>, SaveError> {
        self.to_png_with(SaveOptions::default())
    }

    /// The figure as PNG bytes with save options; the PNG entries of the
    /// metadata ([`Metadata::png`], matplotlib's `Software` entry when no
    /// metadata is given) become text chunks.
    ///
    /// ```
    /// use pltrs::figure::{Figure, Metadata, PngMetadata, SaveOptions};
    /// let fig = Figure::with_size(1.0, 1.0, 10.0);
    /// // matplotlib's `savefig(..., metadata={"Title": "Signal"})`.
    /// let options = SaveOptions {
    ///     metadata: Some(Metadata {
    ///         png: PngMetadata::default().with("Title", "Signal"),
    ///         ..Metadata::default()
    ///     }),
    ///     ..SaveOptions::default()
    /// };
    /// let png = pltrs_codec::png::decode(&fig.to_png_with(options)?)?;
    /// let keywords: Vec<&str> =
    ///     png.metadata().text.iter().map(|entry| entry.keyword.as_str()).collect();
    /// assert_eq!(keywords, ["Software", "Title"]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// As [`Figure::render_raster_with`], or when encoding fails, a PNG
    /// keyword included (1 to 79 printable Latin-1 characters).
    pub fn to_png_with(&self, options: SaveOptions) -> Result<Vec<u8>, SaveError> {
        let text = options
            .metadata
            .as_ref()
            .map_or_else(PngMetadata::default, |metadata| metadata.png.clone());
        self.render_raster_with(options)?
            .to_png_with(&text)
            .map_err(SaveError::Backend)
    }

    /// The figure as a PDF document (matplotlib's `savefig(format="pdf")`).
    pub fn to_pdf(&self) -> Result<Vec<u8>, SaveError> {
        self.to_pdf_with(SaveOptions::default())
    }

    /// The figure as a PDF document with save options; the `pdf.*`
    /// rcParams configure the document.
    ///
    /// # Errors
    ///
    /// When some artists cannot be drawn or the document cannot be written.
    pub fn to_pdf_with(&self, options: SaveOptions) -> Result<Vec<u8>, SaveError> {
        let pdf = pdf_options(options.metadata.clone().unwrap_or_default());
        // Text the backend draws in core fonts is laid out with their metrics.
        crate::text::with_afm_metrics(pdf.afm_directory(), || {
            self.render_vector_with(&options, |width, height, dpi| {
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
    /// When some artists cannot be drawn, the output cannot be encoded or
    /// metadata is given for a format that cannot hold it.
    pub fn to_format(
        &self,
        format: SaveFormat,
        options: SaveOptions,
    ) -> Result<Vec<u8>, SaveError> {
        if options.metadata.is_some() && !format.takes_metadata() {
            return Err(SaveError::MetadataUnsupported(format));
        }
        match format {
            SaveFormat::Png => self.to_png_with(options),
            SaveFormat::Svg => self.to_svg_with(options).map(String::into_bytes),
            SaveFormat::Svgz => self
                .to_svg_with(options)
                .map(|svg| pltrs_render::gzip(svg.as_bytes())),
            SaveFormat::Pdf => self.to_pdf_with(options),
            SaveFormat::Ps => self.postscript(&options, false),
            SaveFormat::Eps => self.postscript(&options, true),
            SaveFormat::Pgf => self.to_pgf_with(options).map(String::into_bytes),
            SaveFormat::Jpeg => self.to_bitmap(BitmapFormat::Jpeg, options),
            SaveFormat::Tiff => self.to_bitmap(BitmapFormat::Tiff, options),
            SaveFormat::Gif => self.to_bitmap(BitmapFormat::Gif, options),
            SaveFormat::Webp => self.to_bitmap(BitmapFormat::Webp, options),
            SaveFormat::Avif => self.to_bitmap(BitmapFormat::Avif, options),
            SaveFormat::Raw => self.to_bitmap(BitmapFormat::Raw, options),
        }
    }

    /// The canvas rendered with save options and encoded as `format`
    /// (Agg's `print_raw`, and `print_jpg`, `print_tif`, `print_gif`,
    /// `print_webp` and `print_avif` through `imsave`): the save
    /// resolution recorded, JPEG flattened onto white.
    fn to_bitmap(&self, format: BitmapFormat, options: SaveOptions) -> Result<Vec<u8>, SaveError> {
        let defaults = BitmapSettings::default();
        let jpeg_quality = options.jpeg_quality.unwrap_or(defaults.jpeg_quality);
        let avif_quality = options.avif_quality.unwrap_or(defaults.avif_quality);
        let canvas = self.render_raster_with(options)?;
        let settings = BitmapSettings {
            dpi: canvas.dpi(),
            jpeg_quality,
            avif_quality,
            ..defaults
        };
        let image = canvas.to_image();
        format
            .encode(image.width(), image.height(), image.data(), settings)
            .map_err(SaveError::Bitmap)
    }

    /// Writes the figure to `path` in the format its extension names
    /// (PNG, JPEG, TIFF, GIF, WebP, AVIF, raw RGBA, SVG, SVGZ, PDF, PS, EPS
    /// or PGF;
    /// `savefig.format` and an appended extension without one).
    pub fn savefig(&self, path: impl AsRef<FsPath>) -> Result<(), SaveError> {
        self.savefig_with(path, SaveOptions::default())
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
        options: SaveOptions,
    ) -> Result<(), SaveError> {
        let (path, format) = resolve_format(path.as_ref(), options.format)?;
        match format {
            SaveFormat::Svg | SaveFormat::Svgz => {
                return self.save_svg(&path, &options, format == SaveFormat::Svgz);
            }
            SaveFormat::Pgf => return self.save_pgf(&path, &options),
            _ => {}
        }
        let document = if matches!(format, SaveFormat::Ps | SaveFormat::Eps) {
            let metadata = Some(file_metadata(&options, &path));
            let titled = SaveOptions {
                metadata,
                ..options
            };
            self.to_format(format, titled)?
        } else {
            self.to_format(format, options)?
        };
        std::fs::write(path, document).map_err(SaveError::Io)
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
    use pltrs_core::Bbox;

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
            assert!((fig.facecolor().resolve().unwrap().r - 0.75).abs() < 1e-12);
        });
    }

    #[test]
    fn savefig_reports_io_errors() {
        let fig = Figure::with_size(0.5, 0.5, 10.0);
        let missing = std::env::temp_dir().join("pltrs-missing-dir/x/figure.png");
        assert!(matches!(fig.savefig(missing), Err(SaveError::Io(_))));
    }

    #[test]
    fn bitmap_formats_record_the_save_resolution() {
        let fig = Figure::with_size(1.0, 0.5, 50.0);
        let options = SaveOptions {
            dpi: Some(30.0),
            ..SaveOptions::default()
        };
        let raw = fig.to_format(SaveFormat::Raw, options.clone()).unwrap();
        assert_eq!(raw.len(), 30 * 15 * 4);
        let jpeg = fig.to_format(SaveFormat::Jpeg, options.clone()).unwrap();
        assert_eq!(&jpeg[6..11], b"JFIF\0");
        assert_eq!(&jpeg[14..18], [0, 30, 0, 30], "JFIF density");
        let tiff = fig.to_format(SaveFormat::Tiff, options.clone()).unwrap();
        assert_eq!(&tiff[190..198], [30, 0, 0, 0, 1, 0, 0, 0], "x resolution");
        let gif = fig.to_format(SaveFormat::Gif, options.clone()).unwrap();
        assert_eq!(&gif[..10], b"GIF89a\x1e\0\x0f\0");
        // WebP is lossless: it decodes to the canvas.
        let webp = fig.to_format(SaveFormat::Webp, options).unwrap();
        let decoded = pltrs_codec::webp::decode(&webp)
            .unwrap()
            .to_rgba8()
            .concat();
        assert_eq!(decoded, raw);
    }

    #[test]
    fn avif_follows_quality_and_transparency() {
        // matplotlib's test_pil_kwargs_avif and test_avif_alpha.
        let fig = Figure::with_size(2.0, 1.5, 50.0);
        fig.subplot()
            .plot([0.0, 1.0, 2.0], [0.0, 1.0, 0.0])
            .unwrap();
        let save = |quality, transparent| {
            let options = SaveOptions {
                avif_quality: Some(quality),
                transparent: Some(transparent),
                ..SaveOptions::default()
            };
            fig.to_format(SaveFormat::Avif, options).unwrap()
        };
        let (small, large) = (save(1, false), save(100, false));
        assert_eq!(&small[4..12], b"ftypavif");
        assert!(large.len() > small.len());
        let has_alpha = |file: &[u8]| file.windows(4).any(|w| w == b"auxC");
        assert!(!has_alpha(&small));
        assert!(has_alpha(&save(75, true)));
    }

    #[test]
    fn svg_images_can_go_beside_the_file() {
        let dir = std::env::temp_dir().join(format!("pltrs-svg-images-{}", std::process::id()));
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
            .to_format(SaveFormat::Svgz, SaveOptions::default())
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
                .set_halign(crate::text::HAlign::Center);
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
        let cropped = String::from_utf8_lossy(&fig.to_pdf_with(tight).unwrap()).into_owned();
        assert!(cropped.contains("/MediaBox [ 0 0 72 36 ]"));
        let scratch = crate::scratch::Scratch::new("save-formats");
        assert!(matches!(
            fig.savefig(scratch.path("figure.heic")),
            Err(SaveError::UnsupportedFormat(_))
        ));
    }

    #[test]
    fn only_png_and_vector_formats_take_metadata() {
        // matplotlib's test_savefig_metadata and test_savefig_metadata_error.
        let fig = Figure::with_size(1.0, 1.0, 20.0);
        let options = SaveOptions {
            metadata: Some(Metadata::default()),
            ..SaveOptions::default()
        };
        for format in [
            SaveFormat::Png,
            SaveFormat::Svg,
            SaveFormat::Pdf,
            SaveFormat::Eps,
        ] {
            assert!(fig.to_format(format, options.clone()).is_ok(), "{format:?}");
        }
        for format in [
            SaveFormat::Jpeg,
            SaveFormat::Tiff,
            SaveFormat::Webp,
            SaveFormat::Avif,
            SaveFormat::Raw,
        ] {
            assert!(matches!(
                fig.to_format(format, options.clone()),
                Err(SaveError::MetadataUnsupported(rejected)) if rejected == format
            ));
        }
    }
}
