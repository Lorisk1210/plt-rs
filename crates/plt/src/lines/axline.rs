//! Lines extending across the whole view (matplotlib's `AxLine`).

use plt_core::{Bbox, Path, Point};

use crate::artist::{ArtistId, LayoutContext};
use crate::transforms::ResolvedTransform;

/// What fixes an infinite line besides its first point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AxLineThrough {
    /// A second point on the line.
    Point(Point),
    /// The slope in scaled data coordinates (infinite = vertical).
    Slope(f64),
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
    /// image of scaled data space, so straight lines stay straight.
    pub(crate) fn display_segment(
        &self,
        layout: LayoutContext<'_>,
        resolved: &ResolvedTransform,
        axes: Option<ArtistId>,
    ) -> Result<Path, String> {
        let view = axes
            .and_then(|axes| layout.axes_extent(axes))
            .ok_or("an infinite line needs an axes")?;
        let (p1, p2) = match self.through {
            AxLineThrough::Point(xy2) => (
                resolved.transform_point(self.xy1),
                resolved.transform_point(xy2),
            ),
            AxLineThrough::Slope(slope) => {
                let scaled = resolved.transform_point_non_affine(self.xy1);
                let step = if slope.is_infinite() {
                    Point::new(0.0, 1.0)
                } else {
                    Point::new(1.0, slope)
                };
                let affine = resolved.affine_part();
                (
                    affine.transform_point(scaled),
                    affine.transform_point(scaled + step),
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
