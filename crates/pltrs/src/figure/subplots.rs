//! Adding axes to a figure: free placement, grid cells and subplot grids
//! with shared axes (matplotlib's `add_axes`, `add_subplot`, `subplots`).

use std::cell::RefCell;
use std::rc::Rc;

use pltrs_core::{Bbox, Point};
use pltrs_text::HAlign;

use super::{AxesRectError, Figure, FigureState, axes_bounds};
use crate::artist::{Artist, ArtistHandle, ArtistId, RawHandle};
use crate::axes::{Axes, AxesState, AxisName, share_free};
use crate::fonts::FontProperties;
use crate::gridspec::{GridSpec, GridSpecError, SubplotSpec};
use crate::numeric::Grid;
use crate::projections::{ProjectionError, ProjectionSpec};
use crate::rc::{ColorOrAuto, RcParams};
use crate::spines::{SpineArtist, SpineSide};
use crate::text::TextArtist;
use crate::transforms::Transform;

/// The figure-level grid that `grid` lies in, through its parents.
fn topmost_grid(grid: &GridSpec) -> GridSpec {
    grid.parent()
        .map_or_else(|| grid.clone(), |cell| topmost_grid(cell.gridspec()))
}

/// Which axes of a subplot grid share an axis (matplotlib's `sharex` and
/// `sharey` arguments of `subplots`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Share {
    /// Every axes has its own axis.
    #[default]
    None,
    /// All axes share the axis.
    All,
    /// The axes of each row share the axis.
    Row,
    /// The axes of each column share the axis.
    Col,
}

impl Share {
    /// The index of the earlier cell that cell `(row, col)` of a grid with
    /// `ncols` columns shares with, if it has been created already.
    fn target(self, row: usize, col: usize, ncols: usize) -> Option<usize> {
        let target = match self {
            Self::None => return None,
            Self::All => 0,
            Self::Row => row * ncols,
            Self::Col => col,
        };
        (target < row * ncols + col).then_some(target)
    }
}

impl Figure {
    /// Adds axes occupying `[left, bottom, width, height]` in figure
    /// fractions (matplotlib's `add_axes(rect)`).
    ///
    /// ```
    /// use pltrs::figure::{AxesRectError, Figure};
    /// let fig = Figure::new();
    /// let ax = fig.add_axes([0.25, 0.25, 0.5, 0.5])?;
    /// assert_eq!(fig.axes(), [ax]);
    /// let refused = fig.add_axes([f64::NAN, 0.0, 1.0, 1.0]);
    /// assert!(matches!(refused, Err(AxesRectError::NonFinite(_))));
    /// # Ok::<(), AxesRectError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When an entry of `rect` is NaN or infinite, or its width or height
    /// is negative.
    pub fn add_axes(&self, rect: [f64; 4]) -> Result<Axes, AxesRectError> {
        let position = axes_bounds(rect)?;
        Ok(insert_axes(&self.state, self.root.id(), position, None))
    }

    /// The current axes (matplotlib's `gca`): the axes added last or made
    /// current with [`Figure::sca`], or a new subplot if the figure has
    /// none.
    pub fn gca(&self) -> Axes {
        self.current_axes().unwrap_or_else(|| self.subplot())
    }

    /// The current axes, `None` when the figure has none (matplotlib's
    /// `_axstack.current()`).
    pub(crate) fn current_axes(&self) -> Option<Axes> {
        let mut axes = self.axes();
        let current = self.state.borrow().current_axes;
        match current.and_then(|id| axes.iter().position(|ax| ax.id() == id)) {
            Some(index) => Some(axes.swap_remove(index)),
            None => axes.pop(),
        }
    }

    /// Makes `ax` the current axes (matplotlib's `sca`).
    pub fn sca(&self, ax: &Axes) -> &Self {
        self.state.borrow_mut().current_axes = Some(ax.id());
        self
    }

    /// Adds axes filling the default subplot area (`fig.add_subplot()`).
    ///
    /// When the `figure.subplot.*` parameters leave no room (`left >=
    /// right` or `bottom >= top`), where matplotlib's `Figure()` raises,
    /// the axes fill the figure and drawing it reports the error.
    pub fn subplot(&self) -> Axes {
        let spec = self
            .numbered_cell(1, 1, 1)
            .expect("a 1x1 grid has a cell 1");
        self.add_subplot_spec(&spec).unwrap_or_else(|_| {
            let whole = Bbox::from_extents(0.0, 0.0, 1.0, 1.0);
            insert_axes(&self.state, self.root.id(), whole, Some(spec))
        })
    }

    /// Adds axes in cell `index` (1-based, row-major) of an
    /// `nrows × ncols` grid (`fig.add_subplot(nrows, ncols, index)`); the
    /// grid is the one the figure's other subplots of that shape are on.
    pub fn add_subplot(
        &self,
        nrows: usize,
        ncols: usize,
        index: usize,
    ) -> Result<Axes, GridSpecError> {
        let spec = self.numbered_cell(nrows, ncols, index)?;
        self.add_subplot_spec(&spec)
    }

    /// Adds axes spanning the cells from `first` to `last` (1-based,
    /// row-major, inclusive) of an `nrows × ncols` grid
    /// (`fig.add_subplot(nrows, ncols, (first, last))`); the grid is the
    /// one the figure's other subplots of that shape are on.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// let tall = fig.add_subplot_span(3, 2, 3, 5)?;
    /// let beside = fig.add_subplot(3, 2, 4)?;
    /// let (tall, beside) = (tall.subplot_spec().unwrap(), beside.subplot_spec().unwrap());
    /// assert_eq!((tall.rows(), tall.cols()), (1..3, 0..1));
    /// assert!(tall.gridspec().is_same(beside.gridspec()));
    /// # Ok::<(), pltrs::gridspec::GridSpecError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the grid is empty, a cell is outside it or `last` comes before
    /// `first`.
    pub fn add_subplot_span(
        &self,
        nrows: usize,
        ncols: usize,
        first: usize,
        last: usize,
    ) -> Result<Axes, GridSpecError> {
        let spec = self.figure_grid(nrows, ncols)?.numbered_span(first, last)?;
        self.add_subplot_spec(&spec)
    }

    /// Cell `index` (1-based, row-major) of the figure's `nrows × ncols`
    /// grid (see [`Figure::figure_grid`]; matplotlib's
    /// `SubplotSpec._from_subplot_args`).
    pub(crate) fn numbered_cell(
        &self,
        nrows: usize,
        ncols: usize,
        index: usize,
    ) -> Result<SubplotSpec, GridSpecError> {
        self.figure_grid(nrows, ncols)?.numbered(index)
    }

    /// The figure's `nrows × ncols` grid: the grid an axes of the figure
    /// is already placed on, so that layout engines see a single grid, or
    /// a new one.
    pub(crate) fn figure_grid(
        &self,
        nrows: usize,
        ncols: usize,
    ) -> Result<GridSpec, GridSpecError> {
        match self.existing_gridspec(nrows, ncols) {
            Some(grid) => Ok(grid),
            None => GridSpec::new(nrows, ncols),
        }
    }

    /// The topmost grid of shape `(nrows, ncols)` an axes of the figure is
    /// placed on (matplotlib's `GridSpec._check_gridspec_exists`).
    pub(crate) fn existing_gridspec(&self, nrows: usize, ncols: usize) -> Option<GridSpec> {
        let planar = self.axes().into_iter().filter_map(|ax| ax.subplot_spec());
        let spatial = self.axes3d().into_iter().filter_map(|ax| ax.subplot_spec());
        planar
            .chain(spatial)
            .map(|spec| topmost_grid(spec.gridspec()))
            .find(|grid| grid.shape() == (nrows, ncols))
    }

    /// Adds axes covering a grid cell or block (`fig.add_subplot(gs[...])`).
    pub fn add_subplot_spec(&self, spec: &SubplotSpec) -> Result<Axes, GridSpecError> {
        let position = spec.position(&self.subplot_params())?;
        Ok(insert_axes(
            &self.state,
            self.root.id(),
            position,
            Some(spec.clone()),
        ))
    }

    /// A grid of `nrows × ncols` axes (`fig.subplots(nrows, ncols)`),
    /// indexed by `(row, col)`.
    pub fn subplots(&self, nrows: usize, ncols: usize) -> Result<Grid<Axes>, GridSpecError> {
        self.subplots_shared(nrows, ncols, Share::None, Share::None)
    }

    /// A grid of axes sharing their x and y axes as requested
    /// (`fig.subplots(nrows, ncols, sharex=..., sharey=...)`).
    ///
    /// ```
    /// use pltrs::figure::{Figure, Share};
    /// let fig = Figure::new();
    /// let axs = fig.subplots_shared(2, 1, Share::All, Share::None)?;
    /// axs[(0, 0)].set_xlim(0.0, 5.0)?;
    /// assert_eq!(axs[(1, 0)].xlim(), (0.0, 5.0));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn subplots_shared(
        &self,
        nrows: usize,
        ncols: usize,
        sharex: Share,
        sharey: Share,
    ) -> Result<Grid<Axes>, GridSpecError> {
        self.subplots_on(&GridSpec::new(nrows, ncols)?, sharex, sharey)
    }

    /// One axes per cell of `grid`, sharing axes as requested
    /// (matplotlib's `GridSpec.subplots`). When columns share x (or rows
    /// share y), only the outer axes keep their tick labels.
    pub fn subplots_on(
        &self,
        grid: &GridSpec,
        sharex: Share,
        sharey: Share,
    ) -> Result<Grid<Axes>, GridSpecError> {
        Self::subplots_made(
            grid,
            sharex,
            sharey,
            |spec| self.add_subplot_spec(spec),
            |axes, name, target| {
                share_free(axes, name, target);
                Ok(())
            },
        )
    }

    /// A grid of axes with `projection`, sharing their x and y axes as
    /// requested (matplotlib's `fig.subplots(nrows, ncols, sharex=...,
    /// sharey=..., subplot_kw={"projection": ..., **options})`); see
    /// [`subplots_on_with_projection`](Self::subplots_on_with_projection).
    ///
    /// ```
    /// use pltrs::axes::ProjectionKind;
    /// use pltrs::figure::{Figure, Share};
    /// let fig = Figure::new();
    /// let maps = fig.subplots_with_projection(1, 2, Share::None, Share::None, ProjectionKind::Hammer)?;
    /// assert!(maps.as_slice().iter().all(|ax| ax.projection() == ProjectionKind::Hammer));
    /// let nowhere = ProjectionKind::Registered("nowhere");
    /// assert!(fig.subplots_with_projection(2, 2, Share::All, Share::All, nowhere).is_err());
    /// assert_eq!(fig.axes().len(), 2);
    /// # Ok::<(), pltrs::projections::ProjectionError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// As [`subplots_on_with_projection`](Self::subplots_on_with_projection),
    /// and [`ProjectionError::Grid`] when the grid has no cells.
    pub fn subplots_with_projection(
        &self,
        nrows: usize,
        ncols: usize,
        sharex: Share,
        sharey: Share,
        projection: impl Into<ProjectionSpec>,
    ) -> Result<Grid<Axes>, ProjectionError> {
        let grid = GridSpec::new(nrows, ncols)?;
        self.subplots_on_with_projection(&grid, sharex, sharey, projection)
    }

    /// One axes with `projection` per cell of `grid`, sharing axes as
    /// requested (matplotlib's `GridSpec.subplots(subplot_kw=...)`). As in
    /// [`subplots_on`](Self::subplots_on), the outer axes alone keep their
    /// tick labels where axes share, unless their projection draws them
    /// in a shape other than a rectangle.
    ///
    /// # Errors
    ///
    /// [`ProjectionError::Unknown`] for a projection not registered on this
    /// thread and [`ProjectionError::Options`] for options it does not
    /// take, before any axes is added; [`ProjectionError::SetUp`] when it
    /// fails to set an axes up, which leaves the axes made before;
    /// [`ProjectionError::Share`] when axes of a projection fixing their
    /// limits, such as the geographic ones, are to share an axis (the
    /// axes that would share is not added, as matplotlib fails to make
    /// it); [`ProjectionError::Grid`] when the subplot parameters leave no
    /// room for a cell.
    pub fn subplots_on_with_projection(
        &self,
        grid: &GridSpec,
        sharex: Share,
        sharey: Share,
        projection: impl Into<ProjectionSpec>,
    ) -> Result<Grid<Axes>, ProjectionError> {
        let projection = projection.into();
        projection.check()?;
        Self::subplots_made(
            grid,
            sharex,
            sharey,
            |spec| self.add_subplot_spec_with_projection(spec, projection.clone()),
            |axes, name, target| {
                axes.share(name, target).map_err(ProjectionError::Share)?;
                Ok(())
            },
        )
    }

    /// One axes made by `make` per cell of `grid`, each made to share an
    /// axis with an axes made before it by `share` as requested; an axes
    /// failing to share is removed and the error returned. See
    /// [`subplots_on`](Self::subplots_on).
    pub(crate) fn subplots_made<E: From<GridSpecError>>(
        grid: &GridSpec,
        sharex: Share,
        sharey: Share,
        make: impl Fn(&SubplotSpec) -> Result<Axes, E>,
        share: impl Fn(&Axes, AxisName, &Axes) -> Result<(), E>,
    ) -> Result<Grid<Axes>, E> {
        let (nrows, ncols) = grid.shape();
        let mut cells: Vec<Axes> = Vec::with_capacity(nrows * ncols);
        for row in 0..nrows {
            for col in 0..ncols {
                let axes = make(&grid.cell(row, col)?)?;
                for (kind, name) in [(sharex, AxisName::X), (sharey, AxisName::Y)] {
                    if let Some(target) = kind.target(row, col, ncols)
                        && let Err(err) = share(&axes, name, &cells[target])
                    {
                        axes.remove();
                        return Err(err);
                    }
                }
                cells.push(axes);
            }
        }
        for (share, name) in [(sharex, AxisName::X), (sharey, AxisName::Y)] {
            let outer = match name {
                AxisName::X => matches!(share, Share::Col | Share::All),
                AxisName::Y => matches!(share, Share::Row | Share::All),
            };
            if outer {
                for axes in &cells {
                    axes.label_outer_shared(name);
                }
            }
        }
        Ok(Grid::from_vec(nrows, ncols, cells).expect("one axes per grid cell"))
    }

    /// A grid specification using this figure's subplot parameters.
    pub fn add_gridspec(&self, nrows: usize, ncols: usize) -> Result<GridSpec, GridSpecError> {
        GridSpec::new(nrows, ncols)
    }
}

/// Adds an axes at `position` (fractions of `frame`) with its spines and
/// titles.
pub(crate) fn insert_axes(
    figure: &Rc<RefCell<FigureState>>,
    frame: ArtistId,
    position: Bbox,
    spec: Option<SubplotSpec>,
) -> Axes {
    make_axes(figure, Some(frame), position, spec)
}

/// Makes an axes at `position` (fractions of `frame`) with its spines and
/// titles: in `frame`, or removed from the figure without a frame (as the
/// twin of a removed axes is). An axes made in the figure becomes its
/// current axes.
pub(crate) fn make_axes(
    figure: &Rc<RefCell<FigureState>>,
    frame: Option<ArtistId>,
    position: Bbox,
    spec: Option<SubplotSpec>,
) -> Axes {
    let rc = crate::rc::params();
    let mut axes = AxesState::new(position, Rc::clone(&rc));
    axes.subplot_spec = spec;
    let mut state = figure.borrow_mut();
    let id = match frame {
        Some(frame) => state.adopt(frame, Box::new(axes)),
        None => state.insert_removed(Box::new(axes)),
    };
    let spines: Vec<_> = SpineSide::ALL
        .into_iter()
        .map(|side| state.adopt(id, Box::new(SpineArtist::new(side, id, Rc::clone(&rc)))))
        .collect();
    let titles: Vec<_> = TITLE_PLACES
        .into_iter()
        .map(|(x, halign)| state.adopt(id, Box::new(axes_title(id, x, halign, &rc))))
        .collect();
    if let Some(axes) = state.artist_as_mut::<AxesState>(id) {
        axes.spines = spines;
        axes.titles = titles;
    }
    if state.is_attached(id) {
        state.current_axes = Some(id);
    }
    drop(state);
    Axes::from_raw(RawHandle::new(Rc::clone(figure), id))
}

/// Where the left, centre and right titles of an axes sit: axes fraction
/// and alignment.
pub(crate) const TITLE_PLACES: [(f64, HAlign); 3] = [
    (0.0, HAlign::Left),
    (0.5, HAlign::Center),
    (1.0, HAlign::Right),
];

/// An empty axes title at axes fraction `x` (matplotlib's `Axes.title`,
/// `_left_title` and `_right_title`).
pub(crate) fn axes_title(axes: ArtistId, x: f64, halign: HAlign, rc: &Rc<RcParams>) -> TextArtist {
    let y = rc.axes_titley.unwrap_or(1.0);
    let mut title = TextArtist::new(Point::new(x, y), "", Rc::clone(rc));
    title.halign = halign;
    title.font = FontProperties::from_rc(rc)
        .with_size(rc.axes_titlesize, rc.font_size)
        .with_weight(rc.axes_titleweight);
    if let ColorOrAuto::Color(color) = &rc.axes_titlecolor {
        title.color = color.clone();
    }
    title.base_mut().transform = Some(Transform::Axes(axes).offset_points(0.0, rc.axes_titlepad));
    title
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn margins_without_room_are_reported_not_panicked_on() {
        crate::rc::context(|| {
            crate::rc::set("figure.subplot.right", "0.1").expect("a float");
            let fig = Figure::new();
            let ax = fig.subplot();
            assert_eq!(ax.position(), Bbox::from_extents(0.0, 0.0, 1.0, 1.0));
            let err = fig.to_png().expect_err("matplotlib refuses such a figure");
            assert!(err.to_string().contains("left cannot be >= right"), "{err}");
        });
    }

    #[test]
    fn subplots_added_one_by_one_share_their_grid() {
        let fig = Figure::new();
        let grid = |ax: &Axes| ax.subplot_spec().expect("a grid cell").gridspec().clone();
        let first = fig.add_subplot(1, 3, 1).expect("cell 1");
        let second = fig.add_subplot(1, 3, 2).expect("cell 2");
        assert!(grid(&first).is_same(&grid(&second)));
        let other = fig.add_subplot(2, 1, 2).expect("another shape");
        assert!(!grid(&first).is_same(&grid(&other)));
    }

    #[test]
    fn nested_grids_count_as_their_topmost_grid() {
        let fig = Figure::new();
        let grid = |ax: &Axes| ax.subplot_spec().expect("a grid cell").gridspec().clone();
        let outer = GridSpec::new(2, 1).expect("a grid");
        let parent = outer.cell(1, 0).expect("cell");
        let nested = GridSpec::nested(&parent, 1, 3, None, None, (None, None)).expect("nested");
        let inner = fig
            .add_subplot_spec(&nested.cell(0, 0).expect("cell"))
            .expect("nested cell");
        let row = fig.add_subplot(1, 3, 2).expect("a 1 x 3 cell");
        assert!(
            !grid(&row).is_same(&grid(&inner)),
            "the nested grid is not reused"
        );
        let top = fig.add_subplot(2, 1, 1).expect("a cell of the outer grid");
        assert!(grid(&top).is_same(&outer));
    }

    #[test]
    fn share_targets_follow_matplotlib() {
        // Cell (1, 2) of a 2x3 grid.
        assert_eq!(Share::All.target(1, 2, 3), Some(0));
        assert_eq!(Share::Row.target(1, 2, 3), Some(3));
        assert_eq!(Share::Col.target(1, 2, 3), Some(2));
        assert_eq!(Share::None.target(1, 2, 3), None);
        // The first cell of each group has nothing to share with yet.
        assert_eq!(Share::All.target(0, 0, 3), None);
        assert_eq!(Share::Row.target(1, 0, 3), None);
        assert_eq!(Share::Col.target(0, 2, 3), None);
    }

    #[test]
    fn numbered_subplots_of_one_shape_share_a_grid() {
        let fig = Figure::new();
        let grid_of = |ax: &Axes| ax.subplot_spec().expect("a subplot").gridspec().clone();
        let top = fig.add_subplot(2, 1, 1).expect("cell 1");
        let bottom = fig.add_subplot(2, 1, 2).expect("cell 2");
        assert!(grid_of(&top).is_same(&grid_of(&bottom)));
        let side = fig.add_subplot(1, 2, 2).expect("cell 2");
        assert!(!grid_of(&side).is_same(&grid_of(&top)));
        // A grid nested in a cell counts as the grid it lies in.
        let nested = grid_of(&side).cell(0, 0).expect("a cell");
        let inner =
            GridSpec::nested(&nested, 2, 2, None, None, (None, None)).expect("a nested grid");
        fig.add_subplot_spec(&inner.cell(1, 1).expect("a cell"))
            .expect("room for it");
        let later = fig.add_subplot(1, 2, 1).expect("cell 1");
        assert!(grid_of(&later).is_same(&grid_of(&side)));
    }
}
