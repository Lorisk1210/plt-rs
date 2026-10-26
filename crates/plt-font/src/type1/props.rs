//! matplotlib's `Type1Font.prop`: the font's dictionary entries, with the
//! encoding, the charstrings and the subroutines in their own types.

use std::collections::{BTreeMap, HashMap};

use super::standard::standard_encoding;
use super::value::{PsNumber, PsValue};

/// A charstring or subroutine: its bytes as written in the font and
/// decrypted with matplotlib's handling of `lenIV`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Program {
    pub(crate) encrypted: Vec<u8>,
    pub(crate) decrypted: Vec<u8>,
}

impl Program {
    /// The bytes as written in the font (encrypted unless `lenIV` is
    /// negative).
    pub fn encrypted(&self) -> &[u8] {
        &self.encrypted
    }

    /// The program after charstring decryption, without the leading
    /// `lenIV` bytes.
    pub fn decrypted(&self) -> &[u8] {
        &self.decrypted
    }
}

/// The `/CharStrings` dictionary: every definition in file order, looked
/// up by name like matplotlib's dict (a later definition of a name
/// replaces the earlier one's program but keeps its place).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CharStrings {
    definitions: Vec<(String, Program)>,
    by_name: HashMap<String, usize>,
    order: Vec<String>,
    declared: i64,
}

impl CharStrings {
    /// An empty dictionary declared to hold `declared` glyphs.
    pub(crate) fn with_declared(declared: i64) -> Self {
        Self {
            declared,
            ..Self::default()
        }
    }

    /// The glyph count the font declares (`/CharStrings <count> dict`).
    pub fn declared(&self) -> i64 {
        self.declared
    }

    /// Adds a definition.
    pub(crate) fn push(&mut self, name: String, program: Program) {
        let index = self.definitions.len();
        if self.by_name.insert(name.clone(), index).is_none() {
            self.order.push(name.clone());
        }
        self.definitions.push((name, program));
    }

    /// The decrypted program of the glyph `name`.
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.program(name).map(Program::decrypted)
    }

    /// The program of the glyph `name`.
    pub fn program(&self, name: &str) -> Option<&Program> {
        self.by_name
            .get(name)
            .map(|&index| &self.definitions[index].1)
    }

    /// Whether the glyph `name` is defined.
    pub fn contains(&self, name: &str) -> bool {
        self.by_name.contains_key(name)
    }

    /// The glyph names, each once, in the order of their first definition.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.order.iter().map(String::as_str)
    }

    /// Every definition in file order, repeated names included (the order
    /// FreeType numbers glyphs in).
    pub fn definitions(&self) -> impl Iterator<Item = (&str, &Program)> {
        self.definitions
            .iter()
            .map(|(name, program)| (name.as_str(), program))
    }

    /// The number of distinct glyph names.
    pub fn len(&self) -> usize {
        self.order.len()
    }

    /// Whether no glyph is defined.
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
}

/// A font encoding: character codes to glyph names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Type1Encoding {
    codes: BTreeMap<i64, String>,
    standard: bool,
}

impl Type1Encoding {
    /// The Adobe Standard Encoding, as a font naming `StandardEncoding`
    /// has it (matplotlib's `_StandardEncoding`).
    pub fn standard() -> Self {
        Self {
            codes: standard_encoding()
                .map(|(code, name)| (i64::from(code), name.to_owned()))
                .collect(),
            standard: true,
        }
    }

    /// An encoding array with the given entries.
    pub fn from_codes(codes: impl IntoIterator<Item = (i64, String)>) -> Self {
        Self {
            codes: codes.into_iter().collect(),
            standard: false,
        }
    }

    /// Whether the font names the Standard Encoding instead of writing an
    /// array.
    pub fn is_standard(&self) -> bool {
        self.standard
    }

    /// The glyph name of `code`.
    pub fn get(&self, code: i64) -> Option<&str> {
        self.codes.get(&code).map(String::as_str)
    }

    /// The entries in code order.
    pub fn iter(&self) -> impl Iterator<Item = (i64, &str)> {
        self.codes.iter().map(|(&code, name)| (code, name.as_str()))
    }

    /// The number of entries.
    pub fn len(&self) -> usize {
        self.codes.len()
    }

    /// Whether the encoding has no entries.
    pub fn is_empty(&self) -> bool {
        self.codes.is_empty()
    }
}

/// One dictionary entry with the text it was written as.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Entry {
    pub(crate) key: String,
    pub(crate) value: PsValue,
    /// The value's source text (empty for defaults).
    pub(crate) source: String,
}

/// matplotlib's `Type1Font.prop`.
///
/// Plain entries (`/FontName /CMR10 def`, `/FontMatrix [...] readonly
/// def`, ...) keep their last definition in [`get`](Self::get), with
/// matplotlib's defaults for `Weight`, `ItalicAngle`, `isFixedPitch`,
/// `UnderlinePosition`, `UnderlineThickness` and the names. The entries
/// read by their own rules have their own accessors.
#[derive(Clone, Debug, PartialEq)]
pub struct Type1Properties {
    pub(crate) entries: Vec<Entry>,
    pub(crate) font_bbox: [PsNumber; 4],
    pub(crate) font_bbox_source: String,
    pub(crate) encoding: Option<Type1Encoding>,
    pub(crate) char_strings: CharStrings,
    pub(crate) subrs: Option<Vec<Program>>,
    pub(crate) other_subrs: Option<Vec<u8>>,
    /// The leading bytes of each charstring (`lenIV`, default 4), as
    /// matplotlib discards and writes them.
    pub(crate) len_iv: i64,
}

impl Type1Properties {
    /// The value of the plain entry `key`.
    pub fn get(&self, key: &str) -> Option<&PsValue> {
        self.entry(key).map(|entry| &entry.value)
    }

    pub(crate) fn entry(&self, key: &str) -> Option<&Entry> {
        self.entries.iter().find(|entry| entry.key == key)
    }

    /// The source text of the plain entry `key`.
    pub(crate) fn source(&self, key: &str) -> Option<&str> {
        self.entry(key).map(|entry| entry.source.as_str())
    }

    /// The plain entries in the order they were first defined.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &PsValue)> {
        self.entries
            .iter()
            .map(|entry| (entry.key.as_str(), &entry.value))
    }

    /// The value of an entry the parser always fills in.
    fn name(&self, key: &str) -> &PsValue {
        self.get(key)
            .expect("the parser fills in the names and matplotlib's defaults")
    }

    /// `lenIV`: how many leading bytes each charstring has (default 4).
    pub fn len_iv(&self) -> i64 {
        self.len_iv
    }

    /// `FontName`: the PostScript name (else `FullName`, `FamilyName` or
    /// `Unknown`).
    pub fn font_name(&self) -> &PsValue {
        self.name("FontName")
    }

    /// `FullName` (else the font name).
    pub fn full_name(&self) -> &PsValue {
        self.name("FullName")
    }

    /// `FamilyName` (else the full name without style words).
    pub fn family_name(&self) -> &PsValue {
        self.name("FamilyName")
    }

    /// `Weight` (default `Regular`).
    pub fn weight(&self) -> &PsValue {
        self.name("Weight")
    }

    /// `ItalicAngle` (default 0).
    pub fn italic_angle(&self) -> &PsValue {
        self.name("ItalicAngle")
    }

    /// `isFixedPitch` (default false).
    pub fn is_fixed_pitch(&self) -> &PsValue {
        self.name("isFixedPitch")
    }

    /// `FontBBox`: `[x_min, y_min, x_max, y_max]`.
    pub fn font_bbox(&self) -> [PsNumber; 4] {
        self.font_bbox
    }

    /// `Encoding`, if the font has one.
    pub fn encoding(&self) -> Option<&Type1Encoding> {
        self.encoding.as_ref()
    }

    /// `CharStrings`.
    pub fn char_strings(&self) -> &CharStrings {
        &self.char_strings
    }

    /// `Subrs`, indexed by subroutine number, if the font has them.
    pub fn subrs(&self) -> Option<&[Program]> {
        self.subrs.as_deref()
    }

    /// The source of the `OtherSubrs` procedures, if the font has them.
    pub fn other_subrs(&self) -> Option<&[u8]> {
        self.other_subrs.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program(byte: u8) -> Program {
        Program {
            encrypted: vec![byte],
            decrypted: vec![byte],
        }
    }

    #[test]
    fn charstrings_behave_like_a_python_dict() {
        let mut cs = CharStrings::default();
        cs.push("b".into(), program(1));
        cs.push("a".into(), program(2));
        cs.push("b".into(), program(3));
        assert_eq!(cs.len(), 2);
        assert_eq!(cs.get("b"), Some(&[3][..]));
        assert_eq!(cs.names().collect::<Vec<_>>(), ["b", "a"]);
        assert_eq!(cs.definitions().count(), 3);
        assert!(cs.contains("a") && !cs.contains("c") && !cs.is_empty());
        assert_eq!(cs.program("a").map(Program::encrypted), Some(&[2][..]));
    }

    #[test]
    fn encodings() {
        let standard = Type1Encoding::standard();
        assert!(standard.is_standard());
        assert_eq!(standard.get(65), Some("A"));
        assert_eq!(standard.get(0), Some(".notdef"));
        assert_eq!(standard.get(1), None);
        let custom = Type1Encoding::from_codes([(5, "Pi".to_owned())]);
        assert_eq!((custom.len(), custom.get(5)), (1, Some("Pi")));
        assert!(!custom.is_standard() && !custom.is_empty());
        assert_eq!(custom.iter().collect::<Vec<_>>(), [(5, "Pi")]);
    }
}
