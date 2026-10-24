//! Lines extending across the whole view (matplotlib's `AxLine`).

use std::fmt;

use plt_core::{Bbox, Path, Point};

use super::{Line2D, LineArtist};
use crate::artist::{ArtistHandle, ArtistId, LayoutContext};
use crate::transforms::{ResolvedTransform, Transform};

/// What fixes an infinite line besides its first point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AxLineThrough {
    /// A second point on the line.
    Point(Point),
    /// The slope in scaled data coordinates (infinite = vertical).
    Slope(f64),
}

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
/// use plt::lines::{AxLineError, AxLineThrough, LineProps};
/// use plt::prelude::*;
/// use plt::transforms::Point;
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

/// An infinite line through `xy1` (in the line's coordinates).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct InfiniteLine {
    pub(crate) xy1: Point,
    pub(crate) through: AxLineThrough,
}

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
    ) -> Result<Path, String> {
        let axes = axes.ok_or("an infinite line needs an axes")?;
        let view = layout
            .axes_extent(axes)
            .ok_or("an infinite line needs an axes")?;
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
                    .ok_or("the axes' data coordinates are unavailable")?
                    .affine_part();
                let to_scaled = scaled_to_display
                    .inverted()
                    .ok_or("the axes' data coordinates are singular")?;
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
        let [start, stop] =
            clip_to_view(p1, p2, &view).ok_or("cannot draw a line through two identical points")?;
        Ok(Path::polyline(vec![start, stop]))
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
            line.set_axline_slope(1.0).err(),
            Some(AxLineError::PointIsSet)
        );
        let plain = ax.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
        assert_eq!(plain.axline_xy1(), None);
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
