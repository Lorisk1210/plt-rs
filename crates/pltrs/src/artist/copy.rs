//! Copying the common properties of one artist to another (matplotlib's
//! `Artist.update_from`).

use super::{ArtistBase, RawHandle};

impl ArtistBase {
    /// Takes the properties matplotlib's `Artist.update_from` copies from
    /// `other`: visibility, alpha, label, clipping on or off, sketch, path
    /// effects and sticky edges, and with `references` the transform, the
    /// clip box and the clip path too, which refer to artists of a figure
    /// by id.
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
    fn an_artist_updated_from_itself_is_unchanged() {
        let ax = Figure::new().subplot();
        let line = ax.plot([0.0, 1.0], [1.0, 0.0]).expect("a line");
        line.set_label("same");
        assert!(line.raw().update_from(&line.raw().clone()));
        assert_eq!(line.label(), "same");
        assert_eq!(line.transform(), Some(ax.trans_data()));
    }
}
