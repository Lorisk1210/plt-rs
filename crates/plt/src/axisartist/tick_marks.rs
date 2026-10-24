//! Tick marks drawn on their own at given locations and angles
//! (matplotlib's `axis_artist.Ticks` added to an axes).

use plt_core::{Bbox, Point};
use plt_render::GraphicsContext;

use super::ticks::TicksState;
use crate::artist::{Artist, ArtistBase, ArtistHandle, ArtistId, DrawContext, RawHandle};
use crate::axes::{Axes, AxesState, DataUpdate, TickDirection};
use crate::colors::Color;
use crate::ticker::AxisName;
use crate::transforms::Transform;

/// The z-order of tick marks (matplotlib's `Ticks` are lines).
const ZORDER: f64 = 2.0;

/// Tick marks to draw at given locations, each turned by its angle
/// (matplotlib's `Ticks` with `set_locs_angles`), for
/// [`Axes::add_tick_marks`].
#[derive(Clone, Debug, PartialEq)]
pub struct TickMarks {
    /// Where the marks go, in data coordinates, with their angles in
    /// degrees (0 points along +x).
    pub locs_angles: Vec<(Point, f64)>,
    /// Length in points.
    pub size: f64,
    /// Which way the marks run from their locations: `In` along their
    /// angle, `Out` against it, `InOut` across.
    pub direction: TickDirection,
    /// The axis whose major ticks give the colour and width left unset
    /// (matplotlib's `axis` argument).
    pub axis: AxisName,
    /// The colour (`None` = that of the axis' major ticks).
    pub color: Option<Color>,
    /// The line width in points (`None` = that of the axis' major ticks).
    pub width: Option<f64>,
}

impl TickMarks {
    /// Marks `size` points long at `locs_angles`, running along their
    /// angles and styled like the major ticks of `axis` (matplotlib's
    /// defaults).
    pub fn new(size: f64, axis: AxisName, locs_angles: Vec<(Point, f64)>) -> Self {
        Self {
            locs_angles,
            size,
            direction: TickDirection::In,
            axis,
            color: None,
            width: None,
        }
    }
}

/// A handle to tick marks drawn on their own (see
/// [`Axes::add_tick_marks`]).
#[derive(Clone, Debug, PartialEq)]
pub struct TickMarksArtist {
    raw: RawHandle,
}

impl ArtistHandle for TickMarksArtist {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

/// Tick marks of an axes (matplotlib's `Ticks`).
#[derive(Debug)]
struct TickMarksState {
    base: ArtistBase,
    axes: ArtistId,
    marks: TickMarks,
}

impl TickMarksState {
    /// The pen of the marks: their colour and width, or those of the
    /// major ticks of their axis (matplotlib's `"auto"` colour and width).
    fn gc(&self, axes: &AxesState) -> Result<GraphicsContext, String> {
        let style = &axes.axis(self.marks.axis).major_style;
        let color = self.marks.color.as_ref().unwrap_or(&style.color);
        Ok(GraphicsContext {
            foreground: color.resolve().map_err(|e| e.to_string())?,
            linewidth: self.marks.width.unwrap_or(style.width),
            alpha: self.base.alpha,
            ..GraphicsContext::default()
        })
    }
}

/// Whether `bbox` contains `point`, edges included, in either orientation
/// (matplotlib's `Bbox.contains`).
fn contains(bbox: &Bbox, point: Point) -> bool {
    let within = |v: f64, a: f64, b: f64| (a <= v && v <= b) || (b <= v && v <= a);
    within(point.x, bbox.x0(), bbox.x1()) && within(point.y, bbox.y0(), bbox.y1())
}

impl Artist for TickMarksState {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    /// Draws the marks inside the view limits, unclipped (matplotlib's
    /// `Ticks.draw`, which compares the data locations of linear axes
    /// with the view limits and sets no clip).
    fn draw(&self, ctx: &mut DrawContext<'_>) {
        let layout = *ctx.layout();
        let (Some(axes), Some(transform)) = (
            layout.store().get_as::<AxesState>(self.axes),
            layout.resolve(&Transform::Data(self.axes)),
        ) else {
            return;
        };
        let gc = match self.gc(axes) {
            Ok(gc) => gc,
            Err(problem) => return ctx.report(self.base.id(), problem),
        };
        let shown: Vec<(Point, f64)> = self
            .marks
            .locs_angles
            .iter()
            .copied()
            .filter(|&(loc, _)| contains(&axes.view_lim, loc))
            .collect();
        TicksState::new(self.marks.size, self.marks.direction).draw(ctx, &shown, &transform, &gc);
    }
}

impl Axes {
    /// Adds `marks`, which neither clip to the axes nor change its data
    /// limits; those outside the view limits are left out (matplotlib's
    /// `ax.add_artist(Ticks(size, axis=...))` after `set_locs_angles`).
    ///
    /// ```
    /// use plt::axisartist::TickMarks;
    /// use plt::prelude::*;
    /// use plt::axes::AxisName;
    /// use plt::transforms::Point;
    /// let ax = Figure::new().subplot();
    /// let marks = TickMarks::new(10.0, AxisName::X, vec![(Point::new(0.5, 0.0), 90.0)]);
    /// let artist = ax.add_tick_marks(marks);
    /// assert_eq!(artist.zorder(), 2.0);
    /// ```
    pub fn add_tick_marks(&self, marks: TickMarks) -> TickMarksArtist {
        let state = TickMarksState {
            base: ArtistBase::with_zorder(ZORDER),
            axes: self.id(),
            marks,
        };
        TickMarksArtist {
            raw: self.adopt(state, &DataUpdate::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;
    use crate::testing::{Call, RecordingRenderer};

    /// An axes over the unit square filling a 100 × 100 px figure, with
    /// nothing of its own drawn.
    fn bare_axes() -> (Figure, Axes) {
        let fig = Figure::with_size(1.0, 1.0, 100.0);
        let ax = fig
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        ax.set_axis_off();
        (fig, ax)
    }

    #[test]
    fn marks_outside_the_view_are_left_out() {
        let (fig, ax) = bare_axes();
        let locs_angles = [-0.1, 0.0, 0.5, 1.0, 1.1]
            .map(|x| (Point::new(x, 0.0), 90.0))
            .to_vec();
        ax.add_tick_marks(TickMarks::new(7.2, AxisName::X, locs_angles));
        let mut renderer = RecordingRenderer::new(100.0, 100.0, 100.0);
        fig.draw(&mut renderer).expect("drawable");
        let marks: Vec<Vec<Point>> = renderer
            .calls
            .into_iter()
            .filter_map(|call| match call {
                Call::Path {
                    points, fill: None, ..
                } => Some(points),
                _ => None,
            })
            .collect();
        let starts: Vec<f64> = marks.iter().map(|points| points[0].x).collect();
        assert_eq!(starts, [0.0, 50.0, 100.0], "the view edges included");
        // 7.2 points at 100 dpi, pointing up the angle.
        assert!((marks[1][1].y - 10.0).abs() < 1e-9, "{marks:?}");
    }

    #[test]
    fn unset_colours_and_widths_follow_the_axis() {
        let (_fig, ax) = bare_axes();
        let state = |marks: TickMarks| TickMarksState {
            base: ArtistBase::with_zorder(ZORDER),
            axes: ax.id(),
            marks,
        };
        let pen = |marks: TickMarks| {
            ax.raw()
                .with::<AxesState, _>(|axes| state(marks).gc(axes))
                .expect("a pen")
        };
        let auto = pen(TickMarks::new(5.0, AxisName::Y, Vec::new()));
        assert_eq!(
            (auto.foreground, auto.linewidth),
            (plt_core::Rgba::BLACK, 0.8)
        );
        let styled = pen(TickMarks {
            color: Some(Color::from("r")),
            width: Some(2.0),
            ..TickMarks::new(5.0, AxisName::Y, Vec::new())
        });
        let red = Color::from("r").resolve().expect("a colour");
        assert_eq!((styled.foreground, styled.linewidth), (red, 2.0));
    }
}
