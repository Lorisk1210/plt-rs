//! Lines extending across the whole view (matplotlib's `AxLine`).

use std::fmt;

use pltrs_core::{Bbox, Point};

use super::{Line2D, LineArtist};
use crate::artist::{Artist, ArtistHandle, ArtistId, DrawErrorKind, LayoutContext};
use crate::transforms::{ResolvedTransform, Transform, unit_to_bbox};

/// What fixes an infinite line besides its first point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AxLineThrough {
    /// A second point on the line.
    Point(Point),
    /// The slope in scaled data coordinates (infinite = vertical).
    Slope(f64),
}

pltrs_core::archive_enum!(AxLineThrough {
    Point(v0),
    Slope(v0),
});

/// Why the points or slope of an infinite line cannot be changed as asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxLineError {
    /// The line was not made by `Axes::axline`.
    NotInfinite,
    /// A second point was given to a line fixed by its slope.
    SlopeIsSet,
    /// A slope was given to a line fixed by a second point.
    PointIsSet,
}

impl fmt::Display for AxLineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotInfinite => "the line is not an infinite line",
            Self::SlopeIsSet => "Cannot set an 'xy2' value while 'slope' is set",
            Self::PointIsSet => "Cannot set a 'slope' value while 'xy2' is set",
        })
    }
}

impl std::error::Error for AxLineError {}

/// The points and slope of the lines made by `Axes::axline` (matplotlib's
/// `AxLine` getters and setters); other lines have none.
///
/// ```
/// use pltrs::lines::{AxLineError, AxLineThrough, LineProps};
/// use pltrs::prelude::*;
/// use pltrs::transforms::Point;
/// let ax = Figure::new().subplot();
/// let line = ax.axline(Point::new(0.1, 0.1), AxLineThrough::Slope(0.6), LineProps::default())?;
/// line.set_axline_xy1(Point::new(0.2, 0.3))?.set_axline_slope(2.4)?;
/// assert_eq!(line.axline_xy1(), Some(Point::new(0.2, 0.3)));
/// assert_eq!((line.axline_xy2(), line.axline_slope()), (None, Some(2.4)));
/// assert_eq!(line.set_axline_xy2(Point::new(1.0, 1.0)).err(), Some(AxLineError::SlopeIsSet));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
impl Line2D {
    /// The infinite line's own description, if it is one.
    fn infinite(&self) -> Option<InfiniteLine> {
        self.raw().with::<LineArtist, _>(|line| line.infinite)
    }

    /// Changes the infinite line's description with `update`, which may
    /// refuse the change.
    fn update_infinite(
        &self,
        update: impl FnOnce(&mut InfiniteLine) -> Result<(), AxLineError>,
    ) -> Result<&Self, AxLineError> {
        self.raw().with_mut::<LineArtist, _>(|line| {
            line.infinite
                .as_mut()
                .map_or(Err(AxLineError::NotInfinite), update)
        })?;
        Ok(self)
    }

    /// The first point of an infinite line (matplotlib's `get_xy1`).
    pub fn axline_xy1(&self) -> Option<Point> {
        self.infinite().map(|line| line.xy1)
    }

    /// The second point of an infinite line given one (matplotlib's
    /// `get_xy2`).
    pub fn axline_xy2(&self) -> Option<Point> {
        match self.infinite()?.through {
            AxLineThrough::Point(xy2) => Some(xy2),
            AxLineThrough::Slope(_) => None,
        }
    }

    /// The slope of an infinite line given one (matplotlib's
    /// `get_slope`).
    pub fn axline_slope(&self) -> Option<f64> {
        match self.infinite()?.through {
            AxLineThrough::Slope(slope) => Some(slope),
            AxLineThrough::Point(_) => None,
        }
    }

    /// The part of an infinite line inside its axes, from start to stop, in
    /// display pixels in the figure's current layout (matplotlib's
    /// `AxLine.get_transform` maps the diagonal of the unit square onto
    /// it); `None` for other lines, or when the line cannot be placed.
    pub fn axline_display_segment(&self) -> Option<(Point, Point)> {
        self.infinite()?;
        let state = self.raw().figure_state().prepared();
        let layout = LayoutContext::new(&state);
        let line = layout.store().get_as::<LineArtist>(self.raw().id())?;
        let resolved = layout.resolve(&line.transform())?;
        let [start, stop] = line
            .infinite
            .as_ref()?
            .display_segment(layout, &resolved, line.base().parent)
            .ok()?;
        Some((start, stop))
    }

    /// Moves the first point of an infinite line (matplotlib's
    /// `set_xy1`).
    ///
    /// # Errors
    ///
    /// When the line is not an infinite line.
    pub fn set_axline_xy1(&self, xy1: Point) -> Result<&Self, AxLineError> {
        self.update_infinite(|line| {
            line.xy1 = xy1;
            Ok(())
        })
    }

    /// Moves the second point of an infinite line given one (matplotlib's
    /// `set_xy2`).
    ///
    /// # Errors
    ///
    /// When the line is not an infinite line or is fixed by its slope.
    pub fn set_axline_xy2(&self, xy2: Point) -> Result<&Self, AxLineError> {
        self.update_infinite(|line| match line.through {
            AxLineThrough::Point(_) => {
                line.through = AxLineThrough::Point(xy2);
                Ok(())
            }
            AxLineThrough::Slope(_) => Err(AxLineError::SlopeIsSet),
        })
    }

    /// Changes the slope of an infinite line given one (matplotlib's
    /// `set_slope`).
    ///
    /// # Errors
    ///
    /// When the line is not an infinite line or is fixed by a second
    /// point.
    pub fn set_axline_slope(&self, slope: f64) -> Result<&Self, AxLineError> {
        self.update_infinite(|line| match line.through {
            AxLineThrough::Slope(_) => {
                line.through = AxLineThrough::Slope(slope);
                Ok(())
            }
            AxLineThrough::Point(_) => Err(AxLineError::PointIsSet),
        })
    }
}

/// Why an infinite line has no segment to draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SegmentError {
    /// The line is in no axes.
    NoAxes,
    /// The data coordinates of its axes are unavailable.
    NoDataCoordinates,
    /// The data coordinates of its axes are singular.
    SingularDataCoordinates,
    /// Its two points coincide on the screen.
    IdenticalPoints,
}

impl SegmentError {
    /// The kind of draw error it makes: matplotlib refuses a line
    /// through two identical points with a `ValueError` when drawing it.
    pub(crate) fn kind(self) -> DrawErrorKind {
        match self {
            Self::IdenticalPoints => DrawErrorKind::InvalidValue,
            _ => DrawErrorKind::Other,
        }
    }
}

impl fmt::Display for SegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NoAxes => "an infinite line needs an axes",
            Self::NoDataCoordinates => "the axes' data coordinates are unavailable",
            Self::SingularDataCoordinates => "the axes' data coordinates are singular",
            Self::IdenticalPoints => "cannot draw a line through two identical points",
        })
    }
}

/// An infinite line through `xy1` (in the line's coordinates).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct InfiniteLine {
    pub(crate) xy1: Point,
    pub(crate) through: AxLineThrough,
}

pltrs_core::archive_struct!(InfiniteLine { xy1, through });

impl InfiniteLine {
    /// The part of the line inside the axes, in display coordinates
    /// (matplotlib's `AxLine.get_transform`). Display space is an affine
    /// image of scaled data space, so straight lines stay straight; a
    /// slope is a slope in scaled data space, whatever coordinates the
    /// line's point is given in.
    pub(crate) fn display_segment(
        &self,
        layout: LayoutContext<'_>,
        resolved: &ResolvedTransform,
        axes: Option<ArtistId>,
    ) -> Result<[Point; 2], SegmentError> {
        let axes = axes.ok_or(SegmentError::NoAxes)?;
        let view = layout.axes_extent(axes).ok_or(SegmentError::NoAxes)?;
        let (p1, p2) = match self.through {
            AxLineThrough::Point(xy2) => (
                resolved.transform_point(self.xy1),
                resolved.transform_point(xy2),
            ),
            AxLineThrough::Slope(slope) => {
                // Scaled data to display: the affine part of the axes'
                // data transform (matplotlib's `transLimits + transAxes`).
                let scaled_to_display = layout
                    .resolve(&Transform::Data(axes))
                    .ok_or(SegmentError::NoDataCoordinates)?
                    .affine_part();
                let to_scaled = scaled_to_display
                    .inverted()
                    .ok_or(SegmentError::SingularDataCoordinates)?;
                let scaled = to_scaled.transform_point(resolved.transform_point(self.xy1));
                let step = if slope.is_infinite() {
                    Point::new(0.0, 1.0)
                } else {
                    Point::new(1.0, slope)
                };
                (
                    scaled_to_display.transform_point(scaled),
                    scaled_to_display.transform_point(scaled + step),
                )
            }
        };
        clip_to_view(p1, p2, &view).ok_or(SegmentError::IdenticalPoints)
    }

    /// The transform of the line's data (the diagonal of the unit square,
    /// unless the program set others): the unit square onto the box from
    /// the start to the stop of the part of the line inside the axes, in
    /// display coordinates (matplotlib's `AxLine.get_transform`,
    /// `BboxTransformTo(Bbox([start, stop]))` and on to display).
    pub(crate) fn display_map(
        &self,
        layout: LayoutContext<'_>,
        resolved: &ResolvedTransform,
        axes: Option<ArtistId>,
    ) -> Result<ResolvedTransform, SegmentError> {
        let [start, stop] = self.display_segment(layout, resolved, axes)?;
        Ok(ResolvedTransform::affine(unit_to_bbox(&Bbox::new(
            start, stop,
        ))))
    }
}

/// Where the line through `p1` and `p2` crosses `view`: the middle two of
/// its intersections with the four edge lines. `None` when the points
/// coincide.
fn clip_to_view(p1: Point, p2: Point, view: &Bbox) -> Option<[Point; 2]> {
    let (dx, dy) = (p2.x - p1.x, p2.y - p1.y);
    if dx == 0.0 && dy == 0.0 {
        return None;
    }
    if dy == 0.0 {
        return Some([Point::new(view.x0(), p1.y), Point::new(view.x1(), p1.y)]);
    }
    if dx == 0.0 {
        return Some([Point::new(p1.x, view.y0()), Point::new(p1.x, view.y1())]);
    }
    let slope = dy / dx;
    let mut crossings = [
        Point::new(view.x0(), p1.y + (view.x0() - p1.x) * slope),
        Point::new(view.x1(), p1.y + (view.x1() - p1.x) * slope),
        Point::new(p1.x + (view.y0() - p1.y) / slope, view.y0()),
        Point::new(p1.x + (view.y1() - p1.y) / slope, view.y1()),
    ];
    crossings.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    Some([crossings[1], crossings[2]])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;
    use crate::lines::LineProps;

    #[test]
    fn setters_move_the_drawn_line() {
        let fig = Figure::with_size(2.0, 2.0, 72.0);
        let ax = fig
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        ax.set_xlim(0.0, 1.0)
            .expect("finite limits")
            .set_ylim(0.0, 1.0)
            .expect("finite limits");
        let through = AxLineThrough::Point(Point::new(1.0, 1.0));
        let line = ax
            .axline(Point::ORIGIN, through, LineProps::default())
            .unwrap();
        line.set_axline_xy2(Point::new(1.0, 0.5)).unwrap();
        assert_eq!(
            line.window_extent(),
            Some(Bbox::from_extents(0.0, 0.0, 144.0, 72.0))
        );
        assert_eq!(
            line.axline_display_segment(),
            Some((Point::ORIGIN, Point::new(144.0, 72.0)))
        );
        assert_eq!(
            line.set_axline_slope(1.0).err(),
            Some(AxLineError::PointIsSet)
        );
        let plain = ax.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
        assert_eq!(
            (plain.axline_xy1(), plain.axline_display_segment()),
            (None, None)
        );
        assert_eq!(
            plain.set_axline_xy1(Point::ORIGIN).err(),
            Some(AxLineError::NotInfinite)
        );
    }

    #[test]
    fn diagonal_is_clipped_to_the_view() {
        let view = Bbox::from_extents(0.0, 0.0, 10.0, 10.0);
        let [a, b] = clip_to_view(Point::new(0.0, 0.0), Point::new(1.0, 1.0), &view).unwrap();
        assert_eq!((a, b), (Point::new(0.0, 0.0), Point::new(10.0, 10.0)));
        let [a, b] = clip_to_view(Point::new(0.0, 5.0), Point::new(2.0, 4.0), &view).unwrap();
        assert_eq!((a, b), (Point::new(0.0, 5.0), Point::new(10.0, 0.0)));
    }

    #[test]
    fn axis_parallel_lines_span_the_view() {
        let view = Bbox::from_extents(0.0, 0.0, 4.0, 3.0);
        let horizontal = clip_to_view(Point::new(1.0, 2.0), Point::new(5.0, 2.0), &view);
        assert_eq!(
            horizontal,
            Some([Point::new(0.0, 2.0), Point::new(4.0, 2.0)])
        );
        let vertical = clip_to_view(Point::new(1.0, 2.0), Point::new(1.0, 7.0), &view);
        assert_eq!(vertical, Some([Point::new(1.0, 0.0), Point::new(1.0, 3.0)]));
        assert_eq!(clip_to_view(Point::ORIGIN, Point::ORIGIN, &view), None);
    }
}
