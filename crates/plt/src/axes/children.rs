//! The children of an axes: listing them by kind (matplotlib's
//! `ax.collections`, `ax.patches`, `ax.texts`, `ax.tables`,
//! `ax.artists`), adding artists and accounting for them in the data
//! limits.

use std::any::Any;
use std::rc::Rc;

use plt_core::Point;

use super::{Axes, AxesState, DataLimits};
use crate::artist::{Artist, ArtistHandle, ArtistLookup, LimitPoints, RawHandle};
use crate::axisartist::AxisArtistState;
use crate::collections::{Collection, CollectionArtist, TriMeshArtist};
use crate::colorbar::ColorbarArtist;
use crate::contour::ContourSetArtist;
use crate::image::{AxesImageArtist, NonUniformImageArtist, PcolorImageArtist};
use crate::legend::LegendArtist;
use crate::lines::LineArtist;
use crate::spines::SpineArtist;
use crate::streamplot::StreamArrowArtist;
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
    /// were added.
    fn children_where(&self, keep: impl Fn(&dyn Artist) -> bool) -> Vec<RawHandle> {
        let figure = self.raw().figure_state();
        let state = figure.borrow();
        let children = self
            .raw()
            .with::<AxesState, _>(|axes| axes.children.clone());
        children
            .into_iter()
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
    /// use plt::prelude::*;
    /// let ax = Figure::new().subplot();
    /// assert!(ax.collections().is_empty());
    /// ax.scatter(vec![0.0, 1.0], vec![1.0, 0.0])?;
    /// assert_eq!(ax.collections().len(), 1);
    /// # Ok::<(), plt::axes::PlotError>(())
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
    /// use plt::prelude::*;
    /// let ax = Figure::new().subplot();
    /// assert!(ax.patches().is_empty());
    /// ax.bar([1.0, 2.0], [3.0, 4.0], Default::default())?;
    /// assert_eq!(ax.patches().len(), 2);
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    pub fn patches(&self) -> Vec<RawHandle> {
        self.children_where(|artist| artist.as_patch().is_some())
    }

    /// The texts and annotations of the axes in the order they were added
    /// (matplotlib's `ax.texts`); the titles and axis labels are not among
    /// them.
    ///
    /// ```
    /// use plt::prelude::*;
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
    /// use plt::prelude::*;
    /// use plt::table::TableOptions;
    /// let ax = Figure::new().subplot();
    /// assert!(ax.tables().is_empty());
    /// let options = TableOptions { cell_text: vec![vec!["1".into(), "2".into()]], ..Default::default() };
    /// let table = ax.table(options)?;
    /// assert_eq!(ax.tables(), [table]);
    /// # Ok::<(), plt::table::TableError>(())
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
    /// use plt::offsetbox::{AnchorLoc, AnchoredBox, AnchoredOptions};
    /// use plt::prelude::*;
    /// use plt::text::TextProps;
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

    /// Adds `artist` as a child and applies `update` (matplotlib's
    /// `add_line`/`add_patch`/`add_collection` bookkeeping): an artist
    /// without a clip path of its own is clipped to the axes' box.
    pub(crate) fn adopt(&self, mut artist: impl Artist, update: &DataUpdate) -> RawHandle {
        let axes_id = self.id();
        let base = artist.base_mut();
        if base.clip_path.is_none() {
            base.clip_box = Some(axes_id);
        }
        let figure = self.raw().figure_state();
        let mut state = figure.borrow_mut();
        let id = state.adopt(axes_id, Box::new(artist));
        drop(state);
        self.update_data(update);
        RawHandle::new(Rc::clone(figure), id)
    }

    /// Adds a collection as a child the way matplotlib's `add_collection`
    /// does when it updates the limits (`autolim=True`): pending
    /// autoscaling is applied to the view first, with the collection among
    /// the children but not yet in the data limits, and only then does
    /// `update` extend the data limits and request autoscaling again. So
    /// the view the earlier artists asked for is kept if autoscaling is
    /// turned off before the next draw.
    pub(crate) fn adopt_collection(&self, artist: impl Artist, update: &DataUpdate) -> RawHandle {
        let handle = self.adopt(artist, &DataUpdate::default());
        self.unstale_view();
        self.update_data(update);
        handle
    }

    /// Recomputes the data limits from the lines, patches, images and
    /// collections of the axes, leaving hidden ones out when
    /// `visible_only` (matplotlib's `relim`). The view follows at the next
    /// [`Axes::autoscale_view`].
    ///
    /// ```
    /// use plt::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let line = ax.plot([0.0, 1.0], [0.0, 1.0])?;
    /// line.set_data(&[0.0, 10.0], &[0.0, 5.0]);
    /// assert_eq!(ax.data_lim().bbox.p1.x, 1.0, "set_data leaves the limits alone");
    /// ax.relim(false);
    /// assert_eq!(ax.data_lim().bbox.p1.x, 10.0);
    /// # Ok::<(), plt::axes::PlotError>(())
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
            if visible_only && !artist.base().visible {
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
    artist.as_patch().is_some()
        || any.is::<LineArtist>()
        || any.is::<CollectionArtist>()
        || any.is::<TriMeshArtist>()
        || any.is::<ContourSetArtist>()
        || any.is::<StreamArrowArtist>()
        || any.is::<TextArtist>()
        || any.is::<AxesImageArtist>()
        || any.is::<NonUniformImageArtist>()
        || any.is::<PcolorImageArtist>()
        || any.is::<TableArtist>()
        || any.is::<SpineArtist>()
        || any.is::<LegendArtist>()
        || any.is::<ColorbarArtist>()
        || any.is::<AxisArtistState>()
}

#[cfg(test)]
mod tests {
    use crate::axes::{Axes, FillBetweenOptions};
    use crate::figure::Figure;

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
    fn collections_on_fresh_axes_leave_the_view_alone() {
        let ax = Figure::new().subplot();
        fill(&ax, 2.0, 3.0);
        ax.set_autoscale_on(false);
        assert_eq!(ax.ylim(), (0.0, 1.0), "nothing was pending before the fill");
    }
}
