//! The object-level interface of a PDF document (matplotlib's `PdfFile`):
//! objects reserved, written and referenced, streams, the named resources
//! pages draw with (alpha states, hatches, shadings, images, marker and
//! path forms, fonts), and the end matter (fonts, resources, page tree,
//! document information, cross-reference table, trailer), each part
//! written when the program asks, as matplotlib's `PdfFile` methods write
//! them.

use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;

use pltrs_core::{Affine2D, Path, Point, Rgba};
use pltrs_font::{GlyphId, Type1Font};
use pltrs_text::tex::{DviFont, TexFiles};
use pltrs_text::{AfmDirectory, AfmFont, FontId, FontSource, TextEngine};

use super::fonts::{self, FontKey, SubsetFont};
use super::object::Obj;
use super::resources::{HatchStyle, MarkerPaint, ShadingColors, TemplatePaint};
use super::{PdfRenderer, pages, tex};
use crate::image::RgbaImage;
use crate::vector::{Subset, VectorError};

/// A font a document names (a key of matplotlib's `PdfFile.fontNames`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PdfFont {
    /// One subset of a face of a TrueType file.
    TrueType {
        /// The font file: its path, or the bare file name of a bundled
        /// font.
        file: PathBuf,
        /// The face in the file.
        index: u32,
        /// The subset of its glyphs.
        subset: usize,
    },
    /// One of the 14 PDF core fonts, by the file name of its metrics.
    Core(&'static str),
}

/// A hatch pattern a document holds (matplotlib's `hatchPatterns`): the
/// edge and face colours and the hatch string.
pub type PdfHatch = (Option<Rgba>, Option<Rgba>, String);

impl PdfRenderer {
    /// Reserves an object number (`reserveObject`).
    pub fn reserve_object(&mut self) -> u32 {
        self.writer.reserve()
    }

    /// Records that object `id` starts at the current end of the file
    /// (`recordXref`), for objects the program writes itself with
    /// [`write`](Self::write).
    pub fn record_xref(&mut self, id: u32) {
        self.writer.record(id);
    }

    /// Writes object `id` with `contents` (`writeObject`).
    ///
    /// # Errors
    ///
    /// When `contents` holds a non-finite number.
    pub fn write_object(&mut self, id: u32, contents: &Obj) -> Result<(), VectorError> {
        self.writer.write_object(id, contents)
    }

    /// Appends `data` to the open stream, or to the file when none is open
    /// (`write`).
    pub fn write(&mut self, data: &[u8]) {
        self.writer.write(data);
    }

    /// Writes `objects` as one line of the open stream or of the file
    /// (`output`).
    ///
    /// # Errors
    ///
    /// When an object holds a non-finite number.
    pub fn output_objects(&mut self, objects: &[Obj]) -> Result<(), VectorError> {
        self.writer.output(objects)
    }

    /// The object of the open stream, if any (`currentstream`): the
    /// content stream of the page being drawn, or a stream begun with
    /// [`begin_stream`](Self::begin_stream).
    pub fn open_stream(&self) -> Option<u32> {
        self.writer.open_stream()
    }

    /// Opens the stream object `id` whose length goes to object `length`
    /// (inline for `None`), with the `extra` entries in its dictionary and,
    /// for data PNG-predicted already, the decode parameters `png`
    /// (`beginStream`).
    ///
    /// # Errors
    ///
    /// When a stream is open already.
    pub fn begin_stream(
        &mut self,
        id: u32,
        length: Option<u32>,
        extra: Vec<(String, Obj)>,
        png: Option<Obj>,
    ) -> Result<(), VectorError> {
        if self.writer.open_stream().is_some() {
            return Err(VectorError::StreamOpen);
        }
        self.writer.begin_stream(id, length, extra, png);
        Ok(())
    }

    /// Closes the open stream, compressed at `pdf.compression`, if any
    /// (`endStream`); the content stream of a page after restoring the
    /// graphics state its drawing saved.
    ///
    /// # Errors
    ///
    /// When its dictionary holds a non-finite number.
    pub fn end_stream(&mut self) -> Result<(), VectorError> {
        if self.page_open {
            self.end_page()
        } else {
            self.writer.end_stream()
        }
    }

    /// Writes the stream object `id` holding `data` (`outputStream`).
    ///
    /// # Errors
    ///
    /// When a stream is open already, or `extra` holds a non-finite number.
    pub fn output_stream(
        &mut self,
        id: u32,
        data: &[u8],
        extra: Vec<(String, Obj)>,
    ) -> Result<(), VectorError> {
        self.writer.output_stream(id, data, extra)
    }

    /// Writes the stream object `id` holding `data` whole, with the `extra`
    /// entries and, for data PNG-predicted already, the decode parameters
    /// `png`, whether or not a stream is open: matplotlib's `Stream` of an
    /// inline length keeps its data until it ends, then writes itself to
    /// the end of the file.
    ///
    /// # Errors
    ///
    /// When `extra` holds a non-finite number.
    pub fn write_stream(
        &mut self,
        id: u32,
        extra: Vec<(String, Obj)>,
        png: Option<Obj>,
        data: Vec<u8>,
    ) -> Result<(), VectorError> {
        self.writer.write_stream(id, extra, png, data)
    }

    /// How many objects the file numbers so far, the zero object included
    /// (the length of matplotlib's ``xrefTable``).
    pub fn object_count(&self) -> usize {
        self.writer.object_count()
    }

    /// The name of the graphics state setting the stroke and fill alpha
    /// `alphas` (`alphaState`), made on first use.
    pub fn alpha_state(&mut self, alphas: (f64, f64)) -> String {
        self.resources.alpha_state(alphas)
    }

    /// A new graphics state setting the alphas `alphas`, though another
    /// may set the same: `alphaState` for a value the program has not
    /// given before (matplotlib keys its states by the program's values,
    /// which may be of other types but of equal numbers).
    pub fn new_alpha_state(&mut self, alphas: (f64, f64)) -> String {
        self.resources.new_alpha_state(alphas)
    }

    /// The name of the tiling pattern drawing `pattern` (matplotlib's
    /// hatch string) in `edge`, over `face`, with lines `linewidth` wide
    /// (`hatchPattern`), made on first use.
    pub fn hatch_pattern(
        &mut self,
        edge: Option<Rgba>,
        face: Option<Rgba>,
        pattern: &str,
        linewidth: f64,
    ) -> String {
        self.resources
            .hatch_pattern(hatch_style(edge, face, pattern, linewidth))
    }

    /// A new tiling pattern, though another may draw the same:
    /// `hatchPattern` for a style the program has not given before (see
    /// [`new_alpha_state`](Self::new_alpha_state)).
    pub fn new_hatch_pattern(
        &mut self,
        edge: Option<Rgba>,
        face: Option<Rgba>,
        pattern: &str,
        linewidth: f64,
    ) -> String {
        self.resources
            .new_hatch_pattern(hatch_style(edge, face, pattern, linewidth))
    }

    /// The hatch patterns made so far with their names, in order.
    pub fn hatch_patterns(&self) -> Vec<(PdfHatch, String)> {
        self.resources
            .hatch_styles()
            .map(|(style, name)| {
                let hatch = (style.edge, style.face, style.pattern.clone());
                (hatch, name.to_owned())
            })
            .collect()
    }

    /// The name and object of a new shading of the triangles `points`
    /// (display coordinates) in the vertex colours `colors`
    /// (`addGouraudTriangles`).
    pub fn add_gouraud_triangles(
        &mut self,
        points: Vec<[Point; 3]>,
        colors: ShadingColors,
    ) -> (String, u32) {
        self.resources.gouraud(&mut self.writer, points, colors)
    }

    /// The name of a new image `XObject` showing `image` (`imageObject`).
    pub fn image_object(&mut self, image: &RgbaImage) -> String {
        self.resources.image(&mut self.writer, image)
    }

    /// The name of a new form `XObject` drawing the marker `path` under
    /// `transform` as `paint` says (`markerObject`).
    pub fn marker_object(
        &mut self,
        path: &Path,
        transform: Affine2D,
        paint: MarkerPaint,
    ) -> String {
        self.resources
            .marker(&mut self.writer, path, transform, paint)
    }

    /// The name of a new form `XObject` drawing `path` of a collection
    /// under `transform` as `paint` says (`pathCollectionObject`).
    pub fn path_collection_object(
        &mut self,
        path: &Path,
        transform: Affine2D,
        paint: TemplatePaint,
    ) -> String {
        self.resources
            .path_template(&mut self.writer, path, transform, paint)
    }

    /// Writes the dictionary of the graphics states (`writeExtGSTates`)
    /// and the transparency groups of the soft masks.
    ///
    /// # Errors
    ///
    /// When an alpha is not finite.
    pub fn write_ext_gstates(&mut self) -> Result<(), VectorError> {
        self.resources
            .write_ext_gstates(&mut self.writer, self.ids.resources.ext_gstates)?;
        self.resources.write_soft_mask_groups(&mut self.writer)
    }

    /// Writes the tiling patterns and their dictionary (`writeHatches`).
    ///
    /// # Errors
    ///
    /// When a hatch colour or width is not finite.
    pub fn write_hatches(&mut self) -> Result<(), VectorError> {
        self.resources
            .write_hatches(&mut self.writer, self.ids.resources.patterns, self.height)
    }

    /// Writes the shadings and their dictionary
    /// (`writeGouraudTriangles`).
    ///
    /// # Errors
    ///
    /// When a vertex is not finite.
    pub fn write_gouraud_triangles(&mut self) -> Result<(), VectorError> {
        self.resources
            .write_shadings(&mut self.writer, self.ids.resources.shadings)
    }

    /// Writes the image `XObject`s (`writeImages`).
    ///
    /// # Errors
    ///
    /// When an image cannot be encoded.
    pub fn write_images(&mut self) -> Result<(), VectorError> {
        let compressed = self.options.compression > 0;
        self.resources.write_images(&mut self.writer, compressed)
    }

    /// Writes the marker forms (`writeMarkers`).
    ///
    /// # Errors
    ///
    /// When a marker is not finite.
    pub fn write_markers(&mut self) -> Result<(), VectorError> {
        self.resources.write_markers(&mut self.writer)
    }

    /// Writes the path-collection templates
    /// (`writePathCollectionTemplates`).
    ///
    /// # Errors
    ///
    /// When a path is not finite.
    pub fn write_path_collection_templates(&mut self) -> Result<(), VectorError> {
        self.resources.write_path_templates(&mut self.writer)
    }

    /// The name of subset `subset` of face `index` of the TrueType file
    /// `path` (`fontName`), assigned on first use.
    ///
    /// # Errors
    ///
    /// When the file cannot be read or has no face `index`.
    pub fn font_name(
        &mut self,
        path: &FsPath,
        index: u32,
        subset: usize,
    ) -> Result<String, VectorError> {
        let font = face_of_file(path, index)?;
        Ok(self.font_names.name(font, subset))
    }

    /// The name of the PDF core font whose metrics file is `file`
    /// (`fontName` under `pdf.use14corefonts`), assigned on first use.
    ///
    /// # Errors
    ///
    /// When `file` is not the metrics of a core font.
    pub fn core_font_name(&mut self, file: &str) -> Result<String, VectorError> {
        let font = AfmFont::all()
            .iter()
            .find(|font| font.directory == AfmDirectory::PdfCore && font.file == file)
            .ok_or(VectorError::Unsupported(
                "a PDF core font is named by its bundled metrics",
            ))?;
        Ok(self.font_names.core_name(font))
    }

    /// A name for a font the program names that pltrs cannot embed (a
    /// file it cannot read, a subset that is no count): matplotlib's
    /// `fontName` names any file, failing only when it writes the fonts.
    /// The name takes its place among the others and is written as no
    /// font.
    pub fn reserve_font_name(&mut self) -> String {
        self.font_names.reserve()
    }

    /// The fonts named so far with their names, in first-use order
    /// (matplotlib's `fontNames`), but those of
    /// [`reserve_font_name`](Self::reserve_font_name).
    pub fn font_names(&self) -> Vec<(PdfFont, String)> {
        let db = TextEngine::global().database();
        self.font_names
            .entries()
            .filter_map(|(key, name)| {
                let font = match key {
                    FontKey::Embedded(id, subset) => PdfFont::TrueType {
                        file: match db.source(id) {
                            Some(FontSource::File(path)) => path.clone(),
                            Some(FontSource::Bundled(name)) => PathBuf::from(name),
                            None => PathBuf::new(),
                        },
                        index: db.face_index(id).unwrap_or_default(),
                        subset,
                    },
                    FontKey::Core(font) => PdfFont::Core(font.file),
                    FontKey::Reserved(_) => return None,
                };
                Some((font, name.to_owned()))
            })
            .collect()
    }

    /// The name of the TeX font `font` (`F-<texname>`, `dviFontName`),
    /// whose files `files` finds, registered to be embedded with the
    /// glyphs drawn in it.
    pub fn dvi_font_name(&mut self, font: Arc<DviFont>, files: &'static TexFiles) -> String {
        self.dvi_fonts.register(font, files)
    }

    /// Whether glyphs of the TeX font registered under `name`
    /// ([`dvi_font_name`](Self::dvi_font_name)) were drawn in the document,
    /// which `writeFonts` needs to embed it.
    pub fn dvi_font_used(&self, name: &str) -> bool {
        self.dvi_fonts.is_used(name)
    }

    /// Writes the font dictionaries of every font named and their
    /// resource dictionary (`writeFonts`).
    ///
    /// # Errors
    ///
    /// When a font cannot be read or embedded.
    pub fn write_fonts(&mut self) -> Result<(), VectorError> {
        let tex_fonts = self.dvi_fonts.embed(&mut self.writer)?;
        fonts::write_fonts(
            &mut self.writer,
            tex_fonts,
            &mut self.fonts,
            &self.font_names,
            &self.tracker,
            self.ids.resources.fonts,
            self.options.fonttype,
        )
    }

    /// Whether glyphs of face `index` of the font file `path` were drawn
    /// in the document (matplotlib's character tracker holds the font,
    /// which `writeFonts` and `embedTTF` read).
    ///
    /// # Errors
    ///
    /// When the file cannot be read or has no face `index`.
    pub fn tracks_font(&self, path: &FsPath, index: u32) -> Result<bool, VectorError> {
        let id = face_of_file(path, index)?;
        Ok(self.tracker.subset(id, 0).is_some())
    }

    /// Embeds the glyphs `charmap` (character codes and the glyphs they
    /// show) of face `index` of the TrueType file `path` as subset
    /// `subset_index`, in the format of `pdf.fonttype` (`embedTTF`);
    /// returns the font dictionary.
    ///
    /// # Errors
    ///
    /// When `charmap` is empty or the font cannot be read.
    pub fn embed_ttf(
        &mut self,
        path: &FsPath,
        index: u32,
        subset_index: usize,
        charmap: Vec<(u32, GlyphId)>,
    ) -> Result<u32, VectorError> {
        if charmap.is_empty() {
            return Err(VectorError::Unsupported(
                "a font is embedded with at least one glyph",
            ));
        }
        let id = face_of_file(path, index)?;
        let subset = Subset::from_entries(charmap);
        let subset_font = SubsetFont {
            font: self.fonts.get(id)?,
            id,
            index: subset_index,
            subset: &subset,
        };
        fonts::embed_subset(
            &mut self.writer,
            subset_font,
            &self.tracker,
            self.options.fonttype,
        )
    }

    /// Writes the font descriptor and the program of the Type 1 font
    /// `font` (`createType1Descriptor`); returns the descriptor.
    ///
    /// # Errors
    ///
    /// When a number of the font is not finite.
    pub fn create_type1_descriptor(&mut self, font: &Type1Font) -> Result<u32, VectorError> {
        let name = font
            .prop()
            .font_name()
            .as_text()
            .unwrap_or_default()
            .to_owned();
        tex::type1_descriptor(&mut self.writer, font, &name)
    }

    /// Writes the document information dictionary (`writeInfoDict`);
    /// returns its object.
    ///
    /// # Errors
    ///
    /// Never in practice: the information is text and dates.
    pub fn write_info_dict(&mut self) -> Result<u32, VectorError> {
        let info = self.writer.reserve();
        self.writer
            .write_object(info, &pages::info_dict(&self.options.metadata))?;
        self.info = Some(info);
        Ok(info)
    }

    /// The first object reserved but not written yet, if any (the one
    /// [`write_xref`](Self::write_xref) would fail on).
    pub fn unwritten_object(&self) -> Option<usize> {
        self.writer.unwritten()
    }

    /// Writes the cross-reference table (`writeXref`).
    ///
    /// # Errors
    ///
    /// When an object was reserved but never written.
    pub fn write_xref(&mut self) -> Result<(), VectorError> {
        self.writer.write_xref()
    }

    /// Writes the trailer (`writeTrailer`), after the document information
    /// if it was not written yet.
    ///
    /// # Errors
    ///
    /// As [`write_info_dict`](Self::write_info_dict).
    pub fn write_trailer(&mut self) -> Result<(), VectorError> {
        let info = match self.info {
            Some(info) => info,
            None => self.write_info_dict()?,
        };
        self.writer.write_trailer(self.ids.root, info)
    }

    /// Closes the page and writes the deferred objects and the end of the
    /// file (`finalize`): what [`write_deferred`](Self::write_deferred)
    /// writes, then the cross-reference table and the trailer.
    ///
    /// # Errors
    ///
    /// As [`write_deferred`](Self::write_deferred) and
    /// [`write_xref`](Self::write_xref).
    pub fn finalize(&mut self) -> Result<(), VectorError> {
        self.write_deferred()?;
        self.write_xref()?;
        self.write_trailer()
    }

    /// Closes the page and writes the deferred objects, all `finalize`
    /// writes before the cross-reference table: annotations, fonts,
    /// graphics states, hatches, shadings, `XObject`s, the page tree and
    /// the document information.
    ///
    /// # Errors
    ///
    /// When a font, image or number met while drawing could not be
    /// written.
    pub fn write_deferred(&mut self) -> Result<(), VectorError> {
        self.end_page()?;
        self.pages.write_annotations(&mut self.writer)?;
        self.write_fonts()?;
        self.resources.write(
            &mut self.writer,
            self.ids.resources,
            self.height,
            self.options.compression > 0,
        )?;
        self.pages.write_tree(&mut self.writer, self.ids.pages)?;
        self.write_info_dict()?;
        Ok(())
    }

    /// The file written so far, after closing the open stream (matplotlib's
    /// `close`).
    ///
    /// # Errors
    ///
    /// The first problem met while drawing (a font, image or number that
    /// could not be written), or one of closing the stream.
    pub fn close(mut self) -> Result<Vec<u8>, VectorError> {
        self.end_stream()?;
        self.problems.into_result(self.writer.into_bytes())
    }
}

/// The face `index` of the font file `path`, as the text engine knows it
/// (a bundled font by its place in the data directory, or its bare name).
fn face_of_file(path: &FsPath, index: u32) -> Result<FontId, VectorError> {
    let faces = TextEngine::global().file_faces(path)?;
    usize::try_from(index)
        .ok()
        .and_then(|index| faces.get(index).copied())
        .ok_or(VectorError::Unsupported(
            "a font is named by a face its file has",
        ))
}

/// The style of a hatch pattern.
fn hatch_style(
    edge: Option<Rgba>,
    face: Option<Rgba>,
    pattern: &str,
    linewidth: f64,
) -> HatchStyle {
    HatchStyle {
        edge,
        face,
        pattern: pattern.to_owned(),
        linewidth,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::PdfOptions;

    fn document() -> PdfRenderer {
        PdfRenderer::document(PdfOptions {
            compression: 0,
            ..PdfOptions::default()
        })
        .unwrap()
    }

    fn text(pdf: &PdfRenderer) -> String {
        String::from_utf8_lossy(pdf.writer.bytes()).into_owned()
    }

    #[test]
    fn objects_and_streams_are_written_where_the_program_says() {
        let mut pdf = document();
        let object = pdf.reserve_object();
        pdf.write_object(object, &Obj::Array(vec![Obj::Int(1), Obj::Null]))
            .unwrap();
        let stream = pdf.reserve_object();
        pdf.begin_stream(stream, None, Vec::new(), None).unwrap();
        assert_eq!(pdf.open_stream(), Some(stream));
        assert!(matches!(
            pdf.begin_stream(stream, None, Vec::new(), None),
            Err(VectorError::StreamOpen)
        ));
        pdf.output_objects(&[Obj::Real(1.5), Obj::op("w")]).unwrap();
        pdf.end_stream().unwrap();
        assert_eq!(pdf.open_stream(), None);
        let text = text(&pdf);
        assert!(text.contains(&format!("{object} 0 obj\n[ 1 null ]\nendobj\n")));
        assert!(text.contains(&format!("{stream} 0 obj\n<< /Length 6 >>\nstream\n1.5 w\n")));
    }

    #[test]
    fn unwritten_objects_fail_the_cross_reference_table() {
        // The page tree (object 2) is written when the document ends.
        let mut pdf = document();
        assert_eq!(pdf.unwritten_object(), Some(2));
        assert!(matches!(
            pdf.write_xref(),
            Err(VectorError::UnwrittenObject(2))
        ));
    }

    #[test]
    fn resources_are_named_and_the_file_finished_on_demand() {
        let mut pdf = document();
        assert_eq!(pdf.alpha_state((0.5, 0.5)), "A1");
        assert_eq!(pdf.alpha_state((0.5, 0.5)), "A1");
        assert_eq!(pdf.new_alpha_state((0.5, 0.5)), "A2");
        assert_eq!(pdf.hatch_pattern(Some(Rgba::BLACK), None, "/", 1.0), "H1");
        assert_eq!(
            pdf.hatch_patterns(),
            [((Some(Rgba::BLACK), None, "/".to_owned()), "H1".to_owned())]
        );
        let (name, _) = pdf.add_gouraud_triangles(
            vec![[
                Point::new(0.0, 0.0),
                Point::new(1.0, 0.0),
                Point::new(0.0, 1.0),
            ]],
            ShadingColors::Gray(vec![[0.0, 0.5, 1.0]]),
        );
        assert_eq!(name, "GT0");
        assert_eq!(pdf.core_font_name("Helvetica.afm").unwrap(), "F1");
        assert!(pdf.core_font_name("nothing.afm").is_err());
        assert_eq!(
            pdf.font_names(),
            [(PdfFont::Core("Helvetica.afm"), "F1".to_owned())]
        );
        assert_eq!(
            pdf.font_name(FsPath::new("DejaVuSans.ttf"), 0, 0).unwrap(),
            "F2"
        );
        assert!(pdf.font_name(FsPath::new("DejaVuSans.ttf"), 3, 0).is_err());
        assert_eq!(pdf.reserve_font_name(), "F3");
        assert_eq!(
            pdf.font_name(FsPath::new("DejaVuSans.ttf"), 0, 1).unwrap(),
            "F4"
        );
        assert_eq!(pdf.font_names().len(), 3);
        pdf.new_page(1.0, 1.0, 72.0).unwrap();
        pdf.finalize().unwrap();
        let file = pdf.close().unwrap();
        let text = String::from_utf8_lossy(&file);
        assert!(text.contains("/A1 << /Type /ExtGState /CA 0.5 /ca 0.5 >>"));
        assert!(text.contains("/ShadingType 4"));
        assert!(text.contains("/BaseFont /Helvetica"));
        assert!(text.ends_with("%%EOF\n"));
    }

    #[test]
    fn the_end_matter_is_written_part_by_part() {
        let mut pdf = document();
        pdf.write_info_dict().unwrap();
        pdf.write_fonts().unwrap();
        pdf.write_ext_gstates().unwrap();
        pdf.write_hatches().unwrap();
        pdf.write_gouraud_triangles().unwrap();
        pdf.write_images().unwrap();
        pdf.write_markers().unwrap();
        pdf.write_path_collection_templates().unwrap();
        let text = text(&pdf);
        assert!(text.contains("/Creator"));
        assert!(
            text.contains("4 0 obj\n<< >>\nendobj\n"),
            "the graphics states"
        );
    }
}
