//! Font queries: matplotlib's `FontProperties` and generic family lists.

use std::fmt;

/// Font slant.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub enum FontStyle {
    /// Upright.
    #[default]
    Normal,
    /// Italic.
    Italic,
    /// Oblique (slanted roman).
    Oblique,
}

impl FontStyle {
    /// Parses `normal`, `italic` or `oblique` (`roman` is accepted as an
    /// alias of `normal`, as in fontconfig patterns).
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "normal" | "roman" => Some(Self::Normal),
            "italic" => Some(Self::Italic),
            "oblique" => Some(Self::Oblique),
            _ => None,
        }
    }
}

/// Font variant.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub enum FontVariant {
    /// Regular glyphs.
    #[default]
    Normal,
    /// Small capitals.
    SmallCaps,
}

/// A CSS font weight, 0–1000 (400 normal, 700 bold).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Weight(pub u16);

impl Weight {
    /// `normal` / `regular` (400).
    pub const NORMAL: Self = Self(400);
    /// `bold` (700).
    pub const BOLD: Self = Self(700);

    /// Parses a matplotlib weight name (`ultralight`, `light`, `normal`,
    /// `regular`, `book`, `medium`, `roman`, `semibold`, `demibold`, `demi`,
    /// `bold`, `heavy`, `extra bold`, `black`) or a number in 0–1000.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim().to_ascii_lowercase();
        let named = match text.as_str() {
            "ultralight" => 100,
            "light" => 200,
            "normal" | "regular" | "book" => 400,
            "medium" | "roman" => 500,
            "semibold" | "demibold" | "demi" => 600,
            "bold" => 700,
            "heavy" | "extra bold" => 800,
            "black" => 900,
            number => number.parse().ok().filter(|&value| value <= 1000)?,
        };
        Some(Self(named))
    }
}

impl Default for Weight {
    fn default() -> Self {
        Self::NORMAL
    }
}

/// A CSS font stretch, 0–1000 (500 normal).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Stretch(pub u16);

impl Stretch {
    /// `normal` (500).
    pub const NORMAL: Self = Self(500);
    /// `condensed` (300).
    pub const CONDENSED: Self = Self(300);
    /// `semi-condensed` (400).
    pub const SEMI_CONDENSED: Self = Self(400);
    /// `expanded` (700).
    pub const EXPANDED: Self = Self(700);

    /// Parses a matplotlib stretch name (`ultra-condensed` … `ultra-expanded`,
    /// with `extended` synonyms) or a number in 0–1000.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim().to_ascii_lowercase();
        let named = match text.as_str() {
            "ultra-condensed" => 100,
            "extra-condensed" => 200,
            "condensed" => 300,
            "semi-condensed" => 400,
            "normal" => 500,
            "semi-expanded" | "semi-extended" => 600,
            "expanded" | "extended" => 700,
            "extra-expanded" | "extra-extended" => 800,
            "ultra-expanded" | "ultra-extended" => 900,
            number => number.parse().ok().filter(|&value| value <= 1000)?,
        };
        Some(Self(named))
    }
}

impl Default for Stretch {
    fn default() -> Self {
        Self::NORMAL
    }
}

/// The generic family aliases matplotlib resolves through rcParams.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum GenericFamily {
    /// `serif`.
    Serif,
    /// `sans-serif` (also `sans` and `sans serif`).
    SansSerif,
    /// `cursive`.
    Cursive,
    /// `fantasy`.
    Fantasy,
    /// `monospace`.
    Monospace,
}

impl GenericFamily {
    /// Recognises a generic family name (case-insensitive).
    pub fn parse(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "serif" => Some(Self::Serif),
            "sans-serif" | "sans serif" | "sans" => Some(Self::SansSerif),
            "cursive" => Some(Self::Cursive),
            "fantasy" => Some(Self::Fantasy),
            "monospace" => Some(Self::Monospace),
            _ => None,
        }
    }
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|name| (*name).to_owned()).collect()
}

/// Preference lists for the generic families (`font.serif`,
/// `font.sans-serif`, ... rcParams).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GenericFamilies {
    /// `font.serif`.
    pub serif: Vec<String>,
    /// `font.sans-serif`.
    pub sans_serif: Vec<String>,
    /// `font.cursive`.
    pub cursive: Vec<String>,
    /// `font.fantasy`.
    pub fantasy: Vec<String>,
    /// `font.monospace`.
    pub monospace: Vec<String>,
}

impl GenericFamilies {
    /// The list for one generic family.
    pub fn get(&self, family: GenericFamily) -> &[String] {
        match family {
            GenericFamily::Serif => &self.serif,
            GenericFamily::SansSerif => &self.sans_serif,
            GenericFamily::Cursive => &self.cursive,
            GenericFamily::Fantasy => &self.fantasy,
            GenericFamily::Monospace => &self.monospace,
        }
    }
}

impl Default for GenericFamilies {
    /// matplotlib 3.11's default rcParams.
    fn default() -> Self {
        Self {
            serif: names(&[
                "DejaVu Serif",
                "Bitstream Vera Serif",
                "Computer Modern Roman",
                "New Century Schoolbook",
                "Century Schoolbook L",
                "Utopia",
                "ITC Bookman",
                "Bookman",
                "Nimbus Roman No9 L",
                "Times New Roman",
                "Times",
                "Palatino",
                "Charter",
                "serif",
            ]),
            sans_serif: names(&[
                "DejaVu Sans",
                "Bitstream Vera Sans",
                "Computer Modern Sans Serif",
                "Lucida Grande",
                "Verdana",
                "Geneva",
                "Lucid",
                "Arial",
                "Helvetica",
                "Avant Garde",
                "sans-serif",
            ]),
            cursive: names(&[
                "Apple Chancery",
                "Textile",
                "Zapf Chancery",
                "Sand",
                "Script MT",
                "Felipa",
                "Comic Neue",
                "Comic Sans MS",
                "cursive",
            ]),
            fantasy: names(&[
                "Chicago",
                "Charcoal",
                "Impact",
                "Western",
                "xkcd script",
                "fantasy",
            ]),
            monospace: names(&[
                "DejaVu Sans Mono",
                "Bitstream Vera Sans Mono",
                "Computer Modern Typewriter",
                "Andale Mono",
                "Nimbus Mono L",
                "Courier New",
                "Courier",
                "Fixed",
                "Terminal",
                "monospace",
            ]),
        }
    }
}

/// What to look for: matplotlib's `FontProperties`.
#[derive(Clone, PartialEq, Debug)]
pub struct FontQuery {
    /// Family names in decreasing priority; may contain generic families.
    pub families: Vec<String>,
    /// Slant.
    pub style: FontStyle,
    /// Variant.
    pub variant: FontVariant,
    /// Weight.
    pub weight: Weight,
    /// Stretch.
    pub stretch: Stretch,
    /// Size in points (only matters for non-scalable fonts).
    pub size: f64,
}

impl Default for FontQuery {
    /// matplotlib's defaults: `sans-serif`, normal style/variant/weight/
    /// stretch, 10 pt.
    fn default() -> Self {
        Self {
            families: vec!["sans-serif".to_owned()],
            style: FontStyle::Normal,
            variant: FontVariant::Normal,
            weight: Weight::NORMAL,
            stretch: Stretch::NORMAL,
            size: 10.0,
        }
    }
}

impl FontQuery {
    /// A default query for the given families.
    pub fn families<I, S>(families: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            families: families.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }

    /// Sets the style.
    #[must_use]
    pub fn with_style(mut self, style: FontStyle) -> Self {
        self.style = style;
        self
    }

    /// Sets the weight.
    #[must_use]
    pub fn with_weight(mut self, weight: Weight) -> Self {
        self.weight = weight;
        self
    }

    /// Sets the stretch.
    #[must_use]
    pub fn with_stretch(mut self, stretch: Stretch) -> Self {
        self.stretch = stretch;
        self
    }

    /// Sets the variant.
    #[must_use]
    pub fn with_variant(mut self, variant: FontVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl fmt::Display for FontQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:style={:?}:variant={:?}:weight={}:stretch={}:size={}",
            self.families.join(","),
            self.style,
            self.variant,
            self.weight.0,
            self.stretch.0,
            self.size
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_names_and_numbers() {
        assert_eq!(Weight::parse("bold"), Some(Weight::BOLD));
        assert_eq!(Weight::parse("Extra Bold"), Some(Weight(800)));
        assert_eq!(Weight::parse("350"), Some(Weight(350)));
        assert_eq!(Weight::parse("1001"), None);
        assert_eq!(Weight::parse("fat"), None);
        assert_eq!(Stretch::parse("semi-extended"), Some(Stretch(600)));
        assert_eq!(Stretch::parse("condensed"), Some(Stretch::CONDENSED));
        assert_eq!(Stretch::parse("x"), None);
        assert_eq!(FontStyle::parse("Italic"), Some(FontStyle::Italic));
        assert_eq!(FontStyle::parse("slanted"), None);
        assert_eq!(GenericFamily::parse("Sans"), Some(GenericFamily::SansSerif));
        assert_eq!(GenericFamily::parse("DejaVu Sans"), None);
    }

    #[test]
    fn defaults_match_matplotlib() {
        let query = FontQuery::default();
        assert_eq!(query.families, vec!["sans-serif"]);
        assert_eq!((query.weight, query.stretch), (Weight(400), Stretch(500)));
        let generics = GenericFamilies::default();
        assert_eq!(
            generics.get(GenericFamily::Monospace)[0],
            "DejaVu Sans Mono"
        );
        assert_eq!(generics.get(GenericFamily::Fantasy).len(), 6);
        let query = FontQuery::families(["serif"])
            .with_style(FontStyle::Italic)
            .with_weight(Weight::BOLD);
        assert!(query.to_string().starts_with("serif:style=Italic"));
        assert_eq!(
            query
                .with_variant(FontVariant::SmallCaps)
                .with_stretch(Stretch::EXPANDED)
                .stretch,
            Stretch(700)
        );
    }
}
