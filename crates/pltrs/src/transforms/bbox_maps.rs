//! Affine maps between boxes (matplotlib's `BboxTransform`,
//! `BboxTransformTo` and `BboxTransformFrom`).
//!
//! The matrices are built with matplotlib's exact arithmetic and IEEE
//! division, so a degenerate source box yields infinite or NaN
//! coefficients rather than an error, except in [`bbox_to_unit`].

use pltrs_core::{Affine2D, Bbox};

/// The affine map taking `boxin` onto `boxout` (`BboxTransform`):
/// `x' = (x - inl) * outw / inw + outl`, likewise for y.
pub fn bbox_transform(boxin: &Bbox, boxout: &Bbox) -> Affine2D {
    let (inl, inb, inw, inh) = boxin.bounds();
    let (outl, outb, outw, outh) = boxout.bounds();
    let (sx, sy) = (outw / inw, outh / inh);
    Affine2D::from_values(sx, 0.0, 0.0, sy, -inl * sx + outl, -inb * sy + outb)
}

/// The affine map taking the unit square onto `bbox` (`BboxTransformTo`).
pub fn unit_to_bbox(bbox: &Bbox) -> Affine2D {
    let (x0, y0, width, height) = bbox.bounds();
    Affine2D::from_values(width, 0.0, 0.0, height, x0, y0)
}

/// The affine map taking `bbox` onto the unit square
/// (`BboxTransformFrom`); a zero-sized side gives infinite scale.
pub fn bbox_transform_from(bbox: &Bbox) -> Affine2D {
    let (width, height) = (bbox.width(), bbox.height());
    unit_from(bbox, 1.0 / width, 1.0 / height)
}

/// Like [`bbox_transform_from`], but a zero-sized side maps everything to
/// the origin instead of producing infinite coefficients, so degenerate
/// view limits still draw finitely.
pub fn bbox_to_unit(bbox: &Bbox) -> Affine2D {
    let reciprocal = |v: f64| if v == 0.0 { 0.0 } else { 1.0 / v };
    unit_from(bbox, reciprocal(bbox.width()), reciprocal(bbox.height()))
}

/// Translation by the box's first corner, then scaling by `(sx, sy)`.
fn unit_from(bbox: &Bbox, sx: f64, sy: f64) -> Affine2D {
    Affine2D::from_values(sx, 0.0, 0.0, sy, -bbox.x0() * sx, -bbox.y0() * sy)
}

#[cfg(test)]
mod tests {
    use pltrs_core::Point;

    use super::*;

    fn boxes() -> [Bbox; 3] {
        [
            Bbox::from_extents(1.0, 2.0, 4.0, 6.0),
            Bbox::from_extents(0.125, 0.11, 0.9, 0.88),
            Bbox::from_extents(4.0, 6.0, 1.0, 2.0),
        ]
    }

    #[test]
    fn maps_corners_onto_corners() {
        let [boxin, boxout, _] = boxes();
        let t = bbox_transform(&boxin, &boxout);
        assert!((t.transform_point(boxin.p0) - boxout.p0).length() < 1e-15);
        assert!((t.transform_point(boxin.p1) - boxout.p1).length() < 1e-15);
    }

    #[test]
    fn to_and_from_the_unit_square() {
        for bbox in boxes() {
            let to = unit_to_bbox(&bbox);
            assert_eq!(to.transform_point(Point::new(1.0, 1.0)), bbox.p1);
            let from = bbox_transform_from(&bbox);
            assert_eq!(from.transform_point(bbox.p0), Point::ORIGIN);
            assert_eq!(from, bbox_to_unit(&bbox));
        }
        assert_eq!(
            unit_to_bbox(&Bbox::from_extents(1.0, 2.0, 4.0, 6.0)).matrix(),
            [[3.0, 0.0, 1.0], [0.0, 4.0, 2.0], [0.0, 0.0, 1.0]]
        );
    }

    #[test]
    fn degenerate_boxes() {
        let point = Bbox::from_extents(2.0, 2.0, 2.0, 2.0);
        let [a, _, _, d, e, _] = bbox_transform_from(&point).values();
        assert_eq!((a, d, e), (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY));
        assert_eq!(bbox_to_unit(&point), Affine2D::scaling(0.0, 0.0));
        let line = Bbox::from_extents(-3.0, 10.0, 7.0, 10.0);
        let [a, _, _, d, e, f] = bbox_transform(&line, &Bbox::UNIT).values();
        assert_eq!((a, d), (0.1, f64::INFINITY));
        assert!((e - 0.3).abs() < 1e-15 && f == f64::NEG_INFINITY);
        let [a, ..] = bbox_transform(&point, &point).values();
        assert!(a.is_nan(), "0 / 0");
    }
}
