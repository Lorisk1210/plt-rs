//! The dividers of a colorbar: with `drawedges`, lines between its colours
//! inside the view and at the extended ends of the view, placed from the
//! long axis' limits as they are (matplotlib's `_update_dividers`, which
//! runs again whenever those limits change); without, no lines.

use pltrs_core::Path;

use super::artists::across;
use crate::orientation::Orientation;

/// Where the dividers of a colorbar go.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Dividers {
    pub(crate) orientation: Orientation,
    /// The block edges along the long axis.
    pub(crate) edges: Vec<f64>,
    /// Whether the lower end is extended.
    pub(crate) lower: bool,
    /// Whether the upper end is extended.
    pub(crate) upper: bool,
}

impl Dividers {
    /// The positions along the long axis for the view `(lo, hi)`, in the
    /// axis' order: the edges strictly inside, then the view's ends where
    /// they are extended.
    fn positions(&self, (lo, hi): (f64, f64)) -> Vec<f64> {
        let mut positions: Vec<f64> = self
            .edges
            .iter()
            .copied()
            .filter(|&y| lo < y && y < hi)
            .collect();
        if self.lower {
            positions.insert(0, lo);
        }
        if self.upper {
            positions.push(hi);
        }
        positions
    }

    /// The segments across the colorbar for the view `(lo, hi)`, in data
    /// coordinates.
    pub(super) fn segments(&self, view: (f64, f64)) -> Vec<Path> {
        across(self.orientation, &self.positions(view))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dividers_follow_the_view_and_mark_extended_ends() {
        let dividers = Dividers {
            orientation: Orientation::Horizontal,
            edges: vec![0.0, 1.0, 2.0, 3.0, 4.0],
            lower: true,
            upper: false,
        };
        assert_eq!(dividers.positions((0.0, 4.0)), [0.0, 1.0, 2.0, 3.0]);
        assert_eq!(dividers.positions((1.1, 2.9)), [1.1, 2.0]);
        let both = Dividers {
            upper: true,
            ..dividers
        };
        assert_eq!(both.positions((1.1, 2.9)), [1.1, 2.0, 2.9]);
        // An inverted view keeps matplotlib's comparison: nothing inside.
        assert_eq!(both.positions((2.9, 1.1)), [2.9, 1.1]);
    }
}
