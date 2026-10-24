//! Where an axis artist runs and where its ticks and label go: the axis
//! artist helpers of matplotlib's rectilinear grid
//! (`FixedAxisArtistHelperRectilinear`,
//! `FloatingAxisArtistHelperRectilinear`), of curvilinear grids
//! (`grid_helper_curvelinear`) and of floating axes (`floating_axes`).

use std::rc::Rc;

use plt_core::{Bbox, Path, Point};

use super::floating_axes::{self, FloatingGridInfo};
use super::grid_helper_curvelinear::{
    self as curvelinear, AxisGrid, FloatingLine, GridHelperCurveLinear, TickSpec,
};
use crate::artist::{ArtistId, LayoutContext};
use crate::axes::{AxesState, Tick};
use crate::spines::SpineSide;
use crate::ticker::AxisName;
use crate::transforms::{ResolvedTransform, Transform};

/// The relative tolerance of matplotlib's `_interval_contains_close`.
const CLOSE: f64 = 1e-10;

/// The problem reported when the axes of an axis artist was removed.
pub(crate) const GONE: &str = "the axes of the axis artist is gone";

/// The line an axis artist draws along and how it is ticked.
#[derive(Clone, Debug)]
pub(crate) enum AxisHelper {
    /// An edge of the axes, ticked by the axes' own axis (matplotlib's
    /// `FixedAxisArtistHelperRectilinear`).
    Fixed(SpineSide),
    /// Where the data coordinate across `nth` equals `value`: `nth = X`
    /// runs horizontally at `y = value`, `nth = Y` vertically at
    /// `x = value` (matplotlib's `FloatingAxisArtistHelperRectilinear`).
    Floating {
        /// The coordinate varying along the line (and ticked).
        nth: AxisName,
        /// The constant other coordinate, in data units.
        value: f64,
    },
    /// An edge of the axes ticked where the lines of a curvilinear grid
    /// cross it (matplotlib's `grid_helper_curvelinear
    /// .FixedAxisArtistHelper`).
    CurveFixed {
        /// The grid.
        grid: Rc<GridHelperCurveLinear>,
        /// The edge.
        side: SpineSide,
        /// The curvilinear coordinate whose ticks are labelled.
        nth_ticks: AxisName,
    },
    /// A line of constant curvilinear coordinate (matplotlib's
    /// `FloatingAxisArtistHelper`).
    CurveFloating {
        /// The grid.
        grid: Rc<GridHelperCurveLinear>,
        /// The line.
        line: FloatingLine,
    },
    /// An edge of the curvilinear extremes of floating axes (matplotlib's
    /// `floating_axes.FixedAxisArtistHelper`).
    FloatingFixed {
        /// The grid.
        grid: Rc<floating_axes::GridHelperCurveLinear>,
        /// The edge of the extremes.
        side: SpineSide,
    },
}

/// A tick of an axis artist: its location in the tick coordinates, the
/// angles (degrees) normal and tangent to the line, and its label.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TickInfo {
    pub(crate) loc: Point,
    pub(crate) angle_normal: f64,
    pub(crate) angle_tangent: f64,
    pub(crate) label: String,
}

/// The major and minor ticks of an axis artist.
pub(crate) type TickLevels = (Vec<TickInfo>, Vec<TickInfo>);

/// What helpers consult: the figure geometry and the axes.
#[derive(Clone, Copy)]
pub(crate) struct HelperContext<'a> {
    pub(crate) layout: LayoutContext<'a>,
    pub(crate) axes: &'a AxesState,
    pub(crate) id: ArtistId,
}

impl HelperContext<'_> {
    fn resolve(&self, transform: &Transform) -> Result<ResolvedTransform, String> {
        self.layout
            .resolve(transform)
            .ok_or_else(|| GONE.to_owned())
    }

    /// Display pixels to axes fractions.
    fn display_to_axes(&self) -> Result<ResolvedTransform, String> {
        self.resolve(&Transform::Axes(self.id).inverted())
    }

    /// Curvilinear coordinates of `grid` to display pixels.
    fn grid_to_display(&self, grid: &GridHelperCurveLinear) -> Result<ResolvedTransform, String> {
        let data = self.resolve(&Transform::Data(self.id))?;
        Ok(grid.grid_finder().transform().resolved().then(data))
    }
}

impl AxisHelper {
    /// The line and its coordinates (matplotlib's `get_line` and
    /// `get_line_transform`).
    pub(crate) fn line(&self, ctx: HelperContext<'_>) -> Result<(Path, Transform), String> {
        let along = |nth: AxisName, constant: f64| {
            Path::polyline(vec![to_xy(nth, 0.0, constant), to_xy(nth, 1.0, constant)])
        };
        Ok(match self {
            Self::Fixed(side) | Self::CurveFixed { side, .. } => {
                (along(side.axis(), edge(*side)), Transform::Axes(ctx.id))
            }
            Self::Floating { nth, value } => {
                let constant = data_to_axes(ctx.axes, other(*nth), *value);
                (along(*nth, constant), Transform::Axes(ctx.id))
            }
            Self::CurveFloating { grid, line } => {
                let (_, points) = line
                    .grid(grid.grid_finder(), ctx.axes.view_lim)
                    .map_err(|e| e.to_string())?;
                (Path::polyline(points), Transform::Data(ctx.id))
            }
            Self::FloatingFixed { grid, side } => {
                let info = floating_info(grid, ctx)?;
                (
                    Path::polyline(info.edge_line(*side)),
                    Transform::Data(ctx.id),
                )
            }
        })
    }

    /// The coordinates of tick locations (matplotlib's
    /// `get_tick_transform`): data along the line and axes across it for
    /// rectilinear edges, data for rectilinear floating lines and
    /// curvilinear edges, display pixels for curvilinear lines.
    pub(crate) fn tick_transform(&self, axes: ArtistId) -> Transform {
        let data = Transform::Data(axes);
        let across = Transform::Axes(axes);
        match self {
            Self::Fixed(side) if side.axis() == AxisName::X => Transform::blended(data, across),
            Self::Fixed(_) => Transform::blended(across, data),
            Self::Floating { .. } | Self::CurveFixed { .. } => data,
            Self::CurveFloating { .. } | Self::FloatingFixed { .. } => Transform::Display,
        }
    }

    /// The major and minor ticks on the line (matplotlib's
    /// `get_tick_iterators`): rectilinear lines take those of the ticked
    /// axis, `ticks`, inside the axes.
    pub(crate) fn ticks(&self, ctx: HelperContext<'_>) -> Result<TickLevels, String> {
        let grid_error = |e: super::grid_finder::GridError| e.to_string();
        match self {
            Self::Fixed(side) => {
                rectilinear_levels(ctx.axes, side.axis(), Across::Edge(edge(*side)))
            }
            Self::Floating { nth, value } => {
                rectilinear_levels(ctx.axes, *nth, Across::Data(*value))
            }
            Self::CurveFixed {
                grid,
                side,
                nth_ticks,
            } => {
                let view = ctx.axes.view_lim;
                let info = grid.grid_finder().grid_info(view).map_err(grid_error)?;
                let (v1, v2) = match side.axis() {
                    AxisName::X => view.interval_y(),
                    AxisName::Y => view.interval_x(),
                };
                Ok((
                    curvelinear::fixed_ticks(&info, *side, *nth_ticks, v1 > v2),
                    Vec::new(),
                ))
            }
            Self::CurveFloating { grid, line } => {
                let (axis_grid, _) = line
                    .grid(grid.grid_finder(), ctx.axes.view_lim)
                    .map_err(grid_error)?;
                let (e0, e1) = line.extremes;
                let spec = TickSpec {
                    mask: (e0, e1),
                    xlim: (f64::NEG_INFINITY, f64::INFINITY),
                    ylim: (e0, e1),
                };
                Ok((
                    curve_ticks(ctx, grid, &axis_grid, *line, &spec)?,
                    Vec::new(),
                ))
            }
            Self::FloatingFixed { grid, side } => {
                let info = floating_info(grid, ctx)?;
                let line = grid.edge_line(*side);
                let (x0, x1, y0, y1) = grid.extremes();
                let (xlim, ylim) = (sorted(x0, x1), sorted(y0, y1));
                let spec = TickSpec {
                    mask: match line.nth {
                        AxisName::X => ylim,
                        AxisName::Y => xlim,
                    },
                    xlim,
                    ylim,
                };
                Ok((
                    curve_ticks(ctx, grid.curvelinear(), &info.grid, line, &spec)?,
                    Vec::new(),
                ))
            }
        }
    }

    /// Where the axis label goes in display pixels and the direction of
    /// the line there (matplotlib's `get_axislabel_pos_angle` through
    /// `get_axislabel_transform`); `None` for a floating line outside the
    /// axes.
    pub(crate) fn label_position(
        &self,
        ctx: HelperContext<'_>,
    ) -> Result<Option<(Point, f64)>, String> {
        let in_axes = |xy: Point, angle: f64| -> Result<Option<(Point, f64)>, String> {
            let axes = ctx.resolve(&Transform::Axes(ctx.id))?;
            Ok(Some((axes.transform_point(xy), angle)))
        };
        match self {
            Self::Fixed(side) | Self::CurveFixed { side, .. } => {
                let (xy, angle) = edge_label(*side);
                in_axes(xy, angle)
            }
            Self::Floating { nth, value } => {
                let constant = data_to_axes(ctx.axes, other(*nth), *value);
                let angle = match nth {
                    AxisName::X => 0.0,
                    AxisName::Y => 90.0,
                };
                if (0.0..=1.0).contains(&constant) {
                    in_axes(to_xy(*nth, 0.5, constant), angle)
                } else {
                    Ok(None)
                }
            }
            Self::CurveFloating { grid, line } => {
                let (axis_grid, _) = line
                    .grid(grid.grid_finder(), ctx.axes.view_lim)
                    .map_err(|e| e.to_string())?;
                curve_label(ctx, grid, axis_grid.extremes, *line)
            }
            Self::FloatingFixed { grid, side } => {
                let info = floating_info(grid, ctx)?;
                curve_label(
                    ctx,
                    grid.curvelinear(),
                    info.grid.extremes,
                    grid.edge_line(*side),
                )
            }
        }
    }
}

impl AxisHelper {
    /// The origin of the label coordinates in display pixels, where a
    /// label without a position stays (matplotlib's initial label
    /// position): the axes corner, or the canvas corner for curvilinear
    /// lines, whose labels are placed in display coordinates.
    pub(crate) fn label_origin(&self, ctx: HelperContext<'_>) -> Result<Point, String> {
        match self {
            Self::CurveFloating { .. } | Self::FloatingFixed { .. } => Ok(Point::ORIGIN),
            Self::Fixed(_) | Self::Floating { .. } | Self::CurveFixed { .. } => Ok(ctx
                .resolve(&Transform::Axes(ctx.id))?
                .transform_point(Point::ORIGIN)),
        }
    }
}

/// Where a rectilinear line runs across its coordinate.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Across {
    /// Along an edge, at this axes coordinate.
    Edge(f64),
    /// Through this data value.
    Data(f64),
}

/// The major and minor ticks of the axis `nth` on a rectilinear line.
fn rectilinear_levels(
    axes: &AxesState,
    nth: AxisName,
    across: Across,
) -> Result<TickLevels, String> {
    let ticked = axes.axis(nth).ticks.as_ref().map_err(ToString::to_string)?;
    Ok((
        rectilinear_ticks(axes, nth, across, &ticked.major),
        rectilinear_ticks(axes, nth, across, &ticked.minor_all),
    ))
}

/// The ticks among `ticks` of axis `nth` on a rectilinear line inside the
/// axes: within a relative tolerance along an edge, strictly inside for
/// a line through the data.
fn rectilinear_ticks(
    axes: &AxesState,
    nth: AxisName,
    across: Across,
    ticks: &[Tick],
) -> Vec<TickInfo> {
    let (angle_normal, angle_tangent) = match nth {
        AxisName::X => (90.0, 0.0),
        AxisName::Y => (0.0, 90.0),
    };
    ticks
        .iter()
        .filter_map(|tick| {
            let (loc, shown) = match across {
                Across::Edge(constant) => {
                    let along = data_to_axes(axes, nth, tick.loc);
                    (to_xy(nth, tick.loc, constant), contains_close(along))
                }
                Across::Data(value) => {
                    let loc = to_xy(nth, tick.loc, value);
                    let c1 = data_to_axes(axes, AxisName::X, loc.x);
                    let c2 = data_to_axes(axes, AxisName::Y, loc.y);
                    (loc, (0.0..=1.0).contains(&c1) && (0.0..=1.0).contains(&c2))
                }
            };
            shown.then(|| TickInfo {
                loc,
                angle_normal,
                angle_tangent,
                label: tick.label.clone(),
            })
        })
        .collect()
}

/// The grid information of floating axes over the view.
fn floating_info(
    grid: &floating_axes::GridHelperCurveLinear,
    ctx: HelperContext<'_>,
) -> Result<FloatingGridInfo, String> {
    grid.grid_info(ctx.axes.view_lim).map_err(|e| e.to_string())
}

/// The ticks of a curvilinear line of constant coordinate.
fn curve_ticks(
    ctx: HelperContext<'_>,
    grid: &GridHelperCurveLinear,
    axis_grid: &AxisGrid,
    line: FloatingLine,
    spec: &TickSpec,
) -> Result<Vec<TickInfo>, String> {
    Ok(curvelinear::curve_ticks(
        axis_grid,
        line,
        spec,
        &ctx.grid_to_display(grid)?,
        &ctx.display_to_axes()?,
    ))
}

/// The label position of a curvilinear line of constant coordinate.
fn curve_label(
    ctx: HelperContext<'_>,
    grid: &GridHelperCurveLinear,
    extremes: Bbox,
    line: FloatingLine,
) -> Result<Option<(Point, f64)>, String> {
    Ok(curvelinear::curve_label_position(
        extremes,
        line,
        &ctx.grid_to_display(grid)?,
        &ctx.display_to_axes()?,
    ))
}

/// The label reference point of an edge in axes coordinates and the
/// direction of the edge.
fn edge_label(side: SpineSide) -> (Point, f64) {
    match side {
        SpineSide::Left => (Point::new(0.0, 0.5), 90.0),
        SpineSide::Right => (Point::new(1.0, 0.5), 90.0),
        SpineSide::Bottom => (Point::new(0.5, 0.0), 0.0),
        SpineSide::Top => (Point::new(0.5, 1.0), 0.0),
    }
}

/// The constant axes coordinate of an edge.
fn edge(side: SpineSide) -> f64 {
    f64::from(u8::from(side.is_high()))
}

/// `(value, constant)` as a point, `value` along coordinate `nth`.
fn to_xy(nth: AxisName, value: f64, constant: f64) -> Point {
    match nth {
        AxisName::X => Point::new(value, constant),
        AxisName::Y => Point::new(constant, value),
    }
}

/// `(a, b)` in increasing order.
fn sorted(a: f64, b: f64) -> (f64, f64) {
    if b < a { (b, a) } else { (a, b) }
}

/// The other coordinate.
pub(crate) fn other(name: AxisName) -> AxisName {
    match name {
        AxisName::X => AxisName::Y,
        AxisName::Y => AxisName::X,
    }
}

/// Data value `v` of axis `name` in axes coordinates, computed like
/// matplotlib's `transData - transAxes` (the scale, then `transLimits`).
pub(crate) fn data_to_axes(axes: &AxesState, name: AxisName, v: f64) -> f64 {
    let axis = axes.axis(name);
    let (v0, v1) = axes.view_interval(name);
    let (low, high) = (axis.forward(v0), axis.forward(v1));
    let scale = 1.0 / (high - low);
    axis.forward(v) * scale + (-low * scale)
}

/// Whether an axes coordinate lies in `[0, 1]` up to a relative tolerance
/// (matplotlib's `_interval_contains_close((0, 1), value)`).
fn contains_close(value: f64) -> bool {
    (-CLOSE..=1.0 + CLOSE).contains(&value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::testing::ticks_at;

    #[test]
    fn floating_lines_sit_at_their_value() {
        let fig = Figure::new();
        let ax = fig
            .add_axes([0.1, 0.1, 0.8, 0.8])
            .expect("a valid rectangle");
        ax.set_xlim(-2.0, 2.0)
            .and_then(|ax| ax.set_ylim(-3.0, 3.0))
            .expect("finite limits");
        let state = fig.raw().figure_state().borrow();
        let axes = state.store.get_as::<AxesState>(ax.id()).unwrap();
        assert_eq!(data_to_axes(axes, AxisName::Y, 1.5), 0.75);
        let shown = rectilinear_ticks(
            axes,
            AxisName::X,
            Across::Data(1.5),
            &ticks_at(&[-3.0, -2.0, 0.0, 2.0, 2.5]),
        );
        let locs: Vec<f64> = shown.iter().map(|t| t.loc.x).collect();
        assert_eq!(locs, vec![-2.0, 0.0, 2.0]);
        assert_eq!(shown[0].loc.y, 1.5);
    }

    #[test]
    fn fixed_lines_follow_the_edges() {
        let fig = Figure::new();
        let ax = fig
            .add_axes([0.1, 0.1, 0.8, 0.8])
            .expect("a valid rectangle");
        ax.set_xlim(0.0, 1.0)
            .and_then(|ax| ax.set_ylim(0.0, 1.0))
            .expect("finite limits");
        let state = fig.raw().figure_state().borrow();
        let axes = state.store.get_as::<AxesState>(ax.id()).unwrap();
        let shown = rectilinear_ticks(
            axes,
            AxisName::Y,
            Across::Edge(edge(SpineSide::Right)),
            &ticks_at(&[0.0, 1.0 + 1e-12, 1.5]),
        );
        assert_eq!(shown.len(), 2, "within the tolerance of the edge");
        assert_eq!((shown[0].angle_normal, shown[0].angle_tangent), (0.0, 90.0));
        assert_eq!(edge_label(SpineSide::Top), (Point::new(0.5, 1.0), 0.0));
    }
}
