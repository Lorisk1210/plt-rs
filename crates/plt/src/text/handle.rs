//! The `Text` handle.

use plt_core::Point;
use plt_text::{HAlign, RotationMode, VAlign};

use super::{
    Feature, LaidOutText, Language, LineSpacing, TextArtist, TextBoxPatch, TextBoxProps, TextError,
};
use crate::artist::{ArtistHandle, LayoutContext, RawHandle, TypedHandle};
use crate::colors::Color;
use crate::fonts::{FontProperties, FontSize};
use crate::ticker::AxisName;

/// A handle to a text artist (matplotlib's `Text`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Text {
    raw: RawHandle,
}

impl ArtistHandle for Text {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl TypedHandle for Text {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        raw.is::<TextArtist>().then(|| Self::from_raw(raw.clone()))
    }
}

impl Text {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    /// Sets the font size: points (sizes below 1 pt become 1 pt) or a
    /// keyword relative to `font.size` (matplotlib's `set_fontsize`).
    ///
    /// ```
    /// use plt::figure::Figure;
    /// use plt::fonts::RelativeSize;
    /// let text = Figure::new().text(0.5, 0.5, "note");
    /// text.set_fontsize(14.0);
    /// assert_eq!(text.font().size, 14.0);
    /// text.set_fontsize(RelativeSize::Large);
    /// assert_eq!(text.font().size, 1.2 * plt::rc::params().font_size);
    /// ```
    pub fn set_fontsize(&self, size: impl Into<FontSize>) -> &Self {
        let size = size.into();
        self.raw.with_mut::<TextArtist, _>(|t| {
            let base = t.rc.font_size;
            t.font.set_size(size, base);
        });
        self
    }

    /// Moves the text horizontally, keeping its y (matplotlib's `set_x`).
    pub fn set_x(&self, x: f64) -> &Self {
        self.raw.with_mut::<TextArtist, _>(|t| {
            t.position.x = x;
            t.unit_sources.forget(AxisName::X);
        });
        self
    }

    /// Moves the text vertically, keeping its x (matplotlib's `set_y`).
    pub fn set_y(&self, y: f64) -> &Self {
        self.raw.with_mut::<TextArtist, _>(|t| {
            t.position.y = y;
            t.unit_sources.forget(AxisName::Y);
        });
        self
    }

    /// Sets the fonts of math text (matplotlib's `set_math_fontfamily`).
    pub fn set_math_fontfamily(&self, fontset: crate::fonts::Fontset) -> &Self {
        self.raw
            .with_mut::<TextArtist, _>(|t| t.font.math_fontfamily = fontset);
        self
    }

    /// The layout matplotlib's `Text._get_layout` computes: the (wrapped)
    /// lines, their placement relative to the anchor and the bounds of the
    /// rotated block. Fails for malformed math.
    pub fn layout(&self) -> Result<LaidOutText, TextError> {
        let state = self.raw.figure_state().borrow();
        self.raw
            .with::<TextArtist, _>(|t| t.layout(LayoutContext::new(&state)))
    }

    /// The text with the line breaks wrapping adds (the text itself when
    /// wrapping is off; matplotlib's `_get_wrapped_text`).
    pub fn wrapped_text(&self) -> Result<String, TextError> {
        let state = self.raw.figure_state().borrow();
        self.raw
            .with::<TextArtist, _>(|t| t.wrapped_text(LayoutContext::new(&state)))
    }

    /// Sets the font families (names or generic families).
    pub fn set_family<I: IntoIterator<Item = S>, S: Into<String>>(&self, families: I) -> &Self {
        let families: Vec<String> = families.into_iter().map(Into::into).collect();
        self.raw
            .with_mut::<TextArtist, _>(|t| t.font.families = families);
        self
    }

    /// Sets the font weight.
    pub fn set_weight(&self, weight: crate::fonts::FontWeight) -> &Self {
        self.raw
            .with_mut::<TextArtist, _>(|t| t.font.weight = weight);
        self
    }

    /// Sets the font style: upright, italic or oblique (matplotlib's
    /// `set_fontstyle`).
    pub fn set_style(&self, style: crate::fonts::FontStyle) -> &Self {
        self.raw.with_mut::<TextArtist, _>(|t| t.font.style = style);
        self
    }

    /// Sets the font variant: normal or small caps (matplotlib's
    /// `set_fontvariant`).
    pub fn set_variant(&self, variant: crate::fonts::FontVariant) -> &Self {
        self.raw
            .with_mut::<TextArtist, _>(|t| t.font.variant = variant);
        self
    }

    /// Sets the font stretch (matplotlib's `set_fontstretch`).
    pub fn set_stretch(&self, stretch: crate::fonts::FontStretch) -> &Self {
        self.raw
            .with_mut::<TextArtist, _>(|t| t.font.stretch = stretch);
        self
    }

    /// Draws a box behind the text, or removes it with `None`
    /// (matplotlib's `set_bbox`). The padding is fixed from the current
    /// font size, so set the size first.
    pub fn set_bbox(&self, props: Option<TextBoxProps>) -> &Self {
        self.raw.with_mut::<TextArtist, _>(|t| {
            t.bbox = props.map(|props| TextBoxPatch::new(props, t.font.size, &t.rc));
        });
        self
    }

    /// Sets the colour behind the text (matplotlib's
    /// `set_backgroundcolor`): the face of its box, creating a box with
    /// this face and edge colour if there is none.
    pub fn set_backgroundcolor(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.raw.with_mut::<TextArtist, _>(|t| match &mut t.bbox {
            Some(bbox) => bbox.set_facecolor(color),
            None => t.bbox = Some(TextBoxPatch::background(color, t.font.size, &t.rc)),
        });
        self
    }

    /// Whether the text has a box.
    pub fn has_bbox(&self) -> bool {
        self.raw.with::<TextArtist, _>(|t| t.bbox.is_some())
    }

    /// The box behind the text, if any (matplotlib's `get_bbox_patch`).
    pub fn bbox_patch(&self) -> Option<TextBoxPatch> {
        self.raw.with::<TextArtist, _>(|t| t.bbox.clone())
    }
}

artist_properties! {
    Text => TextArtist, TextProps {
        /// The string shown (lines separated by `\n`).
        text, set_text: String;
        /// Position in the text's coordinate system.
        position, set_position: Point => position_replaced;
        /// Text colour.
        color, set_color: Color;
        /// Counter-clockwise rotation in degrees.
        rotation, set_rotation: f64;
        /// Whether the rotation is a direction in the text's coordinates,
        /// turned by its transform (matplotlib's `transform_rotates_text`).
        transform_rotates_text, set_transform_rotates_text: bool;
        /// Horizontal alignment relative to the position.
        halign, set_halign: HAlign;
        /// Vertical alignment relative to the position.
        valign, set_valign: VAlign;
        /// Alignment of lines within a multi-line block (`None` follows `halign`).
        multialign, set_multialign: Option<HAlign>;
        /// How rotation and alignment combine.
        rotation_mode, set_rotation_mode: RotationMode;
        /// Spacing between lines.
        linespacing, set_linespacing: LineSpacing;
        /// Font description.
        font, set_font: FontProperties;
        /// Anti-aliasing.
        antialiased, set_antialiased: bool;
        /// Whether lines are broken to fit in the figure.
        wrap, set_wrap: bool;
        /// Whether `$...$` is typeset as math (`text.parse_math`).
        parse_math, set_parse_math: bool;
        /// OpenType feature settings on top of the font's defaults
        /// (matplotlib's `fontfeatures`, e.g. `-kern`, `ss01`, `aalt=2`,
        /// `liga[0:3]=0`; parse them with `str::parse`), applied when the
        /// text is drawn as a single line of plain text. They do not change
        /// its extent: matplotlib measures text without them.
        fontfeatures, set_fontfeatures: Vec<Feature>;
        /// The language of the text, which picks the font's language
        /// system (e.g. Serbian letter forms for `"sr"`), applied like the
        /// features. Setting `None` takes `text.language`, as a new text
        /// does.
        language, set_language: Option<Language> => resolve_language;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;
    use crate::rc;
    use crate::text::HAlign;

    #[test]
    fn new_texts_take_the_rc_language_and_none_restores_it() {
        rc::context(|| {
            rc::set("text.language", "sr").expect("valid rc");
            let fig = Figure::with_size(2.0, 1.0, 72.0);
            let text = fig.text(0.5, 0.5, "\u{431}");
            assert_eq!(text.language(), Some(Language::from("sr")));
            text.set_language(Language::from("ru"));
            assert_eq!(text.language(), Some(Language::from("ru")));
            text.set_language(None);
            assert_eq!(text.language(), Some(Language::from("sr")));
        });
        let fig = Figure::with_size(2.0, 1.0, 72.0);
        let text = fig.text(0.5, 0.5, "x");
        assert_eq!(text.language(), None, "text.language defaults to None");
        assert_eq!(text.fontfeatures(), []);
    }

    #[test]
    fn variant_changes_only_the_variant() {
        let fig = Figure::with_size(2.0, 1.0, 72.0);
        let text = fig.text(0.5, 0.5, "Small caps");
        let before = text.font();
        text.set_variant(crate::fonts::FontVariant::SmallCaps);
        assert_eq!(
            text.font(),
            FontProperties {
                variant: crate::fonts::FontVariant::SmallCaps,
                ..before
            }
        );
    }

    #[test]
    fn features_and_language_leave_the_extent_alone() {
        let fig = Figure::with_size(4.0, 1.0, 100.0);
        let text = fig.text(0.5, 0.5, "AVAVA office");
        text.set_halign(HAlign::Right);
        let before = text.window_extent();
        let features: Vec<Feature> = ["-kern", "-liga"]
            .iter()
            .map(|f| f.parse().expect("valid feature"))
            .collect();
        text.set_fontfeatures(features.clone())
            .set_language(Language::from("ca"));
        assert_eq!(text.fontfeatures(), features);
        assert!(before.is_some());
        assert_eq!(text.window_extent(), before);
    }
}
