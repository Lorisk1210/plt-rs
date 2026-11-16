//! The parts of an axis of a 3-D axes as `_native.axis` mirrors them (see
//! [`super::mirrors`]): its ticks, offset text and label.
//!
//! A 3-D axis draws every tick with the style of its level and what the
//! tick has of its own (its pad, the properties of its labels and the
//! colour of its mark); it places its texts and marks anew at every draw,
//! as matplotlib's `axis3d.Axis.draw` moves the artists of its ticks, its
//! offset text and its label. A mirror is filled with where the last
//! layout pass placed them (a tick out of view, or an axes with no place
//! in its figure, keeps matplotlib's `XTick` position), and what a program
//! sets on it that matplotlib's draw overwrites (positions, the width and
//! visibility of the marks) is not read back.

use pltrs::artist::ArtistHandle;
use pltrs::lines::{Line2D, LineStyle};
use pltrs::markers::{Marker, MarkerSymbol, NoMarker};
use pltrs::mplot3d::{Axis3D, AxisText3D, AxisTextPart};
use pltrs::text::{HAlign, Text, VAlign};
use pltrs::transforms::Bbox;
use pltrs_core::Point;

use pltrs::axes::{TickLineProps, TickMarkProps};

use super::mirrors::{TickMirrors, TickState, label_props, own};
use crate::binding::prelude::*;

/// The text `part` of `axis` as the axes last placed it.
fn drawn(axis: &Axis3D, part: AxisTextPart) -> Option<AxisText3D> {
    axis.texts().into_iter().find(|text| text.part == part)
}

/// The location and label of the major (or `minor`) tick at `index`
/// among those the locator places; a tick beyond them, or of an axis
/// whose ticks cannot be computed, sits at 0 without a label
/// (matplotlib's tick objects are made without asking the locator and
/// formatter, which place them when the axis is next drawn).
fn located(axis: &Axis3D, minor: bool, index: usize) -> (f64, String) {
    let Some(ticks) = axis.computed_ticks() else {
        return (0.0, String::new());
    };
    let unclipped = if minor {
        &ticks.minor_unclipped
    } else {
        &ticks.major_unclipped
    };
    unclipped
        .get(index)
        .map_or((0.0, String::new()), |tick| (tick.loc, tick.label.clone()))
}

/// Fills the mirrors of the major (or `minor`) tick at `index` of `axis`;
/// returns the rest of the tick's state.
pub(super) fn sync_tick(
    axis: &Axis3D,
    minor: bool,
    index: usize,
    (label1, label2, tick1, tick2, grid): TickMirrors,
) -> PyResult<TickState> {
    let (loc, label) = located(axis, minor, index);
    let position = drawn(
        axis,
        AxisTextPart::TickLabel {
            minor,
            index,
            second: false,
        },
    )
    .map_or(Point::new(loc, 0.0), |text| text.position);
    let rc = pltrs::rc::params();
    for (text, first_side) in [(&label1, true), (&label2, false)] {
        let style = axis.tick_style_at(minor, index, first_side);
        let visible = if first_side {
            style.labels.first
        } else {
            style.labels.second
        };
        text.set_text(label.clone())
            .set_position(position)
            .set_color(style.label_color.clone())
            .set_font(style.label_font(&rc))
            .set_rotation(style.label_rotation)
            .set_rotation_mode(style.label_rotation_mode)
            .set_halign(style.label_halign.unwrap_or(HAlign::Center))
            .set_valign(style.label_valign.unwrap_or(VAlign::Top));
        text.set_visible(visible);
    }
    let style = axis.tick_style_at(minor, index, true);
    // Marks only of ticks that can be computed: a tick object is made
    // without asking the locator, which raises nothing (nor warns).
    let marks = match axis.computed_ticks() {
        Some(_) => axis.ticklines(minor).unwrap_or_default(),
        None => Vec::new(),
    };
    let mark = marks.get(index);
    let (xs, ys) = mark
        .and_then(|mark| mark.marks.first())
        .map_or((Vec::new(), Vec::new()), |[a, b]| {
            (vec![a.x, b.x], vec![a.y, b.y])
        });
    let width = mark.map_or(style.width, |mark| mark.width);
    // matplotlib's `_tick_update_position`: the first mark drawn as a
    // line, the second hidden.
    for (line, visible) in [(&tick1, true), (&tick2, false)] {
        line.set_data(xs.clone(), ys.clone());
        line.set_color(style.color.clone())
            .set_linestyle(LineStyle::Solid)
            .set_linewidth(width)
            .set_marker(Marker::from(MarkerSymbol::Nothing(NoMarker::Empty)))
            .set_markersize(style.size)
            .set_markeredgewidth(style.width);
        line.set_visible(visible);
    }
    // The gridline collapses to the origin of the projection plane, the
    // 3-D gridlines being the axes' own.
    grid.set_data(vec![0.0], vec![0.0]);
    grid.set_color(style.grid.color.clone())
        .set_linewidth(style.grid.linewidth)
        .set_linestyle(style.grid.linestyle.clone())
        .set_marker(Marker::default());
    grid.set_alpha(Some(style.grid.alpha))
        .set_visible(style.grid_on);
    let rotation_mode = if style.label_rotation_auto {
        "auto"
    } else {
        "default"
    };
    Ok((
        Some(loc),
        style.size,
        style.width,
        axis.tick_pad(minor, index),
        style.direction.name(),
        (rotation_mode, style.label_rotation),
    ))
}

/// Reads the label mirror `text` on the first or `second` side of the
/// major (or `minor`) tick at `index` back into the axis.
pub(super) fn push_tick_label(
    axis: &Axis3D,
    minor: bool,
    index: usize,
    second: bool,
    text: &Text,
) -> PyResult<()> {
    let level = axis.tick_style(minor);
    let props = label_props(text, &level, (HAlign::Center, VAlign::Top), second);
    axis.set_tick_label_props_on(minor, index, !second, props);
    Ok(())
}

/// Reads the colour of the mark mirror `line` on the first side of the
/// major (or `minor`) tick at `index` back into the axis: a 3-D axis
/// draws the first mark of every tick, as wide as its level says, and
/// never the second (matplotlib's `_tick_update_position`).
pub(super) fn push_tick_mark(
    axis: &Axis3D,
    minor: bool,
    index: usize,
    second: bool,
    line: &Line2D,
) -> PyResult<()> {
    if !second {
        let level = axis.tick_style(minor);
        let previous = axis.tick_line_props(minor, index);
        let props = TickLineProps {
            marks: TickMarkProps {
                color: own(line.color(), &level.color),
                ..previous.marks.clone()
            },
            ..previous
        };
        axis.set_tick_line_props(minor, index, props);
    }
    Ok(())
}

/// Gives the major (or `minor`) tick at `index` its own label pad.
pub(super) fn push_tick_pad(axis: &Axis3D, minor: bool, index: usize, pad: f64) -> PyResult<()> {
    if minor {
        axis.set_minor_tick_pad(index, pad);
    } else {
        axis.set_major_tick_pad(index, pad);
    }
    Ok(())
}

/// The display box of the label on the first side of the major (or
/// `minor`) tick at `index`, if it is drawn (3-D axes draw no second
/// label).
pub(super) fn tick_label_extent(
    axis: &Axis3D,
    minor: bool,
    index: usize,
    second: bool,
) -> PyResult<Option<Bbox>> {
    if second {
        return Ok(None);
    }
    Ok(drawn(
        axis,
        AxisTextPart::TickLabel {
            minor,
            index,
            second: false,
        },
    )
    .map(|text| text.extent))
}

/// Fills the mirror `text` of the label of `axis`.
pub(super) fn sync_axis_label(axis: &Axis3D, text: &Text) -> PyResult<()> {
    let position = drawn(axis, AxisTextPart::Label).map_or(Point::new(0.5, 0.0), |t| t.position);
    text.set_text(axis.axes().label(axis.dir()))
        .set_position(position)
        .set_color(axis.label_color())
        .set_font(axis.label_font());
    Ok(())
}

/// Reads the text, colour and font of the mirror `text` of the label of
/// `axis` back, where they differ from those it was filled with; the axis
/// places, aligns and turns its label itself.
pub(super) fn push_axis_label(axis: &Axis3D, text: &Text) -> PyResult<()> {
    let synced = Text::loose(0.0, 0.0, "");
    sync_axis_label(axis, &synced)?;
    let label = text.text();
    if label != synced.text() {
        axis.axes().set_label(axis.dir(), label);
    }
    if text.color() != synced.color() {
        axis.set_label_color(text.color());
    }
    if text.font() != synced.font() {
        axis.set_label_font(text.font());
    }
    Ok(())
}

/// The display box of the label of `axis`, if it is drawn.
pub(super) fn axis_label_extent(axis: &Axis3D) -> PyResult<Option<Bbox>> {
    Ok(drawn(axis, AxisTextPart::Label).map(|text| text.extent))
}

/// Fills the mirror `text` of the offset text of `axis`.
pub(super) fn sync_offset_text(axis: &Axis3D, text: &Text) -> PyResult<()> {
    let position = drawn(axis, AxisTextPart::Offset).map_or(Point::new(1.0, 0.0), |t| t.position);
    let rc = pltrs::rc::params();
    text.set_text(axis.offset_text()?)
        .set_position(position)
        .set_color(axis.offset_text_color())
        .set_font(
            pltrs::fonts::FontProperties::from_rc(&rc).with_size(rc.xtick_labelsize, rc.font_size),
        );
    text.set_visible(axis.offset_text_visible());
    Ok(())
}

/// Reads whether the mirror `text` of the offset text of `axis` is shown
/// back into the axis.
pub(super) fn push_offset_text(axis: &Axis3D, text: &Text) -> PyResult<()> {
    let visible = text.visible();
    if visible != axis.offset_text_visible() {
        axis.set_offset_text_shown(Some(visible));
    }
    Ok(())
}

/// The display box of the offset text of `axis`, if it is drawn.
pub(super) fn offset_text_extent(axis: &Axis3D) -> PyResult<Option<Bbox>> {
    Ok(drawn(axis, AxisTextPart::Offset).map(|text| text.extent))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs::figure::Figure;
    use pltrs::lines::LineProps;

    /// Loose artists to mirror one tick.
    fn mirrors() -> TickMirrors {
        let line = || Line2D::loose(Vec::new(), Vec::new(), LineProps::default());
        (
            Text::loose(0.0, 0.0, ""),
            Text::loose(0.0, 0.0, ""),
            line(),
            line(),
            line(),
        )
    }

    #[test]
    fn ticks_mirror_their_level_and_keep_their_own_properties() {
        let ax = Figure::new().subplot_3d();
        let axis = ax.yaxis();
        let parts = mirrors();
        let (label1, _, tick1, tick2, _) = parts.clone();
        let state = sync_tick(&axis, false, 1, parts.clone()).unwrap();
        assert_eq!(state.0, Some(axis.ticklocs(false).unwrap()[1]));
        assert_eq!(label1.text(), axis.ticklabels(false).unwrap()[1]);
        assert!(tick1.visible() && !tick2.visible());
        assert_eq!(tick1.data().0.len(), 2, "the projected mark");
        let red = pltrs::colors::Color::from("r");
        let level = axis.tick_style(false);
        push_tick_mark(&axis, false, 1, false, &tick1).unwrap();
        assert_eq!(axis.tick_line_props(false, 1), TickLineProps::default());
        tick1.set_color(red.clone());
        push_tick_mark(&axis, false, 1, false, &tick1).unwrap();
        assert_eq!(axis.tick_style_at(false, 1, true).color, red);
        assert_eq!(axis.tick_style_at(false, 2, true).color, level.color);
        label1.set_color(red.clone());
        push_tick_label(&axis, false, 1, false, &label1).unwrap();
        assert_eq!(axis.tick_style_at(false, 1, true).label_color, red);
        sync_tick(&axis, false, 1, parts).unwrap();
        assert_eq!(label1.color(), red, "filled with the tick's own colour");
        push_tick_pad(&axis, false, 1, state.3 + 5.0).unwrap();
        assert_eq!(axis.tick_pad(false, 1), state.3 + 5.0);
        assert_eq!(axis.tick_pad(false, 2), state.3);
        push_tick_pad(&axis, true, 0, 99.0).unwrap();
        assert_eq!(axis.tick_pad(true, 0), 99.0);
    }

    #[test]
    fn the_label_and_offset_text_mirror_the_axis() {
        let ax = Figure::new().subplot_3d();
        ax.set_zlim(0.1, 0.100_000_001).unwrap();
        let axis = ax.zaxis();
        let offset = Text::loose(0.0, 0.0, "");
        sync_offset_text(&axis, &offset).unwrap();
        assert!(!offset.text().is_empty() && offset.visible());
        assert!(offset_text_extent(&axis).unwrap().is_some());
        offset.set_visible(false);
        push_offset_text(&axis, &offset).unwrap();
        assert!(!axis.offset_text_visible());
        let label = Text::loose(0.0, 0.0, "");
        sync_axis_label(&axis, &label).unwrap();
        label.set_text("depth");
        push_axis_label(&axis, &label).unwrap();
        assert_eq!(ax.label(axis.dir()), "depth");
        assert!(axis_label_extent(&axis).unwrap().is_some());
    }
}
