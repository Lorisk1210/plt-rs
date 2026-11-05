//! The children of an axes: listing them by kind (matplotlib's
//! `ax.collections`, `ax.patches`, `ax.texts`, `ax.tables`,
//! `ax.artists`), adding artists and accounting for them in the data
//! limits.

use std::any::Any;
use std::collections::BTreeSet;
use std::rc::Rc;

use pltrs_core::Point;

use super::{Axes, AxesState, DataLimits};
use crate::artist::{Artist, ArtistHandle, ArtistId, ArtistLookup, LimitPoints, RawHandle};
use crate::axisartist::AxisArtistState;
use crate::collections::{Collection, CollectionArtist};
use crate::colorbar::ColorbarArtist;
use crate::contour::ContourSetArtist;
use crate::image::{AxesImageArtist, NonUniformImageArtist, PcolorImageArtist};
use crate::legend::LegendArtist;
use crate::lines::LineArtist;
use crate::spines::SpineArtist;
use crate::table::{Table, TableArtist};
use crate::text::{Text, TextArtist};

/// How a new child changes the data limits and autoscaling.
#[derive(Clone, Debug, Default)]
pub(crate) struct DataUpdate {
    /// Points to include, in data coordinates for the updated directions.
    pub(crate) points: Vec<Point>,
    /// Whether x and y data limits take the points into account.
    pub(crate) update: (bool, bool),
    /// Whether x and y view limits are re-autoscaled.
    pub(crate) autoscale: (bool, bool),
}

impl DataUpdate {
    /// Points in data coordinates that extend both limits and request
    /// autoscaling in both directions.
    pub(crate) fn data(points: Vec<Point>) -> Self {
        Self {
            points,
            update: (true, true),
            autoscale: (true, true),
        }
    }

    /// The corners and smallest positive coordinates of `limits`,
    /// extending both limits and requesting autoscaling (matplotlib's
    /// `add_collection`).
    pub(crate) fn limits(limits: DataLimits) -> Self {
        Self::data(vec![limits.bbox.p0, limits.bbox.p1, limits.minpos])
    }

    /// What an artist adds to the data limits, without autoscaling the
    /// view (matplotlib's `add_patch` and `add_line`).
    pub(crate) fn extending(limits: Option<LimitPoints>) -> Self {
        let LimitPoints { points, directions } = limits.unwrap_or_default();
        Self {
            points,
            update: directions,
            autoscale: (false, false),
        }
    }

    /// What an artist adds to the data limits, autoscaling the view
    /// (matplotlib's `add_collection`).
    pub(crate) fn autoscaled(limits: Option<LimitPoints>) -> Self {
        Self {
            autoscale: (true, true),
            ..Self::extending(limits)
        }
    }
}

impl Axes {
    /// Handles to the children of type `T`, in the order they were added
    /// (matplotlib's `ax.lines`, `ax.images`, ...).
    pub(crate) fn children_of<T: Artist>(&self) -> Vec<RawHandle> {
        self.children_where(|artist| (artist as &dyn Any).is::<T>())
    }

    /// Handles to the children for which `keep` holds, in the order they
    /// were added; spines are no children in this sense, whatever their
    /// kind (a colorbar's outline is a patch), nor are the collections
    /// drawing the bands of a hatched filled contour set but its first,
    /// which stands for the set (matplotlib's set is one collection,
    /// drawing its bands one by one).
    fn children_where(&self, keep: impl Fn(&dyn Artist) -> bool) -> Vec<RawHandle> {
        let figure = self.raw().figure_state();
        let state = figure.borrow();
        let (children, spines) = self
            .raw()
            .with::<AxesState, _>(|axes| (axes.children.clone(), axes.spines.clone()));
        let bands: BTreeSet<ArtistId> = children
            .iter()
            .filter_map(|&id| state.store.get_as::<ContourSetArtist>(id))
            .flat_map(|contour| contour.collections.iter().skip(1).copied())
            .collect();
        children
            .into_iter()
            .filter(|id| !spines.contains(id) && !bands.contains(id))
            .filter(|&id| state.artist(id).is_some_and(&keep))
            .map(|id| RawHandle::new(Rc::clone(figure), id))
            .collect()
    }

    /// The collections of the axes in the order they were added: those
    /// of scatter plots, fills, error bars, stems, ... (matplotlib's
    /// `ax.collections`; contour sets and Gouraud-shaded triangle meshes
    /// have handles of their own).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// assert!(ax.collections().is_empty());
    /// ax.scatter(vec![0.0, 1.0], vec![1.0, 0.0])?;
    /// assert_eq!(ax.collections().len(), 1);
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn collections(&self) -> Vec<Collection> {
        self.children_of::<CollectionArtist>()
            .into_iter()
            .map(Collection::from_raw)
            .collect()
    }

    /// The patches of the axes in the order they were added, whatever
    /// their shape (matplotlib's `ax.patches`); the background patch and
    /// the spines are not among them.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// assert!(ax.patches().is_empty());
    /// ax.bar([1.0, 2.0], [3.0, 4.0], Default::default())?;
    /// assert_eq!(ax.patches().len(), 2);
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn patches(&self) -> Vec<RawHandle> {
        self.children_where(|artist| artist.as_patch().is_some())
    }

    /// The texts and annotations of the axes in the order they were added
    /// (matplotlib's `ax.texts`); the titles and axis labels are not among
    /// them.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.set_title("title");
    /// assert!(ax.texts().is_empty());
    /// ax.text(0.5, 0.5, "note");
    /// assert_eq!(ax.texts()[0].text(), "note");
    /// ```
    pub fn texts(&self) -> Vec<Text> {
        let titles = self.raw().with::<AxesState, _>(|axes| axes.titles.clone());
        self.children_of::<TextArtist>()
            .into_iter()
            .filter(|text| !titles.contains(&text.id()))
            .map(Text::from_raw)
            .collect()
    }

    /// The tables of the axes in the order they were added (matplotlib's
    /// `ax.tables`).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::table::TableOptions;
    /// let ax = Figure::new().subplot();
    /// assert!(ax.tables().is_empty());
    /// let options = TableOptions { cell_text: vec![vec!["1".into(), "2".into()]], ..Default::default() };
    /// let table = ax.table(options)?;
    /// assert_eq!(ax.tables(), [table]);
    /// # Ok::<(), pltrs::table::TableError>(())
    /// ```
    pub fn tables(&self) -> Vec<Table> {
        self.children_of::<TableArtist>()
            .into_iter()
            .map(Table::from_raw)
            .collect()
    }

    /// The other artists of the axes in the order they were added: those
    /// none of [`lines`](Self::lines), [`patches`](Self::patches),
    /// [`collections`](Self::collections), [`texts`](Self::texts),
    /// [`images`](Self::images) and [`tables`](Self::tables) lists, such as
    /// anchored boxes, annotation boxes, quiver keys and custom artists
    /// (matplotlib's `ax.artists`). The legend, the spines and the axis
    /// decorations are not among them.
    ///
    /// ```
    /// use pltrs::offsetbox::{AnchorLoc, AnchoredBox, AnchoredOptions};
    /// use pltrs::prelude::*;
    /// use pltrs::text::TextProps;
    /// let ax = Figure::new().subplot();
    /// ax.plot([0.0, 1.0], [0.0, 1.0])?;
    /// ax.legend(Default::default())?;
    /// assert!(ax.artists().is_empty());
    /// let note = AnchoredBox::text("note", TextProps::default(), AnchoredOptions::new(AnchorLoc::UpperLeft))?;
    /// let anchored = ax.add_anchored_box(note);
    /// assert_eq!(ax.artists(), [anchored.raw().clone()]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn artists(&self) -> Vec<RawHandle> {
        self.children_where(|artist| !listed_by_kind(artist))
    }

    /// Every artist added to the axes, in the order they were added
    /// (matplotlib's `ax._children`): what [`lines`](Self::lines),
    /// [`patches`](Self::patches), [`collections`](Self::collections),
    /// [`texts`](Self::texts), [`images`](Self::images),
    /// [`tables`](Self::tables) and [`artists`](Self::artists) list. The
    /// spines, the titles, the legend and the child axes are parts of the
    /// axes, not among them.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.set_title("title");
    /// let note = ax.text(0.5, 0.5, "note");
    /// let line = ax.plot([0.0, 1.0], [0.0, 1.0])?;
    /// assert_eq!(ax.added_children(), [note.raw().clone(), line.raw().clone()]);
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn added_children(&self) -> Vec<RawHandle> {
        let parts = self
            .raw()
            .with::<AxesState, _>(|axes| [axes.spines.clone(), axes.titles.clone()].concat());
        self.children_where(|artist| {
            let any = artist as &dyn Any;
            !(any.is::<AxesState>()
                || any.is::<LegendArtist>()
                || any.is::<ColorbarArtist>()
                || any.is::<AxisArtistState>())
        })
        .into_iter()
        .filter(|child| !parts.contains(&child.id()))
        .collect()
    }

    /// Whether a line, patch, collection or image was added to the axes
    /// (matplotlib's `has_data`); texts, tables and the other artists do
    /// not count, nor do removed artists.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.text(0.5, 0.5, "note");
    /// assert!(!ax.has_data());
    /// ax.plot([0.0, 1.0], [0.0, 1.0])?;
    /// assert!(ax.has_data());
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn has_data(&self) -> bool {
        !self.children_where(is_data_artist).is_empty()
    }

    /// Adds `artist` as a child and applies `update` (matplotlib's
    /// `add_artist` bookkeeping, and that of `add_collection(...,
    /// autolim=False)`): an artist without a clip path of its own is
    /// clipped to the axes' box. [`relim`](Self::relim) leaves the artist
    /// out; [`adopt_in_autoscale`](Self::adopt_in_autoscale) adds one it
    /// counts.
    pub(crate) fn adopt(&self, mut artist: impl Artist, update: &DataUpdate) -> RawHandle {
        let axes_id = self.id();
        let label = self.child_label(&artist);
        let base = artist.base_mut();
        if base.clip_path.is_none() {
            base.clip_box = Some(axes_id);
        }
        if let Some(label) = label {
            base.label = label;
        }
        let figure = self.raw().figure_state();
        let mut state = figure.borrow_mut();
        let id = state.adopt(axes_id, Box::new(artist));
        drop(state);
        self.update_data(update);
        RawHandle::new(Rc::clone(figure), id)
    }

    /// The label matplotlib gives `artist` as the axes adds it, `None` to
    /// keep its own: a line, collection or image added without a label is
    /// `_child<n>`, `n` the number of children before it (`add_line`,
    /// `add_collection`, `add_image`).
    pub(crate) fn child_label(&self, artist: &dyn Artist) -> Option<String> {
        (is_labelled_child(artist) && artist.base().label.is_empty())
            .then(|| format!("_child{}", self.listed_children()))
    }

    /// Adds `artist` as [`adopt`](Self::adopt) does, counted by
    /// [`relim`](Self::relim) from then on (matplotlib's `add_line`,
    /// `add_patch` and `add_image`, which set `_in_autoscale`).
    pub(crate) fn adopt_in_autoscale(
        &self,
        mut artist: impl Artist,
        update: &DataUpdate,
    ) -> RawHandle {
        artist.base_mut().in_autoscale = true;
        self.adopt(artist, update)
    }

    /// Adds a collection as a child the way matplotlib's `add_collection`
    /// does when it updates the limits (`autolim=True`): pending
    /// autoscaling is applied to the view first, with the collection among
    /// the children but not yet in the data limits, and only then does
    /// `update` extend the data limits and request autoscaling again. So
    /// the view the earlier artists asked for is kept if autoscaling is
    /// turned off before the next draw. [`relim`](Self::relim) counts the
    /// collection from then on.
    pub(crate) fn adopt_collection(&self, artist: impl Artist, update: &DataUpdate) -> RawHandle {
        let handle = self.adopt_in_autoscale(artist, &DataUpdate::default());
        self.unstale_view();
        self.update_data(update);
        handle
    }

    /// Recomputes the data limits from the lines, patches, images and
    /// collections of the axes, leaving hidden ones out when
    /// `visible_only` (matplotlib's `relim`). The view follows at the next
    /// [`Axes::autoscale_view`].
    ///
    /// As in matplotlib, only the artists added the way `add_line`,
    /// `add_patch`, `add_image` and `add_collection` add them count:
    /// patches given to [`Axes::add_artist`], the collections of methods
    /// that set the data limits once themselves (`hlines`, `vlines`,
    /// `eventplot`, `hexbin`, `pcolor`, `pcolormesh`, `tripcolor`, contour
    /// sets, ...), collections made
    /// [`without_autolim`](crate::collections::CollectionSpec::without_autolim)
    /// and infinite lines are left out.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let line = ax.plot([0.0, 1.0], [0.0, 1.0])?;
    /// line.set_data(&[0.0, 10.0], &[0.0, 5.0]);
    /// assert_eq!(ax.data_lim().bbox.p1.x, 1.0, "set_data leaves the limits alone");
    /// ax.relim(false);
    /// assert_eq!(ax.data_lim().bbox.p1.x, 10.0);
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn relim(&self, visible_only: bool) -> &Self {
        let axes = self.id();
        let figure = self.raw().figure_state();
        let mut state = figure.borrow_mut();
        let children = state
            .artist_as::<AxesState>(axes)
            .map(|a| a.children.clone())
            .unwrap_or_default();
        let mut limits = DataLimits::NULL;
        for artist in children.into_iter().filter_map(|id| state.artist(id)) {
            let base = artist.base();
            if !base.in_autoscale || (visible_only && !base.visible) {
                continue;
            }
            if let Some(LimitPoints {
                points,
                directions: (x, y),
            }) = artist.limit_points(axes)
            {
                limits.update_from_points(&points, x, y);
            }
        }
        if let Some(axes) = state.artist_as_mut::<AxesState>(axes) {
            axes.data_lim = limits;
        }
        self
    }

    /// Extends the data limits to include `points` in x when `updatex` and
    /// in y when `updatey` (matplotlib's `update_datalim`).
    pub fn update_datalim(&self, points: &[Point], updatex: bool, updatey: bool) -> &Self {
        self.update_data(&DataUpdate {
            points: points.to_vec(),
            update: (updatex, updatey),
            autoscale: (false, false),
        });
        self
    }

    /// Applies `update` to the data limits and autoscaling state
    /// (matplotlib's `update_datalim` plus `_request_autoscale_view`).
    pub(crate) fn update_data(&self, update: &DataUpdate) {
        self.raw().with_mut::<AxesState, _>(|axes| {
            let (x, y) = update.update;
            axes.data_lim.update_from_points(&update.points, x, y);
            let (x, y) = update.autoscale;
            axes.request_autoscale(x, y);
        });
    }
}

/// Whether `artist` is one of the kinds matplotlib lists apart from
/// `ax.artists`: lines, patches, collections (contour sets and triangle
/// meshes included), texts, images and tables, or a part of the axes it
/// does not count as a child (the spines, the legend, a colorbar,
/// axisartist axes).
fn listed_by_kind(artist: &dyn Artist) -> bool {
    let any = artist as &dyn Any;
    is_data_artist(artist)
        || any.is::<TextArtist>()
        || any.is::<TableArtist>()
        || any.is::<SpineArtist>()
        || any.is::<LegendArtist>()
        || any.is::<ColorbarArtist>()
        || any.is::<AxisArtistState>()
}

/// Whether matplotlib labels `artist` `_child<n>` when it is added
/// without a label: a line, a collection (contour sets and triangle
/// meshes included) or an image, which `add_line`, `add_collection` and
/// `add_image` label (`add_patch` does not).
fn is_labelled_child(artist: &dyn Artist) -> bool {
    let any = artist as &dyn Any;
    any.is::<LineArtist>()
        || any.is::<CollectionArtist>()
        || any.is::<ContourSetArtist>()
        || any.is::<AxesImageArtist>()
        || any.is::<NonUniformImageArtist>()
        || any.is::<PcolorImageArtist>()
}

/// Whether `artist` is a line, patch, collection (contour sets and
/// triangle meshes included) or image: the kinds matplotlib's
/// `has_data` looks for.
fn is_data_artist(artist: &dyn Artist) -> bool {
    let any = artist as &dyn Any;
    artist.as_patch().is_some()
        || any.is::<LineArtist>()
        || any.is::<CollectionArtist>()
        || any.is::<ContourSetArtist>()
        || any.is::<AxesImageArtist>()
        || any.is::<NonUniformImageArtist>()
        || any.is::<PcolorImageArtist>()
}

#[cfg(test)]
mod tests {
    use pltrs_core::Point;

    use crate::artist::{ArtistHandle, RawHandle};
    use crate::axes::{
        Axes, BoxArtist, BoxProps, BoxplotOptions, BxpOptions, EventplotOptions,
        FillBetweenOptions, HexbinOptions, ImshowOptions, Mesh, PcolorOptions,
    };
    use crate::contour::{ContourOptions, QuadField};
    use crate::figure::Figure;
    use crate::lines::{AxLineThrough, LineProps};
    use crate::numeric::Grid;
    use crate::patches::{PatchProps, Rectangle};

    /// The data limits as `(x0, y0, x1, y1)`.
    fn limits(ax: &Axes) -> (f64, f64, f64, f64) {
        let bbox = ax.data_lim().bbox;
        (bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1())
    }

    /// A unit square at `(x, y)`.
    fn square(x: f64, y: f64) -> Rectangle {
        Rectangle::new(Point::new(x, y), 1.0, 1.0)
    }

    /// Fills between `y1` and `y2` over `x`, from 0 to 1.
    fn fill(ax: &Axes, y1: f64, y2: f64) {
        ax.fill_between([0.0, 1.0], [y1], [y2], FillBetweenOptions::default())
            .expect("valid curves");
    }

    #[test]
    fn collections_settle_the_view_of_earlier_artists_first() {
        // matplotlib's resample example: the line's autoscaled view stays
        // when autoscaling is turned off after the fill is added.
        let ax = Figure::new().subplot();
        ax.plot([0.0, 1.0], [0.0, 1.0]).expect("valid line");
        fill(&ax, 2.0, 3.0);
        ax.set_autoscale_on(false);
        let (bottom, top) = ax.ylim();
        assert!((bottom + 0.05).abs() < 1e-12 && (top - 1.05).abs() < 1e-12);
        assert_eq!(
            ax.data_lim().bbox.y1(),
            3.0,
            "the fill is in the data limits"
        );
    }

    #[test]
    fn relim_counts_only_what_matplotlib_marks_in_autoscale() {
        let ax = Figure::new().subplot();
        let grid = Grid::from_fn(2, 2, |r, c| (r + c) as f64);
        ax.add_artist(square(10.0, 10.0));
        ax.hlines([5.0], [0.0], [9.0]).expect("valid segments");
        ax.eventplot(&[vec![7.0]], EventplotOptions::default())
            .expect("valid events");
        ax.hexbin(&[0.0, 8.0], &[0.0, 8.0], HexbinOptions::default())
            .expect("valid points");
        ax.pcolormesh(Mesh::new(grid.clone()), PcolorOptions::default())
            .expect("a valid mesh");
        ax.contour(&QuadField::new(grid.clone()), ContourOptions::default())
            .expect("a valid field");
        let through = AxLineThrough::Point(Point::new(6.0, 6.0));
        ax.axline(Point::new(0.0, 0.0), through, LineProps::default())
            .expect("a valid line");
        assert_eq!(
            limits(&ax).2,
            9.0,
            "the plotting methods set the data limits once, add_artist does not"
        );
        ax.relim(false);
        assert!(ax.data_lim().bbox.x0().is_infinite(), "none of them counts");

        ax.plot([-1.0, 0.0], [-1.0, 0.0]).expect("a valid line");
        ax.add_patch(square(3.0, 3.0));
        ax.imshow(grid, ImshowOptions::default())
            .expect("a valid image");
        ax.relim(false);
        assert_eq!(limits(&ax), (-1.0, -1.0, 4.0, 4.0));
    }

    #[test]
    fn plotted_lines_collections_and_images_are_labelled_as_children() {
        // matplotlib's add_line, add_collection and add_image label an
        // unlabelled artist `_child<n>`; add_patch leaves patches alone.
        let ax = Figure::new().subplot();
        ax.text(0.5, 0.5, "counted");
        let line = ax.axhline(0.5, LineProps::default());
        let patch = ax.add_artist(square(0.0, 0.0));
        let image = ax
            .imshow(Grid::filled(2, 2, 1.0), ImshowOptions::default())
            .expect("an image");
        let mesh = ax
            .pcolormesh(Mesh::new(Grid::filled(2, 3, 1.0)), PcolorOptions::default())
            .expect("a valid mesh");
        assert_eq!(line.label(), "_child1");
        assert_eq!(patch.label(), "");
        assert_eq!(image.label(), "_child3");
        assert_eq!(mesh.label(), "_child4");
    }

    #[test]
    fn attached_artists_count() {
        let ax = Figure::new().subplot();
        let mesh = ax
            .pcolormesh(Mesh::new(Grid::filled(2, 3, 1.0)), PcolorOptions::default())
            .expect("a valid mesh");
        mesh.remove();
        ax.attach(&mesh).expect("a removed collection");
        ax.relim(false);
        assert_eq!(limits(&ax), (0.0, 0.0, 3.0, 2.0));
    }

    #[test]
    fn filled_boxes_are_added_as_artists() {
        let ax = Figure::new().subplot();
        let options = BoxplotOptions {
            draw: BxpOptions {
                boxes: Some(BoxProps::Patch(PatchProps::default())),
                ..BxpOptions::default()
            },
            ..BoxplotOptions::default()
        };
        let drawn = ax.boxplot(&[[1.0, 2.0, 3.0]], options).expect("valid data");
        let counted = |artist: &RawHandle| artist.with_base(|base| base.in_autoscale);
        let [BoxArtist::Patch(filled)] = drawn.boxes.as_slice() else {
            panic!("one filled box, not {:?}", drawn.boxes);
        };
        assert!(!counted(filled.raw()), "matplotlib's add_artist");
        assert!(counted(drawn.medians[0].raw()), "add_line");
    }

    #[test]
    fn collections_on_fresh_axes_leave_the_view_alone() {
        let ax = Figure::new().subplot();
        fill(&ax, 2.0, 3.0);
        ax.set_autoscale_on(false);
        assert_eq!(ax.ylim(), (0.0, 1.0), "nothing was pending before the fill");
    }
}
