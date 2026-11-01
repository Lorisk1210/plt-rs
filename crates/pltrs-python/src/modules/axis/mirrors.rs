//! The texts and lines of an axis as artists (matplotlib's `Tick` with its
//! `label1`, `label2`, `tick1line`, `tick2line` and `gridline`, and the
//! axis' `label` and `offsetText`).
//!
//! pltrs (Rust) draws an axis from its state (tick styles, label and
//! offset text) rather than from artists of its own. Python gives each of
//! those texts and lines a *mirror*: a loose `Text` or `Line2D`, which a
//! `sync` function fills from the axis, so that every getter of
//! matplotlib's classes works on it, and which a `push` function reads
//! back into the axis after one of its setters ran. What a single tick
//! has of its own is what differs from the style of its level (see
//! [`pltrs::axes::Axis::set_tick_label_props`]).

use pltrs::artist::ArtistHandle;
use pltrs::axes::{
    Axis, AxisGroup, LabelLoc, TickDirection, TickLabelProps, TickLineProps, TickPart, TickStyle,
};
use pltrs::lines::Line2D;
use pltrs::markers::{Marker, MarkerSymbol};
use pltrs::rc::ColorOrAuto;
use pltrs::text::{HAlign, Text, VAlign};
use pltrs::ticker::AxisName;
use pltrs::transforms::Bbox;
use pltrs_core::Point;

use super::deep;
use super::owner::{AnyAxis, AxisOwner};
use crate::binding::prelude::*;

/// The location and label of the major (or `minor`) tick at `index`
/// among those the locator places; a tick beyond them, or of an axis
/// whose ticks were never placed, sits at 0 without a label (matplotlib's
/// first tick of an empty axis, and its `Tick` objects before
/// `_update_ticks` runs).
fn located(axis: &Axis, minor: bool, index: usize) -> PyResult<(f64, String)> {
    if !axis.ticks_placed() {
        return Ok((0.0, String::new()));
    }
    let ticks = axis.axes().ticks(axis.name())?;
    let unclipped = if minor {
        &ticks.minor_unclipped
    } else {
        &ticks.major_unclipped
    };
    Ok(unclipped
        .get(index)
        .map_or((0.0, String::new()), |tick| (tick.loc, tick.label.clone())))
}

/// matplotlib's alignment of the label on the first or the second side of
/// an axis (`get_xaxis_text1_transform`, ...): `(halign, valign)`.
fn default_alignment(name: AxisName, first_side: bool) -> (HAlign, VAlign) {
    match (name, first_side) {
        (AxisName::X, true) => (HAlign::Center, VAlign::Top),
        (AxisName::X, false) => (HAlign::Center, VAlign::Bottom),
        (AxisName::Y, true) => (HAlign::Right, VAlign::CenterBaseline),
        (AxisName::Y, false) => (HAlign::Left, VAlign::CenterBaseline),
    }
}

/// A point at `loc` along axis `name`, on its first or second side
/// (data coordinates along the axis, axes fractions across it).
fn on_side(name: AxisName, loc: f64, first_side: bool) -> Point {
    let across = if first_side { 0.0 } else { 1.0 };
    match name {
        AxisName::X => Point::new(loc, across),
        AxisName::Y => Point::new(across, loc),
    }
}

/// Fills the label `text` on the first or second side of a tick of
/// `axis` at `loc` labelled `label` and styled `style`.
fn sync_label(
    text: &Text,
    axis: &Axis,
    loc: f64,
    label: String,
    style: &TickStyle,
    first_side: bool,
) {
    let name = axis.name();
    let (halign, valign) = default_alignment(name, first_side);
    text.set_usetex(axis.usetex());
    text.set_text(label)
        .set_position(on_side(name, loc, first_side))
        .set_color(style.label_color.clone())
        .set_font(style.label_font(&pltrs::rc::params()))
        .set_rotation(style.label_rotation)
        .set_rotation_mode(style.label_rotation_mode)
        .set_halign(style.label_halign.unwrap_or(halign))
        .set_valign(style.label_valign.unwrap_or(valign))
        .set_multialign(style.label_multialign);
    text.set_visible(if first_side {
        style.labels.first
    } else {
        style.labels.second
    })
    .set_in_layout(style.label_in_layout);
    match &style.label_url {
        Some(url) => text.set_url(url.clone()),
        None => text.clear_url(),
    };
}

/// The marker of a tick mark pointing `direction` on the first or the
/// second side of axis `name` (matplotlib's `_apply_tickdir`).
fn tick_marker(name: AxisName, direction: TickDirection, first_side: bool) -> Marker {
    use MarkerSymbol as M;
    let outward = match direction {
        TickDirection::InOut => {
            return Marker::from(match name {
                AxisName::X => M::VLine,
                AxisName::Y => M::HLine,
            });
        }
        TickDirection::Out => first_side,
        TickDirection::In => !first_side,
    };
    Marker::from(match (name, outward) {
        (AxisName::X, true) => M::TickDown,
        (AxisName::X, false) => M::TickUp,
        (AxisName::Y, true) => M::TickLeft,
        (AxisName::Y, false) => M::TickRight,
    })
}

/// Fills the mark `line` on the first or second side of a tick at `loc`
/// styled `style`.
fn sync_mark(line: &Line2D, name: AxisName, loc: f64, style: &TickStyle, first_side: bool) {
    let at = on_side(name, loc, first_side);
    let edge = style
        .edgecolor
        .clone()
        .unwrap_or_else(|| style.color.clone());
    line.set_data(vec![at.x], vec![at.y]);
    line.set_color(style.color.clone())
        .set_linestyle(pltrs::lines::LineStyle::None)
        .set_marker(tick_marker(name, style.direction, first_side))
        .set_markersize(style.size)
        .set_markeredgewidth(style.width)
        .set_markeredgecolor(ColorOrAuto::Color(edge));
    line.set_visible(if first_side {
        style.ticks.first
    } else {
        style.ticks.second
    });
}

/// Fills the gridline `line` of a tick at `loc` styled `style`.
fn sync_gridline(line: &Line2D, name: AxisName, loc: f64, style: &TickStyle) {
    let (start, end) = (on_side(name, loc, true), on_side(name, loc, false));
    line.set_data(vec![start.x, end.x], vec![start.y, end.y]);
    line.set_color(style.grid.color.clone())
        .set_linewidth(style.grid.linewidth)
        .set_linestyle(style.grid.linestyle.clone())
        .set_marker(Marker::default());
    line.set_alpha(Some(style.grid.alpha))
        .set_visible(style.grid_on);
}

/// The artists mirroring one tick: its two labels, two marks and
/// gridline.
pub(super) type TickMirrors = (Text, Text, Line2D, Line2D, Line2D);

/// What a tick keeps besides its artists: its location, mark length and
/// width, label pad, direction and label rotation (`"auto"` or
/// `"default"`, and an angle).
pub(super) type TickState = (f64, f64, f64, f64, &'static str, (&'static str, f64));

/// Fills the mirrors of the major (or `minor`) tick at `index` of axis
/// `name` from the axis; returns the rest of the tick's state.
fn sync_tick(
    owner: AxisOwner,
    name: String,
    minor: bool,
    index: usize,
    mirrors: TickMirrors,
) -> PyResult<TickState> {
    let axis = owner.axis(&name)?;
    sync_part_gids(&axis, minor, index, &mirrors);
    match axis {
        AnyAxis::Flat(axis) => sync_flat_tick(&axis, minor, index, mirrors),
        AnyAxis::Deep(axis) => deep::sync_tick(&axis, minor, index, mirrors),
    }
}

/// Gives each mirror of the major (or `minor`) tick at `index` the gid
/// of its part.
fn sync_part_gids(
    axis: &AnyAxis,
    minor: bool,
    index: usize,
    (label1, label2, tick1, tick2, grid): &TickMirrors,
) {
    let mirrors: [(&dyn ArtistHandle, TickPart); 5] = [
        (label1, TickPart::Label1),
        (label2, TickPart::Label2),
        (tick1, TickPart::Tick1Line),
        (tick2, TickPart::Tick2Line),
        (grid, TickPart::Gridline),
    ];
    for (mirror, part) in mirrors {
        set_mirror_gid(mirror, axis.gid(AxisGroup::TickPart { minor, index, part }));
    }
}

/// Gives the mirror `artist` the gid `gid`, or takes its gid away.
fn set_mirror_gid(artist: &dyn ArtistHandle, gid: Option<String>) {
    match gid {
        Some(gid) => artist.raw().set_gid(gid),
        None => artist.raw().clear_gid(),
    };
}

/// Reads the gid of the mirror `artist` of `part` of the major (or
/// `minor`) tick at `index` back into the axis.
fn push_part_gid(
    axis: &AnyAxis,
    minor: bool,
    index: usize,
    part: TickPart,
    artist: &dyn ArtistHandle,
) {
    axis.set_gid(
        AxisGroup::TickPart { minor, index, part },
        artist.raw().gid(),
    );
}

/// [`sync_tick`] on an axis of a 2-D axes.
fn sync_flat_tick(
    axis: &Axis,
    minor: bool,
    index: usize,
    (label1, label2, tick1, tick2, grid): TickMirrors,
) -> PyResult<TickState> {
    let (loc, label) = located(axis, minor, index)?;
    let style = axis.tick_style_at(minor, index);
    let name = axis.name();
    sync_label(&label1, axis, loc, label.clone(), &style, true);
    sync_label(&label2, axis, loc, label, &style, false);
    sync_mark(&tick1, name, loc, &style, true);
    sync_mark(&tick2, name, loc, &style, false);
    sync_gridline(&grid, name, loc, &style);
    let rotation_mode = if style.label_rotation_auto {
        "auto"
    } else {
        "default"
    };
    Ok((
        loc,
        style.size,
        style.width,
        style.pad,
        style.direction.name(),
        (rotation_mode, style.label_rotation),
    ))
}

/// `value` when it differs from the style's `current`, else nothing (a
/// tick keeps of its own only what differs from its level).
fn own<T: PartialEq>(value: T, current: &T) -> Option<T> {
    (value != *current).then_some(value)
}

/// Reads the label mirror `text` on the first or `second` side of the
/// major (or `minor`) tick at `index` back into the axis.
fn push_tick_label(
    owner: AxisOwner,
    name: String,
    minor: bool,
    index: usize,
    second: bool,
    text: Text,
) -> PyResult<()> {
    let axis = owner.axis(&name)?;
    push_part_gid(&axis, minor, index, TickPart::label(second), &text);
    let axis = match axis {
        AnyAxis::Flat(axis) => axis,
        AnyAxis::Deep(axis) => return deep::push_tick_label(&axis, minor, second, &text),
    };
    let level = axis.tick_style(minor);
    let rc = pltrs::rc::params();
    let level_font = level.label_font(&rc);
    let font = text.font();
    let (halign, valign) = default_alignment(axis.name(), !second);
    let visible = text.visible();
    let previous = axis.tick_label_props(minor, index);
    let (label1_visible, label2_visible) = if second {
        (previous.label1_visible, own(visible, &level.labels.second))
    } else {
        (own(visible, &level.labels.first), previous.label2_visible)
    };
    let props = TickLabelProps {
        rotation: own(text.rotation(), &level.label_rotation),
        rotation_mode: own(text.rotation_mode(), &level.label_rotation_mode),
        halign: own(text.halign(), &level.label_halign.unwrap_or(halign)),
        valign: own(text.valign(), &level.label_valign.unwrap_or(valign)),
        multialign: own(text.multialign(), &level.label_multialign).flatten(),
        color: own(text.color(), &level.label_color),
        fontsize: own(font.size, &level_font.size).map(pltrs::fonts::FontSize::Points),
        fontweight: own(font.weight, &level_font.weight),
        fontstyle: own(font.style, &level_font.style),
        in_layout: own(text.in_layout(), &level.label_in_layout),
        url: own(text.url(), &level.label_url).flatten(),
        family: own(font.families.clone(), &level_font.families)
            .and_then(|families| families.into_iter().next()),
        label1_visible,
        label2_visible,
    };
    axis.set_tick_label_props(minor, index, props);
    Ok(())
}

/// Reads the mark mirror `line` on the first or `second` side of the
/// major (or `minor`) tick at `index` back into the axis.
fn push_tick_mark(
    owner: AxisOwner,
    name: String,
    minor: bool,
    index: usize,
    second: bool,
    line: Line2D,
) -> PyResult<()> {
    let axis = owner.axis(&name)?;
    push_part_gid(&axis, minor, index, TickPart::mark(second), &line);
    let axis = match axis {
        AnyAxis::Flat(axis) => axis,
        AnyAxis::Deep(axis) => return deep::push_tick_mark(&axis, minor, second, &line),
    };
    let level = axis.tick_style(minor);
    let previous = axis.tick_line_props(minor, index);
    let visible = line.visible();
    let (tick1_visible, tick2_visible) = if second {
        (previous.tick1_visible, own(visible, &level.ticks.second))
    } else {
        (own(visible, &level.ticks.first), previous.tick2_visible)
    };
    let edge = level
        .edgecolor
        .clone()
        .unwrap_or_else(|| level.color.clone());
    let props = TickLineProps {
        markeredgecolor: own(line.effective_markeredgecolor(), &edge),
        color: own(line.color(), &level.color),
        size: own(line.markersize(), &level.size),
        width: own(line.markeredgewidth(), &level.width),
        tick1_visible,
        tick2_visible,
        ..previous
    };
    axis.set_tick_line_props(minor, index, props);
    Ok(())
}

/// Reads the gridline mirror `line` of the major (or `minor`) tick at
/// `index` back into the axis.
fn push_gridline(
    owner: AxisOwner,
    name: String,
    minor: bool,
    index: usize,
    line: Line2D,
) -> PyResult<()> {
    let axis = owner.axis(&name)?;
    push_part_gid(&axis, minor, index, TickPart::Gridline, &line);
    let axis = match axis {
        AnyAxis::Flat(axis) => axis,
        AnyAxis::Deep(axis) => return deep::push_gridline(&axis, minor, &line),
    };
    let level = axis.tick_style(minor);
    let props = TickLineProps {
        grid_visible: own(line.visible(), &level.grid_on),
        grid_color: own(line.color(), &level.grid.color),
        grid_linewidth: own(line.linewidth(), &level.grid.linewidth),
        grid_linestyle: own(line.linestyle(), &level.grid.linestyle),
        grid_alpha: own(line.alpha().unwrap_or(1.0), &level.grid.alpha),
        ..axis.tick_line_props(minor, index)
    };
    axis.set_tick_line_props(minor, index, props);
    Ok(())
}

/// Gives the major (or `minor`) tick at `index` the label pad `pad`, in
/// points (`Tick.set_pad`); a minor tick of a 3-D axis only keeps the pad
/// of its level.
fn push_tick_pad(
    owner: AxisOwner,
    name: String,
    minor: bool,
    index: usize,
    pad: f64,
) -> PyResult<()> {
    match owner.axis(&name)? {
        AnyAxis::Deep(axis) => deep::push_tick_pad(&axis, minor, index, pad),
        AnyAxis::Flat(axis) => {
            if own(pad, &axis.tick_style_at(minor, index).pad).is_some() {
                axis.set_tick_pad(minor, index, pad);
            }
            Ok(())
        }
    }
}

/// The display box of the label on the first or `second` side of the
/// major (or `minor`) tick at `index`, if it is drawn.
fn tick_label_extent(
    owner: AxisOwner,
    name: String,
    minor: bool,
    index: usize,
    second: bool,
) -> PyResult<Option<(f64, f64, f64, f64)>> {
    let extent = match owner.axis(&name)? {
        AnyAxis::Flat(axis) => axis.tick_label_extent(minor, index, !second),
        AnyAxis::Deep(axis) => deep::tick_label_extent(&axis, minor, index, second)?,
    };
    Ok(extent.map(extents))
}

/// `(x0, y0, x1, y1)` of `bbox`.
fn extents(bbox: Bbox) -> (f64, f64, f64, f64) {
    (bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1())
}

/// Fills the mirror `text` of the label of axis `name` from the axis.
fn sync_axis_label(owner: AxisOwner, name: String, text: Text) -> PyResult<()> {
    let axis = owner.axis(&name)?;
    set_mirror_gid(&text, axis.gid(AxisGroup::Label));
    let axis = match axis {
        AnyAxis::Flat(axis) => axis,
        AnyAxis::Deep(axis) => return deep::sync_axis_label(&axis, &text),
    };
    let along = axis.label_loc().fraction();
    let position = axis.label_coords().unwrap_or(match axis.name() {
        AxisName::X => Point::new(along, 0.0),
        AxisName::Y => Point::new(0.0, along),
    });
    text.set_text(axis.label_text())
        .set_position(position)
        .set_color(axis.label_color())
        .set_font(axis.label_font())
        .set_rotation(axis.label_rotation())
        .set_halign(axis.label_halign())
        .set_valign(axis.label_valign())
        .set_multialign(axis.label_multialign());
    text.set_visible(axis.label_visible());
    Ok(())
}

/// Where along its axis a label at the fraction `along` sits, if at one
/// of matplotlib's `loc`s.
fn label_loc(along: f64) -> Option<LabelLoc> {
    [LabelLoc::Start, LabelLoc::Center, LabelLoc::End]
        .into_iter()
        .find(|loc| loc.fraction().total_cmp(&along).is_eq())
}

/// Reads the mirror `text` of the label of axis `name` back into the axis.
fn push_axis_label(owner: AxisOwner, name: String, text: Text) -> PyResult<()> {
    let axis = owner.axis(&name)?;
    axis.set_gid(AxisGroup::Label, text.gid());
    let axis = match axis {
        AnyAxis::Flat(axis) => axis,
        AnyAxis::Deep(axis) => return deep::push_axis_label(&axis, &text),
    };
    let position = text.position();
    let along = match axis.name() {
        AxisName::X => position.x,
        AxisName::Y => position.y,
    };
    if let Some(loc) = label_loc(along).filter(|&loc| loc != axis.label_loc()) {
        axis.set_label_loc(loc);
    }
    if let Some(label) = own(text.text(), &axis.label_text()) {
        axis.set_label_text(label);
    }
    if let Some(rotation) = own(text.rotation(), &axis.label_rotation()) {
        axis.set_label_rotation(rotation);
    }
    if let Some(halign) = own(text.halign(), &axis.label_halign()) {
        axis.set_label_halign(halign);
    }
    if let Some(valign) = own(text.valign(), &axis.label_valign()) {
        axis.set_label_valign(valign);
    }
    axis.set_label_color(text.color())
        .set_label_font(text.font())
        .set_label_multialign(text.multialign())
        .set_label_visible(text.visible());
    Ok(())
}

/// The display box of the label of axis `name`, if it is drawn.
fn axis_label_extent(owner: AxisOwner, name: String) -> PyResult<Option<(f64, f64, f64, f64)>> {
    let extent = match owner.axis(&name)? {
        AnyAxis::Flat(axis) => axis.label_extent(),
        AnyAxis::Deep(axis) => deep::axis_label_extent(&axis)?,
    };
    Ok(extent.map(extents))
}

/// Fills the mirror `text` of the offset text of axis `name` from the
/// axis.
fn sync_offset_text(owner: AxisOwner, name: String, text: Text) -> PyResult<()> {
    let axis = owner.axis(&name)?;
    set_mirror_gid(&text, axis.gid(AxisGroup::OffsetText));
    let axis = match axis {
        AnyAxis::Flat(axis) => axis,
        AnyAxis::Deep(axis) => return deep::sync_offset_text(&axis, &text),
    };
    let style = axis.tick_style(false);
    let high = axis.offset_position().is_high();
    // At an end of the axis in axes fractions, beyond the tick labels in
    // display pixels, as matplotlib places the offset text.
    let across = axis.offset_text_anchor().map_or(0.0, |anchor| anchor.y);
    let (position, halign, valign) = match (axis.name(), high) {
        (AxisName::X, false) => (Point::new(1.0, across), HAlign::Right, VAlign::Top),
        (AxisName::X, true) => (Point::new(1.0, across), HAlign::Right, VAlign::Bottom),
        (AxisName::Y, false) => (Point::new(0.0, across), HAlign::Left, VAlign::Baseline),
        (AxisName::Y, true) => (Point::new(1.0, across), HAlign::Right, VAlign::Baseline),
    };
    let rc = pltrs::rc::params();
    text.set_text(axis.offset_text()?)
        .set_position(position)
        .set_color(style.label_color.clone())
        .set_font(pltrs::fonts::FontProperties {
            size: style.label_size,
            ..pltrs::fonts::FontProperties::from_rc(&rc)
        })
        .set_halign(halign)
        .set_valign(valign);
    text.set_visible(axis.offset_text_visible());
    Ok(())
}

/// Reads the visibility of the mirror `text` of the offset text of axis
/// `name` back into the axis.
fn push_offset_text(owner: AxisOwner, name: String, text: Text) -> PyResult<()> {
    let axis = owner.axis(&name)?;
    axis.set_gid(AxisGroup::OffsetText, text.gid());
    match axis {
        AnyAxis::Flat(axis) => {
            axis.set_offset_text_visible(text.visible());
            Ok(())
        }
        AnyAxis::Deep(axis) => deep::push_offset_text(&axis, &text),
    }
}

/// The display box of the offset text of axis `name`, if it is drawn.
fn offset_text_extent(owner: AxisOwner, name: String) -> PyResult<Option<(f64, f64, f64, f64)>> {
    let extent = match owner.axis(&name)? {
        AnyAxis::Flat(axis) => axis.offset_text_extent(),
        AnyAxis::Deep(axis) => deep::offset_text_extent(&axis)?,
    };
    Ok(extent.map(extents))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("sync_tick", sync_tick)?;
    module.add("push_tick_label", push_tick_label)?;
    module.add("push_tick_mark", push_tick_mark)?;
    module.add("push_gridline", push_gridline)?;
    module.add("push_tick_pad", push_tick_pad)?;
    module.add("tick_label_extent", tick_label_extent)?;
    module.add("sync_axis_label", sync_axis_label)?;
    module.add("push_axis_label", push_axis_label)?;
    module.add("axis_label_extent", axis_label_extent)?;
    module.add("sync_offset_text", sync_offset_text)?;
    module.add("push_offset_text", push_offset_text)?;
    module.add("offset_text_extent", offset_text_extent)
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
    fn tick_mirrors_follow_the_axis_and_push_back_what_changed() {
        let ax = Figure::new().subplot();
        ax.set_xticks([0.0, 0.5, 1.0]);
        let parts = mirrors();
        let (label1, label2, tick1, _, grid) = parts.clone();
        let unplaced = sync_tick(AxisOwner::Flat(ax.clone()), "x".into(), false, 1, mirrors());
        assert_eq!(
            unplaced.unwrap().0,
            0.0,
            "matplotlib's ticks sit at 0 until placed"
        );
        ax.xaxis().place_ticks();
        let state = sync_tick(AxisOwner::Flat(ax.clone()), "x".into(), false, 1, parts).unwrap();
        assert_eq!(state.0, 0.5);
        assert_eq!(label1.text(), "0.5");
        assert_eq!(label1.position(), Point::new(0.5, 0.0));
        assert_eq!(
            (label1.halign(), label1.valign()),
            (HAlign::Center, VAlign::Top)
        );
        let level = ax.xaxis().tick_style(false);
        assert_eq!(
            (label1.visible(), label2.visible()),
            (level.labels.first, level.labels.second)
        );
        assert_eq!(
            (tick1.visible(), grid.visible()),
            (level.ticks.first, level.grid_on)
        );
        label1.set_rotation(45.0);
        label1.set_visible(false);
        push_tick_label(
            AxisOwner::Flat(ax.clone()),
            "x".into(),
            false,
            1,
            false,
            label1,
        )
        .unwrap();
        let style = ax.xaxis().tick_style_at(false, 1);
        assert_eq!(style.label_rotation, 45.0);
        assert!(!style.labels.first);
        assert_eq!(ax.xaxis().tick_style_at(false, 0).label_rotation, 0.0);
        grid.set_visible(!level.grid_on);
        push_gridline(AxisOwner::Flat(ax.clone()), "x".into(), false, 2, grid).unwrap();
        assert_eq!(ax.xaxis().tick_style_at(false, 2).grid_on, !level.grid_on);
        assert_eq!(ax.xaxis().tick_style_at(false, 1).grid_on, level.grid_on);
    }

    #[test]
    fn gids_of_tick_parts_and_axis_texts_go_through_their_mirrors() {
        let ax = Figure::new().subplot();
        let owner = || AxisOwner::Flat(ax.clone());
        let (label1, _, _, tick2, _) = mirrors();
        label1.set_gid("first-label");
        push_tick_label(owner(), "x".into(), false, 2, false, label1).unwrap();
        tick2.set_gid("second-mark");
        push_tick_mark(owner(), "x".into(), true, 0, true, tick2).unwrap();
        let gid = |minor, index, part| ax.xaxis().gid(AxisGroup::TickPart { minor, index, part });
        assert_eq!(
            gid(false, 2, TickPart::Label1).as_deref(),
            Some("first-label")
        );
        assert_eq!(
            gid(true, 0, TickPart::Tick2Line).as_deref(),
            Some("second-mark")
        );
        let parts = mirrors();
        sync_tick(owner(), "x".into(), false, 2, parts.clone()).unwrap();
        assert_eq!(parts.0.gid().as_deref(), Some("first-label"));
        assert_eq!(parts.1.gid(), None);
        let label = Text::loose(0.0, 0.0, "");
        label.set_gid("xlabel");
        push_axis_label(owner(), "x".into(), label).unwrap();
        assert_eq!(ax.xaxis().gid(AxisGroup::Label).as_deref(), Some("xlabel"));
    }

    #[test]
    fn axis_texts_mirror_the_label_and_the_offset() {
        let ax = Figure::new().subplot();
        ax.set_ylabel("height");
        let text = Text::loose(0.0, 0.0, "");
        sync_axis_label(AxisOwner::Flat(ax.clone()), "y".into(), text.clone()).unwrap();
        assert_eq!(text.text(), "height");
        assert_eq!(text.rotation(), 90.0);
        text.set_position(Point::new(0.0, 1.0));
        push_axis_label(AxisOwner::Flat(ax.clone()), "y".into(), text).unwrap();
        assert_eq!(ax.yaxis().label_loc(), LabelLoc::End);
        let offset = Text::loose(0.0, 0.0, "");
        sync_offset_text(AxisOwner::Flat(ax.clone()), "x".into(), offset.clone()).unwrap();
        assert_eq!(offset.text(), "");
        offset.set_visible(false);
        push_offset_text(AxisOwner::Flat(ax.clone()), "x".into(), offset).unwrap();
        assert!(!ax.xaxis().offset_text_visible());
    }

    #[test]
    fn marks_point_out_of_the_axes_by_default() {
        assert_eq!(
            tick_marker(AxisName::X, TickDirection::Out, true),
            Marker::from(MarkerSymbol::TickDown)
        );
        assert_eq!(
            tick_marker(AxisName::Y, TickDirection::In, true),
            Marker::from(MarkerSymbol::TickRight)
        );
        assert_eq!(
            tick_marker(AxisName::Y, TickDirection::InOut, false),
            Marker::from(MarkerSymbol::HLine)
        );
        assert_eq!(label_loc(0.5), Some(LabelLoc::Center));
        assert_eq!(label_loc(0.3), None);
    }
}
