//! The `Text` handle.

use plt_core::Point;
use plt_text::{HAlign, RotationMode, VAlign};

use super::{LaidOutText, LineSpacing, TextArtist, TextBox, TextBoxProps, TextError};
use crate::artist::{ArtistHandle, LayoutContext, RawHandle};
use crate::colors::Color;
use crate::fonts::{FontProperties, FontSize};

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

impl Text {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    /// Sets the font size in points (sizes below 1 pt become 1 pt).
    pub fn set_fontsize(&self, points: f64) -> &Self {
        self.set_size(FontSize::Points(points))
    }

    /// Sets the font size from a keyword relative to `font.size`.
    pub fn set_fontsize_relative(&self, size: crate::fonts::RelativeSize) -> &Self {
        self.set_size(FontSize::Relative(size))
    }

    /// Sets the font size in points or relative to `font.size`
    /// (matplotlib's `set_fontsize`).
    pub fn set_size(&self, size: FontSize) -> &Self {
        self.raw.with_mut::<TextArtist, _>(|t| {
            let base = t.rc.font_size;
            t.font.set_size(size, base);
        });
        self
    }

    /// Moves the text horizontally, keeping its y (matplotlib's `set_x`).
    pub fn set_x(&self, x: f64) -> &Self {
        self.raw.with_mut::<TextArtist, _>(|t| t.position.x = x);
        self
    }

    /// Moves the text vertically, keeping its x (matplotlib's `set_y`).
    pub fn set_y(&self, y: f64) -> &Self {
        self.raw.with_mut::<TextArtist, _>(|t| t.position.y = y);
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
            t.bbox = props.map(|props| TextBox::new(props, t.font.size, &t.rc));
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
            None => t.bbox = Some(TextBox::background(color, t.font.size, &t.rc)),
        });
        self
    }

    /// Whether the text has a box.
    pub fn has_bbox(&self) -> bool {
        self.raw.with::<TextArtist, _>(|t| t.bbox.is_some())
    }
}

artist_properties! {
    Text => TextArtist, TextProps {
        /// The string shown (lines separated by `\n`).
        text, set_text: String;
        /// Position in the text's coordinate system.
        position, set_position: Point;
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
    }
}
