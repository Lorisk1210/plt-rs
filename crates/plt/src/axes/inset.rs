//! Inset axes: an axes placed inside another one (matplotlib's
//! `Axes.inset_axes`), in its axes fractions or in other coordinates.

use std::rc::Rc;

use plt_core::Bbox;

use super::{Axes, AxesState};
use crate::artist::{ArtistHandle, ArtistLookup, RawHandle};
use crate::axes_grid1::locate_on_bounds;
use crate::figure::insert_axes;
use crate::transforms::Transform;

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
    /// inset.set_xlim(1.0, 2.0)?;
    /// assert_eq!(fig.axes(), [ax]);
    /// assert_eq!(inset.zorder(), 5.0);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn inset_axes(&self, bounds: [f64; 4]) -> Self {
        let [left, bottom, width, height] = bounds;
        let position = Bbox::from_bounds(left, bottom, width, height);
        let inset = insert_axes(self.raw().figure_state(), self.id(), position, None);
        inset.set_zorder(INSET_ZORDER);
        inset
    }

    /// Adds an axes at `[left, bottom, width, height]` in the coordinates
    /// of `transform`, such as this axes' data coordinates, placed there
    /// anew at every layout pass and drawn above this axes' contents
    /// (matplotlib's `inset_axes(bounds, transform=...)`). Like
    /// [`Axes::inset_axes`], it belongs to this axes.
    ///
    /// ```
    /// use plt::prelude::*;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// ax.set_xlim(0.0, 10.0)?.set_ylim(0.0, 10.0)?;
    /// let inset = ax.inset_axes_in([5.0, 5.0, 5.0, 2.5], ax.trans_data());
    /// let (x, y, width, height) = inset.position().bounds();
    /// let (px, py, pwidth, pheight) = ax.position().bounds();
    /// assert!((x - (px + 0.5 * pwidth)).abs() < 1e-12 && (width - 0.5 * pwidth).abs() < 1e-12);
    /// assert!((y - (py + 0.5 * pheight)).abs() < 1e-12 && (height - 0.25 * pheight).abs() < 1e-12);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn inset_axes_in(&self, bounds: [f64; 4], transform: Transform) -> Self {
        let inset = self.inset_axes(bounds);
        let [left, bottom, width, height] = bounds;
        locate_on_bounds(
            self,
            &inset,
            Bbox::from_bounds(left, bottom, width, height),
            transform,
        );
        inset
    }

    /// The axes placed inside this one, in creation order: its insets and
    /// secondary axes (matplotlib's `child_axes`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let inset = ax.inset_axes([0.6, 0.6, 0.35, 0.3]);
    /// assert_eq!(ax.child_axes(), [inset]);
    /// ```
    pub fn child_axes(&self) -> Vec<Self> {
        let figure = self.raw().figure_state();
        let state = figure.borrow();
        let children = self
            .raw()
            .with::<AxesState, _>(|axes| axes.children.clone());
        children
            .into_iter()
            .filter(|&id| state.artist_as::<AxesState>(id).is_some())
            .map(|id| Self::from_raw(RawHandle::new(Rc::clone(figure), id)))
            .collect()
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
        ax.set_xlim(0.0, 2.0)
            .and_then(|ax| ax.set_ylim(0.0, 1.0))
            .expect("finite limits");
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

    #[test]
    fn insets_keep_their_aspect_and_report_figure_fractions() {
        // matplotlib's test_zoom_inset.
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xlim(1.0, 5.0)
            .and_then(|ax| ax.set_ylim(1.0, 5.0))
            .expect("finite limits");
        ax.set_aspect(Aspect::Equal).unwrap();
        let inset = ax.inset_axes([0.7, 0.7, 0.35, 0.35]);
        inset
            .set_xlim(1.5, 2.15)
            .and_then(|ax| ax.set_ylim(2.0, 2.5))
            .expect("finite limits");
        inset.set_aspect(Aspect::Equal).unwrap();
        fig.layout();
        let expected = [
            0.628,
            0.680_096_153_846_153_7,
            0.830_125,
            0.887_403_846_153_845_9,
        ];
        let got = inset.position();
        for (got, want) in [got.x0(), got.y0(), got.x1(), got.y1()]
            .into_iter()
            .zip(expected)
        {
            assert!((got - want).abs() < 1e-12, "{got} != {want}");
        }
        assert_eq!(inset.original_position().x0(), 0.7, "the bounds as given");
    }
}
