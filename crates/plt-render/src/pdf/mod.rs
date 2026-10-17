//! The PDF backend: matplotlib's `RendererPdf` and `PdfFile` with
//! `pdf.fonttype = 3`.
//!
//! One page is written: a content stream whose graphics state is tracked
//! so that only changes are emitted, text in Type 3 fonts built from the
//! glyph outlines, alpha through extended graphics states, hatches as
//! tiling patterns, markers and large path collections as form `XObject`s,
//! images as (indexed when possible) PNG-predicted image `XObject`s with
//! soft masks, and Gouraud triangles as free-form shadings.
//!
//! Not supported: Type 42 font embedding (`pdf.fonttype = 42`), the 14
//! core fonts (`pdf.use14corefonts`), link annotations for URLs and
//! several pages per file.

mod draw;
mod fonts;
mod image;
mod object;
mod resources;
mod state;
mod text;
mod writer;

use plt_core::path::convert::{SimplifySettings, SketchParams, StringCodes, StringOptions};
use plt_core::{Affine2D, Bbox, Path, Point, Rgba};
use plt_text::TextRun;

use self::fonts::FontNames;
use self::object::{Obj, text_string};
use self::resources::{ResourceIds, Resources};
use self::state::{PdfState, StateStack};
use self::writer::PdfWriter;
use crate::collection::PathCollection;
use crate::gc::GraphicsContext;
use crate::image::RgbaImage;
use crate::renderer::Renderer;
use crate::vector::{CharacterTracker, FontCache, Metadata, Problems, VectorError, check_canvas};

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
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            compression: 6,
            inherit_color: false,
            metadata: Metadata::default(),
            simplify: SimplifySettings::default(),
            composite_images: true,
        }
    }
}

/// Object numbers fixed when the file is started.
#[derive(Clone, Copy, Debug)]
struct Ids {
    root: u32,
    pages: u32,
    resources: ResourceIds,
    annotations: u32,
    page: u32,
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

/// Renders into a one-page PDF document exactly like matplotlib's
/// `RendererPdf`. Coordinates are points with the origin at the bottom
/// left.
#[derive(Debug)]
pub struct PdfRenderer {
    writer: PdfWriter,
    /// Page size in inches.
    width: f64,
    height: f64,
    image_dpi: f64,
    ids: Ids,
    resources: Resources,
    states: StateStack,
    font_names: FontNames,
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
}

impl PdfRenderer {
    /// A document with one page of `width × height` inches whose images
    /// are rasterized at `image_dpi` (matplotlib's `PdfFile` plus
    /// `newPage`).
    pub fn new(
        width: f64,
        height: f64,
        image_dpi: f64,
        options: PdfOptions,
    ) -> Result<Self, VectorError> {
        check_canvas(width, height, image_dpi)?;
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
        let resources = writer.reserve();
        writer.write_object(
            root,
            &Obj::dict([("Type", Obj::name("Catalog")), ("Pages", Obj::Ref(pages))]),
        )?;
        let procsets = ["PDF", "Text", "ImageB", "ImageC", "ImageI"].map(Obj::name);
        writer.write_object(
            resources,
            &Obj::dict([
                ("Font", Obj::Ref(resource_ids.fonts)),
                ("XObject", Obj::Ref(resource_ids.xobjects)),
                ("ExtGState", Obj::Ref(resource_ids.ext_gstates)),
                ("Pattern", Obj::Ref(resource_ids.patterns)),
                ("Shading", Obj::Ref(resource_ids.shadings)),
                ("ProcSet", Obj::Array(procsets.to_vec())),
            ]),
        )?;
        let contents = writer.reserve();
        let annotations = writer.reserve();
        let page = writer.reserve();
        writer.write_object(
            page,
            &Obj::dict([
                ("Type", Obj::name("Page")),
                ("Parent", Obj::Ref(pages)),
                ("Resources", Obj::Ref(resources)),
                (
                    "MediaBox",
                    Obj::Array(vec![
                        Obj::Int(0),
                        Obj::Int(0),
                        Obj::Real(72.0 * width),
                        Obj::Real(72.0 * height),
                    ]),
                ),
                ("Contents", Obj::Ref(contents)),
                ("Annots", Obj::Ref(annotations)),
            ]),
        )?;
        let length = writer.reserve();
        writer.begin_stream(contents, Some(length), Vec::new(), None);
        writer.output(&[Obj::name("DeviceRGB"), Obj::op("CS")])?;
        writer.output(&[Obj::name("DeviceRGB"), Obj::op("cs")])?;
        writer.output(&[Obj::Int(1), Obj::op("j")])?;
        let states = StateStack::new(options.inherit_color);
        Ok(Self {
            writer,
            width,
            height,
            image_dpi,
            ids: Ids {
                root,
                pages,
                resources: resource_ids,
                annotations,
                page,
            },
            resources: Resources::default(),
            states,
            font_names: FontNames::default(),
            tracker: CharacterTracker::new(256),
            fonts: FontCache::default(),
            options,
            generation: 0,
            group_depth: 0,
            problems: Problems::default(),
        })
    }

    /// Closes the page and writes fonts, resources, the page tree, the
    /// document information and the cross-reference table (matplotlib's
    /// `finalize` and `close`).
    ///
    /// # Errors
    ///
    /// When a font, image or number met while drawing could not be
    /// written.
    pub fn finish(mut self) -> Result<Vec<u8>, VectorError> {
        let restore = self.states.finish();
        self.writer.output(&restore)?;
        self.writer.end_stream()?;
        self.writer
            .write_object(self.ids.annotations, &Obj::Array(Vec::new()))?;
        fonts::write_fonts(
            &mut self.writer,
            &mut self.fonts,
            &self.font_names,
            &self.tracker,
            self.ids.resources.fonts,
        )?;
        self.resources.write(
            &mut self.writer,
            self.ids.resources,
            self.height,
            self.options.compression > 0,
        )?;
        self.writer.write_object(
            self.ids.pages,
            &Obj::dict([
                ("Type", Obj::name("Pages")),
                ("Kids", Obj::Array(vec![Obj::Ref(self.ids.page)])),
                ("Count", Obj::Int(1)),
            ]),
        )?;
        let metadata = &self.options.metadata;
        let mut info = Vec::new();
        if let Some(creator) = &metadata.creator {
            info.push(("Creator".to_owned(), text_string(creator)));
        }
        if let Some(producer) = &metadata.producer {
            info.push(("Producer".to_owned(), text_string(producer)));
        }
        if let Some(date) = metadata.date {
            info.push(("CreationDate".to_owned(), text_string(&date.pdf())));
        }
        if let Some(title) = &metadata.title {
            info.push(("Title".to_owned(), text_string(title)));
        }
        let info_id = self.writer.reserve();
        self.writer.write_object(info_id, &Obj::Dict(info))?;
        let file = self.writer.finish(self.ids.root, info_id)?;
        self.problems.into_result(file)
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

    fn open_group(&mut self, _name: &str, _gid: Option<&str>) {
        self.group_depth += 1;
        self.generation += 1;
    }

    fn close_group(&mut self, _name: &str) {
        self.group_depth = self.group_depth.saturating_sub(1);
        self.generation += 1;
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
