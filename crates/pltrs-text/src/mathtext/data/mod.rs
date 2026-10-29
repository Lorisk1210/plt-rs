//! Static tables of mathtext (generated from matplotlib, see
//! `crates/pltrs-text/tools/gen_mathtext_data.py`) and lookups over them.

mod bakoma;
mod stix;
mod symbols;
mod tex2uni;
mod unicode;

use super::font_name::FontName;

pub(crate) use symbols::SPACE_WIDTHS;

/// One of the `BaKoMa` Computer Modern font files.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum BakomaFile {
    /// `cmex10`: extension (big operators and delimiters).
    Cmex10,
    /// `cmmi10`: math italic.
    Cmmi10,
    /// `cmr10`: roman.
    Cmr10,
    /// `cmsy10`: math symbols.
    Cmsy10,
    /// `cmtt10`: typewriter.
    Cmtt10,
}

impl BakomaFile {
    /// The family name `findfont` resolves.
    pub(crate) fn family(self) -> &'static str {
        match self {
            Self::Cmex10 => "cmex10",
            Self::Cmmi10 => "cmmi10",
            Self::Cmr10 => "cmr10",
            Self::Cmsy10 => "cmsy10",
            Self::Cmtt10 => "cmtt10",
        }
    }
}

/// A run of characters a STIX virtual font moves to other code points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VirtualRange {
    /// First source code point.
    pub(crate) start: u32,
    /// Last source code point (inclusive).
    pub(crate) end: u32,
    /// Font the target glyphs come from.
    pub(crate) font: FontName,
    /// Code point `start` maps to.
    pub(crate) target: u32,
}

impl VirtualRange {
    /// Maps `start..=end` to `target..` in `font`.
    pub(crate) const fn new(start: u32, end: u32, font: FontName, target: u32) -> Self {
        Self {
            start,
            end,
            font,
            target,
        }
    }
}

/// A STIX virtual font: one mapping, or one per font class.
#[derive(Clone, Copy, Debug)]
pub(crate) enum VirtualFont {
    /// The same ranges for every font class.
    Single(&'static [VirtualRange]),
    /// Ranges per font class (`rm` is the fallback class).
    ByClass(&'static [(FontName, &'static [VirtualRange])]),
}

impl VirtualFont {
    /// The ranges used for `class` (matplotlib falls back to `rm`).
    pub(crate) fn ranges(self, class: FontName) -> Option<&'static [VirtualRange]> {
        match self {
            Self::Single(ranges) => Some(ranges),
            Self::ByClass(classes) => {
                let find = |wanted| {
                    classes
                        .iter()
                        .find(|(name, _)| *name == wanted)
                        .map(|(_, ranges)| *ranges)
                };
                find(class).or_else(|| find(FontName::Rm))
            }
        }
    }
}

fn sorted_lookup<T: Copy>(table: &[(&str, T)], key: &str) -> Option<T> {
    table
        .binary_search_by(|(name, _)| (*name).cmp(key))
        .ok()
        .map(|index| table[index].1)
}

fn in_sorted(table: &[&str], key: &str) -> bool {
    table.binary_search(&key).is_ok()
}

fn in_ranges(ranges: &[(u32, u32)], code: u32) -> bool {
    ranges
        .binary_search_by(|&(start, end)| {
            if end < code {
                std::cmp::Ordering::Less
            } else if start > code {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// The code point of a TeX symbol name (without the backslash).
pub(crate) fn tex2uni(name: &str) -> Option<u32> {
    sorted_lookup(&tex2uni::TEX2UNI, name)
}

/// matplotlib's `get_unicode_index`: a single character is its own code
/// point; anything else is looked up in `tex2uni` with surrounding
/// backslashes removed.
pub(crate) fn unicode_index(symbol: &str) -> Option<u32> {
    let mut chars = symbol.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(u32::from(c)),
        _ => tex2uni(symbol.trim_matches('\\')),
    }
}

/// Font file and character code of a symbol in the `BaKoMa` fonts.
pub(crate) fn latex_to_bakoma(symbol: &str) -> Option<(BakomaFile, u32)> {
    bakoma::LATEX_TO_BAKOMA
        .binary_search_by(|(name, _, _)| (*name).cmp(symbol))
        .ok()
        .map(|index| {
            let (_, file, code) = bakoma::LATEX_TO_BAKOMA[index];
            (file, code)
        })
}

/// The pre-sized `BaKoMa` variants of a delimiter, smallest first.
pub(crate) fn bakoma_size_alternatives(
    symbol: &str,
) -> Option<&'static [(FontName, &'static str)]> {
    sorted_lookup(&bakoma::SIZE_ALTERNATIVES, symbol)
}

/// The STIX virtual font called `name`.
pub(crate) fn virtual_font(name: FontName) -> Option<VirtualFont> {
    stix::VIRTUAL_FONTS
        .iter()
        .find(|(font, _)| *font == name)
        .map(|(_, mapping)| *mapping)
}

/// `unicodedata.category(c)[0] == "L"` for a BMP character.
pub(crate) fn is_letter(code: u32) -> bool {
    in_ranges(&unicode::LETTERS, code)
}

/// Whether the character name starts with `GREEK CAPITAL`.
pub(crate) fn is_greek_capital(code: u32) -> bool {
    in_ranges(&unicode::GREEK_CAPITALS, code)
}

/// Python's `str.isdigit` for one character.
pub(crate) fn is_digit(c: char) -> bool {
    in_ranges(&unicode::DIGITS, u32::from(c))
}

/// Python's `str.isspace` for one character.
pub(crate) fn is_space(c: char) -> bool {
    in_ranges(&unicode::SPACES, u32::from(c))
}

/// Binary operators (spaced unless they start an expression).
pub(crate) fn is_binary_operator(symbol: &str) -> bool {
    in_sorted(&symbols::BINARY_OPERATORS, symbol)
}

/// Relations.
pub(crate) fn is_relation(symbol: &str) -> bool {
    in_sorted(&symbols::RELATION_SYMBOLS, symbol)
}

/// Symbols surrounded by thick spaces: operators, relations and arrows.
pub(crate) fn is_spaced_symbol(symbol: &str) -> bool {
    is_binary_operator(symbol) || is_relation(symbol) || in_sorted(&symbols::ARROW_SYMBOLS, symbol)
}

/// Punctuation followed by a space.
pub(crate) fn is_punctuation(symbol: &str) -> bool {
    in_sorted(&symbols::PUNCTUATION_SYMBOLS, symbol)
}

/// Big operators taking limits above and below.
pub(crate) fn is_overunder_symbol(symbol: &str) -> bool {
    in_sorted(&symbols::OVERUNDER_SYMBOLS, symbol)
}

/// Function names taking limits above and below.
pub(crate) fn is_overunder_function(name: &str) -> bool {
    in_sorted(&symbols::OVERUNDER_FUNCTIONS, name)
}

/// Integral signs, whose scripts are dropped.
pub(crate) fn is_dropsub_symbol(symbol: &str) -> bool {
    in_sorted(&symbols::DROPSUB_SYMBOLS, symbol)
}

/// Font switch names.
pub(crate) fn is_font_name(name: &str) -> bool {
    in_sorted(&symbols::FONT_NAMES, name)
}

/// Every font switch name.
pub(crate) fn font_names() -> impl Iterator<Item = &'static str> {
    symbols::FONT_NAMES.iter().copied()
}

/// Named functions (`sin`, `lim`, ...).
pub(crate) fn is_function_name(name: &str) -> bool {
    in_sorted(&symbols::FUNCTION_NAMES, name)
}

/// Opening delimiters.
pub(crate) fn is_left_delim(symbol: &str) -> bool {
    in_sorted(&symbols::LEFT_DELIMS, symbol)
}

/// Every delimiter `\left`, `\middle` and `\right` accept.
pub(crate) fn delimiters() -> impl Iterator<Item = &'static str> {
    symbols::LEFT_DELIMS
        .iter()
        .chain(&symbols::RIGHT_DELIMS)
        .chain(&symbols::AMBI_DELIMS)
        .copied()
}

/// Whether `name` is a lowercase Greek letter name (`alpha`, ...).
pub(crate) fn is_small_greek(name: &str) -> bool {
    in_sorted(&symbols::SMALL_GREEK, name)
}

/// Wide accents (`widehat`, ...).
pub(crate) fn is_wide_accent(name: &str) -> bool {
    in_sorted(&symbols::WIDE_ACCENTS, name)
}

/// The symbol an accent command draws.
pub(crate) fn accent_symbol(name: &str) -> Option<&'static str> {
    sorted_lookup(&symbols::ACCENT_MAP, name)
}

/// Every accent command name (narrow and wide).
pub(crate) fn accent_names() -> impl Iterator<Item = &'static str> {
    symbols::ACCENT_MAP
        .iter()
        .map(|(name, _)| *name)
        .chain(symbols::WIDE_ACCENTS.iter().copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted<T: Ord + Copy>(keys: impl Iterator<Item = T>) -> bool {
        let keys: Vec<T> = keys.collect();
        keys.windows(2).all(|pair| pair[0] < pair[1])
    }

    #[test]
    fn tables_are_sorted_for_binary_search() {
        assert!(sorted(tex2uni::TEX2UNI.iter().map(|(k, _)| *k)));
        assert!(sorted(bakoma::LATEX_TO_BAKOMA.iter().map(|(k, _, _)| *k)));
        assert!(sorted(bakoma::SIZE_ALTERNATIVES.iter().map(|(k, _)| *k)));
        assert!(sorted(symbols::ACCENT_MAP.iter().map(|(k, _)| *k)));
        assert!(sorted(symbols::RELATION_SYMBOLS.iter().copied()));
        assert!(sorted(unicode::LETTERS.iter().map(|(a, _)| *a)));
    }

    #[test]
    fn unicode_index_follows_matplotlib() {
        assert_eq!(unicode_index("x"), Some(0x78));
        assert_eq!(unicode_index("\\alpha"), Some(0x3b1));
        assert_eq!(unicode_index("\\{"), Some(0x7b));
        assert_eq!(unicode_index("\\__sqrt__"), Some(0x221a));
        assert_eq!(unicode_index("\\nosuchsymbol"), None);
        assert_eq!(unicode_index(""), None);
    }

    #[test]
    fn symbol_classes() {
        assert!(is_binary_operator("+"));
        assert!(is_binary_operator("\u{2212}"));
        assert!(is_relation("="));
        assert!(is_spaced_symbol("\\rightarrow"));
        assert!(!is_spaced_symbol("x"));
        assert!(is_overunder_symbol("\\sum"));
        assert!(is_overunder_function("lim"));
        assert!(is_dropsub_symbol("\\int"));
        assert!(is_function_name("sin"));
        assert!(is_font_name("bfit"));
        assert!(is_left_delim("\\langle"));
        assert!(delimiters().any(|d| d == "\\Vert"));
        assert!(is_small_greek("alpha"));
        assert_eq!(accent_symbol("hat"), Some("\\circumflexaccent"));
        assert!(is_wide_accent("widehat"));
        assert!(accent_names().any(|name| name == "widetilde"));
    }

    #[test]
    fn bakoma_and_stix_tables() {
        assert_eq!(latex_to_bakoma("\\int"), Some((BakomaFile::Cmex10, 0x5a)));
        assert_eq!(latex_to_bakoma("x"), None);
        let parens = bakoma_size_alternatives("(").expect("parenthesis sizes");
        assert_eq!(parens[0], (FontName::Rm, "("));
        assert_eq!(parens.len(), 5);
        let bb = virtual_font(FontName::Bb).expect("blackboard");
        let ranges = bb.ranges(FontName::Bfit).expect("falls back to rm");
        assert_eq!(ranges[0].target, 0x1d7d8);
        assert!(virtual_font(FontName::Rm).is_none());
    }

    #[test]
    fn unicode_classes() {
        assert!(is_letter(u32::from('a')));
        assert!(is_letter(0x3b1));
        assert!(!is_letter(u32::from('+')));
        assert!(is_greek_capital(0x393));
        assert!(!is_greek_capital(0x3b3));
        assert!(is_digit('7') && is_digit('\u{b2}') && !is_digit('a'));
        assert!(is_space(' ') && is_space('\u{2003}') && !is_space('x'));
        assert_eq!(BakomaFile::Cmsy10.family(), "cmsy10");
    }

    /// Every name the tables know, typeset alone, as a script, as a
    /// command taking an argument and as a delimiter, in every built-in
    /// fontset: each must give a layout or a mathtext error.
    #[test]
    fn every_symbol_name_typesets_or_errors() {
        use crate::TextEngine;
        use crate::mathtext::{Fontset, MathStyle};

        let names = tex2uni::TEX2UNI
            .iter()
            .map(|(name, _)| *name)
            .chain(bakoma::LATEX_TO_BAKOMA.iter().map(|(name, _, _)| *name))
            .chain(bakoma::SIZE_ALTERNATIVES.iter().map(|(name, _)| *name))
            .chain(symbols::ACCENT_MAP.iter().map(|(name, _)| *name))
            .chain(symbols::FUNCTION_NAMES.iter().copied())
            .chain(symbols::FONT_NAMES.iter().copied())
            .chain(symbols::SPACE_WIDTHS.iter().map(|(name, _)| *name))
            .chain(symbols::WIDE_ACCENTS.iter().copied());
        let sources: Vec<String> = names
            .flat_map(|name| {
                let c = if name.starts_with('\\') {
                    name.to_owned()
                } else {
                    format!("\\{name}")
                };
                [
                    format!("${c} x^{c}_{{{c}}}$"),
                    format!("${c}{{y}}$"),
                    format!("$\\left{c}z\\right{c}$"),
                ]
            })
            .collect();
        for fontset in [Fontset::DejaVuSans, Fontset::Cm, Fontset::Stix] {
            let style = MathStyle::default().with_fontset(fontset);
            for source in &sources {
                match TextEngine::global().typeset_math(source, &style) {
                    Ok(layout) => drop(layout.to_text_run()),
                    Err(err) => assert!(matches!(err, crate::TextError::Math(_)), "{source}"),
                }
            }
        }
    }
}
