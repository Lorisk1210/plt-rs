//! The metrics of one AFM font and the queries matplotlib makes of them
//! (`matplotlib._afm.AFM`).

use std::borrow::Cow;
use std::collections::HashMap;

use super::error::AfmError;
use super::names::type1_name;

/// The metrics of one character (matplotlib's `CharMetrics`).
#[derive(Clone, Debug, PartialEq)]
pub struct CharMetrics {
    /// The advance width (`WX`), in 1/1000 of the font size.
    pub width: f64,
    /// The glyph name (`N`).
    pub name: String,
    /// The glyph box (`B`) as `[llx, lly, urx, ury]`, each truncated to
    /// an integer.
    pub bbox: [f64; 4],
}

/// One part of a composite character (matplotlib's `CompositePart`).
#[derive(Clone, Debug, PartialEq)]
pub struct CompositePart {
    /// The name of the glyph drawn, e.g. `acute`.
    pub name: String,
    /// Its horizontal displacement from the origin.
    pub dx: f64,
    /// Its vertical displacement from the origin.
    pub dy: f64,
}

/// The extent of a string as matplotlib's `get_str_bbox_and_descent`
/// reports it, in 1/1000 of the font size.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StrBbox {
    /// The leftmost glyph box edge, never right of the origin.
    pub left: f64,
    /// The lowest glyph box edge (10⁹, matplotlib's starting value, when
    /// every character is a newline).
    pub bottom: f64,
    /// The advance width, kerning included.
    pub width: f64,
    /// The height from `bottom` to the highest `lly + ury` of the glyphs
    /// (matplotlib reads the box as left, bottom, width, height).
    pub height: f64,
    /// How far `bottom` lies below the baseline.
    pub descent: f64,
}

/// The header entries matplotlib reads, when present and readable.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Header {
    pub(super) font_name: Option<String>,
    pub(super) full_name: Option<String>,
    pub(super) family_name: Option<String>,
    pub(super) weight: Option<String>,
    pub(super) italic_angle: Option<f64>,
    pub(super) ascender: Option<f64>,
    pub(super) descender: Option<f64>,
    pub(super) cap_height: Option<f64>,
    pub(super) x_height: Option<f64>,
    pub(super) underline_thickness: Option<f64>,
}

/// Kerning pair distances by first and second glyph name.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct KernTable(HashMap<String, HashMap<String, f64>>);

impl KernTable {
    /// Records the distance of the pair `first`, `second`.
    pub(super) fn insert(&mut self, first: String, second: String, distance: f64) {
        self.0.entry(first).or_default().insert(second, distance);
    }

    /// The distance of the pair, 0 when it is not kerned.
    fn get(&self, first: &str, second: &str) -> f64 {
        self.0
            .get(first)
            .and_then(|seconds| seconds.get(second))
            .copied()
            .unwrap_or(0.0)
    }
}

/// The character metrics of a font by character code and by glyph name.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct CharTable {
    chars: Vec<CharMetrics>,
    by_code: HashMap<i64, usize>,
    by_name: HashMap<String, usize>,
}

impl CharTable {
    /// Records `metrics` under its name and under `code` (unless `-1`, the
    /// code of unencoded glyphs); later entries replace earlier ones.
    pub(super) fn insert(&mut self, code: i64, metrics: CharMetrics) {
        let index = self.chars.len();
        if code != -1 {
            self.by_code.insert(code, index);
        }
        self.by_name.insert(metrics.name.clone(), index);
        self.chars.push(metrics);
    }

    fn by_name(&self, name: &str) -> Option<&CharMetrics> {
        self.by_name.get(name).map(|&index| &self.chars[index])
    }

    fn by_code(&self, code: i64) -> Option<&CharMetrics> {
        self.by_code.get(&code).map(|&index| &self.chars[index])
    }
}

/// The parsed metrics of one AFM file (matplotlib's `AFM`). All
/// dimensions are in 1/1000 of the font size.
#[derive(Clone, Debug, PartialEq)]
pub struct AfmMetrics {
    pub(super) header: Header,
    pub(super) chars: CharTable,
    pub(super) kern: KernTable,
    pub(super) composites: HashMap<String, Vec<CompositePart>>,
}

/// The suffixes `get_familyname` removes from a full name (after a space
/// or hyphen, ignoring case).
const STYLE_WORDS: [&str; 10] = [
    "regular",
    "plain",
    "italic",
    "oblique",
    "bold",
    "semibold",
    "light",
    "ultralight",
    "extra",
    "condensed",
];

/// `name` without one trailing style word and its separator, if it ends
/// with one.
fn strip_style_word(name: &str) -> Option<&str> {
    STYLE_WORDS.iter().find_map(|word| {
        let split = name.len().checked_sub(word.len())?;
        let (rest, tail) = (name.get(..split)?, name.get(split..)?);
        if tail.eq_ignore_ascii_case(word) {
            rest.strip_suffix([' ', '-'])
        } else {
            None
        }
    })
}

impl AfmMetrics {
    /// Parses AFM data (matplotlib's `AFM(fh)`).
    ///
    /// # Errors
    ///
    /// Where matplotlib raises: the data does not start with
    /// `StartFontMetrics`, a section is cut short or one of its lines is
    /// malformed.
    pub fn parse(data: &[u8]) -> Result<Self, AfmError> {
        super::parse::parse(data)
    }

    /// The metrics of the glyph called `name`.
    pub fn char_by_name(&self, name: &str) -> Option<&CharMetrics> {
        self.chars.by_name(name)
    }

    /// The metrics of the character with code `code` in the font's
    /// encoding.
    pub fn char_by_code(&self, code: i64) -> Option<&CharMetrics> {
        self.chars.by_code(code)
    }

    /// The glyph name of character code `code` (`get_glyph_name`).
    pub fn glyph_name(&self, code: i64) -> Option<&str> {
        self.char_by_code(code).map(|c| c.name.as_str())
    }

    /// The width of character code `code` (`get_width_char`).
    pub fn width_char(&self, code: i64) -> Option<f64> {
        self.char_by_code(code).map(|c| c.width)
    }

    /// The width of the glyph called `name` (`get_width_from_char_name`).
    pub fn width_from_char_name(&self, name: &str) -> Option<f64> {
        self.char_by_name(name).map(|c| c.width)
    }

    /// The kerning distance between the glyphs `first` and `second`, 0
    /// when the pair is not kerned (`get_kern_dist_from_name`).
    pub fn kern_dist_from_name(&self, first: &str, second: &str) -> f64 {
        self.kern.get(first, second)
    }

    /// The parts of the composite character `name`, if it is one.
    pub fn composite(&self, name: &str) -> Option<&[CompositePart]> {
        self.composites.get(name).map(Vec::as_slice)
    }

    /// The glyph matplotlib measures `c` with: the one named after it
    /// ([`type1_name`]), or `question` when the font has none (an error
    /// when it has neither).
    fn char_or_question(&self, c: char) -> Result<(Cow<'static, str>, &CharMetrics), AfmError> {
        let name = type1_name(c);
        if let Some(metrics) = self.char_by_name(&name) {
            return Ok((name, metrics));
        }
        self.char_by_name("question")
            .map(|metrics| (Cow::Borrowed("question"), metrics))
            .ok_or_else(|| AfmError::MissingGlyph("question".to_owned()))
    }

    /// matplotlib's `get_str_bbox_and_descent`: the extent of `s` with
    /// kerning, newlines skipped and unknown characters measured as `?`.
    ///
    /// # Errors
    ///
    /// When `s` has a character the font lacks and the font has no `?`.
    pub fn str_bbox_and_descent(&self, s: &str) -> Result<StrBbox, AfmError> {
        if s.is_empty() {
            return Ok(StrBbox::default());
        }
        let mut total_width = 0.0;
        let mut previous = Cow::Borrowed("");
        let (mut left, mut bottom, mut top) = (0.0_f64, 1e9_f64, 0.0_f64);
        for c in s.chars().filter(|&c| c != '\n') {
            let (name, metrics) = self.char_or_question(c)?;
            total_width += metrics.width + self.kern.get(&previous, &name);
            let [l, b, _, h] = metrics.bbox;
            left = left.min(l);
            bottom = bottom.min(b);
            top = top.max(b + h);
            previous = name;
        }
        Ok(StrBbox {
            left,
            bottom,
            width: total_width,
            height: top - bottom,
            descent: -bottom,
        })
    }

    /// A required header entry.
    fn field<T>(value: Option<T>, key: &'static str) -> Result<T, AfmError> {
        value.ok_or(AfmError::MissingField(key))
    }

    /// The PostScript name, e.g. `Times-Roman` (`get_fontname`).
    ///
    /// # Errors
    ///
    /// When the header has no `FontName`.
    pub fn font_name(&self) -> Result<&str, AfmError> {
        Self::field(self.header.font_name.as_deref(), "FontName")
    }

    /// The full name, else the PostScript name (`get_fullname`).
    ///
    /// # Errors
    ///
    /// When the header has neither `FullName` nor `FontName`.
    pub fn full_name(&self) -> Result<&str, AfmError> {
        self.header
            .full_name
            .as_deref()
            .map_or_else(|| self.font_name(), Ok)
    }

    /// The family name, else the full name without trailing style words
    /// such as `Bold` or `Italic` (`get_familyname`).
    ///
    /// # Errors
    ///
    /// When the header names the font in none of these ways.
    pub fn family_name(&self) -> Result<&str, AfmError> {
        if let Some(family) = &self.header.family_name {
            return Ok(family);
        }
        let mut name = self.full_name()?;
        while let Some(rest) = strip_style_word(name) {
            name = rest;
        }
        Ok(name)
    }

    /// The weight keyword, e.g. `Bold` or `Roman` (`get_weight`).
    ///
    /// # Errors
    ///
    /// When the header has no `Weight`.
    pub fn weight(&self) -> Result<&str, AfmError> {
        Self::field(self.header.weight.as_deref(), "Weight")
    }

    /// The italic angle in degrees (`get_angle`).
    ///
    /// # Errors
    ///
    /// When the header has no `ItalicAngle`.
    pub fn angle(&self) -> Result<f64, AfmError> {
        Self::field(self.header.italic_angle, "ItalicAngle")
    }

    /// The ascender (`get_ascender`).
    ///
    /// # Errors
    ///
    /// When the header has no `Ascender` (Symbol and `ZapfDingbats`).
    pub fn ascender(&self) -> Result<f64, AfmError> {
        Self::field(self.header.ascender, "Ascender")
    }

    /// The descender, negative below the baseline (`get_descender`).
    ///
    /// # Errors
    ///
    /// When the header has no `Descender`.
    pub fn descender(&self) -> Result<f64, AfmError> {
        Self::field(self.header.descender, "Descender")
    }

    /// The capital height (`get_capheight`).
    ///
    /// # Errors
    ///
    /// When the header has no `CapHeight`.
    pub fn cap_height(&self) -> Result<f64, AfmError> {
        Self::field(self.header.cap_height, "CapHeight")
    }

    /// The x height (`get_xheight`).
    ///
    /// # Errors
    ///
    /// When the header has no `XHeight`.
    pub fn x_height(&self) -> Result<f64, AfmError> {
        Self::field(self.header.x_height, "XHeight")
    }

    /// The underline thickness (`get_underline_thickness`).
    ///
    /// # Errors
    ///
    /// When the header has no `UnderlineThickness`.
    pub fn underline_thickness(&self) -> Result<f64, AfmError> {
        Self::field(self.header.underline_thickness, "UnderlineThickness")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"StartFontMetrics 4.1\n\
        Comment a sample\n\
        FontName Sample-BoldItalic\n\
        FullName Sample Bold Italic\n\
        Weight Bold\n\
        ItalicAngle -12,5\n\
        Ascender 700\n\
        Descender oops\n\
        StartCharMetrics 4\n\
        C 32 ; WX 250 ; N space ; B 0 0 0 0 ;\n\
        C 63 ; WX 444 ; N question ; B 30 0 400 700 ;\n\
        C 65 ; WX 700 ; N A ; B -10 0 710 680 ;\n\
        C 86 ; WX 680 ; N V ; B 5 -20 675 680 ;\n\
        C -1 ; WX 600 ; N Aacute ; B -10 0 710 900 ;\n\
        EndCharMetrics\n\
        StartKernData\n\
        StartKernPairs 2\n\
        KPX A V -80\n\
        \n\
        KPX V A -60\n\
        EndKernPairs\n\
        EndKernData\n\
        StartComposites 1\n\
        CC Aacute 2 ; PCC A 0 0 ; PCC acute 160 170 ;\n\
        EndComposites\n\
        EndFontMetrics\n";

    fn sample() -> AfmMetrics {
        AfmMetrics::parse(SAMPLE).expect("valid sample")
    }

    #[test]
    fn header_entries() {
        let afm = sample();
        assert_eq!(afm.font_name(), Ok("Sample-BoldItalic"));
        assert_eq!(afm.full_name(), Ok("Sample Bold Italic"));
        assert_eq!(afm.family_name(), Ok("Sample"));
        assert_eq!(afm.weight(), Ok("Bold"));
        assert_eq!(afm.angle(), Ok(-12.5));
        assert_eq!(afm.ascender(), Ok(700.0));
        assert_eq!(
            afm.descender(),
            Err(AfmError::MissingField("Descender")),
            "unreadable entries are skipped"
        );
        assert!(afm.cap_height().is_err());
    }

    #[test]
    fn characters_kerning_and_composites() {
        let afm = sample();
        assert_eq!(afm.glyph_name(65), Some("A"));
        assert_eq!(afm.width_char(86), Some(680.0));
        assert_eq!(afm.width_char(-1), None, "unencoded glyphs have no code");
        assert_eq!(afm.width_from_char_name("Aacute"), Some(600.0));
        assert_eq!(afm.kern_dist_from_name("A", "V"), -80.0);
        assert_eq!(afm.kern_dist_from_name("V", "V"), 0.0);
        let parts = afm.composite("Aacute").expect("a composite");
        assert_eq!(parts.len(), 2);
        assert!(afm.composite("A").is_none());
    }

    #[test]
    fn string_extents_follow_matplotlib() {
        let afm = sample();
        assert_eq!(afm.str_bbox_and_descent(""), Ok(StrBbox::default()));
        // A, V kerned by -80; the box is read as (l, b, w, h).
        let bbox = afm.str_bbox_and_descent("AV\n").expect("known glyphs");
        assert_eq!(
            bbox,
            StrBbox {
                left: -10.0,
                bottom: -20.0,
                width: 700.0 + 680.0 - 80.0,
                height: 680.0 + 20.0,
                descent: 20.0,
            }
        );
        let unknown = afm.str_bbox_and_descent("\u{e9}").expect("measured as ?");
        assert_eq!(unknown.width, 444.0);
        assert_eq!(unknown.height, 700.0);
    }

    #[test]
    fn missing_question_marks_are_errors() {
        let data = b"StartFontMetrics 2\nStartCharMetrics 1\n\
                     C 65 ; WX 700 ; N A ; B 0 0 1 1 ;\nEndCharMetrics\n";
        let afm = AfmMetrics::parse(data).expect("valid");
        assert!(afm.str_bbox_and_descent("A").is_ok());
        assert_eq!(
            afm.str_bbox_and_descent("B"),
            Err(AfmError::MissingGlyph("question".to_owned()))
        );
        assert_eq!(afm.font_name(), Err(AfmError::MissingField("FontName")));
        assert!(afm.full_name().is_err());
    }

    #[test]
    fn family_names_without_style_words() {
        assert_eq!(strip_style_word("Foo-SemiBold"), Some("Foo"));
        assert_eq!(strip_style_word("Foo Bold"), Some("Foo"));
        assert_eq!(strip_style_word("FooBold"), None);
        assert_eq!(strip_style_word("Bold"), None);
        let data = b"StartFontMetrics 2\nFullName Foo Extra Bold Condensed\n\
                     StartCharMetrics 0\nEndCharMetrics\n";
        let afm = AfmMetrics::parse(data).expect("valid");
        assert_eq!(afm.family_name(), Ok("Foo"));
    }
}
