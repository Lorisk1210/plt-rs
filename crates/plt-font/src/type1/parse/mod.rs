//! The dictionary parser of matplotlib's `Type1Font._parse` ("Adobe Type
//! Manager Compatibility", chapter 10 of the Type 1 specification): plain
//! `/key value def` entries anywhere in the font, the `/Subrs`,
//! `/CharStrings`, `/Encoding` and `/OtherSubrs` entries by their own
//! rules, the positions of every definition, and the font's names for
//! the `RD`, `ND` and `NP` procedures.

mod reader;

use std::collections::BTreeMap;

use super::crypt::{Type1Cipher, decrypt};
use super::error::Type1Error;
use super::props::{CharStrings, Entry, Program, Type1Properties};
use super::python::ascii_lossy;
use super::token::{PsTokenKind, Token, Tokenizer};
use super::value::{PsNumber, PsValue};
use reader::{Parser, Sections, read_section};

/// The names a font gives the procedures that read a binary string
/// (`RD`), define a value (`ND`) and store an array element (`NP`)
/// (matplotlib's `Type1Font._abbr`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Abbreviations {
    /// The `{string currentfile exch readstring pop}` procedure.
    pub rd: String,
    /// The `{noaccess def}` procedure.
    pub nd: String,
    /// The `{noaccess put}` procedure.
    pub np: String,
}

impl Default for Abbreviations {
    fn default() -> Self {
        Self {
            rd: "RD".to_owned(),
            nd: "ND".to_owned(),
            np: "NP".to_owned(),
        }
    }
}

/// Where each definition of a key starts and ends in the cleartext part
/// followed by the decrypted part (matplotlib's `Type1Font._pos`).
pub(crate) type Spans = BTreeMap<String, Vec<(usize, usize)>>;

/// What parsing a font yields.
#[derive(Debug)]
pub(crate) struct Parsed {
    pub(crate) prop: Type1Properties,
    pub(crate) spans: Spans,
    pub(crate) abbreviations: Abbreviations,
}

/// The style words matplotlib strips from the full name to make a family
/// name.
const STYLE_WORDS: [&str; 10] = [
    "regular",
    "plain",
    "italic",
    "oblique",
    "semibold",
    "bold",
    "ultralight",
    "light",
    "extra",
    "condensed",
];

/// The full name without trailing style words (`Foo Bold Italic` gives
/// `Foo`), as matplotlib's regular expression strips them.
fn family_from_full_name(full: &str) -> String {
    let (body, newline) = match full.strip_suffix('\n') {
        Some(body) => (body, "\n"),
        None => (full, ""),
    };
    let mut end = body.len();
    while let Some(separator) = body[..end].rfind([' ', '-']) {
        let word = body[separator + 1..end].to_ascii_lowercase();
        if !STYLE_WORDS.contains(&word.as_str()) {
            break;
        }
        end = separator;
    }
    format!("{}{newline}", &body[..end])
}

/// The keyword tokens that may stand between a value and its `def`.
const ACCESS: [&str; 3] = ["readonly", "noaccess", "executeonly"];

/// Sets entry `key`, keeping the place of an earlier definition.
fn set(entries: &mut Vec<Entry>, key: &str, value: PsValue, source: String) {
    match entries.iter_mut().find(|entry| entry.key == key) {
        Some(entry) => {
            entry.value = value;
            entry.source = source;
        }
        None => entries.push(Entry {
            key: key.to_owned(),
            value,
            source,
        }),
    }
}

/// matplotlib's defaults for entries a font may leave out.
fn defaults() -> Vec<Entry> {
    let entry = |key: &str, value: PsValue| Entry {
        key: key.to_owned(),
        value,
        source: String::new(),
    };
    vec![
        entry("Weight", PsValue::from("Regular")),
        entry("ItalicAngle", PsValue::Number(PsNumber::Real(0.0))),
        entry("isFixedPitch", PsValue::Bool(false)),
        entry("UnderlinePosition", PsValue::Number(PsNumber::Int(-100))),
        entry("UnderlineThickness", PsValue::Number(PsNumber::Int(50))),
    ]
}

/// Parses the cleartext part followed by the decrypted part.
pub(crate) fn parse(data: &[u8]) -> Result<Parsed, Type1Error> {
    let mut parser = Parser {
        data,
        tokens: Tokenizer::new(data, true),
        abbreviations: Abbreviations::default(),
    };
    let mut entries = defaults();
    let mut spans = Spans::new();
    let mut sections = Sections::default();
    while let Some(token) = parser.tokens.next_token()? {
        if token.is_delimiter() {
            // Skip over expressions: only top-level keys count.
            parser.expression(token)?;
        }
        if !token.is_slash_name() {
            continue;
        }
        let key = ascii_lossy(&token.raw[1..]);
        let key_pos = token.pos;
        if let Some(end) = read_section(&mut parser, &key, &mut sections)? {
            spans.entry(key).or_default().push((key_pos, end));
            continue;
        }
        let Some(token) = parser.tokens.next_token()? else {
            break;
        };
        if token.kind == PsTokenKind::Keyword {
            // `FontDirectory /Name known {...}`: not a definition.
            continue;
        }
        let (value, source) = if token.is_delimiter() {
            let source = parser.expression(token)?;
            (PsValue::Expression(source.clone()), source)
        } else {
            (token.value()?, token.text())
        };
        let Some(keyword) = parser.tokens.next_matching(|t| !t.is_keyword(&ACCESS))? else {
            break;
        };
        let abbreviations = &parser.abbreviations;
        if keyword.is_keyword(&["def", abbreviations.nd.as_str(), abbreviations.np.as_str()]) {
            spans
                .entry(key.clone())
                .or_default()
                .push((key_pos, keyword.end()));
            set(&mut entries, &key, value.clone(), source);
        }
        let abbreviations = &mut parser.abbreviations;
        match value.as_text() {
            Some("{noaccess def}") => abbreviations.nd = key,
            Some("{noaccess put}") => abbreviations.np = key,
            Some("{string currentfile exch readstring pop}") => abbreviations.rd = key,
            _ => {}
        }
    }
    let prop = finish(entries, sections)?;
    Ok(Parsed {
        prop,
        spans,
        abbreviations: parser.abbreviations,
    })
}

/// The `FontBBox` source parsed as an array of four numbers.
fn font_bbox(value: &PsValue) -> Result<[PsNumber; 4], Type1Error> {
    let text = value
        .as_text()
        .ok_or(Type1Error::InvalidValue("FontBBox"))?;
    if !text.is_ascii() {
        return Err(Type1Error::InvalidValue("FontBBox"));
    }
    let mut tokenizer = Tokenizer::new(text.as_bytes(), true);
    let mut tokens = Vec::new();
    while let Some(token) = tokenizer.next_token()? {
        tokens.push(token);
    }
    let invalid = || Type1Error::FontBBox(text.to_owned());
    let [open, numbers @ .., close] = tokens.as_slice() else {
        return Err(invalid());
    };
    if numbers.len() != 4
        || !open.is_delimiter()
        || !close.is_delimiter()
        || !numbers.iter().all(Token::is_number)
        || open.opposite() != Some(close.raw)
    {
        return Err(invalid());
    }
    let mut bbox = [PsNumber::Int(0); 4];
    for (slot, token) in bbox.iter_mut().zip(numbers) {
        *slot = token.number()?;
    }
    Ok(bbox)
}

/// Fills in the names, parses `FontBBox` and decrypts the programs.
fn finish(mut entries: Vec<Entry>, sections: Sections) -> Result<Type1Properties, Type1Error> {
    let find = |entries: &[Entry], key: &str| {
        entries
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| entry.value.clone())
    };
    let font_name = find(&entries, "FontName").unwrap_or_else(|| {
        let name = [find(&entries, "FullName"), find(&entries, "FamilyName")]
            .into_iter()
            .flatten()
            .find(PsValue::is_truthy)
            .unwrap_or_else(|| PsValue::from("Unknown"));
        set(&mut entries, "FontName", name.clone(), String::new());
        name
    });
    let full_name = find(&entries, "FullName").unwrap_or_else(|| {
        set(&mut entries, "FullName", font_name.clone(), String::new());
        font_name
    });
    if find(&entries, "FamilyName").is_none() {
        let full = full_name
            .as_text()
            .ok_or(Type1Error::InvalidValue("FullName"))?;
        let family = PsValue::from(family_from_full_name(full).as_str());
        set(&mut entries, "FamilyName", family, String::new());
    }
    let bbox_at = entries
        .iter()
        .position(|entry| entry.key == "FontBBox")
        .ok_or(Type1Error::MissingKey("FontBBox"))?;
    let bbox_entry = entries.remove(bbox_at);
    let bbox = font_bbox(&bbox_entry.value)?;

    let discard = match find(&entries, "lenIV") {
        None => 4,
        Some(PsValue::Number(PsNumber::Int(value))) => value,
        Some(PsValue::Bool(value)) => i64::from(value),
        Some(_) => return Err(Type1Error::InvalidValue("lenIV")),
    };
    let program = |encrypted: Vec<u8>| Program {
        decrypted: decrypt(&encrypted, Type1Cipher::Charstring, discard),
        encrypted,
    };
    let (declared, glyphs) = sections
        .char_strings
        .ok_or(Type1Error::MissingKey("CharStrings"))?;
    let mut char_strings = CharStrings::with_declared(declared);
    for (name, encrypted) in glyphs {
        char_strings.push(name, program(encrypted));
    }
    Ok(Type1Properties {
        entries,
        font_bbox: bbox,
        font_bbox_source: bbox_entry.source,
        encoding: sections.encoding,
        char_strings,
        subrs: sections
            .subrs
            .map(|subrs| subrs.into_iter().map(program).collect()),
        other_subrs: sections.other_subrs,
        len_iv: discard,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A font's dictionaries with `private` as the part after `eexec`
    /// (unencrypted here: the parser reads both parts as one).
    pub(super) fn font(private: &str) -> Vec<u8> {
        format!("/FontName /X def\n/FontBBox [0 0 1 1] def\n{private}").into_bytes()
    }

    pub(super) fn charstring(body: &str) -> String {
        format!("/CharStrings 1 begin\n/.notdef 5 RD \x00\x01\x02\x03\x04 ND\n{body}end\n")
    }

    #[test]
    fn family_names_lose_style_words() {
        assert_eq!(family_from_full_name("Foo Bold Italic"), "Foo");
        assert_eq!(family_from_full_name("Foo-SemiBold"), "Foo");
        assert_eq!(family_from_full_name("Foo  Bold"), "Foo ");
        assert_eq!(family_from_full_name("Foo BoldItalic"), "Foo BoldItalic");
        assert_eq!(family_from_full_name("Computer Modern"), "Computer Modern");
        assert_eq!(family_from_full_name("Foo Light\n"), "Foo\n");
        assert_eq!(family_from_full_name(" Bold"), "");
    }

    #[test]
    fn plain_entries_and_names() {
        let parsed = parse(&font(&charstring(""))).unwrap();
        let prop = &parsed.prop;
        assert_eq!(prop.font_name(), &PsValue::Name("X".into()));
        assert_eq!(prop.full_name(), prop.font_name());
        assert_eq!(prop.family_name(), &PsValue::from("X"));
        assert_eq!(prop.weight(), &PsValue::from("Regular"));
        assert_eq!(prop.font_bbox()[3], PsNumber::Int(1));
        assert!(prop.get("FontBBox").is_none());
        assert_eq!(prop.char_strings().len(), 1);
        assert_eq!(prop.char_strings().get(".notdef").map(<[u8]>::len), Some(1));
        assert_eq!(parsed.spans["FontName"], vec![(0, 16)]);
        assert_eq!(parsed.abbreviations, Abbreviations::default());
    }

    #[test]
    fn encodings_abbreviations_and_errors() {
        let private = format!(
            "/|- {{noaccess def}} def\n/Encoding 256 array\n0 1 255 {{1 index exch /.notdef put}} for\ndup 5 /Pi put\ndup x /bad put\ndup 6 7 put\nreadonly def\n/OtherSubrs [{{}} {{x}}] |-\n{}",
            charstring("")
        );
        let parsed = parse(&font(&private)).unwrap();
        assert_eq!(parsed.abbreviations.nd, "|-");
        let encoding = parsed.prop.encoding().unwrap();
        assert_eq!((encoding.len(), encoding.get(5)), (1, Some("Pi")));
        assert_eq!(parsed.prop.other_subrs(), Some(&b"[{} {x}] |-"[..]));
        let standard = parse(&font(&format!(
            "/Encoding StandardEncoding def\n{}",
            charstring("")
        )))
        .unwrap();
        assert!(standard.prop.encoding().unwrap().is_standard());
        assert_eq!(
            parse(b"/FontName /X def /CharStrings 0 begin end").unwrap_err(),
            Type1Error::MissingKey("FontBBox")
        );
        assert_eq!(
            parse(b"/FontBBox {1 2 3} def /CharStrings 0 begin end").unwrap_err(),
            Type1Error::FontBBox("{1 2 3}".into())
        );
        assert_eq!(
            parse(b"/FontBBox [0 0 1 1] def").unwrap_err(),
            Type1Error::MissingKey("CharStrings")
        );
        assert_eq!(
            parse(b"/FontBBox [0 0 1 1] def ]").unwrap_err(),
            Type1Error::UnbalancedDelimiter(24)
        );
        assert_eq!(parse(b"[ 1 2").unwrap_err(), Type1Error::UnexpectedEnd);
        // A negative length moves back, as in matplotlib.
        let back = parse(&font("/CharStrings 1 begin /a -5 RD x end")).unwrap();
        assert_eq!(back.prop.char_strings().get("a"), Some(&[][..]));
        assert_eq!(
            parse(&font("/CharStrings 1 begin /a -99 RD x end")).unwrap_err(),
            Type1Error::NegativeLength(-98)
        );
    }
}
