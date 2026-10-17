//! Inset axes: an axes placed inside another one (matplotlib's
//! `Axes.inset_axes`).

use plt_core::Bbox;

use super::Axes;
use crate::artist::ArtistHandle;
use crate::figure::insert_axes;

/// The z-order of inset axes within their parent (matplotlib's default).
const INSET_ZORDER: f64 = 5.0;

impl Axes {
    /// Adds an axes at `[left, bottom, width, height]` in this axes'
    /// fractions, drawn above this axes' contents (matplotlib's
    /// `inset_axes` with its default `transform=ax.transAxes`).
    ///
    /// The inset belongs to this axes rather than the figure: it follows
    /// this axes when it moves or shrinks to an aspect, and is not among
    /// [`Figure::axes`](crate::figure::Figure::axes).
    ///
    /// ```
    /// use plt::prelude::*;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// let inset = ax.inset_axes([0.6, 0.6, 0.35, 0.3]);
    /// inset.set_xlim(1.0, 2.0);
    /// assert_eq!(fig.axes(), [ax]);
    /// assert_eq!(inset.zorder(), 5.0);
    /// ```
    #[must_use]
    pub fn inset_axes(&self, bounds: [f64; 4]) -> Self {
        let [left, bottom, width, height] = bounds;
        let position = Bbox::from_bounds(left, bottom, width, height);
        let inset = insert_axes(self.raw().figure_state(), self.id(), position, None);
        inset.set_zorder(INSET_ZORDER);
        inset
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::rc::Aspect;
    use crate::transforms::TransformContext;

    #[test]
    fn insets_follow_the_parent_box() {
        let fig = Figure::with_size(4.0, 4.0, 100.0);
        let ax = fig.subplot();
        ax.set_xlim(0.0, 2.0).set_ylim(0.0, 1.0);
        let inset = ax.inset_axes([0.1, 0.9, 0.8, 0.05]);
        ax.set_aspect(Aspect::Equal).unwrap();
        fig.layout();
        let state = ax.raw().figure_state().borrow();
        let host = state.axes_bbox(ax.id()).unwrap();
        let placed = state.axes_bbox(inset.id()).unwrap();
        assert!((placed.x0() - (host.x0() + 0.1 * host.width())).abs() < 1e-9);
        assert!((placed.width() - 0.8 * host.width()).abs() < 1e-9);
        assert!((placed.y0() - (host.y0() + 0.9 * host.height())).abs() < 1e-9);
    }
}
