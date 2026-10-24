//! Clearing a figure (matplotlib's `Figure.clear`, `clf` and `delaxes`).

use super::frame::SuperTexts;
use super::{Figure, FigureFrame, FigureState};
use crate::artist::{ArtistHandle, ArtistId};
use crate::axes::{Axes, AxesState};
use crate::gridspec::SubplotParams;

impl FigureState {
    /// Removes everything in frame `frame` (matplotlib's
    /// `FigureBase.clear`): each axes is cleared, which resets the scales
    /// of axes shared with it, then removed with the subfigures, texts,
    /// legends and other artists; the subplot parameters return to the
    /// rc defaults.
    fn clear_frame(&mut self, frame: ArtistId) {
        let Some(node) = self.store.get_as::<FigureFrame>(frame) else {
            return;
        };
        let children = node.children.clone();
        for &child in &children {
            if self.store.get_as::<AxesState>(child).is_some() {
                self.clear_axes(child);
            }
        }
        for child in children {
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
    fn clearing_resets_the_subplot_parameters() {
        // matplotlib's test_clf_subplotpars.
        let fig = Figure::new();
        let defaults = fig.subplot_params();
        fig.subplots_adjust(&crate::gridspec::SubplotOverrides {
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
