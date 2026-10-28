//! TeX text (`RendererPdf.draw_tex`): the DVI page's glyphs shown in the
//! Type 1 fonts `pdftex.map` names, consecutive glyphs on a baseline
//! joined into strings with kerns, the rules filled as paths; and the
//! fonts (`dviFontName`, `_embedTeXFont`, `createType1Descriptor`),
//! embedded as subsets of their Type 1 programs, re-encoded and slanted or
//! extended as the map says, with widths from the TeX font metrics.

use std::collections::BTreeMap;
use std::sync::Arc;

use plt_core::{Affine2D, Path, PathCode, Point};
use plt_font::{GlyphId, PsNumber, PsValue, Type1Encoding, Type1Font};
use plt_text::tex::{DviBox, DviFont, DviPage, TexError, TexFile, TexFiles, TexText};

use super::PdfRenderer;
use super::fonts::subset_prefix;
use super::object::Obj;
use super::pages::Link;
use super::writer::PdfWriter;
use crate::gc::GraphicsContext;
use crate::vector::{POINTS_DPI, VectorError};

/// A TeX font a document uses: the DVI font last registered under its
/// name (`_dviFontInfo`) and the glyph of every character code shown
/// (`CharacterTracker.used`).
#[derive(Debug)]
struct UsedFont {
    font: Arc<DviFont>,
    glyphs: BTreeMap<i64, GlyphId>,
}

/// The TeX fonts of a document by their resource names (`F-<texname>`),
/// and where their files are found.
#[derive(Debug, Default)]
pub(super) struct DviFonts {
    fonts: BTreeMap<String, UsedFont>,
    files: Option<&'static TexFiles>,
}

/// One element of the text of a TeX page being collected: a font change,
/// or strings and kerns starting at `(x, y)` and ending at `end`.
#[derive(Clone, Debug, PartialEq)]
enum Item {
    Font(String, f64),
    Text {
        x: f64,
        y: f64,
        shown: Vec<Shown>,
        end: f64,
    },
}

/// A string or a kern of a `TJ` array.
#[derive(Clone, Debug, PartialEq)]
enum Shown {
    Codes(Vec<u8>),
    Kern(f64),
}

/// Joins consecutive strings on one baseline: a gap under 0.1 point is
/// dropped, a larger one becomes a kern in thousandths of the font size
/// (the loop of `draw_tex`).
#[expect(
    clippy::float_cmp,
    reason = "matplotlib joins strings on exactly one baseline"
)]
fn join_strings(mut seq: Vec<Item>) -> Vec<Item> {
    let mut i = 0;
    let mut fontsize = 0.0;
    while i + 1 < seq.len() {
        match (&seq[i], &seq[i + 1]) {
            (Item::Font(_, size), _) => fontsize = *size,
            (
                Item::Text { y, end, .. },
                Item::Text {
                    x: x2,
                    y: y2,
                    shown: next,
                    end: end2,
                },
            ) if y == y2 => {
                let offset = end - x2;
                let (next, x2, end2) = (next.clone(), *x2, *end2);
                if let Item::Text { shown, end, .. } = &mut seq[i] {
                    if offset.abs() < 0.1 {
                        if let (Some(Shown::Codes(last)), Some(Shown::Codes(first))) =
                            (shown.last_mut(), next.first())
                        {
                            last.extend_from_slice(first);
                        }
                        *end += end2 - x2;
                    } else {
                        shown.push(Shown::Kern(offset * 1000.0 / fontsize));
                        shown.extend(next.into_iter().take(1));
                        *end = end2;
                    }
                }
                seq.remove(i + 1);
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    seq
}

/// A Type 1 problem as a TeX error.
fn type1_error(err: impl std::fmt::Display) -> TexError {
    TexError::malformed(TexFile::Type1, err.to_string())
}

/// `pdfRepr` of a number of a Type 1 font's dictionary.
fn number(value: PsNumber) -> Obj {
    match value {
        PsNumber::Int(int) => Obj::Int(int),
        PsNumber::Real(real) => Obj::Real(real),
    }
}

impl PdfRenderer {
    /// `draw_tex`: the line typeset at its size, measured at 72 dpi.
    pub(super) fn pdf_tex(
        &mut self,
        gc: &GraphicsContext,
        tex: &TexText,
        origin: Point,
        angle: f64,
    ) -> Result<(), TexError> {
        let page = tex.page(tex.size(), POINTS_DPI)?;
        if let Some(url) = &gc.url {
            self.pages.add_link(Link::over_block(
                url,
                origin,
                page.width,
                page.height,
                angle,
            ));
        }
        let files = tex.manager.files();
        self.dvi_fonts.files.get_or_insert(files);
        let seq = self.track_text(&page, files)?;
        let placement = Affine2D::identity()
            .rotate_deg(angle)
            .translate(origin.x, origin.y);
        self.check_gc(gc, Some(gc.stroke_color()));
        self.show_items(join_strings(seq), &placement, angle);
        self.fill_rules(gc, &page.boxes, &placement);
        Ok(())
    }

    /// The font changes and glyphs of `page`, each glyph recorded as used
    /// in its font (`dviFontName` and `track_glyph`).
    fn track_text(&mut self, page: &DviPage, files: &TexFiles) -> Result<Vec<Item>, TexError> {
        let mut seq = Vec::new();
        let mut old_font: Option<&Arc<DviFont>> = None;
        for text in &page.text {
            let name = format!("F-{}", text.font.name());
            if old_font != Some(&text.font) {
                seq.push(Item::Font(name.clone(), text.font.size()));
                old_font = Some(&text.font);
            }
            let code = u8::try_from(text.glyph).map_err(|_| {
                TexError::malformed(
                    TexFile::Dvi,
                    format!("character {} is not a byte", text.glyph),
                )
            })?;
            seq.push(Item::Text {
                x: text.x,
                y: text.y,
                shown: vec![Shown::Codes(vec![code])],
                end: text.x + text.width,
            });
            let index = files.glyph_index(&text.font, text.glyph)?;
            let used = self
                .dvi_fonts
                .fonts
                .entry(name)
                .or_insert_with(|| UsedFont {
                    font: Arc::clone(&text.font),
                    glyphs: BTreeMap::new(),
                });
            used.font = Arc::clone(&text.font);
            used.glyphs.entry(text.glyph).or_insert(index);
        }
        Ok(seq)
    }

    /// Writes the text object: font changes, then every string placed by
    /// `placement` and shown with `Tj`, or with `TJ` and its kerns.
    fn show_items(&mut self, items: Vec<Item>, placement: &Affine2D, angle: f64) {
        self.output(&[Obj::op("BT")]);
        let mut old = Point::ORIGIN;
        for item in items {
            match item {
                Item::Font(name, size) => {
                    self.output(&[Obj::name(name), Obj::Real(size), Obj::op("Tf")]);
                }
                Item::Text { x, y, shown, .. } => {
                    let at = placement.transform_point(Point::new(x, y));
                    self.setup_textpos(at, angle, old);
                    old = at;
                    if let [Shown::Codes(codes)] = shown.as_slice() {
                        self.output(&[Obj::Str(codes.clone()), Obj::op("Tj")]);
                    } else {
                        let array = shown
                            .into_iter()
                            .map(|item| match item {
                                Shown::Codes(codes) => Obj::Str(codes),
                                Shown::Kern(kern) => Obj::Real(kern),
                            })
                            .collect();
                        self.output(&[Obj::Array(array), Obj::op("TJ")]);
                    }
                }
            }
        }
        self.output(&[Obj::op("ET")]);
    }

    /// Fills the page's rules placed by `placement` in the text colour,
    /// with a line width of 0.
    fn fill_rules(&mut self, gc: &GraphicsContext, rules: &[DviBox], placement: &Affine2D) {
        let mut box_gc = gc.clone();
        box_gc.linewidth = 0.0;
        let codes = vec![
            PathCode::MoveTo,
            PathCode::LineTo,
            PathCode::LineTo,
            PathCode::LineTo,
            PathCode::ClosePoly,
        ];
        for b in rules {
            let (x, y, w, h) = (b.x, b.y, b.width, b.height);
            let vertices = vec![
                Point::new(x, y),
                Point::new(x + w, y),
                Point::new(x + w, y + h),
                Point::new(x, y + h),
                Point::ORIGIN,
            ];
            if let Ok(path) = Path::new(vertices, codes.clone()) {
                crate::renderer::Renderer::draw_path(
                    self,
                    &box_gc,
                    &path,
                    placement,
                    Some(gc.stroke_color()),
                );
            }
        }
    }

    /// `_setup_textpos`: moves to `at` from `old` with `Td` when the text
    /// is not rotated, else sets the text matrix.
    pub(super) fn setup_textpos(&mut self, at: Point, angle: f64, old: Point) {
        if angle == 0.0 {
            self.output(&[
                Obj::Real(at.x - old.x),
                Obj::Real(at.y - old.y),
                Obj::op("Td"),
            ]);
        } else {
            let (sin, cos) = angle.to_radians().sin_cos();
            self.output(&[
                Obj::Real(cos),
                Obj::Real(sin),
                Obj::Real(-sin),
                Obj::Real(cos),
                Obj::Real(at.x),
                Obj::Real(at.y),
                Obj::op("Tm"),
            ]);
            self.output(&[Obj::Int(0), Obj::Int(0), Obj::op("Td")]);
        }
    }
}

impl DviFonts {
    /// Embeds every TeX font the document uses, by name (`writeFonts`
    /// does them first, sorted); returns the font resource entries.
    pub(super) fn embed(&self, writer: &mut PdfWriter) -> Result<Vec<(String, Obj)>, VectorError> {
        let Some(files) = self.files else {
            return Ok(Vec::new());
        };
        self.fonts
            .iter()
            .map(|(name, used)| Ok((name.clone(), Obj::Ref(embed_font(writer, files, used)?))))
            .collect()
    }
}

/// `_embedTeXFont`: the font dictionary of one TeX font.
fn embed_font(
    writer: &mut PdfWriter,
    files: &TexFiles,
    used: &UsedFont,
) -> Result<u32, VectorError> {
    let psfont = files.psfont(used.font.texname())?;
    let filename = psfont
        .filename
        .clone()
        .ok_or_else(|| TexError::NoType1Font {
            psname: String::from_utf8_lossy(&psfont.psname).into_owned(),
            texname: String::from_utf8_lossy(&psfont.texname).into_owned(),
        })?;
    let dict_id = writer.reserve();
    let mut font = Type1Font::open(&filename).map_err(type1_error)?;
    if let Some(encoding) = &psfont.encoding {
        let names = files.encoding(encoding)?;
        let codes = names
            .iter()
            .enumerate()
            .map(|(i, name)| (i as i64, name.clone()));
        font = font
            .with_encoding(&Type1Encoding::from_codes(codes))
            .map_err(type1_error)?;
    }
    if psfont.effects.slant.is_some() || psfont.effects.extend.is_some() {
        font = font.transform(psfont.effects).map_err(type1_error)?;
    }
    let glyphs: Vec<u16> = used.glyphs.values().map(|glyph| glyph.0).collect();
    let codes = used
        .glyphs
        .keys()
        .filter_map(|&code| u32::try_from(code).ok());
    let font = font
        .subset(codes, &subset_prefix(&glyphs))
        .map_err(type1_error)?;
    let prop = font.prop();
    let font_name = prop.font_name().as_text().unwrap_or_default().to_owned();
    let descriptor_id = type1_descriptor(writer, &font, &font_name)?;
    let encoding: Vec<(i64, String)> = prop
        .encoding()
        .map(|e| {
            e.iter()
                .map(|(code, name)| (code, name.to_owned()))
                .collect()
        })
        .unwrap_or_default();
    let first = encoding.first().map_or(0, |(code, _)| *code);
    let last = encoding.last().map_or(255, |(code, _)| *code);
    let mut differences = Vec::new();
    let mut previous = None;
    for (code, name) in &encoding {
        if previous.is_none_or(|p: i64| p + 1 != *code) {
            differences.push(Obj::Int(*code));
        }
        differences.push(Obj::name(name.clone()));
        previous = Some(*code);
    }
    let metrics = used.font.metrics();
    let widths: Vec<Obj> = (first..=last)
        .map(|code| {
            Obj::Int(
                metrics
                    .metrics(code)
                    .map_or(0, |m| (1000 * i64::from(m.width)) >> 20),
            )
        })
        .collect();
    let widths_id = writer.reserve();
    writer.write_object(widths_id, &Obj::Array(widths))?;
    writer.write_object(
        dict_id,
        &Obj::dict([
            ("Type", Obj::name("Font")),
            ("Subtype", Obj::name("Type1")),
            ("BaseFont", Obj::name(font_name)),
            ("FontDescriptor", Obj::Ref(descriptor_id)),
            (
                "Encoding",
                Obj::dict([
                    ("Type", Obj::name("Encoding")),
                    ("Differences", Obj::Array(differences)),
                ]),
            ),
            ("FirstChar", Obj::Int(first)),
            ("LastChar", Obj::Int(last)),
            ("Widths", Obj::Ref(widths_id)),
        ]),
    )?;
    Ok(dict_id)
}

/// `createType1Descriptor`: writes the font descriptor and the font
/// program (cleartext and encrypted parts); returns the descriptor.
fn type1_descriptor(
    writer: &mut PdfWriter,
    font: &Type1Font,
    font_name: &str,
) -> Result<u32, VectorError> {
    let descriptor_id = writer.reserve();
    let file_id = writer.reserve();
    let prop = font.prop();
    let italic_angle = prop.italic_angle();
    let flags = i64::from(prop.is_fixed_pitch().is_truthy())
        | 1 << 2
        | i64::from(italic_angle.is_truthy()) << 6;
    let mut charset: Vec<String> = prop
        .encoding()
        .map(|e| {
            e.iter()
                .filter(|(_, name)| *name != ".notdef")
                .map(|(_, name)| format!("/{name}"))
                .collect()
        })
        .unwrap_or_default();
    charset.sort();
    let bbox = prop.font_bbox();
    let italic = match italic_angle {
        PsValue::Number(value) => number(*value),
        other => Obj::Real(other.to_f64().unwrap_or(0.0)),
    };
    let family = prop
        .family_name()
        .as_text()
        .unwrap_or_default()
        .as_bytes()
        .to_vec();
    writer.write_object(
        descriptor_id,
        &Obj::dict([
            ("Type", Obj::name("FontDescriptor")),
            ("FontName", Obj::name(font_name)),
            ("Flags", Obj::Int(flags)),
            (
                "FontBBox",
                Obj::Array(bbox.iter().copied().map(number).collect()),
            ),
            ("ItalicAngle", italic),
            ("Ascent", number(bbox[3])),
            ("Descent", number(bbox[1])),
            ("CapHeight", Obj::Int(1000)),
            ("XHeight", Obj::Int(500)),
            ("FontFile", Obj::Ref(file_id)),
            ("FontFamily", Obj::Str(family)),
            ("StemV", Obj::Int(50)),
            ("CharSet", Obj::Str(charset.concat().into_bytes())),
        ]),
    )?;
    let parts = font.parts();
    let program = [parts.cleartext.as_slice(), parts.encrypted.as_slice()].concat();
    writer.output_stream(
        file_id,
        &program,
        vec![
            ("Length1".to_owned(), Obj::Int(parts.cleartext.len() as i64)),
            ("Length2".to_owned(), Obj::Int(parts.encrypted.len() as i64)),
            ("Length3".to_owned(), Obj::Int(0)),
        ],
    )?;
    Ok(descriptor_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(x: f64, y: f64, code: u8, width: f64) -> Item {
        Item::Text {
            x,
            y,
            shown: vec![Shown::Codes(vec![code])],
            end: x + width,
        }
    }

    #[test]
    fn strings_join_with_kerns_as_matplotlib_joins_them() {
        let joined = join_strings(vec![
            Item::Font("F-cmr10".into(), 10.0),
            text(0.0, 0.0, b'a', 5.0),
            text(5.05, 0.0, b'b', 5.0),
            text(11.0, 0.0, b'c', 5.0),
            text(16.0, 2.0, b'd', 5.0),
        ]);
        assert_eq!(
            joined,
            [
                Item::Font("F-cmr10".into(), 10.0),
                Item::Text {
                    x: 0.0,
                    y: 0.0,
                    shown: vec![
                        Shown::Codes(b"ab".to_vec()),
                        Shown::Kern((10.0 - 11.0) * 1000.0 / 10.0),
                        Shown::Codes(b"c".to_vec()),
                    ],
                    end: 16.0,
                },
                text(16.0, 2.0, b'd', 5.0),
            ]
        );
    }
}
