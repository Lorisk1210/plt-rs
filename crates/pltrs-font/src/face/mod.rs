//! [`Face`]: a zero-copy view of one font face.
//!
//! This module parses the face and gives access to its tables, vertical
//! metrics and kerning; the rest of the face's API is grouped by topic:
//! [`charmap`] (characters to glyphs), [`glyphs`] (glyph metrics and
//! outlines), [`names`] (glyph and face names) and [`style`] (style
//! descriptors).

mod charmap;
mod glyph_order;
mod glyphs;
mod names;
mod style;
mod variations;

pub use style::StyleFlags;

use crate::cff::{Cff, Version};
use crate::error::Result;
use crate::layout::common::LayoutTable;
use crate::layout::gdef::Gdef;
use crate::region::{LazyFile, Region};
use crate::tables::cmap::Cmap;
use crate::tables::directory::{SfntFlavor, TableDirectory};
use crate::tables::glyf::Glyf;
use crate::tables::head::{Head, Hhea, MaxProfile};
use crate::tables::hmtx::Hmtx;
use crate::tables::kern::Kern;
use crate::tables::name::Name;
use crate::tables::os2::Os2;
use crate::tables::post::{Post, PostTable};
use crate::types::{GlyphId, Rect, Tag};
use crate::var::{Fvar, Variations};

/// Outline source of a face.
#[derive(Clone, Debug)]
enum Outlines<'a> {
    TrueType(Glyf<'a>),
    Cff(Box<Cff<'a>>),
    None,
}

/// A parsed font face borrowing the font file bytes.
///
/// All metrics are in font units unless documented otherwise; use
/// [`Size`](crate::Size) for FreeType-compatible scaled values.
#[derive(Clone, Debug)]
pub struct Face<'a> {
    data: &'a [u8],
    /// The file `data` are the held bytes of, for a face of a file held
    /// for drawing: what they lack is read from it.
    file: Option<&'a dyn LazyFile>,
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
    /// The variation tables and instance of a variable font.
    variations: Option<Box<Variations<'a>>>,
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
        Self::parse_in(data, index, None)
    }

    /// Parses face `index` of a font file held for drawing: from its held
    /// bytes, reading its glyph programs and the tables not held from the
    /// file when they are used.
    pub(crate) fn parse_held(file: &'a dyn LazyFile, index: u32) -> Result<Self> {
        Self::parse_in(file.held(), index, Some(file))
    }

    /// Parses face `index` of `data`, the held bytes of `file` if given.
    fn parse_in(data: &'a [u8], index: u32, file: Option<&'a dyn LazyFile>) -> Result<Self> {
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
        let variations =
            optional(&directory, *b"fvar", Fvar::parse).map(|fvar| Box::new(Variations::new(fvar)));
        let coords = variations
            .as_ref()
            .map_or(&[][..], |variations| &variations.coords[..]);
        let outlines = Self::outlines(&directory, &head, index, file, coords)?;
        Ok(Self {
            data,
            file,
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
            variations,
        })
    }

    /// The outlines of face `index` of `directory`, whose file `file` is
    /// when it is held for drawing, at the instance of normalized
    /// coordinates `coords`: `glyf`, else `CFF2` (which FreeType prefers),
    /// else `CFF `. An empty `CFF2` table, as the catalogue's view of a
    /// file leaves it, has no outlines.
    fn outlines(
        directory: &TableDirectory<'a>,
        head: &Head,
        index: u32,
        file: Option<&'a dyn LazyFile>,
        coords: &[i64],
    ) -> Result<Outlines<'a>> {
        let elsewhere =
            |tag| file.and_then(|file| Some(Region::lazy(file, file.table(index, tag)?)));
        let glyf = elsewhere(Tag::new(b"glyf"))
            .or_else(|| directory.table(Tag::new(b"glyf")).map(Region::Held));
        if let (Some(loca), Some(glyf)) = (directory.table(Tag::new(b"loca")), glyf) {
            return Ok(Outlines::TrueType(Glyf::new(
                loca,
                glyf,
                head.index_to_loc_format != 0,
            )));
        }
        let units_per_em = head.units_per_em;
        if let Some(parts) = file.and_then(|file| file.cff(index)) {
            let cff = Cff::parse_parts(parts, units_per_em, coords)?;
            return Ok(Outlines::Cff(Box::new(cff)));
        }
        for version in [Version::Cff2, Version::Cff] {
            let Some(table) = directory.table(version.tag()) else {
                continue;
            };
            if version == Version::Cff2 && table.is_empty() {
                break;
            }
            let cff = Cff::parse(table, version, units_per_em, coords)?;
            return Ok(Outlines::Cff(Box::new(cff)));
        }
        Ok(Outlines::None)
    }

    /// The whole font file this face was parsed from (read now for a face
    /// of a file held for drawing, which reads the parts it uses).
    ///
    /// # Errors
    ///
    /// When the file of a face held for drawing cannot be read.
    pub fn data(&self) -> Result<&'a [u8]> {
        match self.file {
            Some(file) => file.whole(),
            None => Ok(self.data),
        }
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
        match self
            .file
            .and_then(|file| Some((file, file.table(self.index, tag)?)))
        {
            Some((file, elsewhere)) => file.read(elsewhere),
            None => self.directory.table(tag),
        }
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

    /// The ascender, descender and line height at the face's instance:
    /// `MVAR`'s typographic deltas added to FreeType's choice of metrics
    /// (`tt_apply_mvar`).
    fn varied_vertical_metrics(&self) -> (i16, i16, i16) {
        let (ascender, descender, height) = self.vertical_metrics();
        let Some(variations) = &self.variations else {
            return (ascender, descender, height);
        };
        let deltas = variations.line_metrics();
        let line_gap = height.wrapping_sub(ascender).wrapping_add(descender);
        let ascender = ascender.wrapping_add(deltas.ascender);
        let descender = descender.wrapping_add(deltas.descender);
        let height = ascender
            .wrapping_sub(descender)
            .wrapping_add(line_gap)
            .wrapping_add(deltas.line_gap);
        (ascender, descender, height)
    }

    /// Typographic ascender (FreeType's `face->ascender`).
    pub fn ascender(&self) -> i16 {
        self.varied_vertical_metrics().0
    }

    /// Typographic descender, usually negative (FreeType's
    /// `face->descender`).
    pub fn descender(&self) -> i16 {
        self.varied_vertical_metrics().1
    }

    /// Baseline-to-baseline distance (FreeType's `face->height`).
    pub fn height(&self) -> i16 {
        self.varied_vertical_metrics().2
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

    /// The `maxp` table as FreeType loads it, including the bytecode
    /// interpreter's limits.
    pub const fn max_profile(&self) -> &MaxProfile {
        &self.maxp
    }

    /// The `name` table.
    pub(crate) const fn name_table(&self) -> &Name<'a> {
        &self.name
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

    pub(crate) const fn gdef(&self) -> &Gdef<'a> {
        &self.gdef
    }

    pub(crate) const fn gsub(&self) -> Option<&LayoutTable<'a>> {
        self.gsub.as_ref()
    }

    pub(crate) const fn gpos(&self) -> Option<&LayoutTable<'a>> {
        self.gpos.as_ref()
    }
}
