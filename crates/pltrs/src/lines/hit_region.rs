//! Where a point hits a line, as a shape: the region matplotlib's
//! `Line2D.contains` accepts, which annotation arrows given a line as
//! `patchA` or `patchB` are clipped to.

use std::f64::consts::TAU;

use pltrs_core::{Path, Point};

use crate::patches::ClipRegion;

/// Sides of the polygons standing in for the disks around vertices: their
/// edges stay within a thousandth of a pixel of the circle for the usual
/// pick radii.
const DISK_SIDES: u32 = 256;

/// The points within `radius` pixels of the display vertices `points`
/// (a line without a line style), or of the segments joining them
/// (`with_segments`), as `Line2D.contains` and `segment_hits` test them.
pub(super) fn hit_region(points: &[Point], radius: f64, with_segments: bool) -> ClipRegion {
    let finite: Vec<Point> = points.iter().copied().filter(|p| p.is_finite()).collect();
    let mut pieces: Vec<Path> = finite.iter().map(|&p| disk(p, radius)).collect();
    if with_segments {
        pieces.extend(
            points
                .windows(2)
                .filter(|pair| pair.iter().all(|p| p.is_finite()))
                .filter_map(|pair| band(pair[0], pair[1], radius)),
        );
    }
    ClipRegion {
        path: Path::compound(&pieces),
        radius: 0.0,
    }
}

/// The disk of `radius` around `center`, as a fine polygon.
fn disk(center: Point, radius: f64) -> Path {
    Path::polygon((0..DISK_SIDES).map(|i| {
        let (sin, cos) = (TAU * f64::from(i) / f64::from(DISK_SIDES)).sin_cos();
        Point::new(center.x + radius * cos, center.y + radius * sin)
    }))
}

/// The points within `radius` of the segment from `a` to `b` whose
/// nearest point on the segment's line lies between them; `None` for a
/// segment of zero length.
fn band(a: Point, b: Point, radius: f64) -> Option<Path> {
    let length = (b - a).length();
    if length == 0.0 {
        return None;
    }
    let normal = Point::new(a.y - b.y, b.x - a.x) * (radius / length);
    Some(Path::polygon([
        a + normal,
        b + normal,
        b - normal,
        a - normal,
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_only_hit_near_their_points() {
        let region = hit_region(
            &[Point::new(10.0, 10.0), Point::new(40.0, 10.0)],
            5.0,
            false,
        );
        assert!(region.contains(Point::new(14.99, 10.0)));
        assert!(region.contains(Point::new(40.0, 5.01)));
        assert!(!region.contains(Point::new(15.01, 10.0)));
        assert!(!region.contains(Point::new(25.0, 10.0)), "between markers");
    }

    #[test]
    fn lines_hit_along_their_segments() {
        let points = [
            Point::new(0.0, 0.0),
            Point::new(30.0, 0.0),
            Point::new(30.0, 30.0),
        ];
        let region = hit_region(&points, 5.0, true);
        assert!(region.contains(Point::new(15.0, 4.99)));
        assert!(!region.contains(Point::new(15.0, 5.01)));
        assert!(region.contains(Point::new(-4.99, 0.0)), "around the ends");
        assert!(region.contains(Point::new(33.5, -3.5)), "around the corner");
        assert!(!region.contains(Point::new(34.0, -4.0)));
    }

    #[test]
    fn gaps_and_repeated_points_add_no_segments() {
        let nan = Point::new(f64::NAN, 0.0);
        let points = [
            Point::new(0.0, 0.0),
            nan,
            Point::new(30.0, 0.0),
            Point::new(30.0, 0.0),
        ];
        let region = hit_region(&points, 5.0, true);
        assert!(region.contains(Point::new(30.0, 4.0)));
        assert!(
            !region.contains(Point::new(15.0, 0.0)),
            "no segment across the gap"
        );
        assert!(!hit_region(&[], 5.0, true).contains(Point::new(0.0, 0.0)));
    }
}
