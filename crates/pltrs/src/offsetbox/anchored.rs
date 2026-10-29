//! Placing a box at one of nine positions inside a container
//! (matplotlib's `AnchoredOffsetbox.codes`, `_get_anchored_bbox` and
//! `bbox_to_anchor`).

use std::fmt;
use std::str::FromStr;

use pltrs_core::{Anchor, Bbox, Point};

use crate::transforms::ResolvedTransform;

/// A position inside a container box, with matplotlib's location codes
/// `1..=10` (`AnchoredOffsetbox.codes`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnchorLoc {
    /// `"upper right"`, code 1.
    UpperRight,
    /// `"upper left"`, code 2.
    UpperLeft,
    /// `"lower left"`, code 3.
    LowerLeft,
    /// `"lower right"`, code 4.
    LowerRight,
    /// `"right"`, code 5 (placed like `"center right"`).
    Right,
    /// `"center left"`, code 6.
    CenterLeft,
    /// `"center right"`, code 7.
    CenterRight,
    /// `"lower center"`, code 8.
    LowerCenter,
    /// `"upper center"`, code 9.
    UpperCenter,
    /// `"center"`, code 10.
    Center,
}

impl AnchorLoc {
    /// Every location in code order.
    pub const ALL: [Self; 10] = [
        Self::UpperRight,
        Self::UpperLeft,
        Self::LowerLeft,
        Self::LowerRight,
        Self::Right,
        Self::CenterLeft,
        Self::CenterRight,
        Self::LowerCenter,
        Self::UpperCenter,
        Self::Center,
    ];

    /// matplotlib's numeric code (1 to 10).
    pub fn code(self) -> u8 {
        let index = Self::ALL
            .iter()
            .position(|&loc| loc == self)
            .expect("every location is listed in ALL");
        u8::try_from(index + 1).expect("ten locations fit in a byte")
    }

    /// The location with matplotlib's numeric `code`, if it is `1..=10`.
    pub fn from_code(code: i64) -> Option<Self> {
        let index = usize::try_from(code.checked_sub(1)?).ok()?;
        Self::ALL.get(index).copied()
    }

    /// matplotlib's name of the location.
    pub fn name(self) -> &'static str {
        match self {
            Self::UpperRight => "upper right",
            Self::UpperLeft => "upper left",
            Self::LowerLeft => "lower left",
            Self::LowerRight => "lower right",
            Self::Right => "right",
            Self::CenterLeft => "center left",
            Self::CenterRight => "center right",
            Self::LowerCenter => "lower center",
            Self::UpperCenter => "upper center",
            Self::Center => "center",
        }
    }

    /// The anchor of the box within its container.
    pub fn anchor(self) -> Anchor {
        match self {
            Self::UpperRight => Anchor::NE,
            Self::UpperLeft => Anchor::NW,
            Self::LowerLeft => Anchor::SW,
            Self::LowerRight => Anchor::SE,
            Self::Right | Self::CenterRight => Anchor::E,
            Self::CenterLeft => Anchor::W,
            Self::LowerCenter => Anchor::S,
            Self::UpperCenter => Anchor::N,
            Self::Center => Anchor::C,
        }
    }
}

impl fmt::Display for AnchorLoc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A string that names no [`AnchorLoc`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownLocation(pub String);

impl fmt::Display for UnknownLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} is not a valid location", self.0)
    }
}

impl std::error::Error for UnknownLocation {}

impl FromStr for AnchorLoc {
    type Err = UnknownLocation;

    /// Parses matplotlib's location names (exact spelling).
    fn from_str(name: &str) -> Result<Self, UnknownLocation> {
        Self::ALL
            .into_iter()
            .find(|loc| loc.name() == name)
            .ok_or_else(|| UnknownLocation(name.to_owned()))
    }
}

/// The box an anchored box or a legend is placed in (matplotlib's
/// `bbox_to_anchor`), in the coordinates of its `bbox_transform`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BboxToAnchor {
    /// A point: the corner of the box named by `loc` goes there.
    Point(f64, f64),
    /// A box `(x, y, width, height)` the box is placed in.
    Bounds(f64, f64, f64, f64),
}

impl BboxToAnchor {
    /// The anchor as a box (a point is a box of zero size).
    pub fn bbox(self) -> Bbox {
        match self {
            Self::Point(x, y) => Bbox::from_bounds(x, y, 0.0, 0.0),
            Self::Bounds(x, y, width, height) => Bbox::from_bounds(x, y, width, height),
        }
    }
}

impl From<(f64, f64)> for BboxToAnchor {
    fn from((x, y): (f64, f64)) -> Self {
        Self::Point(x, y)
    }
}

impl From<(f64, f64, f64, f64)> for BboxToAnchor {
    fn from((x, y, width, height): (f64, f64, f64, f64)) -> Self {
        Self::Bounds(x, y, width, height)
    }
}

/// The box around the corners of `bbox` mapped by `transform`, keeping its
/// orientation (matplotlib's `TransformedBbox`).
pub(crate) fn transformed_bbox(bbox: &Bbox, transform: &ResolvedTransform) -> Bbox {
    let corners = [
        Point::new(bbox.p0.x, bbox.p0.y),
        Point::new(bbox.p1.x, bbox.p0.y),
        Point::new(bbox.p0.x, bbox.p1.y),
        Point::new(bbox.p1.x, bbox.p1.y),
    ]
    .map(|p| transform.transform_point(p));
    let (mut x0, mut x1) = min_max(corners.iter().map(|p| p.x));
    let (mut y0, mut y1) = min_max(corners.iter().map(|p| p.y));
    if bbox.p0.x > bbox.p1.x {
        (x0, x1) = (x1, x0);
    }
    if bbox.p0.y > bbox.p1.y {
        (y0, y1) = (y1, y0);
    }
    Bbox::from_extents(x0, y0, x1, y1)
}

fn min_max(values: impl Iterator<Item = f64>) -> (f64, f64) {
    values.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
        (lo.min(v), hi.max(v))
    })
}

/// The lower-left corner of `bbox` placed at `loc` inside `parent` shrunk
/// by `pad_x` and `pad_y` on each side (matplotlib's
/// `offsetbox._get_anchored_bbox`).
///
/// ```
/// use pltrs::offsetbox::{AnchorLoc, anchored_bbox};
/// use pltrs::transforms::{Bbox, Point};
/// let corner = anchored_bbox(
///     AnchorLoc::UpperRight,
///     &Bbox::from_bounds(0.0, 0.0, 2.0, 1.0),
///     &Bbox::from_bounds(0.0, 0.0, 10.0, 10.0),
///     1.0,
///     1.0,
/// );
/// assert_eq!(corner, Point::new(7.0, 8.0));
/// ```
pub fn anchored_bbox(loc: AnchorLoc, bbox: &Bbox, parent: &Bbox, pad_x: f64, pad_y: f64) -> Point {
    let container = parent.padded(-pad_x, -pad_y);
    bbox.anchored(loc.anchor(), &container).p0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_and_names_round_trip() {
        for loc in AnchorLoc::ALL {
            assert_eq!(AnchorLoc::from_code(i64::from(loc.code())), Some(loc));
            assert_eq!(loc.name().parse(), Ok(loc));
        }
        assert_eq!(AnchorLoc::UpperRight.code(), 1);
        assert_eq!(AnchorLoc::Center.code(), 10);
        assert_eq!(AnchorLoc::from_code(0), None);
        assert_eq!(AnchorLoc::from_code(11), None);
        assert!("Upper Right".parse::<AnchorLoc>().is_err());
    }

    #[test]
    fn right_places_like_center_right() {
        let bbox = Bbox::from_bounds(0.0, 0.0, 2.0, 2.0);
        let parent = Bbox::from_bounds(0.0, 0.0, 10.0, 6.0);
        assert_eq!(
            anchored_bbox(AnchorLoc::Right, &bbox, &parent, 0.5, 0.5),
            anchored_bbox(AnchorLoc::CenterRight, &bbox, &parent, 0.5, 0.5)
        );
        assert_eq!(
            anchored_bbox(AnchorLoc::LowerLeft, &bbox, &parent, 0.5, 0.25),
            Point::new(0.5, 0.25)
        );
    }
}
