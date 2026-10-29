//! matplotlib's `LockableBbox`: a box following another, with some of its
//! edges locked at fixed values.

use pltrs_core::Bbox;

/// A box following another, its child, except for the edges locked at
/// fixed values (matplotlib's `LockableBbox`, which polar axes use to pin
/// the radius at the centre while the view limits change).
///
/// matplotlib's child is a box changed in place by whoever owns it. pltrs
/// boxes are values, so the child is a field: change it, or assign the box
/// it should follow now, and [`bbox`](Self::bbox) follows. The locks are
/// the `locked_*` fields, as matplotlib's `locked_x0` ... `locked_y1`
/// properties: a value locks the edge, `None` lets it follow the child.
/// Edges are the stored corners `p0` and `p1`, so a flipped child keeps
/// its orientation.
///
/// ```python
/// child = Bbox.unit()
/// locked = LockableBbox(child, x1=2)
/// child.set_points(child.get_points() + 10)
/// assert locked.locked_x1 == 2
/// assert locked.bounds == (10, 10, -8, 1)
/// locked.locked_x1 = None
/// ```
///
/// ```
/// use pltrs::transforms::{Bbox, LockableBbox};
///
/// let mut locked = LockableBbox {
///     locked_x1: Some(2.0),
///     ..LockableBbox::new(Bbox::UNIT)
/// };
/// locked.child = Bbox::from_extents(10.0, 10.0, 11.0, 11.0);
/// assert_eq!(locked.bbox(), Bbox::from_extents(10.0, 10.0, 2.0, 11.0));
/// locked.locked_x1 = None;
/// assert_eq!(locked.bbox(), locked.child);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LockableBbox {
    /// The box followed (matplotlib's `bbox` argument).
    pub child: Bbox,
    /// The value `x0` is locked at, if any (`locked_x0`).
    pub locked_x0: Option<f64>,
    /// The value `y0` is locked at, if any (`locked_y0`).
    pub locked_y0: Option<f64>,
    /// The value `x1` is locked at, if any (`locked_x1`).
    pub locked_x1: Option<f64>,
    /// The value `y1` is locked at, if any (`locked_y1`).
    pub locked_y1: Option<f64>,
}

impl LockableBbox {
    /// A box following `child`, no edge locked (matplotlib's
    /// `LockableBbox(bbox)`).
    pub const fn new(child: Bbox) -> Self {
        Self {
            child,
            locked_x0: None,
            locked_y0: None,
            locked_x1: None,
            locked_y1: None,
        }
    }

    /// The box now: the child's corners with the locked edges replaced by
    /// their values (matplotlib's `get_points`).
    pub fn bbox(&self) -> Bbox {
        let Bbox { p0, p1 } = self.child;
        Bbox::from_extents(
            self.locked_x0.unwrap_or(p0.x),
            self.locked_y0.unwrap_or(p0.y),
            self.locked_x1.unwrap_or(p1.x),
            self.locked_y1.unwrap_or(p1.y),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The edges of `bbox`, `[x0, y0, x1, y1]`.
    fn edges(bbox: Bbox) -> [f64; 4] {
        [bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1()]
    }

    /// The lock of edge `index` of `locked`, in the order of [`edges`].
    fn lock(locked: &mut LockableBbox, index: usize) -> &mut Option<f64> {
        match index {
            0 => &mut locked.locked_x0,
            1 => &mut locked.locked_y0,
            2 => &mut locked.locked_x1,
            _ => &mut locked.locked_y1,
        }
    }

    /// matplotlib's `test_lockable_bbox`, for each edge in turn.
    #[test]
    fn locked_edges_stay_and_the_others_follow() {
        let moved = Bbox::from_extents(10.0, 10.0, 11.0, 11.0);
        for index in 0..4 {
            let expected = |child: Bbox, value: f64| {
                let mut edges = edges(child);
                edges[index] = value;
                edges
            };
            let mut locked = LockableBbox::new(Bbox::UNIT);
            *lock(&mut locked, index) = Some(2.0);
            assert_eq!(edges(locked.bbox()), expected(Bbox::UNIT, 2.0));
            locked.child = moved;
            assert_eq!(
                edges(locked.bbox()),
                expected(moved, 2.0),
                "the lock survives the child"
            );
            *lock(&mut locked, index) = None;
            assert_eq!(locked.bbox(), moved, "unlocked, it follows");
            *lock(&mut locked, index) = Some(3.0);
            assert_eq!(edges(locked.bbox()), expected(moved, 3.0));
        }
    }

    #[test]
    fn flipped_nan_and_fully_locked_boxes() {
        let flipped = LockableBbox {
            locked_y0: Some(-1.0),
            ..LockableBbox::new(Bbox::from_extents(4.0, 3.0, 1.0, 0.0))
        };
        assert_eq!(flipped.bbox(), Bbox::from_extents(4.0, -1.0, 1.0, 0.0));

        let nan = LockableBbox {
            locked_x0: Some(f64::NAN),
            locked_x1: Some(5.0),
            ..LockableBbox::new(Bbox::from_extents(0.0, f64::NAN, 1.0, 1.0))
        };
        let bbox = nan.bbox();
        assert!(bbox.x0().is_nan() && bbox.y0().is_nan());
        assert_eq!((bbox.x1(), bbox.y1()), (5.0, 1.0));

        let pinned = LockableBbox {
            child: Bbox::NULL,
            locked_x0: Some(0.0),
            locked_y0: Some(1.0),
            locked_x1: Some(2.0),
            locked_y1: Some(3.0),
        };
        assert_eq!(pinned.bbox(), Bbox::from_extents(0.0, 1.0, 2.0, 3.0));
    }
}
