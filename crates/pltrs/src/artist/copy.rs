//! Copying the common properties of one artist to another (matplotlib's
//! `Artist.update_from`), and copying an artist apart from its figure's
//! tree (matplotlib's `copy.copy` of an artist).

use std::rc::Rc;

use super::{ArtistBase, ArtistLookup, RawHandle};

impl ArtistBase {
    /// Takes the properties matplotlib's `Artist.update_from` copies from
    /// `other`: visibility, alpha, label, clipping on or off, sketch, path
    /// effects and sticky edges, and with `references` the transform, the
    /// clip box and the clip path too, which refer to artists of a figure
    /// by id. Without `references`, a clip region `other` lacks is taken
    /// still: having none means the same in any figure.
    pub fn update_from(&mut self, other: &Self, references: bool) {
        self.visible = other.visible;
        self.alpha = other.alpha;
        self.label.clone_from(&other.label);
        self.clip_on = other.clip_on;
        self.sketch = other.sketch;
        self.path_effects.clone_from(&other.path_effects);
        self.sticky_edges.clone_from(&other.sticky_edges);
        if references {
            self.transform.clone_from(&other.transform);
            self.clip_box = other.clip_box;
            self.clip_path.clone_from(&other.clip_path);
        } else {
            if other.clip_box.is_none() {
                self.clip_box = None;
            }
            if other.clip_path.is_none() {
                self.clip_path = None;
            }
        }
    }
}

impl RawHandle {
    /// Gives the artist the common properties of `other` (matplotlib's
    /// `Artist.update_from`; see [`ArtistBase::update_from`]). The
    /// transform and the clip regions are copied when they mean the same
    /// for both artists: when the artists share a figure, or when this one
    /// is loose, so that it keeps them on joining `other`'s figure; the
    /// result says whether they were.
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::lines::{Line2D, LineProps};
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let line = ax.plot([0.0, 1.0], [1.0, 0.0])?;
    /// line.set_label("data").set_alpha(Some(0.5));
    /// let copy = Line2D::loose(vec![0.0], vec![0.0], LineProps::default());
    /// assert!(copy.raw().update_from(line.raw()));
    /// assert_eq!((copy.label(), copy.alpha()), ("data".to_owned(), Some(0.5)));
    /// assert_eq!(copy.transform(), Some(ax.trans_data()));
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn update_from(&self, other: &RawHandle) -> bool {
        let base = other.with_base(Clone::clone);
        let references = self.shares_figure(other) || self.is_loose();
        self.with_base_mut(|own| own.update_from(&base, references));
        references
    }

    /// A copy of the artist under a new id (matplotlib's `copy.copy` of an
    /// artist): owned by the artist's owner without being one of its
    /// children, so that its figure does not draw it, and kept by the
    /// figure apart from its tree, as a removed artist is. The artists the
    /// artist owns are shared with the copy, not copied.
    ///
    /// ```
    /// use pltrs::artist::{ArtistHandle, TypedHandle};
    /// use pltrs::lines::Line2D;
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let line = ax.plot([0.0, 1.0], [1.0, 0.0])?;
    /// let copy = Line2D::from_raw_handle(&line.raw().detached_copy()).expect("a line");
    /// copy.set_label("copy");
    /// assert_ne!(line.label(), "copy");
    /// assert_eq!(copy.axes(), Some(ax.clone()));
    /// assert_eq!(ax.lines(), [line]);
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    #[must_use]
    pub fn detached_copy(&self) -> Self {
        let figure = Rc::clone(self.figure_state());
        let id = {
            let mut state = figure.borrow_mut();
            let copy = state
                .artist(self.id())
                .expect("a handle reaches its artist")
                .clone_artist();
            let id = state.store.insert(copy);
            let kept = state.store.take(id).expect("the copy was just inserted");
            state.detached.insert(id, kept);
            id
        };
        Self::new(figure, id)
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::transforms::Transform;

    #[test]
    fn artists_of_another_figure_keep_their_transform() {
        let ax = Figure::new().subplot();
        let line = ax.plot([0.0, 1.0], [1.0, 0.0]).expect("a line");
        line.set_visible(false)
            .set_clip_on(false)
            .set_sticky_edges(vec![0.0], vec![1.0, 2.0])
            .set_transform(Transform::Display);
        let other = Figure::new().subplot();
        let target = other.plot([0.0], [0.0]).expect("a line");
        assert!(!target.raw().update_from(line.raw()));
        assert!(!target.visible() && !target.clip_on());
        assert_eq!(target.sticky_edges(), (vec![0.0], vec![1.0, 2.0]));
        assert_eq!(target.transform(), Some(other.trans_data()));
        assert_eq!(target.clip_box(), Some(other.id()));
        let sibling = ax.plot([0.0], [0.0]).expect("a line");
        assert!(sibling.raw().update_from(line.raw()));
        assert_eq!(sibling.transform(), Some(Transform::Display));
    }

    #[test]
    fn no_clip_box_is_taken_from_any_figure() {
        let ax = Figure::new().subplot();
        let line = ax.plot([0.0, 1.0], [1.0, 0.0]).expect("a line");
        assert_eq!(line.clip_box(), Some(ax.id()));
        let loose =
            crate::lines::Line2D::loose(vec![0.0], vec![0.0], crate::lines::LineProps::default());
        assert!(!line.raw().update_from(loose.raw()));
        assert_eq!(line.clip_box(), None);
        assert_eq!(line.transform(), Some(ax.trans_data()));
    }

    #[test]
    fn detached_copies_change_apart_and_are_not_drawn() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let line = ax.plot([0.0, 1.0], [1.0, 0.0]).expect("a line");
        line.set_label("original");
        let copy = crate::lines::Line2D::from_raw(line.raw().detached_copy());
        assert_ne!(copy.id(), line.id());
        assert_eq!(copy.label(), "original");
        copy.set_label("copy").set_linewidth(5.0);
        assert_eq!(
            (line.label(), line.linewidth()),
            ("original".to_owned(), 1.5)
        );
        assert!(!copy.raw().is_attached());
        assert_eq!(copy.axes(), Some(ax.clone()));
        assert_eq!(ax.lines(), std::slice::from_ref(&line));
        copy.remove();
        assert_eq!(
            ax.lines(),
            [line],
            "removing the copy leaves its owner alone"
        );
    }

    #[test]
    fn an_artist_updated_from_itself_is_unchanged() {
        let ax = Figure::new().subplot();
        let line = ax.plot([0.0, 1.0], [1.0, 0.0]).expect("a line");
        line.set_label("same");
        assert!(line.raw().update_from(&line.raw().clone()));
        assert_eq!(line.label(), "same");
        assert_eq!(line.transform(), Some(ax.trans_data()));
    }
}
