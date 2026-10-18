//! [`Face`]: a zero-copy view of one font face.

use crate::cff::Cff;
use crate::error::{FontError, Result};
use crate::layout::common::LayoutTable;
use crate::layout::gdef::Gdef;
use crate::outline::{Midpoint, Outline, OutlineSink};
use crate::tables::cmap::{CharmapInfo, Cmap};
use crate::tables::directory::{SfntFlavor, TableDirectory};
use crate::tables::glyf::{Glyf, Scaling};
use crate::tables::head::{Head, Hhea, MaxProfile};
use crate::tables::hmtx::Hmtx;
use crate::tables::kern::Kern;
use crate::tables::name::{Name, NameKey, id};
use crate::tables::os2::Os2;
use crate::tables::post::{Post, PostTable};
use crate::types::{GlyphId, Rect, Tag};

/// Outline source of a face.
#[derive(Clone, Debug)]
enum Outlines<'a> {
    TrueType(Glyf<'a>),
    Cff(Box<Cff<'a>>),
    None,
}

/// FreeType's style flags.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct StyleFlags {
    /// The face is italic or oblique.
    pub italic: bool,
    /// The face is bold.
    pub bold: bool,
}

/// A parsed font face borrowing the font file bytes.
///
/// All metrics are in font units unless documented otherwise; use
/// [`Size`](crate::Size) for FreeType-compatible scaled values.
#[derive(Clone, Debug)]
pub struct Face<'a> {
    data: &'a [u8],
    index: u32,
    directory: TableDirectory<'a>,
    head: Head,
    hhea: Hhea,
    maxp: MaxProfile,
    hmtx: Hmtx<'a>,
    cmap: Cmap<'a>,
    charmap: Option<usize>,
    outlines: Outlines<'a>,
    name: Name<'a>,
    os2: Option<Os2>,
    post: Option<PostTable<'a>>,
    kern: Kern<'a>,
    gdef: Gdef<'a>,
    gsub: Option<LayoutTable<'a>>,
    gpos: Option<LayoutTable<'a>>,
}

/// Parses an optional table, treating a broken table like an absent one
/// (as FreeType and HarfBuzz do for non-essential tables).
fn optional<'a, T>(
    directory: &TableDirectory<'a>,
    tag: [u8; 4],
    parse: impl FnOnce(&'a [u8]) -> Result<T>,
) -> Option<T> {
    directory.table(Tag(tag)).and_then(|data| parse(data).ok())
}

impl<'a> Face<'a> {
    /// Parses face `index` of a font file (0 for plain `.ttf`/`.otf`).
    pub fn parse(data: &'a [u8], index: u32) -> Result<Self> {
        let directory = TableDirectory::parse(data, index)?;
        let head = Head::parse(directory.required(Tag::new(b"head"))?)?;
        let hhea = Hhea::parse(directory.required(Tag::new(b"hhea"))?)?;
        let maxp = MaxProfile::parse(directory.required(Tag::new(b"maxp"))?)?;
        let hmtx = Hmtx::parse(
            directory.required(Tag::new(b"hmtx"))?,
            hhea.number_of_h_metrics,
            maxp.num_glyphs,
        )?;
        let cmap = match directory.table(Tag::new(b"cmap")) {
            Some(table) => Cmap::parse(table)?,
            None => Cmap::default(),
        };
        let outlines = Self::outlines(&directory, &head)?;
        Ok(Self {
            data,
            index,
            head,
            hhea,
            maxp,
            hmtx,
            charmap: cmap.default_index(),
            cmap,
            outlines,
            name: optional(&directory, *b"name", Name::parse).unwrap_or_default(),
            os2: optional(&directory, *b"OS/2", Os2::parse),
            post: optional(&directory, *b"post", PostTable::parse),
            kern: optional(&directory, *b"kern", Kern::parse).unwrap_or_default(),
            gdef: optional(&directory, *b"GDEF", Gdef::parse).unwrap_or_default(),
            gsub: optional(&directory, *b"GSUB", |d| {
                LayoutTable::parse(d, Tag::new(b"GSUB"), 7)
            }),
            gpos: optional(&directory, *b"GPOS", |d| {
                LayoutTable::parse(d, Tag::new(b"GPOS"), 9)
            }),
            directory,
        })
    }

    fn outlines(directory: &TableDirectory<'a>, head: &Head) -> Result<Outlines<'a>> {
        if let (Some(loca), Some(glyf)) = (
            directory.table(Tag::new(b"loca")),
            directory.table(Tag::new(b"glyf")),
        ) {
            return Ok(Outlines::TrueType(Glyf::new(
                loca,
                glyf,
                head.index_to_loc_format != 0,
            )));
        }
        if let Some(cff) = directory.table(Tag::new(b"CFF ")) {
            return Ok(Outlines::Cff(Box::new(Cff::parse(cff)?)));
        }
        Ok(Outlines::None)
    }

    /// The whole font file this face was parsed from.
    pub const fn data(&self) -> &'a [u8] {
        self.data
    }

    /// Index of this face inside its file.
    pub const fn index(&self) -> u32 {
        self.index
    }

    /// Outline flavour announced by the sfnt header.
    pub const fn flavor(&self) -> SfntFlavor {
        self.directory.flavor()
    }

    /// Raw bytes of table `tag`, if present.
    pub fn table(&self, tag: Tag) -> Option<&'a [u8]> {
        self.directory.table(tag)
    }

    /// Tags of all tables in the face.
    pub fn table_tags(&self) -> impl Iterator<Item = Tag> + 'a {
        self.directory.tags()
    }

    /// The `head` table.
    pub const fn head(&self) -> &Head {
        &self.head
    }

    /// The `hhea` table.
    pub const fn hhea(&self) -> &Hhea {
        &self.hhea
    }

    /// The `OS/2` table, if present and readable.
    pub const fn os2(&self) -> Option<&Os2> {
        self.os2.as_ref()
    }

    /// The `post` table header, if present and readable.
    pub fn post(&self) -> Option<&Post> {
        self.post.as_ref().map(|post| &post.header)
    }

    /// Design units per em.
    pub const fn units_per_em(&self) -> u16 {
        self.head.units_per_em
    }

    /// Ascender, descender and line height as FreeType computes them: from
    /// `hhea`, falling back to the `OS/2` typographic then Windows metrics
    /// when `hhea` has none.
    fn vertical_metrics(&self) -> (i16, i16, i16) {
        // FreeType computes in `int` and stores `FT_Short`s.
        let height = |ascender: i16, descender: i16, gap: i16| {
            (i32::from(ascender) - i32::from(descender) + i32::from(gap)) as i16
        };
        let hhea = &self.hhea;
        if hhea.ascender != 0 || hhea.descender != 0 {
            return (
                hhea.ascender,
                hhea.descender,
                height(hhea.ascender, hhea.descender, hhea.line_gap),
            );
        }
        match &self.os2 {
            Some(os2) if os2.typo_ascender != 0 || os2.typo_descender != 0 => (
                os2.typo_ascender,
                os2.typo_descender,
                height(os2.typo_ascender, os2.typo_descender, os2.typo_line_gap),
            ),
            Some(os2) => {
                let (ascent, descent) = (
                    os2.win_ascent as i16,
                    (os2.win_descent as i16).wrapping_neg(),
                );
                (ascent, descent, height(ascent, descent, 0))
            }
            None => (0, 0, hhea.line_gap),
        }
    }

    /// Typographic ascender (FreeType's `face->ascender`).
    pub fn ascender(&self) -> i16 {
        self.vertical_metrics().0
    }

    /// Typographic descender, usually negative (FreeType's
    /// `face->descender`).
    pub fn descender(&self) -> i16 {
        self.vertical_metrics().1
    }

    /// Baseline-to-baseline distance (FreeType's `face->height`).
    pub fn height(&self) -> i16 {
        self.vertical_metrics().2
    }

    /// Extra line gap from `hhea`.
    pub const fn line_gap(&self) -> i16 {
        self.hhea.line_gap
    }

    /// Union of all glyph bounding boxes (from `head`).
    pub const fn bbox(&self) -> Rect {
        self.head.bbox
    }

    /// Number of glyphs.
    pub const fn num_glyphs(&self) -> u16 {
        self.maxp.num_glyphs
    }

    /// All usable character maps, in file order.
    pub fn charmaps(&self) -> Vec<CharmapInfo> {
        self.cmap.infos().collect()
    }

    /// The active character map.
    pub fn charmap(&self) -> Option<CharmapInfo> {
        self.charmap
            .and_then(|index| self.cmap.get(index))
            .map(|(info, _)| info)
    }

    /// The character map `FT_Select_Charmap(face, FT_ENCODING_UNICODE)`
    /// activates, as an index into [`Self::charmaps`].
    pub(crate) fn freetype_unicode_charmap(&self) -> Option<usize> {
        self.cmap.freetype_unicode_index()
    }

    /// The character map HarfBuzz's own font functions use, as an index
    /// into [`Self::charmaps`].
    pub(crate) fn harfbuzz_charmap(&self) -> Option<usize> {
        self.cmap.harfbuzz_index()
    }

    /// This face with character map `index` active; `None` maps no
    /// character.
    pub(crate) fn with_charmap(&self, index: Option<usize>) -> Self {
        Self {
            charmap: index,
            ..self.clone()
        }
    }

    /// Activates character map `index` (an index into [`Self::charmaps`]),
    /// like FreeType's `FT_Set_Charmap`.
    pub fn select_charmap(&mut self, index: usize) -> Result<()> {
        let count = self.cmap.infos().count();
        if index >= count {
            return Err(FontError::CharmapOutOfRange { index, count });
        }
        self.charmap = Some(index);
        Ok(())
    }

    /// Glyph for a character code in the active character map. Symbol
    /// fonts also map `U+0000..=U+00FF` to `U+F000..=U+F0FF` like
    /// HarfBuzz does.
    pub fn glyph_index_for_code(&self, code: u32) -> Option<GlyphId> {
        let (info, subtable) = self.cmap.get(self.charmap?)?;
        subtable.glyph(code).or_else(|| {
            (info.is_symbol() && code <= 0xFF)
                .then(|| subtable.glyph(0xF000 + code))
                .flatten()
        })
    }

    /// Glyph for `c`, or `None` when the font cannot display it.
    pub fn glyph_index(&self, c: char) -> Option<GlyphId> {
        self.glyph_index_for_code(u32::from(c))
    }

    /// Every `(character code, glyph)` pair of the active character map.
    pub fn mappings(&self) -> Vec<(u32, GlyphId)> {
        let mut out = Vec::new();
        if let Some((_, subtable)) = self.charmap.and_then(|index| self.cmap.get(index)) {
            subtable.for_each(&mut |code, glyph| out.push((code, glyph)));
        }
        out
    }

    /// Horizontal advance of `glyph` in font units.
    pub fn glyph_advance(&self, glyph: GlyphId) -> u16 {
        self.hmtx.advance(glyph)
    }

    /// Left side bearing of `glyph` in font units (from `hmtx`).
    pub fn left_side_bearing(&self, glyph: GlyphId) -> i16 {
        self.hmtx.left_side_bearing(glyph)
    }

    /// The `maxp` values, including the bytecode interpreter's limits.
    pub(crate) const fn max_profile(&self) -> &MaxProfile {
        &self.maxp
    }

    /// The TrueType outlines (`loca` + `glyf`), if the face has them.
    pub(crate) const fn glyf(&self) -> Option<&Glyf<'a>> {
        match &self.outlines {
            Outlines::TrueType(glyf) => Some(glyf),
            Outlines::Cff(_) | Outlines::None => None,
        }
    }

    /// Top side bearing and advance height of `glyph` in font units, from
    /// `vmtx` or, without one, derived from the typographic ascender and
    /// descender and the glyph's top `y_max` (FreeType's
    /// `TT_Get_VMetrics`).
    pub(crate) fn glyph_vertical_metrics(&self, glyph: GlyphId, y_max: i32) -> (i16, u16) {
        let vmtx = self.table(Tag::new(b"vhea")).and_then(|vhea| {
            let long_count = Hhea::parse(vhea).ok()?.number_of_h_metrics;
            let vmtx = self.table(Tag::new(b"vmtx"))?;
            Hmtx::parse(vmtx, long_count, self.maxp.num_glyphs).ok()
        });
        if let Some(vmtx) = vmtx {
            return (vmtx.left_side_bearing(glyph), vmtx.advance(glyph));
        }
        let (ascender, descender) = self
            .os2
            .as_ref()
            .map_or((self.hhea.ascender, self.hhea.descender), |os2| {
                (os2.typo_ascender, os2.typo_descender)
            });
        let ascender = i32::from(ascender);
        (
            (ascender - y_max) as i16,
            (ascender - i32::from(descender)).unsigned_abs() as u16,
        )
    }

    pub(crate) fn check_glyph(&self, glyph: GlyphId) -> Result<()> {
        if glyph.0 >= self.maxp.num_glyphs {
            return Err(FontError::malformed(
                Tag::new(b"maxp"),
                "glyph id beyond glyph count",
            ));
        }
        Ok(())
    }

    /// Loads the tagged points of `glyph` in the given coordinate space,
    /// with FreeType's origin alignment applied.
    pub(crate) fn load_outline(&self, glyph: GlyphId, scaling: Scaling) -> Result<Outline> {
        self.load_with_advance(glyph, scaling)
            .map(|(outline, _)| outline)
    }

    /// Loads `glyph` like FreeType's `FT_Load_Glyph` with
    /// `FT_LOAD_NO_SCALE`: the outline in whole font units (TrueType
    /// composites transformed in fixed point, CFF coordinates floored as
    /// FreeType's CFF engine stores them) and the advance width in font
    /// units.
    pub(crate) fn load_unscaled(&self, glyph: GlyphId) -> Result<(Outline, i64)> {
        let unit = Scaling::Fixed {
            x_scale: 0x10000,
            y_scale: 0x10000,
        };
        match &self.outlines {
            Outlines::Cff(_) => {
                let (mut outline, advance) = self.load_with_advance(glyph, Scaling::Units)?;
                for point in &mut outline.points {
                    point.x = point.x.floor();
                    point.y = point.y.floor();
                }
                Ok((outline, advance))
            }
            Outlines::TrueType(_) | Outlines::None => self.load_with_advance(glyph, unit),
        }
    }

    /// The outline of `glyph` with the origin aligned, and the advance
    /// width in font units.
    fn load_with_advance(&self, glyph: GlyphId, scaling: Scaling) -> Result<(Outline, i64)> {
        self.check_glyph(glyph)?;
        let mut outline = Outline::default();
        let mut advance = i64::from(self.hmtx.advance(glyph));
        match &self.outlines {
            Outlines::TrueType(glyf) => {
                let metrics = |g: GlyphId| (self.hmtx.left_side_bearing(g), self.hmtx.advance(g));
                let phantoms = glyf.load(glyph, scaling, &metrics, &mut outline)?;
                let shift = match scaling {
                    Scaling::Units => phantoms.origin as f64,
                    Scaling::Fixed { x_scale, .. } => {
                        crate::fixed::mul_fix(phantoms.origin, x_scale) as f64
                    }
                };
                if shift != 0.0 {
                    outline.translate(-shift, 0.0);
                }
                advance = phantoms.advance;
            }
            Outlines::Cff(cff) => cff.load(glyph, scaling, &mut outline)?,
            Outlines::None => {}
        }
        Ok((outline, advance))
    }

    /// Emits the outline of `glyph` in font units and returns its control
    /// box (the bounds of all points, rounded outwards), or `None` for empty
    /// glyphs such as the space.
    pub fn outline(&self, glyph: GlyphId, sink: &mut impl OutlineSink) -> Result<Option<Rect>> {
        let outline = self.load_outline(glyph, Scaling::Units)?;
        outline.decompose(sink, Midpoint::Exact);
        Ok(control_rect(&outline))
    }

    /// Control box of `glyph` in font units, `None` for empty glyphs.
    pub fn glyph_bbox(&self, glyph: GlyphId) -> Result<Option<Rect>> {
        Ok(control_rect(&self.load_outline(glyph, Scaling::Units)?))
    }

    /// Whether the face has TrueType or CFF outlines.
    pub const fn has_outlines(&self) -> bool {
        !matches!(self.outlines, Outlines::None)
    }

    /// Kerning from the legacy `kern` table only, in font units — what
    /// FreeType's `FT_Get_Kerning` (and so matplotlib's mathtext) uses.
    pub fn legacy_kerning(&self, left: GlyphId, right: GlyphId) -> i32 {
        self.kern.kerning(left, right)
    }

    /// Whether the face has a legacy `kern` table.
    pub fn has_legacy_kerning(&self) -> bool {
        !self.kern.is_empty()
    }

    /// Horizontal kerning between two glyphs in font units: the `GPOS`
    /// `kern` feature (default script, then Latin) when the font has one,
    /// otherwise the legacy `kern` table.
    pub fn kerning(&self, left: GlyphId, right: GlyphId) -> i32 {
        crate::layout::gpos::pair_kerning(self, left, right)
            .unwrap_or_else(|| self.legacy_kerning(left, right))
    }

    /// Whether glyph names are available.
    pub fn has_glyph_names(&self) -> bool {
        match &self.outlines {
            Outlines::Cff(cff) => cff.has_glyph_names(),
            _ => self.post.as_ref().is_some_and(PostTable::has_names),
        }
    }

    /// PostScript name of `glyph` (from `CFF` or `post`).
    pub fn glyph_name(&self, glyph: GlyphId) -> Option<&'a str> {
        match &self.outlines {
            Outlines::Cff(cff) => cff.glyph_name(glyph),
            _ => self.post.as_ref()?.glyph_name(glyph),
        }
    }

    /// Glyph with PostScript name `name` (FreeType's `FT_Get_Name_Index`).
    pub fn glyph_index_by_name(&self, name: &str) -> Option<GlyphId> {
        match &self.outlines {
            Outlines::Cff(_) => (0..self.maxp.num_glyphs)
                .map(GlyphId)
                .find(|&glyph| self.glyph_name(glyph) == Some(name)),
            _ => self.post.as_ref()?.glyph_by_name(name),
        }
    }

    /// Family name as FreeType reports it (`face->family_name`): the WWS or
    /// typographic family when appropriate, else name id 1, as ASCII.
    pub fn family_name(&self) -> Option<String> {
        self.freetype_names(id::WWS_FAMILY, id::TYPOGRAPHIC_FAMILY, id::FAMILY)
    }

    /// Style name as FreeType reports it (`face->style_name`).
    pub fn style_name(&self) -> Option<String> {
        self.freetype_names(id::WWS_SUBFAMILY, id::TYPOGRAPHIC_SUBFAMILY, id::SUBFAMILY)
    }

    /// FreeType's family/style selection: faces flagged as WWS-conformant
    /// prefer the typographic name; others prefer the WWS name, then the
    /// typographic one; both fall back to the legacy name.
    fn freetype_names(&self, wws: u16, typographic: u16, legacy: u16) -> Option<String> {
        let order: &[u16] = if self.os2.as_ref().is_some_and(Os2::is_wws) {
            &[typographic, legacy]
        } else {
            &[wws, typographic, legacy]
        };
        order
            .iter()
            .find_map(|&name_id| self.name.freetype_string(name_id))
    }

    /// Full name (name id 4) as FreeType would decode it.
    pub fn full_name(&self) -> Option<String> {
        self.name.freetype_string(id::FULL_NAME)
    }

    /// PostScript name as FreeType reports it.
    pub fn postscript_name(&self) -> Option<String> {
        self.name.postscript_name()
    }

    /// The `name` record with exactly this key and id, decoded (Latin-1 for
    /// Macintosh records, UTF-16BE otherwise) — matplotlib's `get_sfnt()`.
    pub fn name_string(&self, key: NameKey, name_id: u16) -> Option<String> {
        self.name.decoded(key, name_id)
    }

    /// FreeType's style flags: from `OS/2` `fsSelection` when present, else
    /// from `head.macStyle`.
    pub fn style_flags(&self) -> StyleFlags {
        match &self.os2 {
            Some(os2) => StyleFlags {
                italic: os2.is_italic(),
                bold: os2.is_bold(),
            },
            None => StyleFlags {
                italic: self.head.mac_style & 2 != 0,
                bold: self.head.mac_style & 1 != 0,
            },
        }
    }

    /// Italic angle in degrees (0 when there is no `post` table).
    pub fn italic_angle(&self) -> f64 {
        self.post().map_or(0.0, |post| post.italic_angle)
    }

    /// Visual weight from `OS/2` (400 regular, 700 bold), if present.
    pub fn weight_class(&self) -> Option<u16> {
        self.os2.as_ref().map(|os2| os2.weight_class)
    }

    /// Width class from `OS/2` (5 normal, lower is condensed), if present.
    pub fn width_class(&self) -> Option<u16> {
        self.os2.as_ref().map(|os2| os2.width_class)
    }

    /// Underline position (top edge, as stored in `post`) and thickness in
    /// font units. FreeType reports the centre instead:
    /// `position - thickness / 2`.
    pub fn underline_metrics(&self) -> Option<(i16, i16)> {
        self.post()
            .map(|post| (post.underline_position, post.underline_thickness))
    }

    pub(crate) const fn gdef(&self) -> &Gdef<'a> {
        &self.gdef
    }

    pub(crate) const fn gsub(&self) -> Option<&LayoutTable<'a>> {
        self.gsub.as_ref()
    }

    pub(crate) const fn gpos(&self) -> Option<&LayoutTable<'a>> {
        self.gpos.as_ref()
    }

    pub(crate) const fn kern_table(&self) -> &Kern<'a> {
        &self.kern
    }
}

fn control_rect(outline: &Outline) -> Option<Rect> {
    let (x0, y0, x1, y1) = outline.control_box()?;
    Some(Rect {
        x_min: x0.floor() as i32,
        y_min: y0.floor() as i32,
        x_max: x1.ceil() as i32,
        y_max: y1.ceil() as i32,
    })
}
