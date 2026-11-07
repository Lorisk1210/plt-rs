//! FreeType's PostScript glyph builder (`ps_builder_*` in `psobjs.c`)
//! as the Adobe engine's outline callbacks drive it (`cf2_builder_*` in
//! `psft.c`).

use super::error::CharstringError;
use crate::outline::{Outline, PointKind};

/// FreeType's limit on outline points and contours (`USHRT_MAX`).
const MAX_ITEMS: usize = 0xFFFF;

/// A device-space point in 16.16, as the engine hands it over.
pub(super) type DevicePoint = (i32, i32);

/// One stored outline point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Point {
    x: i64,
    y: i64,
    on: bool,
}

/// The outline being built.
#[derive(Clone, Debug, Default)]
pub(super) struct Builder {
    points: Vec<Point>,
    /// The last point of each contour (FreeType's `contours`); the last
    /// entry is only final once the contour is closed.
    contours: Vec<usize>,
    path_begun: bool,
}

const fn too_large() -> CharstringError {
    CharstringError::Malformed("too many outline points")
}

impl Builder {
    /// Whether `count` more points fit.
    pub(super) fn has_room(&self, count: usize) -> bool {
        self.points.len() + count <= MAX_ITEMS
    }

    /// Stores a point: the engine's 16.16 coordinates become 26.6
    /// (`x >> 10`).
    fn add_point(&mut self, (x, y): DevicePoint, on: bool) {
        self.points.push(Point {
            x: i64::from(x) >> 10,
            y: i64::from(y) >> 10,
            on,
        });
    }

    fn add_point1(&mut self, point: DevicePoint) -> Result<(), CharstringError> {
        if !self.has_room(1) {
            return Err(too_large());
        }
        self.add_point(point, true);
        Ok(())
    }

    fn add_contour(&mut self) -> Result<(), CharstringError> {
        if self.contours.len() + 1 > MAX_ITEMS {
            return Err(too_large());
        }
        if let Some(last) = self.contours.last_mut() {
            *last = self.points.len().saturating_sub(1);
        }
        self.contours.push(0);
        Ok(())
    }

    fn start_point(&mut self, point: DevicePoint) -> Result<(), CharstringError> {
        if !self.path_begun {
            self.path_begun = true;
            self.add_contour()?;
            self.add_point1(point)?;
        }
        Ok(())
    }

    /// Closes the current contour: a last point on the first one is
    /// dropped, and contours left with one point vanish.
    pub(super) fn close_contour(&mut self) {
        let count = self.contours.len();
        let first = if count <= 1 {
            0
        } else {
            self.contours[count - 2] + 1
        };
        if count > 0 && first == self.points.len() {
            self.contours.pop();
            return;
        }
        if self.points.len() > 1 {
            let (start, last) = (self.points[first], self.points[self.points.len() - 1]);
            if start.x == last.x && start.y == last.y && last.on {
                self.points.pop();
            }
        }
        if count > 0 {
            if first + 1 == self.points.len() {
                self.contours.pop();
                self.points.pop();
            } else if let Some(end) = self.contours.last_mut() {
                *end = self.points.len() - 1;
            }
        }
    }

    /// The `moveTo` callback: closes the contour; the next segment starts
    /// a new one.
    pub(super) fn move_to(&mut self) {
        self.close_contour();
        self.path_begun = false;
    }

    /// The `lineTo` callback.
    pub(super) fn line_to(
        &mut self,
        from: DevicePoint,
        to: DevicePoint,
    ) -> Result<(), CharstringError> {
        if !self.path_begun {
            self.start_point(from)?;
        }
        self.add_point1(to)
    }

    /// The `cubeTo` callback.
    pub(super) fn cube_to(
        &mut self,
        from: DevicePoint,
        controls: [DevicePoint; 3],
    ) -> Result<(), CharstringError> {
        if !self.path_begun {
            self.start_point(from)?;
        }
        if !self.has_room(3) {
            return Err(too_large());
        }
        self.add_point(controls[0], false);
        self.add_point(controls[1], false);
        self.add_point(controls[2], true);
        Ok(())
    }

    /// The finished outline (`cf2_outline_close`).
    pub(super) fn finish(mut self) -> Outline {
        self.close_contour();
        let mut outline = Outline::default();
        let mut start = 0;
        for &last in &self.contours {
            for point in self.points.get(start..=last).unwrap_or_default() {
                let kind = if point.on {
                    PointKind::OnCurve
                } else {
                    PointKind::Cubic
                };
                outline.push(point.x as f64, point.y as f64, kind);
            }
            outline.end_contour();
            start = last + 1;
        }
        outline
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: i32, y: i32) -> DevicePoint {
        (x << 10, y << 10)
    }

    #[test]
    fn contours_drop_closing_points_and_single_points() {
        let mut builder = Builder::default();
        builder.move_to();
        builder.line_to(at(0, 0), at(10, 0)).unwrap();
        builder.line_to(at(10, 0), at(10, 10)).unwrap();
        builder.line_to(at(10, 10), at(0, 0)).unwrap();
        builder.move_to();
        builder.line_to(at(5, 5), at(5, 5)).unwrap();
        builder.move_to();
        builder
            .cube_to(at(0, 0), [at(1, 1), at(2, 1), at(3, 0)])
            .unwrap();
        let outline = builder.finish();
        assert_eq!(outline.contour_ends, vec![3, 7]);
        assert_eq!(outline.points[3].kind, PointKind::OnCurve);
        assert_eq!(outline.points[4].kind, PointKind::Cubic);
        assert_eq!((outline.points[1].x, outline.points[1].y), (10.0, 0.0));
    }

    #[test]
    fn coordinates_are_shifted_to_26_6() {
        let mut builder = Builder::default();
        builder.line_to((-1, 0), (0x7FF, 0x400)).unwrap();
        builder.line_to((0, 0), (0, 0)).unwrap();
        let outline = builder.finish();
        assert_eq!((outline.points[0].x, outline.points[1].x), (-1.0, 1.0));
        assert!(Builder::default().has_room(MAX_ITEMS));
        assert!(!Builder::default().has_room(MAX_ITEMS + 1));
    }
}
