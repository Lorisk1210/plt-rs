//! The texts of an axis artist: tick labels and the axis label placed at a
//! distance from their reference point in the direction away from the
//! line, and the offset text (matplotlib's `LabelBase`, `TickLabels`,
//! `AxisLabel` and `AxisArtist._update_offsetText`).

use plt_core::Point;
use plt_text::mathtext::is_math_text;
use plt_text::{BlockStyle, HAlign, RotationMode, VAlign};

use super::helper::TickInfo;
use crate::artist::LayoutContext;
use crate::axes::{PlacedText, place_text};
use crate::colors::Color;
use crate::fonts::FontProperties;
use crate::rc::RcParams;
use crate::spines::SpineSide;
use crate::text::measure_text;
use crate::transforms::ResolvedTransform;

/// How texts of one kind are styled.
#[derive(Clone, Debug)]
pub(crate) struct TextStyle {
    pub(crate) font: FontProperties,
    pub(crate) color: Color,
}

/// The default vertical and horizontal alignment of tick labels for an
/// axis direction (matplotlib's `TickLabels._default_alignments`).
fn ticklabel_alignment(direction: SpineSide) -> (VAlign, HAlign) {
    match direction {
        SpineSide::Left => (VAlign::Center, HAlign::Right),
        SpineSide::Right => (VAlign::Center, HAlign::Left),
        SpineSide::Bottom | SpineSide::Top => (VAlign::Baseline, HAlign::Center),
    }
}

/// The default text angle of tick labels (`TickLabels._default_angles`).
fn ticklabel_angle(direction: SpineSide) -> f64 {
    match direction {
        SpineSide::Left => 90.0,
        SpineSide::Right => -90.0,
        SpineSide::Bottom => 0.0,
        SpineSide::Top => 180.0,
    }
}

/// The default vertical and horizontal alignment of the axis label
/// (`AxisLabel._default_alignments`).
fn axislabel_alignment(direction: SpineSide) -> (VAlign, HAlign) {
    match direction {
        SpineSide::Left | SpineSide::Top => (VAlign::Bottom, HAlign::Center),
        SpineSide::Right | SpineSide::Bottom => (VAlign::Top, HAlign::Center),
    }
}

/// The default text angle of the axis label (`AxisLabel._default_angles`).
fn axislabel_angle(direction: SpineSide) -> f64 {
    match direction {
        SpineSide::Left | SpineSide::Top => 180.0,
        SpineSide::Right | SpineSide::Bottom => 0.0,
    }
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
    angle: f64,
    align: (VAlign, HAlign),
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
        let (valign, halign) = self.align;
        let block = BlockStyle {
            halign,
            valign,
            multialign: halign,
            rotation: (self.ref_angle + 90.0 + self.angle).rem_euclid(360.0),
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

/// The tick labels of an axis artist, laid out.
pub(crate) struct TickLabels {
    pub(crate) texts: Vec<PlacedText>,
    /// The room they take away from the line, for the axis label
    /// (matplotlib's `_axislabel_pad`), in pixels.
    pub(crate) axislabel_pad: f64,
}

/// Lays out the labels of `ticks` (locations in `transform`'s
/// coordinates) for an axis in `direction`, `pad` pixels beyond the
/// ticks, with their angles turned by `add_angle` (matplotlib's
/// `AxisArtist._get_tick_info` and `TickLabels.draw`).
pub(crate) fn tick_labels(
    layout: LayoutContext<'_>,
    ticks: &[TickInfo],
    transform: &ResolvedTransform,
    (direction, add_angle): (SpineSide, f64),
    pad: f64,
    style: &TextStyle,
    rc: &RcParams,
) -> Result<TickLabels, String> {
    let align = ticklabel_alignment(direction);
    let (offset, total) = label_offsets(layout, ticks, direction, align, style, rc)?;
    let radius = offset + pad;
    let mut texts = Vec::new();
    for tick in ticks.iter().filter(|tick| !tick.label.trim().is_empty()) {
        let label = Label {
            text: &tick.label,
            reference: transform.transform_point(tick.loc),
            ref_angle: tick.angle_tangent - 90.0 + add_angle,
            radius,
            angle: ticklabel_angle(direction),
            align,
        };
        texts.push(label.place(layout, style, rc)?);
    }
    Ok(TickLabels {
        texts,
        axislabel_pad: total + pad,
    })
}

/// How far the anchors of tick labels sit from the ticks for their
/// alignment, and how much room the labels take across the line
/// (matplotlib's `TickLabels._get_ticklabels_offsets`).
fn label_offsets(
    layout: LayoutContext<'_>,
    ticks: &[TickInfo],
    direction: SpineSide,
    (valign, halign): (VAlign, HAlign),
    style: &TextStyle,
    rc: &RcParams,
) -> Result<(f64, f64), String> {
    let metrics = ticks
        .iter()
        .filter(|tick| !tick.label.trim().is_empty())
        .map(|tick| {
            let math = rc.text_parse_math && is_math_text(&tick.label);
            measure_text(&tick.label, &style.font, math, layout.dpi(), rc)
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
    Ok(match direction {
        SpineSide::Left | SpineSide::Right => {
            let far = if direction == SpineSide::Left {
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
            let far = if direction == SpineSide::Bottom {
                VAlign::Bottom
            } else {
                VAlign::Top
            };
            match valign {
                v if v == far => (height, height),
                VAlign::Center => (0.5 * height, height),
                VAlign::Baseline if direction == SpineSide::Bottom => (ascent, ascent + descent),
                VAlign::Baseline => (descent, ascent + descent),
                _ => (0.0, height),
            }
        }
    })
}

/// The axis label of an axis artist at `reference` (display pixels) in
/// direction `ref_angle`, `radius` pixels away (matplotlib's
/// `AxisArtist._update_label` and `AxisLabel.draw`).
pub(crate) fn axis_label(
    layout: LayoutContext<'_>,
    text: &str,
    (reference, ref_angle): (Point, f64),
    direction: SpineSide,
    radius: f64,
    style: &TextStyle,
    rc: &RcParams,
) -> Result<PlacedText, String> {
    Label {
        text,
        reference,
        ref_angle,
        radius,
        angle: axislabel_angle(direction),
        align: axislabel_alignment(direction),
    }
    .place(layout, style, rc)
}

/// The offset text of an axis artist in `direction` of the axes box
/// `extent`, `rise` points above its reference corner (matplotlib's
/// `AxisArtist._init_offsetText` and `_update_offsetText`).
pub(crate) fn offset_text(
    layout: LayoutContext<'_>,
    text: &str,
    extent: plt_core::Bbox,
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
    fn default_text_angles_follow_the_direction_table() {
        // matplotlib's `axis_direction` table: the ticklabel angles are
        // relative to 90 + the direction to the ticklabel.
        assert_eq!(ticklabel_angle(SpineSide::Left), 90.0);
        assert_eq!(ticklabel_angle(SpineSide::Right), -90.0);
        assert_eq!(axislabel_angle(SpineSide::Top), 180.0);
        assert_eq!(
            ticklabel_alignment(SpineSide::Bottom),
            (VAlign::Baseline, HAlign::Center)
        );
        assert_eq!(
            axislabel_alignment(SpineSide::Left),
            (VAlign::Bottom, HAlign::Center)
        );
    }
}
