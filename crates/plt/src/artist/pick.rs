//! Picking (matplotlib's `Artist.set_picker`, `Artist.pick` and
//! `PickEvent`): an artist given a picker fires a pick event when a mouse
//! press or scroll lands on it.

use std::fmt;
use std::rc::Rc;

use plt_core::Point;

use super::RawHandle;
use crate::events::MouseEvent;

/// A custom pick test: the indices of the artist's items under the
/// pointer (empty for the whole artist), or `None` when the pointer misses
/// it (matplotlib's callable picker).
pub type PickFn = dyn Fn(&RawHandle, &MouseEvent) -> Option<Vec<usize>>;

/// How an artist decides that it was picked (matplotlib's `picker`).
#[derive(Clone)]
pub enum Picker {
    /// The artist's own hit test with its default tolerance
    /// (`picker=True` as plotting functions take it): the pick radius of
    /// lines (5 points unless set) and collections (5 pixels unless set),
    /// 1 pixel around patches; texts, images and axes are hit inside their
    /// extent. matplotlib's `line.set_picker(True)` also sets the pick
    /// radius to 1 point: that is `Radius(1.0)`.
    Default,
    /// The artist's own hit test within this tolerance (`picker=5.0`): in
    /// points for lines, in pixels for collections and patches, as in
    /// matplotlib.
    Radius(f64),
    /// A function decides (`picker=callable`).
    Custom(Rc<PickFn>),
}

impl Picker {
    /// A picker deciding with `test`.
    pub fn custom(test: impl Fn(&RawHandle, &MouseEvent) -> Option<Vec<usize>> + 'static) -> Self {
        Self::Custom(Rc::new(test))
    }

    /// The tolerance the picker gives, if any.
    pub(crate) fn radius(&self) -> Option<f64> {
        match self {
            Self::Radius(radius) => Some(*radius),
            Self::Default | Self::Custom(_) => None,
        }
    }
}

impl fmt::Debug for Picker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Default => f.write_str("Default"),
            Self::Radius(radius) => f.debug_tuple("Radius").field(radius).finish(),
            Self::Custom(_) => f.write_str("Custom(..)"),
        }
    }
}

impl PartialEq for Picker {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Default, Self::Default) => true,
            (Self::Radius(a), Self::Radius(b)) => a == b,
            (Self::Custom(a), Self::Custom(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

/// The indices of the points of a polyline within `radius` of `at`, then
/// of the segments passing within `radius` whose nearest point to `at`
/// lies inside them (matplotlib's `lines.segment_hits`, display units).
pub(crate) fn segment_hits(at: Point, points: &[Point], radius: f64) -> Vec<usize> {
    let near = |p: &Point| (at.x - p.x).powi(2) + (at.y - p.y).powi(2) <= radius * radius;
    let point_hits: Vec<bool> = points.iter().map(near).collect();
    let mut hits: Vec<usize> = (0..points.len()).filter(|&i| point_hits[i]).collect();
    if points.len() <= 1 {
        return hits;
    }
    hits.extend(points.windows(2).enumerate().filter_map(|(i, pair)| {
        let (a, b) = (pair[0], pair[1]);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let u = ((at.x - a.x) * dx + (at.y - a.y) * dy) / (dx * dx + dy * dy);
        let candidate = (0.0..=1.0).contains(&u) && !(point_hits[i] || point_hits[i + 1]);
        let foot = Point::new(a.x + u * dx, a.y + u * dy);
        (candidate && near(&foot)).then_some(i)
    }));
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_and_points_are_hit_like_matplotlib() {
        let line = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
        ];
        assert_eq!(segment_hits(Point::new(5.0, 2.0), &line, 3.0), [0]);
        assert_eq!(
            segment_hits(Point::new(9.0, 1.0), &line, 3.0),
            [1],
            "near a vertex"
        );
        assert_eq!(
            segment_hits(Point::new(5.0, 5.0), &line, 3.0),
            Vec::<usize>::new()
        );
        assert_eq!(segment_hits(Point::new(1.0, 1.0), &line[..1], 3.0), [0]);
    }

    #[test]
    fn pickers_compare_by_identity() {
        let custom = Picker::custom(|_, _| None);
        assert_eq!(custom, custom.clone());
        assert_ne!(custom, Picker::custom(|_, _| None));
        assert_eq!(Picker::Radius(2.0).radius(), Some(2.0));
    }
}
