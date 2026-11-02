//! The PDF backend: matplotlib's `RendererPdf` and `PdfFile`.
//!
//! Every page is a content stream whose graphics state is tracked so that
//! only changes are emitted; the pages share one resource dictionary:
//! text in Type 3 fonts built from the glyph outlines, alpha through
//! extended graphics states, hatches as tiling patterns, markers and large
//! path collections as form `XObject`s, images as (indexed when possible)
//! PNG-predicted image `XObject`s with soft masks, and Gouraud triangles as
//! free-form shadings. A document may hold several pages (matplotlib's
//! `PdfPages`), with text notes attached to them.
//!
//! TrueType fonts are embedded as Type 3 fonts or, with `pdf.fonttype =
//! 42`, as `CIDFontType2` subsets of the font program; the fonts of usetex
//! text as subsets of their Type 1 programs (see `tex`). With
//! `pdf.use14corefonts`, text that is not mathtext is shown in one of the
//! 14 PDF core fonts instead, referenced by name and never embedded (see
//! `core_fonts`).
//!
//! Not supported: link annotations for URLs.

mod core_fonts;
mod draw;
mod file;
mod fonts;
mod image;
mod object;
mod pages;
mod resources;
mod state;
mod tex;
mod text;
mod type42;
mod writer;

use pltrs_core::path::convert::{SimplifySettings, SketchParams, StringCodes, StringOptions};
use pltrs_core::{Affine2D, Bbox, Path, Point, Rgba};
use pltrs_text::tex::{TexError, TexText};
use pltrs_text::{AfmDirectory, TextRun};

pub use self::file::{PdfFont, PdfHatch};
pub use self::object::{Obj as PdfObject, text_string as pdf_text_string};
pub use self::resources::{
    MarkerPaint as PdfMarkerPaint, ShadingColors as PdfShadingColors,
    TemplatePaint as PdfTemplatePaint,
};

use self::fonts::FontNames;
use self::object::Obj;
use self::pages::{Note, PageParents, Pages};
use self::resources::{ResourceIds, Resources};
use self::state::{PdfState, StateStack};
use self::tex::DviFonts;
use self::writer::PdfWriter;
use crate::collection::PathCollection;
use crate::gc::GraphicsContext;
use crate::image::RgbaImage;
use crate::renderer::Renderer;
use crate::vector::{
    CharacterTracker, FontCache, FontType, Metadata, Problems, VectorError, check_canvas,
};

/// Settings of a PDF document (matplotlib's `pdf.*` rcParams and the
/// `metadata` argument of `savefig`).
#[derive(Clone, Debug, PartialEq)]
pub struct PdfOptions {
    /// `pdf.compression`: zlib level of the streams, 0 to store them.
    pub compression: u8,
    /// `pdf.inheritcolor`: leave stroke and fill colours unset.
    pub inherit_color: bool,
    /// Title, creator, producer and creation date.
    pub metadata: Metadata,
    /// `path.simplify` and `path.simplify_threshold`.
    pub simplify: SimplifySettings,
    /// `image.composite_image`.
    pub composite_images: bool,
    /// `pdf.fonttype`: how TrueType fonts are embedded.
    pub fonttype: FontType,
    /// The font type whose subset size groups the character codes of the
    /// embedded fonts: `ps.fonttype`, which matplotlib's PDF writer reads
    /// for it (`PdfFile._character_tracker`), whatever `pdf.fonttype`
    /// says.
    pub subset_fonttype: FontType,
    /// `pdf.use14corefonts`: text that is not mathtext is drawn in the 14
    /// PDF core fonts, measured with their metrics.
    pub core_fonts: bool,
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            compression: 6,
            inherit_color: false,
            metadata: Metadata::default(),
            simplify: SimplifySettings::default(),
            composite_images: true,
            fonttype: FontType::Type3,
            subset_fonttype: FontType::Type3,
            core_fonts: false,
        }
    }
}

impl PdfOptions {
    /// The AFM fonts text that is not mathtext is drawn in, and must be
    /// measured with, if any (`RendererPdf._afm_font_dir` when
    /// `pdf.use14corefonts` is set).
    pub fn afm_directory(&self) -> Option<AfmDirectory> {
        self.core_fonts.then_some(AfmDirectory::PdfCore)
    }
}

/// Object numbers fixed when the file is started.
#[derive(Clone, Copy, Debug)]
struct Ids {
    root: u32,
    pages: u32,
    resources: ResourceIds,
    resource_dictionary: u32,
}

/// `pathOperations`: path data in PDF operators (quadratic curves as
/// cubics) with optional canvas clipping, simplification and sketching.
fn path_ops(
    path: &Path,
    transform: Affine2D,
    clip: Option<Bbox>,
    simplify: Option<f64>,
    sketch: Option<SketchParams>,
) -> Vec<u8> {
    path.convert_to_string(&StringOptions {
        transform,
        clip,
        simplify,
        sketch,
        precision: 6,
        codes: StringCodes::PDF,
        postfix: true,
    })
    .into_bytes()
}

/// Renders into a PDF document exactly like matplotlib's `RendererPdf`
/// and `PdfFile`. Coordinates are points with the origin at the bottom
/// left of the current page.
#[derive(Debug)]
pub struct PdfRenderer {
    writer: PdfWriter,
    /// Size of the current page in inches.
    width: f64,
    height: f64,
    image_dpi: f64,
    ids: Ids,
    pages: Pages,
    /// Whether the content stream of a page is open.
    page_open: bool,
    resources: Resources,
    states: StateStack,
    font_names: FontNames,
    /// The TeX fonts of usetex text.
    dvi_fonts: DviFonts,
    tracker: CharacterTracker,
    fonts: FontCache,
    options: PdfOptions,
    /// Identifies the artist drawing: matplotlib compares clip regions by
    /// object identity and every artist has its own, so a clip set by
    /// another artist must be set again. Drawing calls inside a group (one
    /// artist) share a generation; calls outside any group each start one.
    generation: u64,
    group_depth: usize,
    problems: Problems,
    /// The document information dictionary, once written.
    info: Option<u32>,
}

impl PdfRenderer {
    /// A document with one page of `width × height` inches whose images
    /// are rasterized at `image_dpi` (matplotlib's `PdfFile` plus
    /// `newPage`).
    ///
    /// # Errors
    ///
    /// When the page size or resolution is not positive and finite.
    pub fn new(
        width: f64,
        height: f64,
        image_dpi: f64,
        options: PdfOptions,
    ) -> Result<Self, VectorError> {
        check_canvas(width, height, image_dpi)?;
        let mut document = Self::document(options)?;
        document.new_page(width, height, image_dpi)?;
        Ok(document)
    }

    /// A document without pages yet (matplotlib's `PdfFile`); drawing
    /// needs a page started with [`new_page`](Self::new_page).
    ///
    /// # Errors
    ///
    /// Never in practice: the document's first objects are always
    /// writable.
    pub fn document(options: PdfOptions) -> Result<Self, VectorError> {
        let mut writer = PdfWriter::new(options.compression);
        let root = writer.reserve();
        let pages = writer.reserve();
        let resource_ids = ResourceIds {
            fonts: writer.reserve(),
            ext_gstates: writer.reserve(),
            patterns: writer.reserve(),
            shadings: writer.reserve(),
            xobjects: writer.reserve(),
        };
        let resource_dictionary = writer.reserve();
        writer.write_object(
            root,
            &Obj::dict([("Type", Obj::name("Catalog")), ("Pages", Obj::Ref(pages))]),
        )?;
        let procsets = ["PDF", "Text", "ImageB", "ImageC", "ImageI"].map(Obj::name);
        writer.write_object(
            resource_dictionary,
            &Obj::dict([
                ("Font", Obj::Ref(resource_ids.fonts)),
                ("XObject", Obj::Ref(resource_ids.xobjects)),
                ("ExtGState", Obj::Ref(resource_ids.ext_gstates)),
                ("Pattern", Obj::Ref(resource_ids.patterns)),
                ("Shading", Obj::Ref(resource_ids.shadings)),
                ("ProcSet", Obj::Array(procsets.to_vec())),
            ]),
        )?;
        let states = StateStack::new(options.inherit_color);
        Ok(Self {
            writer,
            width: 0.0,
            height: 0.0,
            image_dpi: 72.0,
            ids: Ids {
                root,
                pages,
                resources: resource_ids,
                resource_dictionary,
            },
            pages: Pages::default(),
            page_open: false,
            resources: Resources::default(),
            states,
            font_names: FontNames::default(),
            dvi_fonts: DviFonts::default(),
            tracker: CharacterTracker::new(options.subset_fonttype.subset_size()),
            fonts: FontCache::default(),
            options,
            generation: 0,
            group_depth: 0,
            problems: Problems::default(),
            info: None,
        })
    }

    /// Ends the current page, if any, and starts a page of `width ×
    /// height` inches whose images are rasterized at `image_dpi`, drawn
    /// with a fresh graphics state (matplotlib's `newPage` and a new
    /// `RendererPdf`). Notes attached since the last page go on it.
    ///
    /// # Errors
    ///
    /// When the page size or resolution is not positive and finite, or the
    /// previous page could not be written.
    pub fn new_page(&mut self, width: f64, height: f64, image_dpi: f64) -> Result<(), VectorError> {
        check_canvas(width, height, image_dpi)?;
        self.end_page()?;
        let parents = PageParents {
            pages: self.ids.pages,
            resources: self.ids.resource_dictionary,
        };
        let contents = self.pages.start(&mut self.writer, parents, width, height)?;
        let length = self.writer.reserve();
        self.writer
            .begin_stream(contents, Some(length), Vec::new(), None);
        self.writer
            .output(&[Obj::name("DeviceRGB"), Obj::op("CS")])?;
        self.writer
            .output(&[Obj::name("DeviceRGB"), Obj::op("cs")])?;
        self.writer.output(&[Obj::Int(1), Obj::op("j")])?;
        self.states = StateStack::new(self.options.inherit_color);
        (self.width, self.height, self.image_dpi) = (width, height, image_dpi);
        self.page_open = true;
        Ok(())
    }

    /// Attaches a text note shown at `rect` (`[x0, y0, x1, y1]` in points)
    /// to the next page started (matplotlib's `newTextnote`).
    pub fn attach_note(&mut self, text: &str, rect: [f64; 4]) {
        self.pages.add_note(Note {
            text: text.to_owned(),
            rect,
        });
    }

    /// How many pages the document has.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// The names of the images the document holds so far (`I1`, `I2`,
    /// ...), one per image `XObject` written (matplotlib's
    /// `PdfFile._images`).
    pub fn image_names(&self) -> Vec<String> {
        self.resources.image_names().map(str::to_owned).collect()
    }

    /// The document information written when the document is finished
    /// (matplotlib's `PdfPages.infodict`).
    pub fn metadata_mut(&mut self) -> &mut Metadata {
        &mut self.options.metadata
    }

    /// The AFM fonts the document draws text that is not mathtext in, if
    /// any (see [`PdfOptions::afm_directory`]).
    pub fn afm_directory(&self) -> Option<AfmDirectory> {
        self.options.afm_directory()
    }

    /// Restores the graphics state and closes the content stream of the
    /// current page (`RendererPdf.finalize` and `endStream`).
    fn end_page(&mut self) -> Result<(), VectorError> {
        if !self.page_open {
            return Ok(());
        }
        let restore = self.states.finish();
        self.writer.output(&restore)?;
        self.writer.end_stream()?;
        self.page_open = false;
        Ok(())
    }

    /// Closes the page and writes the annotations, fonts, resources, the
    /// page tree, the document information and the cross-reference table
    /// (matplotlib's `finalize` and `close`).
    ///
    /// # Errors
    ///
    /// When a font, image or number met while drawing could not be
    /// written.
    pub fn finish(mut self) -> Result<Vec<u8>, VectorError> {
        self.finalize()?;
        self.close()
    }

    /// Writes `objects` as one content-stream line.
    fn output(&mut self, objects: &[Obj]) {
        let result = self.writer.output(objects);
        self.problems.record(result);
    }

    /// `check_gc`: emits what changes the graphics state into the one
    /// `gc` and `fill` need.
    fn check_gc(&mut self, gc: &GraphicsContext, fill: Option<Rgba>) {
        let target = PdfState::from_gc(gc, fill, self.generation);
        let cmds = self.states.delta(&target, &mut self.resources);
        if !cmds.is_empty() {
            self.output(&cmds);
        }
    }

    /// Marks the start of a drawing call (see `generation`).
    fn begin_draw(&mut self) {
        if self.group_depth == 0 {
            self.generation += 1;
        }
    }

    /// The page in points.
    fn canvas(&self) -> Bbox {
        Bbox::from_extents(0.0, 0.0, self.width * 72.0, self.height * 72.0)
    }
}

impl Renderer for PdfRenderer {
    fn canvas_size(&self) -> (f64, f64) {
        (self.width * 72.0, self.height * 72.0)
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
        self.begin_draw();
        self.pdf_path(gc, path, *transform, fill);
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
        self.begin_draw();
        self.pdf_markers(gc, marker, *marker_transform, path, *transform, fill);
    }

    fn draw_path_collection(&mut self, gc: &GraphicsContext, collection: &PathCollection<'_>) {
        self.begin_draw();
        self.pdf_path_collection(gc, collection);
    }

    fn draw_gouraud_triangles(
        &mut self,
        gc: &GraphicsContext,
        triangles: &[[Point; 3]],
        colors: &[[Rgba; 3]],
        transform: &Affine2D,
    ) {
        self.begin_draw();
        self.pdf_gouraud_triangles(gc, triangles, colors, *transform);
    }

    fn draw_image(
        &mut self,
        gc: &GraphicsContext,
        origin: Point,
        image: &RgbaImage,
        transform: Option<&Affine2D>,
    ) {
        self.begin_draw();
        self.pdf_image(gc, origin, image, transform.copied());
    }

    fn draw_text(&mut self, gc: &GraphicsContext, run: &TextRun, origin: Point, angle: f64) {
        self.begin_draw();
        self.pdf_text(gc, run, origin, angle);
    }

    fn draw_tex(
        &mut self,
        gc: &GraphicsContext,
        _run: &TextRun,
        tex: &TexText,
        origin: Point,
        angle: f64,
    ) -> Result<(), TexError> {
        self.begin_draw();
        self.pdf_tex(gc, tex, origin, angle)
    }

    fn open_group(&mut self, _name: &str, _gid: Option<&str>) {
        self.group_depth += 1;
        self.generation += 1;
    }

    fn close_group(&mut self, _name: &str) {
        self.group_depth = self.group_depth.saturating_sub(1);
        self.generation += 1;
    }

    /// The parts of an artist draw in its generation.
    fn open_part_group(&mut self, _name: &str, _gid: Option<&str>) {
        self.group_depth += 1;
    }

    fn close_part_group(&mut self, _name: &str) {
        self.group_depth = self.group_depth.saturating_sub(1);
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
