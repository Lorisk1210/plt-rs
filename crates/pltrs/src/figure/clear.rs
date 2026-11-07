//! Clearing a figure (matplotlib's `Figure.clear`, `clf` and `delaxes`).

use std::rc::Rc;

use super::frame::SuperTexts;
use super::{Figure, FigureFrame, FigureState};
use crate::artist::{ArtistHandle, ArtistId, RawHandle};
use crate::axes::{Axes, AxesState};
use crate::gridspec::SubplotParams;

impl FigureState {
    /// The children of frame `frame`, none if it is no frame in the figure.
    fn frame_children(&self, frame: ArtistId) -> Vec<ArtistId> {
        self.store
            .get_as::<FigureFrame>(frame)
            .map(|node| node.children.clone())
            .unwrap_or_default()
    }

    /// Removes everything in frame `frame` (matplotlib's
    /// `FigureBase.clear`), its axes cleared already: the axes, the
    /// subfigures, texts, legends and other artists; the subplot
    /// parameters return to the rc defaults.
    fn clear_frame(&mut self, frame: ArtistId) {
        for child in self.frame_children(frame) {
            self.remove_artist(child);
        }
        if let Some(node) = self.store.get_as_mut::<FigureFrame>(frame) {
            node.super_texts = SuperTexts::default();
            node.subplot_params = SubplotParams::from_rc(&crate::rc::params());
        }
    }
}

impl Figure {
    /// Removes the axes, subfigures, texts, legends and every other artist
    /// and resets the subplot parameters to `figure.subplot.*`, keeping
    /// the size, colours and layout engine (matplotlib's `Figure.clear`).
    pub fn clear(&self) -> &Self {
        // Each axes cleared as `Axes::clear` clears it (resetting the
        // scales of the axes shared with it; a polar or custom axes stays
        // so, removed), before it is removed.
        let children = self.state().frame_children(self.id());
        let axes: Vec<Axes> = children
            .into_iter()
            .filter(|&id| self.state().store.get_as::<AxesState>(id).is_some())
            .map(|id| Axes::from_raw(RawHandle::new(Rc::clone(self.state_cell()), id)))
            .collect();
        for ax in &axes {
            ax.clear();
        }
        self.state_mut().clear_frame(self.id());
        self
    }

    /// Clears the figure (matplotlib's `clf`, an alias of
    /// [`Figure::clear`]).
    pub fn clf(&self) -> &Self {
        self.clear()
    }

    /// Removes `ax` from the figure (matplotlib's `delaxes`).
    pub fn delaxes(&self, ax: &Axes) {
        ax.remove();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clearing_removes_everything_but_the_canvas() {
        let fig = Figure::with_size(5.0, 4.0, 80.0);
        fig.subplots(2, 2).unwrap();
        fig.suptitle("title");
        fig.text(0.5, 0.5, "note");
        let sub = fig
            .subfigures(1, 2, super::super::SubfigureOptions::default())
            .unwrap();
        sub[(0, 1)].subplot();
        fig.clear();
        assert_eq!(fig.axes(), Vec::<Axes>::new());
        assert_eq!(fig.size_inches(), (5.0, 4.0));
        // The figure is usable again, with a fresh current axes.
        let ax = fig.gca();
        assert_eq!(fig.axes(), std::slice::from_ref(&ax));
        fig.delaxes(&ax);
        assert_eq!(fig.axes(), Vec::<Axes>::new());
    }

    #[test]
    fn cleared_polar_axes_stay_polar() {
        // Removed with the figure's clear, a polar axes is still one, as
        // matplotlib's `Axes.clear` leaves it.
        let fig = Figure::new();
        let polar = fig.add_polar_axes([0.1, 0.1, 0.8, 0.8]).unwrap();
        fig.clear();
        assert_eq!(polar.projection(), crate::axes::ProjectionKind::Polar);
        assert_eq!(fig.axes(), Vec::<Axes>::new());
    }

    #[test]
    fn clearing_resets_the_subplot_parameters() {
        // matplotlib's test_clf_subplotpars.
        let fig = Figure::new();
        let defaults = fig.subplot_params();
        fig.subplots_adjust(crate::gridspec::SubplotOverrides {
            left: Some(0.2),
            hspace: Some(0.5),
            ..Default::default()
        })
        .unwrap();
        assert_ne!(fig.subplot_params(), defaults);
        fig.clf();
        assert_eq!(fig.subplot_params(), defaults);
    }
}
