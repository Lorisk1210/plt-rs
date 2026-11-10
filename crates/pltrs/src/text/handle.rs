//! The `Text` handle.

use pltrs_core::{Bbox, Point};
use pltrs_text::{HAlign, RotationMode, VAlign};

use super::{
    AngleReduction, Feature, LaidOutText, Language, LineSpacing, TextArtist, TextBoxPatch,
    TextBoxProps, TextError,
};
use crate::artist::{ArtistHandle, ArtistLookup, LayoutContext, RawHandle, TypedHandle};
use crate::colors::Color;
use crate::fonts::{FontProperties, FontSize};
use crate::patches::{FancyBboxPatch, Patch};
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
    /// use pltrs::figure::Figure;
    /// use pltrs::fonts::RelativeSize;
    /// let text = Figure::new().text(0.5, 0.5, "note");
    /// text.set_fontsize(14.0);
    /// assert_eq!(text.font().size, 14.0);
    /// text.set_fontsize(RelativeSize::Large);
    /// assert_eq!(text.font().size, 1.2 * pltrs::rc::params().font_size);
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

    /// The rotation the text is drawn at in degrees (matplotlib's
    /// `get_rotation`): its rotation, turned by its transform at its
    /// position when [`transform_rotates_text`](Self::transform_rotates_text),
    /// reduced to `[0, 360)`.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::transforms::{Affine2D, Transform};
    /// let ax = Figure::new().subplot();
    /// let text = ax.text(0.0, 0.0, "turned");
    /// text.set_transform(Transform::Affine(Affine2D::IDENTITY.rotate_deg(30.0)));
    /// text.set_transform_rotates_text(true);
    /// assert!((text.drawn_rotation() - 30.0).abs() < 1e-9);
    /// ```
    pub fn drawn_rotation(&self) -> f64 {
        let state = self.raw.figure_state().borrow();
        self.raw
            .with::<TextArtist, _>(|t| t.drawn_rotation(LayoutContext::new(&state)))
    }

    /// The text's extent in display coordinates, measured in its figure as
    /// it is now (as [`RawHandle::extent_as_is`] measures it), were it
    /// anchored at `position` and turned by `rotation` degrees: where
    /// matplotlib's `Text3D.get_window_extent` measures a 3-D text, at the
    /// x and y of its 3-D anchor and at its own rotation, which the
    /// projection replaces only while it draws the text. `None` when the
    /// text takes no space there.
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::figure::Figure;
    /// use pltrs::transforms::Point;
    /// let fig = Figure::new();
    /// let note = fig.text(0.5, 0.5, "note");
    /// let here = note.raw().extent_as_is();
    /// assert_eq!(note.extent_placed(Point::new(0.5, 0.5), 0.0), here);
    /// let turned = note.extent_placed(Point::new(0.5, 0.5), 90.0).expect("measured");
    /// assert!(turned.height() > turned.width());
    /// assert_eq!(note.raw().extent_as_is(), here, "the text itself stays");
    /// ```
    pub fn extent_placed(&self, position: Point, rotation: f64) -> Option<Bbox> {
        let id = self.raw.id();
        let figure = self.raw.figure_state();
        let mut state = figure.borrow_mut();
        let text = state.store.get_as_mut::<TextArtist>(id)?;
        let own = (text.position, text.rotation, text.rotation_reduction);
        (text.position, text.rotation, text.rotation_reduction) =
            (position, rotation, AngleReduction::Modulo360);
        let extent =
            (state.artist(id)).and_then(|text| text.window_extent(&LayoutContext::new(&state)));
        if let Some(text) = state.store.get_as_mut::<TextArtist>(id) {
            (text.position, text.rotation, text.rotation_reduction) = own;
        }
        extent
    }

    /// Whether the text is where its axes can show it: anywhere unless it
    /// is placed in data coordinates outside the domain of the axes'
    /// scales, a negative position on a log axis say (matplotlib's
    /// `_point_in_data_domain`). Such a text takes no space in tight boxes.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.set_yscale("log")?;
    /// assert!(ax.text(0.0, 5.0, "shown").in_data_domain());
    /// assert!(!ax.text(0.0, -5.0, "lost").in_data_domain());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn in_data_domain(&self) -> bool {
        let state = self.raw.figure_state().borrow();
        self.raw
            .with::<TextArtist, _>(|t| t.in_data_domain(LayoutContext::new(&state).store()))
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

    /// Puts `bbox`, a box as another text has it ([`Text::bbox_patch`]),
    /// behind the text, or removes the box with `None`.
    pub fn set_bbox_patch(&self, bbox: Option<TextBoxPatch>) -> &Self {
        self.raw.with_mut::<TextArtist, _>(|t| t.bbox = bbox);
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

    /// The rotation mode as matplotlib's `get_rotation_mode` reports it:
    /// [`RotationMode::Anchor`] once laying the text out wrapped its lines
    /// (matplotlib switches a wrapped text to it), until a mode is set.
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::figure::Figure;
    /// use pltrs::text::RotationMode;
    ///
    /// let text = Figure::new().text(0.5, 0.5, "a text long enough to wrap twice in the figure");
    /// text.set_wrap(true);
    /// assert_eq!(text.reported_rotation_mode(), RotationMode::Default);
    /// assert!(text.window_extent().is_some());
    /// assert_eq!(text.reported_rotation_mode(), RotationMode::Anchor);
    /// text.set_rotation_mode(RotationMode::Default);
    /// assert_eq!(text.reported_rotation_mode(), RotationMode::Default);
    /// ```
    pub fn reported_rotation_mode(&self) -> RotationMode {
        self.raw
            .with::<TextArtist, _>(TextArtist::reported_rotation_mode)
    }

    /// Whether the text has a box.
    pub fn has_bbox(&self) -> bool {
        self.raw.with::<TextArtist, _>(|t| t.bbox.is_some())
    }

    /// The box behind the text, if any (matplotlib's `get_bbox_patch`).
    pub fn bbox_patch(&self) -> Option<TextBoxPatch> {
        self.raw.with::<TextArtist, _>(|t| t.bbox.clone())
    }

    /// Gives the box behind the text the look of `patch` (see
    /// [`TextBoxPatch::to_patch`]); without a box, nothing changes.
    pub fn restyle_bbox(&self, patch: &Patch<FancyBboxPatch>) -> &Self {
        self.raw.with_mut::<TextArtist, _>(|t| {
            if let Some(bbox) = &mut t.bbox {
                bbox.restyle_from(patch);
            }
        });
        self
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
        rotation_mode, set_rotation_mode: RotationMode => rotation_mode_replaced;
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
        /// Whether LaTeX typesets the text (matplotlib's `usetex`, by
        /// default `text.usetex` when the text was made): every line is
        /// typeset by `latex` on its own and drawn from its DVI output.
        /// Drawing fails where TeX is not installed.
        usetex, set_usetex: bool;
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

    #[test]
    fn wrapping_switches_the_reported_rotation_mode() {
        let fig = Figure::with_size(2.0, 1.0, 72.0);
        let plain = fig.text(0.5, 0.5, "plain");
        let wrapped = fig.text(0.5, 0.2, "wrapped text, long enough to wrap");
        wrapped.set_wrap(true);
        assert_eq!(wrapped.reported_rotation_mode(), RotationMode::Default);
        assert!(plain.window_extent().is_some() && wrapped.window_extent().is_some());
        assert_eq!(plain.reported_rotation_mode(), RotationMode::Default);
        assert_eq!(wrapped.reported_rotation_mode(), RotationMode::Anchor);
        // Set, the mode stays until the next wrapping, even unwrapped.
        wrapped.set_rotation_mode(RotationMode::Default);
        assert_eq!(wrapped.reported_rotation_mode(), RotationMode::Default);
        wrapped.set_wrap(false);
        assert!(wrapped.window_extent().is_some());
        assert_eq!(wrapped.reported_rotation_mode(), RotationMode::Default);
        assert_eq!(wrapped.rotation_mode(), RotationMode::Default);
    }
}
