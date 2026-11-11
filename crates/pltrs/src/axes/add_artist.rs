//! Adding ready-made artists and collections to an axes (`add_artist`,
//! `add_collection`), filtered groups of artists, artists made outside
//! any figure, and removed artists back.

use std::collections::BTreeSet;
use std::rc::Rc;

use super::children::DataUpdate;
use super::{Axes, AxesState};
use crate::artist::{
    AggFilter, Artist, ArtistHandle, ArtistId, ArtistLookup, AttachError, Custom, FilteredGroup,
    FilteredGroupArtist, RawHandle, take_in,
};
use crate::collections::{Collection, CollectionSpec, TriMesh, TriMeshSpec};
use crate::contour::ContourSetArtist;
use crate::figure::attach;
use crate::legend::LegendArtist;
use crate::patches::{Patch, PatchArtist, PatchProps, PatchShape};
use crate::transforms::NoFigure;

/// Which of matplotlib's `Axes.add_*` methods [`Axes::adopt_artist`]
/// follows:
/// they differ in how the artist counts in the data limits and whether an
/// unlabelled artist gets a `_child<n>` label.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AddAs {
    /// `add_artist` (and `add_table`): the data limits are left alone.
    Artist,
    /// `_add_text`, how `text` adds the text it made: as
    /// [`AddAs::Artist`], but not clipped to the axes.
    Text,
    /// `add_line`: extends the data limits, counts in
    /// [`relim`](Axes::relim), and labels an unlabelled line.
    Line,
    /// `add_patch`: extends the data limits and counts in
    /// [`relim`](Axes::relim).
    Patch,
    /// `add_image`: counts in [`relim`](Axes::relim), labels an
    /// unlabelled image and, unlike the others, does not clip it to the
    /// axes (images clip themselves to the axes box when drawn).
    Image,
    /// `add_collection`: labels an unlabelled collection; with `autolim`,
    /// extends the data limits, autoscales the view and counts in
    /// [`relim`](Axes::relim).
    Collection {
        /// Whether the collection takes part in the limits.
        autolim: bool,
    },
}

impl Axes {
    /// Adds a patch in data coordinates (unless its shape chose others),
    /// clipped to the axes unless the shape turns clipping off, without
    /// touching the data limits (matplotlib's `add_artist`).
    pub fn add_artist<S: PatchShape>(&self, shape: S) -> Patch<S> {
        self.add_artist_with(shape, PatchProps::default())
    }

    /// [`add_artist`](Self::add_artist) for a patch styled by `props`
    /// before it is added, as matplotlib adds a patch made with its
    /// keyword arguments: a `clip_path` replaces the axes clip box.
    pub fn add_artist_with<S: PatchShape>(&self, shape: S, props: PatchProps) -> Patch<S> {
        let mut patch = PatchArtist::new(shape, crate::rc::params());
        props.apply(&mut patch);
        let base = patch.base_mut();
        if base.transform.is_none() {
            base.transform = Some(self.trans_data());
        }
        Patch::from_raw(self.adopt(patch, &DataUpdate::default()))
    }

    /// Adds an artist of your own (matplotlib's `add_artist` of an
    /// `Artist` subclass; see [`Custom`] for an example): in data
    /// coordinates unless its base has a transform, clipped to the axes
    /// unless it has a clip path, drawn with the other children in
    /// z-order, and leaving the data limits alone (extend them with
    /// [`Axes::update_datalim`]).
    pub fn add_custom_artist<A: Artist>(&self, mut artist: A) -> Custom<A> {
        let base = artist.base_mut();
        if base.transform.is_none() {
            base.transform = Some(self.trans_data());
        }
        Custom::from_raw(self.adopt(artist, &DataUpdate::default()))
    }

    /// Draws `members` a second time, together, through `filter` as one
    /// rasterized layer, at the z-order of the returned group (0 until
    /// changed): the agg filter example's `FilteredArtistList`, which
    /// draws glows behind texts and shadows behind groups of patches. The
    /// members are drawn on their own as well.
    ///
    /// ```
    /// use pltrs::artist::{AggFilter, ArtistHandle, Filtered};
    /// use pltrs::prelude::*;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// let line = ax.plot([0.0, 1.0], [0.0, 1.0])?;
    /// let shifted = AggFilter::new(|image, _dpi| Filtered { image: image.clone(), offset: (3.0, -3.0) });
    /// ax.add_filtered_group(&[line.raw().clone()], shifted).set_zorder(line.zorder() - 0.1);
    /// fig.to_png()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn add_filtered_group(&self, members: &[RawHandle], filter: AggFilter) -> FilteredGroup {
        let ids = members.iter().map(RawHandle::id).collect();
        let group = FilteredGroupArtist::new(ids, filter);
        FilteredGroup::from_raw(self.adopt(group, &DataUpdate::default()))
    }

    /// Adds a collection, in data coordinates unless it is drawn in display
    /// space at offsets, clipped to the axes, and extends the data limits
    /// and autoscales to cover it (matplotlib's `add_collection`) unless it
    /// was made [`without_autolim`](CollectionSpec::without_autolim).
    pub fn add_collection(&self, spec: CollectionSpec) -> Collection {
        let mut artist = spec.artist;
        let base = artist.base_mut();
        if base.transform.is_none() {
            base.transform = Some(self.trans_data());
        }
        let handle = if spec.autolim {
            let update = DataUpdate::autoscaled(
                artist.limit_points(self.id(), &*self.raw().figure_state().borrow()),
            );
            self.adopt_collection(artist, &update)
        } else {
            self.adopt(artist, &DataUpdate::default())
        };
        Collection::from_raw(handle)
    }

    /// Adds a triangle mesh in data coordinates (matplotlib's
    /// `add_collection` with a `TriMesh`).
    pub fn add_trimesh(&self, spec: TriMeshSpec) -> TriMesh {
        let mut artist = spec.artist;
        artist.base_mut().transform = Some(self.trans_data());
        let update = DataUpdate::autoscaled(
            artist.limit_points(self.id(), &*self.raw().figure_state().borrow()),
        );
        TriMesh::from_raw(self.adopt_collection(artist, &update))
    }

    /// Adds a ready-made artist to the axes as matplotlib's `add_*` method
    /// `add_as` names does, and returns its handle in this figure: an
    /// artist made outside any figure ([`RawHandle::is_loose`]) moves in
    /// under a new id, which the returned handle refers to, while an
    /// artist removed from this figure (or only given it, with
    /// [`Figure::take_artist`](crate::figure::Figure::take_artist)) comes
    /// back under its own id, keeping its coordinates; either takes the
    /// axes' data coordinates (`ax.transData`) unless it has a transform.
    /// An artist of this axes is listed (and drawn) once more, as
    /// matplotlib's `add_*` methods append it again. Without a clip path
    /// the artist is clipped to the axes (but for images), it
    /// is drawn above the children of equal z-order, and lines, images and
    /// collections without a label are labelled `_child<n>`, `n` counting
    /// the children before them.
    ///
    /// ```
    /// use pltrs::artist::{ArtistHandle, TypedHandle};
    /// use pltrs::axes::AddAs;
    /// use pltrs::lines::{Line2D, LineProps};
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let line = Line2D::loose(vec![0.0, 4.0], vec![1.0, 3.0], LineProps::default());
    /// let line = Line2D::from_raw_handle(&ax.adopt_artist(line.raw(), AddAs::Line)?).unwrap();
    /// assert_eq!(ax.lines(), [line.clone()]);
    /// assert_eq!(line.label(), "_child0");
    /// assert_eq!(ax.data_lim().bbox.p1.x, 4.0);
    /// # Ok::<(), pltrs::artist::AttachError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`AttachError`] when the artist belongs to another figure, is still
    /// in this one under another container (or was removed only with an
    /// artist owning it), is a figure or a 3-D axes, or owns this axes.
    pub fn adopt_artist(
        &self,
        artist: &RawHandle,
        add_as: AddAs,
    ) -> Result<RawHandle, AttachError> {
        let figure = self.raw().figure_state();
        let artist = take_in(figure, artist)?;
        // matplotlib's add methods but `add_image` (and `_add_text`) clip
        // what they add to the axes' patch unless it has a clip path.
        let clipped = !matches!(add_as, AddAs::Image | AddAs::Text);
        if self.has_child(&artist) {
            // matplotlib lists (and draws) an artist added again once more,
            // clipping it again (a legend added after another replaced it).
            self.raw
                .with_mut::<AxesState, _>(|axes| axes.children.push(artist.id()));
            if let Some(added) = figure.borrow_mut().artist_mut(artist.id()) {
                let base = added.base_mut();
                if clipped && base.clip_path.is_none() {
                    base.clip_box = Some(self.id());
                }
            }
            return Ok(artist);
        }
        let data = self.trans_data();
        let child_count = self.listed_children();
        attach(figure, self.id(), &artist)?;
        let labelled = matches!(
            add_as,
            AddAs::Line | AddAs::Image | AddAs::Collection { .. }
        );
        let counted = !matches!(
            add_as,
            AddAs::Artist | AddAs::Text | AddAs::Collection { autolim: false }
        );
        let adopted = figure.borrow_mut().artist_mut(artist.id()).map(|adopted| {
            let base = adopted.base_mut();
            base.transform.get_or_insert(data);
            if clipped && base.clip_path.is_none() {
                base.clip_box = Some(self.id());
            }
            if labelled && base.label.is_empty() {
                base.label = format!("_child{child_count}");
            }
            base.in_autoscale |= counted;
        });
        // A patch whose path a style of the program's makes is measured
        // apart from the figure, which the style may read and change
        // (matplotlib's `_update_patch_limits`); everything else with the
        // figure as it is, for what is measured through display space.
        let shared = adopted.and_then(|()| figure.borrow().store.shared(artist.id()));
        let limits = shared.and_then(|adopted| {
            if adopted.limits_run_program() {
                adopted.limit_points(self.id(), &NoFigure)
            } else {
                adopted.limit_points(self.id(), &*figure.borrow())
            }
        });
        match add_as {
            AddAs::Artist | AddAs::Text | AddAs::Image | AddAs::Collection { autolim: false } => {}
            AddAs::Line | AddAs::Patch => self.update_data(&DataUpdate::extending(limits)),
            AddAs::Collection { autolim: true } => {
                self.unstale_view();
                self.update_data(&DataUpdate::autoscaled(limits));
            }
        }
        Ok(artist)
    }

    /// Keeps `artist`, made for this axes, outside the figure as a removed
    /// artist with the artists it owns (matplotlib's constructors that
    /// take the axes, `Quiver(ax, ...)`, and leave adding the artist to an
    /// `add_*` method): [`Axes::adopt_artist`] adds it.
    pub(crate) fn removed(&self, artist: impl Artist) -> RawHandle {
        let figure = self.raw().figure_state();
        let id = figure.borrow_mut().insert_removed(Box::new(artist));
        RawHandle::new(Rc::clone(figure), id)
    }

    /// Whether `artist` is a child of this axes now.
    fn has_child(&self, artist: &RawHandle) -> bool {
        Rc::ptr_eq(self.raw().figure_state(), artist.figure_state())
            && artist.figure_state().borrow().is_attached(artist.id())
            && self
                .raw
                .with::<AxesState, _>(|axes| axes.children.contains(&artist.id()))
    }

    /// How many children matplotlib's `ax._children` would hold: the
    /// children but the spines, the titles and the legend, a contour set
    /// counting once.
    pub(crate) fn listed_children(&self) -> usize {
        let figure = self.raw().figure_state().borrow();
        self.raw().with::<AxesState, _>(|axes| {
            // A contour set is one child, whatever collections draw it.
            let drawing: BTreeSet<ArtistId> = axes
                .children
                .iter()
                .filter_map(|&id| figure.artist_as::<ContourSetArtist>(id))
                .flat_map(|contour| contour.collections.iter().copied())
                .collect();
            axes.children
                .iter()
                .filter(|&&id| !axes.is_own_part(id))
                .filter(|id| !drawing.contains(id))
                .filter(|&&id| figure.artist_as::<LegendArtist>(id).is_none())
                .filter(|&&id| {
                    figure
                        .artist(id)
                        .is_none_or(|artist| !super::children::is_axisartist_part(artist))
                })
                .count()
        })
    }

    /// Adds `artist`, removed from this axes' figure with
    /// [`ArtistHandle::remove`], back as an artist of this axes, with the
    /// artists it owns (matplotlib's `add_artist`, `add_line`,
    /// `add_patch`, ... of a removed artist): drawn above the children of
    /// equal z-order, clipped to the axes unless it has a clip path, and
    /// extending the data limits by what it covers and counted by
    /// [`relim`](Self::relim), as `add_line` and `add_patch` do. It keeps
    /// its coordinate system, so an artist moved from another axes stays
    /// in that one's data coordinates.
    ///
    /// ```
    /// use pltrs::artist::AttachError;
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let line = ax.plot([0.0, 1.0], [1.0, 0.0])?;
    /// line.remove();
    /// assert!(ax.lines().is_empty());
    /// ax.attach(&line)?;
    /// assert_eq!(ax.lines(), [line.clone()]);
    /// assert_eq!(ax.attach(&line).unwrap_err(), AttachError::NotRemoved);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`AttachError`] when the artist belongs to another figure, was not
    /// removed (or only with an artist owning it), is a figure or a 3-D
    /// axes, or owns this axes.
    pub fn attach(&self, artist: &impl ArtistHandle) -> Result<&Self, AttachError> {
        if artist.raw().is_loose() {
            return Err(AttachError::OtherFigure);
        }
        if self.has_child(artist.raw()) {
            return Err(AttachError::NotRemoved);
        }
        self.adopt_artist(artist.raw(), AddAs::Patch)?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use pltrs_core::Point;

    use super::*;
    use crate::artist::TypedHandle;
    use crate::figure::Figure;
    use crate::lines::{Line2D, LineProps};
    use crate::patches::Rectangle;
    use crate::transforms::Transform;

    fn loose_line() -> Line2D {
        Line2D::loose(vec![0.0, 4.0], vec![1.0, 3.0], LineProps::default())
    }

    #[test]
    fn a_loose_line_moves_in_as_add_line_adds_it() {
        let ax = Figure::new().subplot();
        let adopted = ax.adopt_artist(loose_line().raw(), AddAs::Line).unwrap();
        let line = Line2D::from_raw_handle(&adopted).unwrap();
        assert_eq!(ax.lines(), std::slice::from_ref(&line));
        assert_eq!(line.label(), "_child0");
        assert_eq!(line.transform(), Some(Transform::Data(ax.id())));
        assert_eq!(ax.data_lim().bbox.p1, Point::new(4.0, 3.0));
        let second = ax.adopt_artist(loose_line().raw(), AddAs::Line).unwrap();
        assert_eq!(second.label(), "_child1", "the first line counts");
        let other = Figure::new().subplot();
        assert_eq!(
            other.adopt_artist(&adopted, AddAs::Line),
            Err(AttachError::OtherFigure)
        );
        ax.adopt_artist(&adopted, AddAs::Line).unwrap();
        assert_eq!(ax.lines().len(), 3, "an artist added again is listed again");
    }

    #[test]
    fn add_artist_leaves_the_limits_alone_and_keeps_a_given_transform() {
        let ax = Figure::new().subplot();
        let patch = Patch::loose(
            Rectangle::new(Point::new(5.0, 5.0), 1.0, 1.0),
            PatchProps::default(),
        );
        patch.set_transform(Transform::Axes(ax.id()));
        let adopted = ax.adopt_artist(patch.raw(), AddAs::Artist).unwrap();
        assert!(ax.data_lim().bbox.is_null());
        assert_eq!(adopted.transform(), Some(Transform::Axes(ax.id())));
        assert_eq!(adopted.label(), "", "add_artist labels nothing");
        assert_eq!(ax.patches(), [adopted]);
    }

    #[test]
    fn collections_take_part_in_the_limits_with_autolim_only() {
        let ax = Figure::new().subplot();
        let segments = vec![vec![Point::new(0.0, 0.0), Point::new(2.0, 2.0)]];
        let without = Collection::loose(CollectionSpec::lines(segments.clone()));
        ax.adopt_artist(without.raw(), AddAs::Collection { autolim: false })
            .unwrap();
        assert!(ax.data_lim().bbox.is_null());
        let with = Collection::loose(CollectionSpec::lines(segments));
        let adopted = ax
            .adopt_artist(with.raw(), AddAs::Collection { autolim: true })
            .unwrap();
        assert_eq!(ax.data_lim().bbox.p1, Point::new(2.0, 2.0));
        assert_eq!(adopted.label(), "_child1");
        assert_eq!(ax.collections().len(), 2);
    }

    #[test]
    fn artists_of_other_figures_are_refused() {
        let ax = Figure::new().subplot();
        let line = Figure::new()
            .subplot()
            .plot([0.0, 1.0], [0.0, 1.0])
            .unwrap();
        assert_eq!(
            ax.adopt_artist(line.raw(), AddAs::Line),
            Err(AttachError::OtherFigure)
        );
        assert_eq!(
            ax.attach(&loose_line()).unwrap_err(),
            AttachError::OtherFigure,
            "attach takes removed artists only"
        );
    }

    #[test]
    fn figures_adopt_loose_artists_in_figure_fractions() {
        let fig = Figure::new();
        let adopted = fig.adopt_artist(loose_line().raw(), true).unwrap();
        assert_eq!(adopted.transform(), Some(Transform::Figure(fig.id())));
        assert_eq!(fig.artists(), [adopted]);
    }
}
