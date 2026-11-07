//! [`Type1Face`]: a Type 1 font as FreeType's Type 1 driver presents it
//! to matplotlib's `FT2Font` (`t1load.c`, `t1objs.c`, `t1gload.c`): glyph
//! indices and names, the font's metrics, the encoding vector `dviread`
//! maps TeX character codes with, the Private dictionary's alignment
//! zones, and glyph outlines, unhinted or hinted.
//!
//! FreeType's Type 1 driver is copyright David Turner, Robert Wilhelm and
//! Werner Lemberg, and ported here under the FreeType License (FTL).

mod charstrings;
mod glyph;
mod metrics;
mod psconv;

pub use glyph::Type1Glyph;

use charstrings::GlyphSet;

use std::path::Path;

use super::crypt::{Type1Cipher, decrypt};
use super::error::Type1Error;
use super::font::Type1Font;
use super::props::{Program, Type1Properties};
use crate::error::{FontError, Result};
use crate::fixed::div_fix;
use crate::matrix::{FontTransform, matrix_is_usable};
use crate::psaux::PrivateDict;
use crate::scale::Size;
use crate::types::{GlyphId, Rect};

/// The glyphs FreeType adds beyond a font's declared count
/// (`TABLE_EXTEND`).
const TABLE_EXTEND: usize = 5;

/// FreeType's `.notdef` for fonts without one: `0 333 hsbw endchar`.
const NOTDEF: [u8; 5] = [0x8B, 0xF7, 0xE1, 0x0D, 0x0E];

/// `t1_parse_font_matrix`: the normalized matrix and the units per em
/// (0 when the matrix is the usual thousandth of an em).
fn font_transform(source: &str) -> Result<(FontTransform, u16), Type1Error> {
    let invalid = Type1Error::InvalidFont("invalid font matrix");
    let mut temp = psconv::to_fixed_array(source, 6, 3).ok_or(invalid.clone())?;
    if temp.len() < 6 {
        return Err(invalid);
    }
    let scale = temp[3].abs();
    if scale == 0 {
        return Err(invalid);
    }
    let mut units_per_em = 0;
    if scale != 0x10000 {
        units_per_em = div_fix(1000, scale) as u16;
        for index in [0, 1, 2, 4, 5] {
            temp[index] = div_fix(temp[index], scale);
        }
        temp[3] = if temp[3] < 0 { -0x10000 } else { 0x10000 };
    }
    if !matrix_is_usable(temp[0], temp[2], temp[1], temp[3]) {
        return Err(invalid);
    }
    let transform = FontTransform {
        xx: temp[0],
        yx: temp[1],
        xy: temp[2],
        yy: temp[3],
        offset: (temp[4] >> 16, temp[5] >> 16),
    };
    Ok((transform, units_per_em))
}

/// `FT_RoundFix`.
const fn round_fix(value: i64) -> i64 {
    value.wrapping_add(0x8000 - (value < 0) as i64) & !0xFFFF
}

/// `FIXED_TO_INT`: a 16.16 value rounded to an integer.
pub(crate) const fn fixed_to_int(value: i64) -> i64 {
    round_fix(value) >> 16
}

/// FreeType's face style name from the full and family names
/// (`T1_Face_Init`): what the full name has beyond the family name, else
/// `Regular` when they match, else the weight.
fn style_name(full: Option<&str>, family: Option<&str>, weight: Option<&str>) -> String {
    let derived = match (full, family) {
        (Some(full), Some(family)) => {
            let (full, family) = (full.as_bytes(), family.as_bytes());
            let (mut f, mut g) = (0, 0);
            let mut result = Some("Regular".to_owned());
            while f < full.len() {
                if family.get(g) == Some(&full[f]) {
                    f += 1;
                    g += 1;
                } else if matches!(full[f], b' ' | b'-') {
                    f += 1;
                } else if matches!(family.get(g), Some(b' ' | b'-')) {
                    g += 1;
                } else {
                    result = (g == family.len())
                        .then(|| String::from_utf8_lossy(&full[f..]).into_owned());
                    break;
                }
            }
            result
        }
        _ => None,
    };
    derived
        .or_else(|| weight.map(str::to_owned))
        .unwrap_or_else(|| "Regular".to_owned())
}

/// A Type 1 font as FreeType loads it.
#[derive(Clone, Debug)]
pub struct Type1Face {
    postscript_name: Option<String>,
    family_name: Option<String>,
    style_name: String,
    units_per_em: u16,
    bbox: Rect,
    max_advance_width: i16,
    underline_position: i16,
    underline_thickness: i16,
    fixed_pitch: bool,
    italic: bool,
    bold: bool,
    transform: FontTransform,
    names: Vec<String>,
    charstrings: Vec<Vec<u8>>,
    subrs: Vec<Vec<u8>>,
    private: PrivateDict,
    encoding: [GlyphId; 256],
}

/// The hinting entries of the Private dictionary, as FreeType reads them:
/// blue arrays of 16-bit integers (an even number of `BlueValues`), handed
/// to the engine in 16.16, `BlueScale` times 1000 in 16.16, and
/// `BlueShift` and `BlueFuzz` reset to their defaults when unlikely.
fn private_dict(prop: &Type1Properties) -> PrivateDict {
    let defined = |key: &str| prop.source(key).filter(|source| !source.is_empty());
    let array = |key: &str, max: usize| {
        defined(key).map_or_else(Vec::new, |source| {
            psconv::to_int_array(source, max)
                .into_iter()
                .map(|value| i32::from(value as i16) << 16)
                .collect()
        })
    };
    let defaults = PrivateDict::default();
    let bounded = |key: &str, default: i32| {
        defined(key)
            .map(|source| psconv::to_int(source) as i32)
            .filter(|value| (0..=1000).contains(value))
            .unwrap_or(default)
    };
    let mut blue_values = array("BlueValues", 14);
    blue_values.truncate(blue_values.len() & !1);
    PrivateDict {
        blue_values,
        other_blues: array("OtherBlues", 10),
        family_blues: array("FamilyBlues", 14),
        family_other_blues: array("FamilyOtherBlues", 10),
        blue_scale: defined("BlueScale").map_or(defaults.blue_scale, |source| {
            psconv::to_fixed(source, 3) as i32
        }),
        blue_shift: bounded("BlueShift", defaults.blue_shift),
        blue_fuzz: bounded("BlueFuzz", defaults.blue_fuzz),
        language_group: defined("LanguageGroup").map_or(0, |source| psconv::to_int(source) as i32),
        vsindex: 0,
    }
}

/// The decrypted program FreeType stores: without the `lenIV` leading
/// bytes, which a program must have (more of them for a charstring).
fn stored(program: &Program, len_iv: i64, charstring: bool) -> Result<Vec<u8>, Type1Error> {
    let Ok(skip) = usize::try_from(len_iv) else {
        return Ok(program.encrypted().to_vec());
    };
    let size = program.encrypted().len();
    if size < skip || (charstring && size == skip) {
        return Err(Type1Error::InvalidFont("program shorter than lenIV"));
    }
    Ok(decrypt(
        program.encrypted(),
        Type1Cipher::Charstring,
        len_iv,
    ))
}

/// The glyph names and programs in FreeType's order: file order, with
/// `.notdef` swapped to index 0, or FreeType's own `.notdef` added there.
fn glyph_table(prop: &Type1Properties) -> Result<(Vec<String>, Vec<Vec<u8>>), Type1Error> {
    let char_strings = prop.char_strings();
    let declared = usize::try_from(char_strings.declared())
        .map_err(|_| Type1Error::InvalidFont("negative glyph count"))?;
    let capacity = declared.saturating_add(1 + TABLE_EXTEND);
    let (mut names, mut programs) = (Vec::new(), Vec::new());
    for (index, (name, program)) in char_strings.definitions().enumerate() {
        if index >= capacity {
            return Err(Type1Error::InvalidFont("more glyphs than declared"));
        }
        // FreeType decrypts the declared glyphs and a few more.
        let code = if index < declared + TABLE_EXTEND {
            stored(program, prop.len_iv(), true)?
        } else {
            program.encrypted().to_vec()
        };
        names.push(name.to_owned());
        programs.push(code);
    }
    if names.is_empty() {
        return Err(Type1Error::InvalidFont("no glyphs"));
    }
    match names.iter().rposition(|name| name == ".notdef") {
        Some(notdef) if names[0] != ".notdef" => {
            names.swap(0, notdef);
            programs.swap(0, notdef);
        }
        Some(_) => {}
        None => {
            if names.len() >= capacity {
                return Err(Type1Error::InvalidFont("no room for .notdef"));
            }
            let first = std::mem::replace(&mut names[0], ".notdef".to_owned());
            let code = std::mem::replace(&mut programs[0], NOTDEF.to_vec());
            names.push(first);
            programs.push(code);
        }
    }
    if names.len() > usize::from(u16::MAX) {
        return Err(Type1Error::InvalidFont("too many glyphs"));
    }
    Ok((names, programs))
}

impl Type1Face {
    /// Loads a font from the contents of a PFA or PFB file.
    pub fn parse(data: &[u8]) -> Result<Self, Type1Error> {
        Self::new(&Type1Font::from_bytes(data)?)
    }

    /// Loads the PFA or PFB file at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let data = std::fs::read(path).map_err(|error| FontError::Io {
            path: path.to_path_buf(),
            kind: error.kind(),
        })?;
        Ok(Self::parse(&data)?)
    }

    /// The face of a font read by matplotlib's [`Type1Font`], such as a
    /// slanted or extended one.
    pub fn new(font: &Type1Font) -> Result<Self, Type1Error> {
        let prop = font.prop();
        let (names, charstrings) = glyph_table(prop)?;
        let subrs = prop
            .subrs()
            .unwrap_or_default()
            .iter()
            .map(|subr| stored(subr, prop.len_iv(), false))
            .collect::<Result<Vec<_>, _>>()?;
        let defined = |key: &str| prop.source(key).filter(|source| !source.is_empty());
        let text = |key: &str| defined(key).and_then(psconv::to_string);
        let (transform, units_per_em) = match defined("FontMatrix") {
            Some(source) => font_transform(source)?,
            None => (FontTransform::ZERO, 0),
        };
        let units_per_em = if units_per_em == 0 {
            1000
        } else {
            units_per_em
        };
        let bbox = psconv::to_fixed_array(&prop.font_bbox_source, 4, 0)
            .filter(|values| values.len() == 4)
            .ok_or(Type1Error::InvalidFont("expected four numbers in FontBBox"))?
            .into_iter()
            .map(round_fix)
            .collect::<Vec<_>>();
        let bbox = Rect {
            x_min: (bbox[0] >> 16) as i32,
            y_min: (bbox[1] >> 16) as i32,
            x_max: ((bbox[2] + 0xFFFF) >> 16) as i32,
            y_max: ((bbox[3] + 0xFFFF) >> 16) as i32,
        };
        let (family, full, weight) = (text("FamilyName"), text("FullName"), text("Weight"));
        let postscript_name = text("FontName");
        let style_name = style_name(full.as_deref(), family.as_deref(), weight.as_deref());
        let number = |key: &str| defined(key).map_or(0, psconv::to_int) as i16;
        let mut face = Self {
            family_name: family.or_else(|| postscript_name.clone()),
            postscript_name,
            style_name,
            units_per_em,
            bbox,
            max_advance_width: 0,
            underline_position: number("UnderlinePosition"),
            underline_thickness: number("UnderlineThickness"),
            fixed_pitch: defined("isFixedPitch").is_some_and(psconv::to_bool),
            italic: defined("ItalicAngle").is_some_and(|angle| psconv::to_fixed(angle, 0) != 0),
            bold: matches!(weight.as_deref(), Some("Bold" | "Black")),
            transform,
            names,
            charstrings,
            subrs,
            private: private_dict(prop),
            encoding: [GlyphId(0); 256],
        };
        face.max_advance_width = face.compute_max_advance();
        if let Some(encoding) = prop.encoding().filter(|encoding| !encoding.is_standard()) {
            for (code, name) in encoding.iter() {
                if let Some(slot) = usize::try_from(code)
                    .ok()
                    .and_then(|c| face.encoding.get_mut(c))
                {
                    *slot = face
                        .names
                        .iter()
                        .position(|glyph| glyph == name)
                        .map_or(GlyphId(0), |index| GlyphId(index as u16));
                }
            }
        }
        Ok(face)
    }

    /// `T1_Compute_Max_Advance`: the largest advance width, where a glyph
    /// whose width cannot be read repeats the previous glyph's.
    fn compute_max_advance(&self) -> i16 {
        let mut advance = 0;
        let mut max = 0;
        for (index, code) in self.charstrings.iter().enumerate() {
            if let Some(width) = metrics::advance_width(code, &self.subrs) {
                advance = width;
            }
            if index == 0 || advance > max {
                max = advance;
            }
        }
        fixed_to_int(max) as i16
    }

    /// The PostScript name (`FontName`), if the font has one.
    pub fn postscript_name(&self) -> Option<&str> {
        self.postscript_name.as_deref()
    }

    /// The family name (`FamilyName`, else the PostScript name).
    pub fn family_name(&self) -> Option<&str> {
        self.family_name.as_deref()
    }

    /// The style name FreeType derives from the full and family names and
    /// the weight.
    pub fn style_name(&self) -> &str {
        &self.style_name
    }

    /// Design units per em, from the font matrix (1000 for the usual
    /// matrix).
    pub fn units_per_em(&self) -> u16 {
        self.units_per_em
    }

    /// The font bounding box in font units, rounded outwards.
    pub fn bbox(&self) -> Rect {
        self.bbox
    }

    /// The ascender: the top of the bounding box.
    pub fn ascender(&self) -> i16 {
        self.bbox.y_max as i16
    }

    /// The descender: the bottom of the bounding box.
    pub fn descender(&self) -> i16 {
        self.bbox.y_min as i16
    }

    /// The line height: 1.2 em, or the bounding box height if taller.
    pub fn height(&self) -> i16 {
        let height = (i32::from(self.units_per_em) * 12 / 10) as i16;
        let span = i32::from(self.ascender()) - i32::from(self.descender());
        if i32::from(height) < span {
            span as i16
        } else {
            height
        }
    }

    /// The largest advance width in font units.
    pub fn max_advance_width(&self) -> i16 {
        self.max_advance_width
    }

    /// `UnderlinePosition` in font units.
    pub fn underline_position(&self) -> i16 {
        self.underline_position
    }

    /// `UnderlineThickness` in font units.
    pub fn underline_thickness(&self) -> i16 {
        self.underline_thickness
    }

    /// Whether the font is monospaced (`isFixedPitch`).
    pub fn is_fixed_pitch(&self) -> bool {
        self.fixed_pitch
    }

    /// Whether the font is italic (a non-zero `ItalicAngle`).
    pub fn is_italic(&self) -> bool {
        self.italic
    }

    /// Whether the font is bold (`Weight` is `Bold` or `Black`).
    pub fn is_bold(&self) -> bool {
        self.bold
    }

    /// The number of glyphs.
    pub fn num_glyphs(&self) -> u16 {
        self.names.len() as u16
    }

    /// The name of `glyph`.
    pub fn glyph_name(&self, glyph: GlyphId) -> Option<&str> {
        self.names.get(usize::from(glyph.0)).map(String::as_str)
    }

    /// The first glyph named `name` (FreeType's `FT_Get_Name_Index`, which
    /// gives 0 where this gives `None`).
    pub fn glyph_index(&self, name: &str) -> Option<GlyphId> {
        self.names
            .iter()
            .position(|glyph| glyph == name)
            .map(|index| GlyphId(index as u16))
    }

    /// The glyph of each character code of the font's encoding array, 0
    /// where it names no glyph (matplotlib's
    /// `FT2Font._get_type1_encoding_vector`); all 0 for fonts using the
    /// Standard Encoding, which have no array.
    pub fn encoding_vector(&self) -> [GlyphId; 256] {
        self.encoding
    }

    /// The font matrix as FreeType applies it to outlines.
    pub(crate) fn transform(&self) -> FontTransform {
        self.transform
    }

    /// The glyph programs and alignment zones charstrings run with.
    pub(crate) fn glyph_set(&self) -> GlyphSet<'_> {
        GlyphSet {
            names: &self.names,
            charstrings: &self.charstrings,
            subrs: &self.subrs,
            private: &self.private,
        }
    }

    /// The size matplotlib's `FT2Font.set_size(points, dpi)` selects for
    /// the face: the point size truncated to 26.6, the resolution to an
    /// integer.
    pub fn size(&self, points: f64, dpi: f64) -> Size {
        Size::for_units_per_em(self.units_per_em, (points * 64.0) as i64, dpi as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::type1::test_support::fixture;

    #[test]
    fn cmr10_as_freetype_sees_it() {
        let face = Type1Face::parse(&fixture("cmr10.pfb")).unwrap();
        assert_eq!(face.num_glyphs(), 132);
        assert_eq!(face.units_per_em(), 1000);
        assert_eq!(
            (face.ascender(), face.descender(), face.height()),
            (750, -250, 1200)
        );
        assert_eq!(
            face.bbox(),
            Rect {
                x_min: -40,
                y_min: -250,
                x_max: 1009,
                y_max: 750
            }
        );
        assert_eq!(face.max_advance_width(), 1027);
        assert_eq!(face.postscript_name(), Some("CMR10"));
        assert_eq!(face.family_name(), Some("Computer Modern"));
        assert_eq!(face.style_name(), "Medium");
        assert_eq!(
            (face.underline_position(), face.underline_thickness()),
            (-100, 50)
        );
        assert!(!face.is_fixed_pitch() && !face.is_italic() && !face.is_bold());
        assert_eq!(face.glyph_name(GlyphId(0)), Some(".notdef"));
        assert_eq!(face.glyph_name(GlyphId(1)), Some("Gamma"));
        assert_eq!(face.glyph_index("Gamma"), Some(GlyphId(1)));
        assert_eq!(face.glyph_index("nonexistent"), None);
        let vector = face.encoding_vector();
        assert_eq!(
            &vector[..4],
            &[GlyphId(1), GlyphId(2), GlyphId(3), GlyphId(4)]
        );
    }

    #[test]
    fn courier_has_no_encoding_array() {
        let face = Type1Face::parse(&fixture("Courier10PitchBT-Bold.pfb")).unwrap();
        assert!(face.is_fixed_pitch() && face.is_bold());
        assert_eq!(face.encoding_vector(), [GlyphId(0); 256]);
    }

    #[test]
    fn style_names_and_matrices() {
        assert_eq!(style_name(Some("Foo Bold"), Some("Foo"), None), "Bold");
        assert_eq!(style_name(Some("Foo-Bold"), Some("Foo"), Some("x")), "Bold");
        assert_eq!(
            style_name(Some("Foo"), Some("Foo"), Some("Book")),
            "Regular"
        );
        assert_eq!(
            style_name(Some("CMR10"), Some("Computer Modern"), Some("Medium")),
            "Medium"
        );
        assert_eq!(style_name(None, None, None), "Regular");
        let (transform, units) = font_transform("[0.0005 0 0 0.0005 0 0]").unwrap();
        assert_eq!(
            (transform.xx, transform.yy, units),
            (0x10000, 0x10000, 2000)
        );
        let (slanted, units) = font_transform("[0.001 0 0.000167 0.001 0 0]").unwrap();
        assert_eq!((slanted.xy, units), (10945, 0));
        assert!(!slanted.is_identity());
        assert!(font_transform("[0.001 0 0 0 0 0]").is_err());
        assert!(font_transform("[1 2]").is_err());
        assert_eq!((fixed_to_int(0x18000), fixed_to_int(-0x18000)), (2, -2));
    }
}
