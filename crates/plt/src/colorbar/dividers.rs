//! The dividers of a colorbar with `drawedges`: lines between its colours
//! inside the view and at the extended ends of the view, placed when drawn
//! from the long axis' limits as they are then (matplotlib's
//! `_update_dividers`, which runs again whenever those limits change).

use std::any::Any;

use plt_core::Path;

use super::artists::across;
use super::long_axis;
use crate::artist::{ArtistId, LayoutContext};
use crate::axes::AxesState;
use crate::collections::DrawTimeShapes;
use crate::orientation::Orientation;
use crate::transforms::Transform;

/// Where the dividers of a colorbar go.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Dividers {
    /// The colorbar axes.
    pub(crate) axes: ArtistId,
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

impl DrawTimeShapes for Dividers {
    fn shapes(&self, ctx: LayoutContext<'_>) -> Result<(Vec<Path>, Transform), String> {
        let axes = ctx
            .store()
            .get_as::<AxesState>(self.axes)
            .ok_or("the colorbar axes is gone")?;
        let view = axes.view_interval(long_axis(self.orientation));
        Ok((self.segments(view), Transform::Data(self.axes)))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;

    #[test]
    fn dividers_follow_the_view_and_mark_extended_ends() {
        let dividers = Dividers {
            axes: Figure::new().subplot().id(),
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
