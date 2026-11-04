//! matplotlib's `FontProperties`: a complete font description, its setters'
//! validation and its fontconfig pattern form.

use std::cell::RefCell;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::Arc;

use pltrs_text::fontconfig::{FontconfigPattern, escape_family, escape_value};
use pltrs_text::mathtext::Fontset;
use pltrs_text::{FontStyle, FontVariant, GenericFamilies, TextEngine, TextError};

use super::{FontPropertyError, FontSize, FontStretch, FontWeight};
use crate::rc::RcParams;
use crate::ticker::pyfmt;

/// Raises sizes below 1 pt to 1 pt, as `FontProperties.set_size` does
/// (NaN passes through).
fn clamp_size(points: f64) -> f64 {
    if points < 1.0 { 1.0 } else { points }
}

/// A complete font description (matplotlib's `FontProperties`) with the
/// size resolved to points.
#[derive(Clone, Debug, PartialEq)]
pub struct FontProperties {
    /// Family names or generic families, in priority order.
    pub families: Vec<String>,
    /// Upright, italic or oblique.
    pub style: FontStyle,
    /// Normal or small caps.
    pub variant: FontVariant,
    /// Weight.
    pub weight: FontWeight,
    /// Stretch.
    pub stretch: FontStretch,
    /// Size in points.
    pub size: f64,
    /// The fonts of math text (`mathtext.fontset` by default).
    pub math_fontfamily: Fontset,
    /// The font file the text is set in whatever the families, style and
    /// weight (matplotlib's `fname`); a bare file name may also name a
    /// bundled font. See [`FontProperties::from_file`].
    pub file: Option<PathBuf>,
}

pltrs_core::archive_struct!(FontProperties {
    families,
    style,
    variant,
    weight,
    stretch,
    size,
    math_fontfamily,
    file
});

/// Parses a font style name exactly as `FontProperties.set_style` accepts
/// it (`normal`, `italic`, `oblique`).
pub fn parse_style(text: &str) -> Result<FontStyle, FontPropertyError> {
    match text {
        "normal" => Ok(FontStyle::Normal),
        "italic" => Ok(FontStyle::Italic),
        "oblique" => Ok(FontStyle::Oblique),
        _ => Err(FontPropertyError::Style(text.to_owned())),
    }
}

/// Parses a font variant name (`normal`, `small-caps`).
pub fn parse_variant(text: &str) -> Result<FontVariant, FontPropertyError> {
    match text {
        "normal" => Ok(FontVariant::Normal),
        "small-caps" => Ok(FontVariant::SmallCaps),
        _ => Err(FontPropertyError::Variant(text.to_owned())),
    }
}

/// Parses a math font family (a `mathtext.fontset` value).
pub fn parse_math_fontfamily(text: &str) -> Result<Fontset, FontPropertyError> {
    text.parse()
        .map_err(|_| FontPropertyError::MathFontFamily(text.to_owned()))
}

impl FontProperties {
    /// The defaults from `font.*` rcParams.
    pub fn from_rc(rc: &RcParams) -> Self {
        Self {
            families: rc.font_family.clone(),
            style: parse_style(&rc.font_style).unwrap_or(FontStyle::Normal),
            variant: parse_variant(&rc.font_variant).unwrap_or(FontVariant::Normal),
            weight: rc.font_weight,
            stretch: rc.font_stretch,
            size: clamp_size(rc.font_size),
            math_fontfamily: rc.mathtext_fontset,
            file: None,
        }
    }

    /// `FontProperties(fname=path)`: the defaults from `rc`, with text set
    /// in the font file at `path` (or the bundled font file so named).
    ///
    /// # Errors
    ///
    /// When the file cannot be read or holds no font. A file set directly
    /// in [`FontProperties::file`] that cannot be read is ignored.
    pub fn from_file(path: impl Into<PathBuf>, rc: &RcParams) -> Result<Self, TextError> {
        let path = path.into();
        TextEngine::global().file_faces(&path)?;
        Ok(Self {
            file: Some(path),
            ..Self::from_rc(rc)
        })
    }

    /// `FontProperties(pattern)`: the defaults from `rc` overridden by a
    /// fontconfig pattern such as `"DejaVu Serif-12:bold"`.
    pub fn from_pattern(pattern: &str, rc: &RcParams) -> Result<Self, FontPropertyError> {
        let mut props = Self::from_rc(rc);
        props.families = vec![pattern.to_owned()];
        props.set_fontconfig_pattern(pattern, rc.font_size)?;
        Ok(props)
    }

    /// Sets the size (`FontProperties.set_size`): relative sizes scale
    /// `base` (the current `font.size`) and sizes below 1 pt become 1 pt.
    pub fn set_size(&mut self, size: FontSize, base: f64) -> &mut Self {
        self.size = clamp_size(size.to_points(base));
        self
    }

    /// The same properties at another size, given relative to `font.size`.
    #[must_use]
    pub fn with_size(mut self, size: FontSize, base: f64) -> Self {
        self.set_size(size, base);
        self
    }

    /// The same properties with another weight.
    #[must_use]
    pub fn with_weight(self, weight: FontWeight) -> Self {
        Self { weight, ..self }
    }

    /// Applies a fontconfig pattern (`set_fontconfig_pattern`): the first
    /// value of every property goes through the matching setter.
    pub fn set_fontconfig_pattern(
        &mut self,
        pattern: &str,
        base_size: f64,
    ) -> Result<&mut Self, FontPropertyError> {
        let parsed = FontconfigPattern::parse(pattern)
            .map_err(|err| FontPropertyError::Pattern(err.to_string()))?;
        for (key, values) in parsed.properties() {
            let value = values.first().map_or("", String::as_str);
            match key.as_str() {
                "family" | "name" => self.families = vec![value.to_owned()],
                "style" | "slant" => self.style = parse_style(value)?,
                "variant" => self.variant = parse_variant(value)?,
                "weight" => self.weight = value.parse()?,
                "stretch" => self.stretch = value.parse()?,
                "size" => {
                    self.set_size(value.parse()?, base_size);
                }
                _ => return Err(FontPropertyError::PatternKey(key.clone())),
            }
        }
        Ok(self)
    }

    /// The fontconfig pattern describing these properties
    /// (`get_fontconfig_pattern`).
    pub fn fontconfig_pattern(&self) -> String {
        let families: Vec<String> = self.families.iter().map(|f| escape_family(f)).collect();
        let properties = [
            ("style", self.style.name().to_owned()),
            ("variant", self.variant.name().to_owned()),
            ("weight", self.weight.to_string()),
            ("stretch", self.stretch.to_string()),
            ("size", pyfmt::repr(self.size)),
        ];
        let mut pattern = families.join(",");
        for (key, value) in properties {
            write!(pattern, ":{key}={}", escape_value(&value))
                .expect("writing to a String cannot fail");
        }
        pattern
    }

    /// [`request`](Self::request) with the `text.hinting` in force now:
    /// what text is laid out and drawn with, as matplotlib's Agg backend
    /// reads `text.hinting` when it draws, whatever the parameters the text
    /// was made under.
    pub(crate) fn drawn_request(&self) -> pltrs_text::FontRequest {
        self.request(&crate::rc::params())
    }

    /// The font request used to pick font files, with the hinting text is
    /// laid out with under `rc` (`text.hinting` unless a vector backend
    /// draws). Generic family names resolve through the `font.serif`,
    /// `font.sans-serif`, ... lists in force when the request is made, and
    /// the last resort font as `font.enable_last_resort` then says, as
    /// matplotlib's `findfont` reads them when it draws (a text made under
    /// other lists is drawn with those of the moment).
    pub fn request(&self, rc: &RcParams) -> pltrs_text::FontRequest {
        let mut query = pltrs_text::FontQuery::families(self.families.iter().cloned());
        query.style = self.style;
        query.variant = self.variant;
        query.weight = pltrs_text::Weight(self.weight.value());
        query.stretch = pltrs_text::Stretch(self.stretch.value());
        query.size = self.size;
        let current = crate::rc::params();
        pltrs_text::FontRequest {
            query,
            generics: generic_families(&current),
            last_resort: current.font_enable_last_resort,
            hinting: crate::text::hinting(rc),
            weight_keyword: self.weight.name(),
            file: self
                .file
                .as_ref()
                .and_then(|path| TextEngine::global().file_faces(path).ok())
                .and_then(|faces| faces.first().copied()),
        }
    }
}

/// The fonts the generic families stand for, in order of preference: the
/// `font.serif`, `font.sans-serif`, ... lists of `rc`. The lists made last
/// are given again while `rc` holds the same, so that the requests of
/// every text share one copy of them.
pub(crate) fn generic_families(rc: &RcParams) -> Arc<GenericFamilies> {
    thread_local! {
        static LAST: RefCell<Option<Arc<GenericFamilies>>> = const { RefCell::new(None) };
    }
    LAST.with_borrow_mut(|last| {
        if let Some(families) = last.as_ref().filter(|families| lists_of(families, rc)) {
            return Arc::clone(families);
        }
        let families = Arc::new(GenericFamilies {
            serif: rc.font_serif.clone(),
            sans_serif: rc.font_sans_serif.clone(),
            cursive: rc.font_cursive.clone(),
            fantasy: rc.font_fantasy.clone(),
            monospace: rc.font_monospace.clone(),
        });
        *last = Some(Arc::clone(&families));
        families
    })
}

/// Whether `families` are the generic-family lists of `rc`.
fn lists_of(families: &GenericFamilies, rc: &RcParams) -> bool {
    families.serif == rc.font_serif
        && families.sans_serif == rc.font_sans_serif
        && families.cursive == rc.font_cursive
        && families.fantasy == rc.font_fantasy
        && families.monospace == rc.font_monospace
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::{FontExt, FontManager, RelativeSize};

    #[test]
    fn text_is_set_in_the_named_file() {
        let rc = RcParams::matplotlib();
        let props = FontProperties::from_file("cmr10.ttf", &rc).unwrap();
        assert_eq!(
            props.families,
            FontProperties::from_rc(&rc).families,
            "the other properties keep their defaults"
        );
        assert!(props.request(&rc).file.is_some());
        assert!(FontProperties::from_rc(&rc).request(&rc).file.is_none());
        assert_eq!(
            FontManager::global()
                .find_font(&props, &rc, FontExt::Ttf, true)
                .unwrap(),
            "cmr10.ttf"
        );
        assert!(FontProperties::from_file("/no/such/font.ttf", &rc).is_err());
        assert!(FontManager::global().add_font("/no/such/font.ttf").is_err());
    }

    #[test]
    fn sizes_are_clamped_and_relative_to_the_base() {
        let mut props = FontProperties::from_rc(&RcParams::matplotlib());
        props.set_size(FontSize::Points(0.5), 10.0);
        assert_eq!(props.size, 1.0);
        props.set_size(FontSize::Points(f64::NAN), 10.0);
        assert!(props.size.is_nan());
        props.set_size(RelativeSize::Small.into(), 12.0);
        assert_eq!(props.size, 0.833 * 12.0);
    }

    #[test]
    fn patterns_set_properties() {
        let rc = RcParams::matplotlib();
        let props = FontProperties::from_pattern("Foo,Bar-12:bold:slant=italic", &rc)
            .expect("valid pattern");
        assert_eq!(props.families, vec!["Foo"]);
        assert_eq!(
            (props.weight, props.style),
            (FontWeight::BOLD, FontStyle::Italic)
        );
        assert_eq!(props.size, 12.0);
        assert_eq!(
            props.fontconfig_pattern(),
            "Foo:style=italic:variant=normal:weight=bold:stretch=normal:size=12.0"
        );
        assert!(FontProperties::from_pattern("sans-serif", &rc).is_err());
        assert!(FontProperties::from_pattern(":condensed", &rc).is_err());
        assert!(FontProperties::from_pattern(":weight=thin", &rc).is_err());
    }

    #[test]
    fn exact_names() {
        assert_eq!(parse_style("oblique"), Ok(FontStyle::Oblique));
        assert!(parse_style("Italic").is_err());
        assert!(parse_variant("smallcaps").is_err());
        assert_eq!(parse_math_fontfamily("cm"), Ok(Fontset::Cm));
        assert!(parse_math_fontfamily("comic").is_err());
    }
}
