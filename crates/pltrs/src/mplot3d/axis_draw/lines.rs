//! The tick marks and gridlines of a 3-D axis handed out as values,
//! computed as they are drawn (matplotlib's `get_ticklines` and
//! `get_gridlines` on the axes of 3-D axes, `Axes3D.get_zticklines` and
//! `get_zgridlines`).
//!
//! matplotlib hands out the `Line2D`s of its tick objects, which it moves
//! at every draw; the values here describe what the last layout pass
//! placed, in the projected coordinates the axes draw in, and change with
//! the view as those lines do.

use pltrs_core::{Bbox, Point};

use super::Painter;
use crate::artist::LayoutContext;
use crate::colors::Color;
use crate::lines::LineStyle;
use crate::mplot3d::coords::Axis3;
use crate::mplot3d::state::Axes3DState;

/// The tick mark of a tick of a 3-D axis (matplotlib's `tick1line` of an
/// `axis3d.Axis` tick, which 3-D axes always draw).
#[derive(Clone, Debug, PartialEq)]
pub struct TickLine3D {
    /// The tick's location along the axis, in data coordinates.
    pub loc: f64,
    /// Whether the tick is in view; the marks are drawn only then.
    pub in_view: bool,
    /// The mark on each edge carrying the ticks (see
    /// [`Axis3D::set_ticks_position`](crate::mplot3d::Axis3D::set_ticks_position)),
    /// from outside the box inwards, in projected coordinates
    /// (matplotlib's `tick1line.get_data()` after a draw).
    pub marks: Vec<[Point; 2]>,
    /// Width in points.
    pub width: f64,
    /// Colour.
    pub color: Color,
}

/// The gridline of a major tick of a 3-D axis: where [`Axes3D::grid`]
/// draws it across the two panes meeting along the far edge
/// (matplotlib's `draw_grid`; the `gridline` of matplotlib's tick objects,
/// which `get_gridlines` returns, is collapsed to a point on 3-D axes).
///
/// [`Axes3D::grid`]: crate::mplot3d::Axes3D::grid
#[derive(Clone, Debug, PartialEq)]
pub struct GridLine3D {
    /// The tick's location along the axis, in data coordinates.
    pub loc: f64,
    /// Whether the tick is in view; a visible gridline is drawn only then.
    pub in_view: bool,
    /// Whether the axes draw gridlines.
    pub visible: bool,
    /// The line from the near edge of one pane through the far edge to
    /// the near edge of the other, in projected coordinates.
    pub points: [Point; 3],
    /// Colour.
    pub color: Color,
    /// Width in points.
    pub width: f64,
    /// Line style.
    pub linestyle: LineStyle,
}

/// The tick marks of every major (or minor) tick of axis `dir` of `axes`,
/// spanning `extent` on the display, in view or not, as the last layout
/// pass placed them; none when the ticks could not be computed or the
/// projection plane is gone.
pub(crate) fn tick_lines(
    axes: &Axes3DState,
    layout: LayoutContext<'_>,
    extent: &Bbox,
    dir: Axis3,
    minor: bool,
) -> Vec<TickLine3D> {
    let axis = axes.axis(dir);
    let (Some(painter), Ok(ticks)) = (Painter::new(axes, layout, extent), &axis.ticks) else {
        return Vec::new();
    };
    let (located, in_view, style, width) = if minor {
        let width = axis.style.minor_tick_width;
        (
            &ticks.minor_unclipped,
            &ticks.minor_index,
            &axis.minor_style,
            width,
        )
    } else {
        let width = axis.style.major_tick_width;
        (
            &ticks.major_unclipped,
            &ticks.major_index,
            &axis.major_style,
            width,
        )
    };
    let spans: Vec<_> = (axis.tick_position.edges().iter())
        .map(|&position| painter.mark_span(dir, position, painter.edge(dir, position).points[0]))
        .collect();
    (located.iter().enumerate())
        .map(|(index, tick)| TickLine3D {
            loc: tick.loc,
            in_view: in_view.contains(&index),
            marks: (spans.iter())
                .map(|span| painter.mark(span, axis, tick.loc))
                .collect(),
            width,
            color: style.color.clone(),
        })
        .collect()
}

/// The gridlines of every major tick of axis `dir` of `axes`, spanning
/// `extent` on the display, in view or not, as the last layout pass
/// placed them; none when the ticks could not be computed or the
/// projection plane is gone.
pub(crate) fn grid_lines(
    axes: &Axes3DState,
    layout: LayoutContext<'_>,
    extent: &Bbox,
    dir: Axis3,
) -> Vec<GridLine3D> {
    let axis = axes.axis(dir);
    let (Some(painter), Ok(ticks)) = (Painter::new(axes, layout, extent), &axis.ticks) else {
        return Vec::new();
    };
    let locs: Vec<f64> = ticks.major_unclipped.iter().map(|tick| tick.loc).collect();
    (locs.iter().enumerate())
        .zip(painter.gridlines(axis, &locs))
        .map(|((index, &loc), points)| GridLine3D {
            loc,
            in_view: ticks.major_index.contains(&index),
            visible: axes.grid_on,
            points,
            color: axis.style.grid_color.clone(),
            width: axis.style.grid_width,
            linestyle: axis.style.grid_style.clone(),
        })
        .collect()
}
