//! The texts of an axis artist: tick labels and the axis label placed at a
//! distance from their reference point in the direction away from the
//! line, and the offset text (matplotlib's `LabelBase`, `TickLabels`,
//! `AxisLabel` and `AxisArtist._update_offsetText`).

use plt_core::{Bbox, Point};
use plt_text::mathtext::is_math_text;
use plt_text::{BlockStyle, HAlign, RotationMode, VAlign};

use crate::artist::LayoutContext;
use crate::axes::{PlacedText, place_text};
use crate::colors::Color;
use crate::fonts::{FontProperties, FontSize, FontWeight};
use crate::rc::RcParams;
use crate::spines::SpineSide;
use crate::text::measure_text;
use crate::transforms::ResolvedTransform;

/// Which of the two label classes a [`LabelState`] follows; they differ
/// in their defaults for each axis direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LabelKind {
    /// The tick labels (matplotlib's `TickLabels`).
    TickLabels,
    /// The axis label (matplotlib's `AxisLabel`).
    AxisLabel,
}

impl LabelKind {
    /// The default vertical and horizontal alignment for an axis
    /// direction (matplotlib's `_default_alignments`).
    fn alignment(self, direction: SpineSide) -> (VAlign, HAlign) {
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
    fn angle(self, direction: SpineSide) -> f64 {
        match (self, direction) {
            (Self::TickLabels, SpineSide::Left) => 90.0,
            (Self::TickLabels, SpineSide::Right) => -90.0,
            (Self::TickLabels, SpineSide::Top)
            | (Self::AxisLabel, SpineSide::Left | SpineSide::Top) => 180.0,
            (Self::TickLabels | Self::AxisLabel, _) => 0.0,
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
    /// The font weight.
    pub(crate) weight: FontWeight,
    /// The text angle in degrees, relative to 90° plus the direction to
    /// the label.
    pub(crate) rotation: f64,
    pub(crate) valign: VAlign,
    pub(crate) halign: HAlign,
    /// The axis direction, which also tells tick labels which side their
    /// extent is measured on.
    pub(crate) direction: SpineSide,
}

impl LabelState {
    /// Labels of `kind` facing `direction`, `pad` points away, in a font
    /// of `size` points and `weight`.
    pub(crate) fn new(
        kind: LabelKind,
        direction: SpineSide,
        pad: f64,
        (size, weight): (f64, FontWeight),
    ) -> Self {
        let (valign, halign) = kind.alignment(direction);
        Self {
            kind,
            visible: true,
            pad,
            color: None,
            size,
            weight,
            rotation: kind.angle(direction).rem_euclid(360.0),
            valign,
            halign,
            direction,
        }
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

    /// How texts of this state are drawn: the rc font at the label size
    /// and weight, in `default_color` unless a colour was set.
    pub(crate) fn text_style(&self, rc: &RcParams, default_color: &Color) -> TextStyle {
        TextStyle {
            font: FontProperties::from_rc(rc)
                .with_size(FontSize::Points(self.size), rc.font_size)
                .with_weight(self.weight),
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
        let block = BlockStyle {
            halign: self.state.halign,
            valign: self.state.valign,
            multialign: self.state.halign,
            rotation: (self.ref_angle + 90.0 + self.state.rotation).rem_euclid(360.0),
            rotation_mode: RotationMode::Anchor,
            ..BlockStyle::default()
        };
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
        let font = (10.0, FontWeight::NORMAL);
        let left = LabelState::new(LabelKind::TickLabels, SpineSide::Left, 4.0, font);
        assert_eq!(
            (left.valign, left.halign, left.rotation),
            (VAlign::Center, HAlign::Right, 90.0)
        );
        let mut right = LabelState::new(LabelKind::TickLabels, SpineSide::Right, 4.0, font);
        assert_eq!(right.rotation, 270.0);
        right.set_axis_direction(SpineSide::Top);
        assert_eq!((right.valign, right.rotation), (VAlign::Baseline, 180.0));
        let label = LabelState::new(LabelKind::AxisLabel, SpineSide::Left, 5.0, font);
        assert_eq!(
            (label.valign, label.halign, label.rotation),
            (VAlign::Bottom, HAlign::Center, 180.0)
        );
        let bottom = LabelState::new(LabelKind::AxisLabel, SpineSide::Bottom, 5.0, font);
        assert_eq!((bottom.valign, bottom.rotation), (VAlign::Top, 0.0));
    }
}
