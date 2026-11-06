//! The texts of an axis artist: tick labels and the axis label placed at a
//! distance from their reference point in the direction away from the
//! line, and the offset text (matplotlib's `LabelBase`, `TickLabels`,
//! `AxisLabel` and `AxisArtist._update_offsetText`).

use pltrs_core::{Bbox, Point};
use pltrs_text::mathtext::is_math_text;
use pltrs_text::{BlockStyle, HAlign, LineSpacing, RotationMode, VAlign};

use crate::artist::{ArtistBase, DrawContext, LayoutContext};
use crate::axes::{PlacedText, place_text};
use crate::colors::Color;
use crate::fonts::{FontProperties, FontSize};
use crate::patches::PatchError;
use crate::rc::RcParams;
use crate::spines::SpineSide;
use crate::text::{BoxPlacement, TextBoxPatch, measure_text};
use crate::transforms::{ResolvedTransform, Transform};

/// Which of the two label classes a [`LabelState`] follows; they differ
/// in their defaults for each axis direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LabelKind {
    /// The tick labels (matplotlib's `TickLabels`).
    TickLabels,
    /// The axis label (matplotlib's `AxisLabel`).
    AxisLabel,
}

pltrs_core::archive_enum!(LabelKind {
    TickLabels,
    AxisLabel,
});

impl LabelKind {
    /// The default vertical and horizontal alignment for an axis
    /// direction (matplotlib's `_default_alignments`).
    pub(crate) fn alignment(self, direction: SpineSide) -> (VAlign, HAlign) {
        match (self, direction) {
            (Self::TickLabels, SpineSide::Left) => (VAlign::Center, HAlign::Right),
            (Self::TickLabels, SpineSide::Right) => (VAlign::Center, HAlign::Left),
            (Self::TickLabels, SpineSide::Bottom | SpineSide::Top) => {
                (VAlign::Baseline, HAlign::Center)
            }
            (Self::AxisLabel, SpineSide::Left | SpineSide::Top) => (VAlign::Bottom, HAlign::Center),
            (Self::AxisLabel, SpineSide::Right | SpineSide::Bottom) => {
                (VAlign::Top, HAlign::Center)
            }
        }
    }

    /// The default text angle for an axis direction, relative to 90° plus
    /// the direction to the label (matplotlib's `_default_angles`).
    pub(crate) fn angle(self, direction: SpineSide) -> f64 {
        match (self, direction) {
            (Self::TickLabels, SpineSide::Left) => 90.0,
            (Self::TickLabels, SpineSide::Right) => -90.0,
            (Self::TickLabels, SpineSide::Top)
            | (Self::AxisLabel, SpineSide::Left | SpineSide::Top) => 180.0,
            (Self::TickLabels | Self::AxisLabel, _) => 0.0,
        }
    }
}

/// How the texts of an axis artist are typeset beyond their size, colour,
/// angle and alignment (the other `Text` properties of matplotlib's
/// `LabelBase`): their font but its size, the alignment and spacing of
/// the lines of multi-line texts, and how rotation and alignment combine.
#[derive(Clone, Debug, PartialEq)]
pub struct TextLook {
    /// The font; the size of the texts replaces its size.
    pub font: FontProperties,
    /// The alignment of the lines of multi-line texts; `None` follows the
    /// horizontal alignment.
    pub multialign: Option<HAlign>,
    /// The spacing between lines.
    pub linespacing: LineSpacing,
    /// How rotation and alignment combine.
    pub rotation_mode: RotationMode,
}

pltrs_core::archive_struct!(TextLook {
    font,
    multialign,
    linespacing,
    rotation_mode
});

impl TextLook {
    /// Texts in `font`, their lines aligned as the texts and spaced
    /// normally, rotated around their anchor (as matplotlib's `LabelBase`
    /// sets its rotation mode).
    ///
    /// ```
    /// use pltrs::axisartist::TextLook;
    /// use pltrs::fonts::FontProperties;
    /// use pltrs::text::RotationMode;
    /// let look = TextLook::new(FontProperties::from_rc(&pltrs::rc::params()));
    /// assert_eq!(look.rotation_mode, RotationMode::Anchor);
    /// ```
    pub fn new(font: FontProperties) -> Self {
        Self {
            font,
            multialign: None,
            linespacing: LineSpacing::Normal,
            rotation_mode: RotationMode::Anchor,
        }
    }

    /// The block style of texts aligned `halign` and `valign`, turned by
    /// `rotation` degrees.
    fn block(&self, halign: HAlign, valign: VAlign, rotation: f64) -> BlockStyle {
        BlockStyle {
            halign,
            valign,
            multialign: self.multialign.unwrap_or(halign),
            rotation,
            rotation_mode: self.rotation_mode,
            linespacing: self.linespacing,
        }
    }
}

/// The look of tick labels or an axis label (matplotlib's `TickLabels`
/// and `AxisLabel` properties).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LabelState {
    pub(crate) kind: LabelKind,
    pub(crate) visible: bool,
    /// The gap to the ticks (tick labels) or tick labels (axis label), in
    /// points.
    pub(crate) pad: f64,
    /// The text colour; `None` follows the reference artist.
    pub(crate) color: Option<Color>,
    /// The font size in points.
    pub(crate) size: f64,
    /// The font but its size, and how the lines and the rotation are
    /// laid out.
    pub(crate) look: TextLook,
    /// The text angle in degrees, relative to 90° plus the direction to
    /// the label.
    pub(crate) rotation: f64,
    pub(crate) valign: VAlign,
    pub(crate) halign: HAlign,
    /// The axis direction, which also tells tick labels which side their
    /// extent is measured on.
    pub(crate) direction: SpineSide,
    /// The transform the program places the texts with, in place of the
    /// helper's tick (or label) transform followed by the artist's offset
    /// (matplotlib's `set_transform` of the `TickLabels` or `AxisLabel`).
    pub(crate) transform: Option<Transform>,
    /// The box drawn behind each text (matplotlib's `Text.set_bbox`).
    pub(crate) bbox: Option<TextBoxPatch>,
}

pltrs_core::archive_struct!(LabelState {
    kind,
    visible,
    pad,
    color,
    size,
    look,
    rotation,
    valign,
    halign,
    direction,
    transform,
    bbox
});

impl LabelState {
    /// Labels of `kind` facing `direction`, `pad` points away, `size`
    /// points large and typeset as `look` says.
    pub(crate) fn new(
        kind: LabelKind,
        direction: SpineSide,
        pad: f64,
        (size, look): (f64, TextLook),
    ) -> Self {
        let (valign, halign) = kind.alignment(direction);
        Self {
            kind,
            visible: true,
            pad,
            color: None,
            size,
            look,
            rotation: kind.angle(direction).rem_euclid(360.0),
            valign,
            halign,
            direction,
            transform: None,
            bbox: None,
        }
    }

    /// Draws the box behind `text`, one of the texts of these labels laid
    /// out (matplotlib's `Text.draw` draws a text's bbox patch first),
    /// with the artist properties `base`.
    pub(crate) fn draw_box(
        &self,
        ctx: &mut DrawContext<'_>,
        base: &ArtistBase,
        rc: &RcParams,
        text: &PlacedText,
    ) -> Result<(), PatchError> {
        let Some(bbox) = &self.bbox else {
            return Ok(());
        };
        let (corner, size) = text.laid_out.block.frame;
        let placement = BoxPlacement {
            anchor: text.anchor,
            corner,
            size,
            rotation: text.rotation,
            fontsize_px: ctx.points_to_pixels(self.size),
        };
        bbox.draw(ctx, base, rc, &placement)
    }

    /// Adopts the alignment and angle of `direction` (matplotlib's
    /// `set_axis_direction`).
    pub(crate) fn set_axis_direction(&mut self, direction: SpineSide) {
        let (valign, halign) = self.kind.alignment(direction);
        self.valign = valign;
        self.halign = halign;
        self.rotation = self.kind.angle(direction).rem_euclid(360.0);
        self.direction = direction;
    }

    /// How texts of this state are drawn: in their font at the label
    /// size, in `default_color` unless a colour was set.
    pub(crate) fn text_style(&self, rc: &RcParams, default_color: &Color) -> TextStyle {
        TextStyle {
            font: (self.look.font.clone()).with_size(FontSize::Points(self.size), rc.font_size),
            color: self.color.clone().unwrap_or_else(|| default_color.clone()),
        }
    }
}

/// How texts of one kind are styled.
#[derive(Clone, Debug)]
pub(crate) struct TextStyle {
    pub(crate) font: FontProperties,
    pub(crate) color: Color,
}

/// A text of `style` at `reference` (display pixels) moved `radius`
/// pixels towards `ref_angle` degrees and turned by `ref_angle + 90 +
/// angle`, aligned around the moved point (matplotlib's
/// `LabelBase.draw`).
struct Label<'a> {
    text: &'a str,
    reference: Point,
    ref_angle: f64,
    radius: f64,
    state: &'a LabelState,
}

impl Label<'_> {
    fn place(
        &self,
        layout: LayoutContext<'_>,
        style: &TextStyle,
        rc: &RcParams,
    ) -> Result<PlacedText, String> {
        let theta = self.ref_angle.to_radians();
        let anchor = Point::new(
            self.reference.x + self.radius * theta.cos(),
            self.reference.y + self.radius * theta.sin(),
        );
        let block = self.state.look.block(
            self.state.halign,
            self.state.valign,
            (self.ref_angle + 90.0 + self.state.rotation).rem_euclid(360.0),
        );
        place_text(
            layout,
            self.text,
            &style.font,
            &style.color,
            &block,
            anchor,
            rc,
        )
    }
}

/// A tick label to place: its location in the tick coordinates, the
/// direction to the label in degrees and its text.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TickLabel {
    pub(crate) loc: Point,
    pub(crate) angle: f64,
    pub(crate) text: String,
}

/// The tick labels of an axis artist, laid out.
pub(crate) struct PlacedTickLabels {
    pub(crate) texts: Vec<PlacedText>,
    /// The room they take away from the line, for the axis label
    /// (matplotlib's `_axislabel_pad`), in pixels.
    pub(crate) axislabel_pad: f64,
}

/// Lays out `labels` (locations in `transform`'s coordinates) as `state`
/// says, `external_pad` pixels beyond the line plus the state's pad
/// (matplotlib's `TickLabels.draw`).
pub(crate) fn tick_labels(
    layout: LayoutContext<'_>,
    labels: &[TickLabel],
    transform: &ResolvedTransform,
    state: &LabelState,
    external_pad: f64,
    style: &TextStyle,
    rc: &RcParams,
) -> Result<PlacedTickLabels, String> {
    let (offset, total) = label_offsets(layout, labels, state, style, rc)?;
    let pad = external_pad + layout.points_to_pixels(state.pad);
    let radius = offset + pad;
    let texts = labels
        .iter()
        .filter(|label| !label.text.trim().is_empty())
        .map(|label| {
            Label {
                text: &label.text,
                reference: transform.transform_point(label.loc),
                ref_angle: label.angle,
                radius,
                state,
            }
            .place(layout, style, rc)
        })
        .collect::<Result<_, _>>()?;
    Ok(PlacedTickLabels {
        texts,
        axislabel_pad: total + pad,
    })
}

/// How far the anchors of tick labels sit from the ticks for their
/// alignment, and how much room the labels take across the line
/// (matplotlib's `TickLabels._get_ticklabels_offsets`).
fn label_offsets(
    layout: LayoutContext<'_>,
    labels: &[TickLabel],
    state: &LabelState,
    style: &TextStyle,
    rc: &RcParams,
) -> Result<(f64, f64), String> {
    let metrics = labels
        .iter()
        .filter(|label| !label.text.trim().is_empty())
        .map(|label| {
            let math = rc.text_parse_math && is_math_text(&label.text);
            measure_text(&label.text, &style.font, math, layout.dpi(), rc)
                .map_err(|e| e.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if metrics.is_empty() {
        return Ok((0.0, 0.0));
    }
    let largest = |values: &mut dyn Iterator<Item = f64>| values.fold(f64::NEG_INFINITY, f64::max);
    let width = largest(&mut metrics.iter().map(|m| m.width));
    let height = largest(&mut metrics.iter().map(|m| m.height));
    let ascent = largest(&mut metrics.iter().map(|m| m.height - m.descent));
    let descent = largest(&mut metrics.iter().map(|m| m.descent));
    let (valign, halign) = (state.valign, state.halign);
    Ok(match state.direction {
        SpineSide::Left | SpineSide::Right => {
            let far = if state.direction == SpineSide::Left {
                HAlign::Left
            } else {
                HAlign::Right
            };
            let offset = match halign {
                h if h == far => width,
                HAlign::Center => 0.5 * width,
                _ => 0.0,
            };
            (offset, width)
        }
        SpineSide::Bottom | SpineSide::Top => {
            let far = if state.direction == SpineSide::Bottom {
                VAlign::Bottom
            } else {
                VAlign::Top
            };
            match valign {
                v if v == far => (height, height),
                VAlign::Center => (0.5 * height, height),
                VAlign::Baseline if state.direction == SpineSide::Bottom => {
                    (ascent, ascent + descent)
                }
                VAlign::Baseline => (descent, ascent + descent),
                _ => (0.0, height),
            }
        }
    })
}

/// The axis label `text` at `reference` (display pixels) in direction
/// `ref_angle`, `radius` pixels away (matplotlib's `AxisLabel.draw`).
pub(crate) fn axis_label(
    layout: LayoutContext<'_>,
    text: &str,
    (reference, ref_angle): (Point, f64),
    radius: f64,
    state: &LabelState,
    style: &TextStyle,
    rc: &RcParams,
) -> Result<PlacedText, String> {
    Label {
        text,
        reference,
        ref_angle,
        radius,
        state,
    }
    .place(layout, style, rc)
}

/// The offset text of an axis artist created facing `direction`, at the
/// corner of the axes box `extent` matching it, `rise` points above
/// (matplotlib's `AxisArtist._init_offsetText` and
/// `_update_offsetText`).
pub(crate) fn offset_text(
    layout: LayoutContext<'_>,
    text: &str,
    extent: Bbox,
    direction: SpineSide,
    rise: f64,
    style: &TextStyle,
    rc: &RcParams,
) -> Result<PlacedText, String> {
    let (x, y, valign, halign) = match direction {
        SpineSide::Left => (0.0, 1.0, VAlign::Bottom, HAlign::Right),
        SpineSide::Right => (1.0, 1.0, VAlign::Bottom, HAlign::Left),
        SpineSide::Bottom => (1.0, 0.0, VAlign::Top, HAlign::Right),
        SpineSide::Top => (1.0, 1.0, VAlign::Bottom, HAlign::Right),
    };
    let anchor = Point::new(
        extent.x0() + x * extent.width(),
        extent.y0() + y * extent.height() + layout.points_to_pixels(rise),
    );
    let block = BlockStyle {
        halign,
        valign,
        multialign: halign,
        ..BlockStyle::default()
    };
    place_text(layout, text, &style.font, &style.color, &block, anchor, rc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_follow_the_direction_tables() {
        // matplotlib's `axis_direction` table: the angles are relative to
        // 90 + the direction to the label.
        let font = (
            10.0,
            TextLook::new(FontProperties::from_rc(&crate::rc::params())),
        );
        let left = LabelState::new(LabelKind::TickLabels, SpineSide::Left, 4.0, font.clone());
        assert_eq!(
            (left.valign, left.halign, left.rotation),
            (VAlign::Center, HAlign::Right, 90.0)
        );
        let mut right = LabelState::new(LabelKind::TickLabels, SpineSide::Right, 4.0, font.clone());
        assert_eq!(right.rotation, 270.0);
        right.set_axis_direction(SpineSide::Top);
        assert_eq!((right.valign, right.rotation), (VAlign::Baseline, 180.0));
        let label = LabelState::new(LabelKind::AxisLabel, SpineSide::Left, 5.0, font.clone());
        assert_eq!(
            (label.valign, label.halign, label.rotation),
            (VAlign::Bottom, HAlign::Center, 180.0)
        );
        let bottom = LabelState::new(LabelKind::AxisLabel, SpineSide::Bottom, 5.0, font);
        assert_eq!((bottom.valign, bottom.rotation), (VAlign::Top, 0.0));
    }
}
