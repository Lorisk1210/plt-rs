//! Font catalogue entries, extracted exactly like matplotlib's
//! `ttfFontProperty` and `_get_font_alt_names`.

use crate::db::query::{FontStyle, FontVariant, Stretch};
use crate::face::Face;
use crate::tables::name::{NameKey, id};

/// Identifies one face stored in a [`FontDatabase`](crate::FontDatabase).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FontId(pub(crate) usize);

/// One searchable catalogue entry (matplotlib's `FontEntry`). A face can
/// have several entries: its FreeType family name plus alternative names
/// from the `name` table.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FontEntry {
    /// The face this entry describes.
    pub font: FontId,
    /// Family name matched by queries.
    pub name: String,
    /// Slant.
    pub style: FontStyle,
    /// Variant.
    pub variant: FontVariant,
    /// Weight (100–1000).
    pub weight: u16,
    /// Stretch (matplotlib only distinguishes condensed, semi-condensed,
    /// normal and expanded for TrueType fonts).
    pub stretch: Stretch,
}

/// fontconfig's style-name weight patterns, in matching order (substring
/// matches; `\bultra` requires a word start).
const WEIGHT_PATTERNS: [(&str, u16); 23] = [
    ("thin", 100),
    ("extralight", 200),
    ("ultralight", 200),
    ("demilight", 350),
    ("semilight", 350),
    ("light", 300),
    ("book", 380),
    ("regular", 400),
    ("normal", 400),
    ("medium", 500),
    ("demibold", 600),
    ("demi", 600),
    ("semibold", 600),
    ("extrabold", 800),
    ("superbold", 800),
    ("ultrabold", 800),
    ("bold", 700),
    ("ultrablack", 1000),
    ("superblack", 1000),
    ("extrablack", 1000),
    ("\\bultra", 1000),
    ("black", 900),
    ("heavy", 900),
];

/// Weight from a style string (spaces removed, case-insensitive), or `None`.
fn weight_from_style(style: &str) -> Option<u16> {
    let style: String = style
        .chars()
        .filter(|&c| c != ' ')
        .collect::<String>()
        .to_lowercase();
    WEIGHT_PATTERNS.iter().find_map(|&(pattern, weight)| {
        let found = match pattern.strip_prefix("\\b") {
            Some(word) => style.match_indices(word).any(|(at, _)| {
                at == 0
                    || style[..at]
                        .chars()
                        .next_back()
                        .is_some_and(|c| !(c.is_alphanumeric() || c == '_'))
            }),
            None => style.contains(pattern),
        };
        found.then_some(weight)
    })
}

/// matplotlib's lookup: the Macintosh record if non-empty, else Windows.
fn sfnt_lower(face: &Face<'_>, name_id: u16) -> String {
    let mac = face
        .name_string(NameKey::MAC_ENGLISH, name_id)
        .unwrap_or_default();
    let chosen = if mac.is_empty() {
        face.name_string(NameKey::WINDOWS_ENGLISH, name_id)
            .unwrap_or_default()
    } else {
        mac
    };
    chosen.to_lowercase()
}

/// The weight-guessing algorithm of `ttfFontProperty` (from fontconfig).
fn guess_weight(face: &Face<'_>) -> u16 {
    if let Some(os2) = face.os2().filter(|os2| os2.version != 0xFFFF) {
        return os2.weight_class;
    }
    let keys = [NameKey::MAC_ENGLISH, NameKey::WINDOWS_ENGLISH];
    let mut styles: Vec<String> = keys
        .iter()
        .flat_map(|&key| {
            [id::WWS_SUBFAMILY, id::TYPOGRAPHIC_SUBFAMILY, id::SUBFAMILY]
                .map(|name_id| face.name_string(key, name_id))
        })
        .flatten()
        .filter(|style| !style.is_empty())
        .collect();
    if styles.is_empty() {
        styles.extend(face.style_name());
    }
    styles
        .iter()
        .find_map(|style| weight_from_style(style))
        .unwrap_or(if face.style_flags().bold { 700 } else { 500 })
}

/// Builds the primary and alternative catalogue entries of `face`.
pub(crate) fn entries_for_face(face: &Face<'_>, font: FontId) -> Vec<FontEntry> {
    let name = face.family_name().unwrap_or_default();
    let full_name = sfnt_lower(face, id::FULL_NAME);
    let subfamily = sfnt_lower(face, id::SUBFAMILY);
    let style = if full_name.contains("oblique") {
        FontStyle::Oblique
    } else if full_name.contains("italic") {
        FontStyle::Italic
    } else if subfamily.contains("regular") {
        FontStyle::Normal
    } else if face.style_flags().italic {
        FontStyle::Italic
    } else {
        FontStyle::Normal
    };
    let variant = match name.to_lowercase().as_str() {
        "capitals" | "small-caps" => FontVariant::SmallCaps,
        _ => FontVariant::Normal,
    };
    let stretch = if ["narrow", "condensed", "cond"]
        .iter()
        .any(|word| full_name.contains(word))
    {
        Stretch::CONDENSED
    } else if full_name.contains("demi cond") {
        Stretch::SEMI_CONDENSED
    } else if ["wide", "expanded", "extended"]
        .iter()
        .any(|word| full_name.contains(word))
    {
        Stretch::EXPANDED
    } else {
        Stretch::NORMAL
    };
    let primary = FontEntry {
        font,
        name: name.clone(),
        style,
        variant,
        weight: guess_weight(face),
        stretch,
    };

    let mut seen = vec![name];
    let mut entries = vec![primary.clone()];
    for (family_id, subfamily_id) in [
        (id::FAMILY, id::SUBFAMILY),
        (id::TYPOGRAPHIC_FAMILY, id::TYPOGRAPHIC_SUBFAMILY),
        (id::WWS_FAMILY, id::WWS_SUBFAMILY),
    ] {
        for key in [NameKey::MAC_ENGLISH, NameKey::WINDOWS_ENGLISH] {
            let alt = face
                .name_string(key, family_id)
                .unwrap_or_default()
                .trim()
                .to_owned();
            if alt.is_empty() || seen.contains(&alt) {
                continue;
            }
            let subfamily = face.name_string(key, subfamily_id).unwrap_or_default();
            let weight = weight_from_style(subfamily.trim()).unwrap_or(400);
            seen.push(alt.clone());
            entries.push(FontEntry {
                name: alt,
                weight,
                ..primary.clone()
            });
        }
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_weights_follow_fontconfig_order() {
        assert_eq!(weight_from_style("Bold"), Some(700));
        assert_eq!(weight_from_style("Extra Bold"), Some(800));
        assert_eq!(weight_from_style("SemiLight"), Some(350));
        assert_eq!(weight_from_style("Light Italic"), Some(300));
        assert_eq!(weight_from_style("Ultra"), Some(1000));
        assert_eq!(weight_from_style("Black"), Some(900));
        assert_eq!(weight_from_style("Italic"), None);
        assert_eq!(weight_from_style("Book"), Some(380));
    }
}
