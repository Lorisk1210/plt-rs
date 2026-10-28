//! The SVG backend: matplotlib's `RendererSVG`.
//!
//! The document mirrors matplotlib's: paths with inline CSS styles,
//! markers and large path collections as reusable `<defs>`, glyphs defined
//! once as paths in `FreeType` units and placed with `<use>`, clip paths and
//! hatch patterns written at the end, images inlined as base64 PNG and
//! Gouraud triangles emulated with three linear gradients. Text is written
//! as glyph paths or, with `svg.fonttype = "none"`, as text elements
//! styled with CSS font properties. Images are always inlined
//! (`svg.image_inline`).

mod defs;
mod draw;
mod gouraud;
mod metadata;
mod style;
mod tex;
mod text;
mod text_elements;
mod xml;

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::str::FromStr;

use plt_codec::sha256::sha256_hex;
use plt_core::path::convert::SimplifySettings;
use plt_core::{Affine2D, Path, Point, Rgba};
use plt_text::tex::TexError;
use plt_text::{TextRun, TextSource};

use self::defs::{Clips, Hatches};
use self::xml::XmlWriter;
use crate::collection::PathCollection;
use crate::gc::GraphicsContext;
use crate::image::RgbaImage;
use crate::renderer::Renderer;
use crate::vector::{FontCache, Metadata, Problems, Timestamp, VectorError, check_canvas, short};

/// The XML declaration and doctype every document starts with.
const PROLOG: &str = "<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"no\"?>\n\
<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\"\n  \
\"http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd\">\n";

/// How an SVG document writes text (matplotlib's `svg.fonttype`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SvgFontType {
    /// Glyphs as paths (`"path"`).
    #[default]
    Path,
    /// Text elements, rendered with the viewer's fonts (`"none"`).
    None,
}

impl SvgFontType {
    /// matplotlib's name (`path` or `none`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::None => "none",
        }
    }
}

impl fmt::Display for SvgFontType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for SvgFontType {
    type Err = UnknownSvgFontType;

    /// Parses matplotlib's name ([`SvgFontType::name`]).
    fn from_str(name: &str) -> Result<Self, UnknownSvgFontType> {
        [Self::Path, Self::None]
            .into_iter()
            .find(|fonttype| fonttype.name() == name)
            .ok_or_else(|| UnknownSvgFontType(name.to_owned()))
    }
}

/// A name that is not an [`SvgFontType`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownSvgFontType(pub String);

impl fmt::Display for UnknownSvgFontType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} is not a valid SVG font type", self.0)
    }
}

impl std::error::Error for UnknownSvgFontType {}

/// Settings of an SVG document (matplotlib's `svg.*` rcParams and the
/// `metadata` argument of `savefig`).
#[derive(Clone, Debug, PartialEq)]
pub struct SvgOptions {
    /// `svg.hashsalt`: salt of the content-derived ids. `None` derives one
    /// from the current time, so that documents embedded in one page do
    /// not share ids.
    pub hashsalt: Option<String>,
    /// `svg.id`: the `id` of the root element.
    pub id: Option<String>,
    /// Title, creator and date (written as Dublin Core metadata).
    pub metadata: Metadata,
    /// `path.simplify` and `path.simplify_threshold`.
    pub simplify: SimplifySettings,
    /// `image.composite_image`: whether images of one axes are composited
    /// into one before drawing.
    pub composite_images: bool,
    /// `svg.fonttype`: glyphs as paths or text elements.
    pub fonttype: SvgFontType,
    /// `svg.image_inline = False`: the file name of the document, beside
    /// which images go as `<name>.image<N>.png` files the document links
    /// to, instead of being embedded (`None`).
    pub image_files: Option<String>,
}

/// An image written beside an SVG document instead of being embedded in
/// it (see [`SvgOptions::image_files`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageFile {
    /// The file name the document links to.
    pub name: String,
    /// The PNG file.
    pub png: Vec<u8>,
}

impl Default for SvgOptions {
    fn default() -> Self {
        Self {
            hashsalt: None,
            id: None,
            metadata: Metadata::default(),
            simplify: SimplifySettings::default(),
            composite_images: true,
            fonttype: SvgFontType::Path,
            image_files: None,
        }
    }
}

/// Renders into an SVG document exactly like matplotlib's `RendererSVG`.
///
/// Coordinates are points (1/72 inch); the canvas has its origin at the
/// bottom left like every renderer, and the document flips it.
#[derive(Debug)]
pub struct SvgRenderer {
    width: f64,
    height: f64,
    image_dpi: f64,
    writer: XmlWriter,
    root: usize,
    salt: String,
    options: SvgOptions,
    groups: HashMap<String, usize>,
    clips: Clips,
    hatches: Hatches,
    markers: HashMap<(String, String), String>,
    path_collection_id: usize,
    has_gouraud: bool,
    gradients: usize,
    glyphs: HashSet<String>,
    fonts: FontCache,
    /// The images written beside the document, in drawing order.
    images: Vec<ImageFile>,
    problems: Problems,
}

impl SvgRenderer {
    /// A document of `width × height` points whose images are rasterized
    /// at `image_dpi` (matplotlib's `RendererSVG(w, h, fh, image_dpi)`).
    pub fn new(
        width: f64,
        height: f64,
        image_dpi: f64,
        options: SvgOptions,
    ) -> Result<Self, VectorError> {
        check_canvas(width, height, image_dpi)?;
        let salt = options
            .hashsalt
            .clone()
            .unwrap_or_else(|| format!("plt-rs-{}", Timestamp::now().0));
        let mut writer = XmlWriter::new(PROLOG);
        let (w, h) = (short(width), short(height));
        let root = writer.start(
            "svg",
            &[
                ("xmlns:xlink", "http://www.w3.org/1999/xlink".to_owned()),
                ("width", format!("{w}pt")),
                ("height", format!("{h}pt")),
                ("viewBox", format!("0 0 {w} {h}")),
                ("xmlns", "http://www.w3.org/2000/svg".to_owned()),
                ("version", "1.1".to_owned()),
                ("id", options.id.clone().unwrap_or_default()),
            ],
        );
        metadata::write(&mut writer, &options.metadata);
        writer.start("defs", &[]);
        writer.element(
            "style",
            "*{stroke-linejoin: round; stroke-linecap: butt}",
            &[("type", "text/css".to_owned())],
        );
        writer.end(true);
        Ok(Self {
            width,
            height,
            image_dpi,
            writer,
            root,
            salt,
            options,
            groups: HashMap::new(),
            clips: Clips::default(),
            hatches: Hatches::default(),
            markers: HashMap::new(),
            path_collection_id: 0,
            has_gouraud: false,
            gradients: 0,
            glyphs: HashSet::new(),
            fonts: FontCache::default(),
            images: Vec::new(),
            problems: Problems::default(),
        })
    }

    /// Writes the clip paths and hatch patterns and closes the document
    /// (matplotlib's `finalize`).
    ///
    /// # Errors
    ///
    /// When a font or image met while drawing could not be written.
    pub fn finish(self) -> Result<String, VectorError> {
        self.finish_with_images().map(|(document, _)| document)
    }

    /// Like [`SvgRenderer::finish`], also returning the images to write
    /// beside the document (empty unless [`SvgOptions::image_files`]).
    ///
    /// # Errors
    ///
    /// When a font or image met while drawing could not be written.
    pub fn finish_with_images(mut self) -> Result<(String, Vec<ImageFile>), VectorError> {
        self.clips.write(&mut self.writer);
        self.hatches.write(&mut self.writer);
        self.writer.close(self.root);
        let images = std::mem::take(&mut self.images);
        self.problems
            .into_result(self.writer.into_string())
            .map(|document| (document, images))
    }

    /// `_make_id`: `type` followed by the first ten hex digits of the
    /// SHA-256 of the salt and `content` (Python's `str` of the key).
    fn make_id(&self, kind: &str, content: &str) -> String {
        let digest = sha256_hex(format!("{}{content}", self.salt).as_bytes());
        format!("{kind}{}", &digest[..10])
    }

    /// `_make_flip_transform`'s second half: y down from the top.
    fn flip(&self) -> Affine2D {
        Affine2D::scaling(1.0, -1.0).translate(0.0, self.height)
    }

    /// Opens a hyperlink around the next element when `gc` has a URL.
    fn open_link(&mut self, gc: &GraphicsContext) -> bool {
        let Some(url) = gc.url.clone() else {
            return false;
        };
        self.writer
            .start("a", &[("xlink:href", url), ("target", "_blank".to_owned())]);
        true
    }

    /// Closes what [`open_link`](Self::open_link) opened.
    fn close_link(&mut self, opened: bool) {
        if opened {
            self.writer.end(true);
        }
    }
}

impl Renderer for SvgRenderer {
    fn canvas_size(&self) -> (f64, f64) {
        (self.width, self.height)
    }

    /// The image resolution, as matplotlib's `MixedModeRenderer.dpi`.
    fn dpi(&self) -> f64 {
        self.image_dpi
    }

    /// Lengths are points already.
    fn points_to_pixels(&self, points: f64) -> f64 {
        points
    }

    fn draw_path(
        &mut self,
        gc: &GraphicsContext,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        self.svg_path(gc, path, *transform, fill);
    }

    fn draw_markers(
        &mut self,
        gc: &GraphicsContext,
        marker: &Path,
        marker_transform: &Affine2D,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        self.svg_markers(gc, marker, *marker_transform, path, *transform, fill);
    }

    fn draw_path_collection(&mut self, gc: &GraphicsContext, collection: &PathCollection<'_>) {
        self.svg_path_collection(gc, collection);
    }

    fn draw_gouraud_triangles(
        &mut self,
        gc: &GraphicsContext,
        triangles: &[[Point; 3]],
        colors: &[[Rgba; 3]],
        transform: &Affine2D,
    ) {
        self.svg_gouraud_triangles(gc, triangles, colors, *transform);
    }

    fn draw_image(
        &mut self,
        gc: &GraphicsContext,
        origin: Point,
        image: &RgbaImage,
        transform: Option<&Affine2D>,
    ) {
        self.svg_image(gc, origin, image, transform.copied());
    }

    fn draw_text(&mut self, gc: &GraphicsContext, run: &TextRun, origin: Point, angle: f64) {
        self.svg_text(gc, run, origin, angle);
    }

    fn draw_tex(
        &mut self,
        gc: &GraphicsContext,
        run: &TextRun,
        origin: Point,
        angle: f64,
    ) -> Result<(), TexError> {
        if let Some(TextSource::Tex(tex)) = &run.source {
            self.svg_tex(gc, tex, origin, angle)
        } else {
            self.svg_text(gc, run, origin, angle);
            Ok(())
        }
    }

    fn open_group(&mut self, name: &str, gid: Option<&str>) {
        let id = gid.filter(|g| !g.is_empty()).map_or_else(
            || {
                let count = self.groups.entry(name.to_owned()).or_insert(0);
                *count += 1;
                format!("{name}_{count}")
            },
            str::to_owned,
        );
        self.writer.start("g", &[("id", id)]);
    }

    fn close_group(&mut self, _name: &str) {
        self.writer.end(true);
    }

    fn option_image_nocomposite(&self) -> bool {
        !self.options.composite_images
    }

    fn option_scale_image(&self) -> bool {
        true
    }

    fn image_magnification(&self) -> f64 {
        self.image_dpi / 72.0
    }
}

#[cfg(test)]
mod tests;
