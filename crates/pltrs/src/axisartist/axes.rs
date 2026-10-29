//! Axes drawn with axis artists (matplotlib's `axisartist.axislines.Axes`,
//! `AxesZero` and the host axes of `axisartist.HostAxes`): the axis
//! dictionary, new fixed and floating axis lines, gridlines and auxiliary
//! axes.

use std::rc::Rc;

use super::artist::AxisArtistState;
use super::aux_transform::AuxTransform;
use super::floating_axes;
use super::grid_helper_curvelinear::{FloatingLine, GridHelperCurveLinear};
use super::gridlines::{Gridlines, SharedGridHelper};
use super::handles::{AxisArtist, Toggle};
use super::helper::AxisHelper;
use crate::artist::{ArtistHandle, ArtistId, ArtistLookup, RawHandle};
use crate::axes::{AxesState, GridOptions};
use crate::axes_grid1::{HostAxes, ParasiteAxes, ParasiteLink};
use crate::collections::{Collection, CollectionArtist};
use crate::spines::SpineSide;
use crate::ticker::AxisName;

/// How an axisartist axes places its axis lines, ticks and gridlines
/// (matplotlib's grid helpers).
#[derive(Debug)]
pub enum GridHelper {
    /// Along the axes' own x and y axes (matplotlib's
    /// `GridHelperRectlinear`).
    Rectilinear,
    /// Along the lines of a curvilinear grid.
    CurveLinear(GridHelperCurveLinear),
    /// Along the lines of a curvilinear grid over fixed extremes.
    Floating(floating_axes::FloatingGridHelper),
}

impl From<GridHelperCurveLinear> for GridHelper {
    fn from(helper: GridHelperCurveLinear) -> Self {
        Self::CurveLinear(helper)
    }
}

impl From<floating_axes::FloatingGridHelper> for GridHelper {
    fn from(helper: floating_axes::FloatingGridHelper) -> Self {
        Self::Floating(helper)
    }
}

/// An axes whose axis lines, ticks and labels are drawn by axis artists,
/// one per entry of its axis dictionary, instead of its spines and axes
/// (matplotlib's `mpl_toolkits.axisartist.Axes`). Besides the four edges,
/// floating axis lines can run through the data, and a grid helper can
/// make the grid curvilinear.
///
/// ```
/// use pltrs::axisartist;
/// use pltrs::prelude::*;
/// use pltrs::axes::AxisName;
/// let fig = Figure::new();
/// let ax = axisartist::AxisArtistAxes::new(fig.subplot());
/// let through_zero = ax.new_floating_axis("y=0", AxisName::X, 0.0);
/// ax.axis("top").expect("an edge").set_visible(false);
/// assert_eq!(through_zero.name(), "y=0");
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct AxisArtistAxes {
    ax: crate::axes::Axes,
}

impl AxisArtistAxes {
    /// Turns `ax` into axes drawn by axis artists on its own grid
    /// (matplotlib's `axisartist.Axes`); see [`AxisArtistAxes::with_grid_helper`].
    pub fn new(ax: crate::axes::Axes) -> Self {
        Self::with_grid_helper(ax, GridHelper::Rectilinear)
    }

    /// Turns `ax` into axes drawn by axis artists placed by `grid_helper`:
    /// its spines and axes are hidden, hidden gridlines are added, and
    /// artists along the four edges take over, those at the top and right
    /// without labels (matplotlib's `axislines.Axes(grid_helper=...)`).
    pub fn with_grid_helper(ax: crate::axes::Axes, grid_helper: impl Into<GridHelper>) -> Self {
        let helper = match grid_helper.into() {
            GridHelper::Rectilinear => SharedGridHelper::Rectilinear,
            GridHelper::CurveLinear(grid) => SharedGridHelper::CurveLinear(Rc::new(grid)),
            GridHelper::Floating(grid) => SharedGridHelper::Floating(Rc::new(grid)),
        };
        let axes = Self { ax };
        axes.install(helper);
        axes
    }

    /// Removes everything drawn in the axes and resets it as
    /// [`crate::axes::Axes::clear`] does, then gives it new gridlines and
    /// axis artists along the four edges from the same grid helper
    /// (matplotlib's `axislines.Axes.clear`): floating axis lines and
    /// restyled edges are gone.
    pub fn clear(&self) -> &Self {
        let helper = self.grid_helper();
        self.ax.clear();
        self.install(helper);
        self
    }

    /// Hides the spines and axes of the underlying axes and adds hidden
    /// gridlines and the four edge artists placed by `helper`.
    fn install(&self, helper: SharedGridHelper) {
        let spines = self.ax.spines();
        for spine in [&spines.left, &spines.right, &spines.bottom, &spines.top] {
            spine.set_visible(false);
        }
        self.ax.xaxis().set_visible(false);
        self.ax.yaxis().set_visible(false);
        let gridlines = Gridlines::collection(self.ax.id(), helper, &axes_rc(&self.ax));
        self.ax
            .raw()
            .figure_state()
            .borrow_mut()
            .adopt(self.ax.id(), Box::new(gridlines));
        for side in [
            SpineSide::Bottom,
            SpineSide::Top,
            SpineSide::Left,
            SpineSide::Right,
        ] {
            let artist = self.new_fixed_axis(side.name(), side, (0.0, 0.0));
            if side.is_high() {
                artist.toggle(Toggle {
                    ticklabels: Some(false),
                    label: Some(false),
                    ..Toggle::default()
                });
            }
        }
    }

    /// [`AxisArtistAxes::new`] plus hidden axis lines through the data origin, `xzero`
    /// along `y = 0` and `yzero` along `x = 0`, clipped to the axes
    /// (matplotlib's `AxesZero`).
    pub fn zero(ax: crate::axes::Axes) -> Self {
        let axes = Self::new(ax);
        for (name, nth, direction) in [
            ("xzero", AxisName::X, SpineSide::Bottom),
            ("yzero", AxisName::Y, SpineSide::Left),
        ] {
            axes.new_floating_axis_facing(name, nth, 0.0, direction)
                .set_visible(false);
        }
        axes
    }

    /// The underlying axes.
    pub fn ax(&self) -> &crate::axes::Axes {
        &self.ax
    }

    /// The children of the axes.
    fn children(&self) -> Vec<ArtistId> {
        self.ax
            .raw()
            .with::<AxesState, _>(|state| state.children.clone())
    }

    /// The axis artists in the order of the axis dictionary (matplotlib's
    /// `ax.axis.values()`, also `ax.axis[:]`).
    pub fn axis_artists(&self) -> Vec<AxisArtist> {
        let figure = self.ax.raw().figure_state();
        let ids: Vec<ArtistId> = {
            let state = figure.borrow();
            self.children()
                .into_iter()
                .filter(|&id| state.artist_as::<AxisArtistState>(id).is_some())
                .collect()
        };
        ids.into_iter()
            .map(|id| AxisArtist::from_raw(RawHandle::new(Rc::clone(figure), id)))
            .collect()
    }

    /// The axis artist under `name` in the axis dictionary (`ax.axis[name]`).
    pub fn axis(&self, name: &str) -> Option<AxisArtist> {
        self.axis_artists()
            .into_iter()
            .find(|artist| artist.name() == name)
    }

    /// The gridlines (matplotlib's `ax.gridlines`); `None` once removed.
    pub fn gridlines(&self) -> Option<Collection> {
        let figure = self.ax.raw().figure_state();
        let id = {
            let state = figure.borrow();
            self.children().into_iter().find(|&id| {
                state
                    .artist_as::<CollectionArtist>(id)
                    .and_then(CollectionArtist::draw_time_state::<Gridlines>)
                    .is_some()
            })
        }?;
        Some(Collection::from_raw(RawHandle::new(Rc::clone(figure), id)))
    }

    /// The grid helper shared by the axis artists and the gridlines.
    pub(super) fn grid_helper(&self) -> SharedGridHelper {
        self.gridlines()
            .and_then(|gridlines| {
                gridlines.raw().with::<CollectionArtist, _>(|collection| {
                    collection
                        .draw_time_state::<Gridlines>()
                        .map(|gridlines| gridlines.helper.clone())
                })
            })
            .unwrap_or(SharedGridHelper::Rectilinear)
    }

    /// Shows (`visible`) or hides the gridlines of `options.which` and
    /// `options.axis`, restyled by `options`, on the axes and on the
    /// gridlines drawn by the grid helper (matplotlib's
    /// `axislines.Axes.grid`).
    pub fn grid(&self, visible: bool, options: GridOptions) -> &Self {
        self.ax.grid_with(visible, options.clone());
        if let Some(gridlines) = self.gridlines() {
            gridlines
                .raw()
                .with_mut::<CollectionArtist, _>(|collection| {
                    Gridlines::apply(collection, visible, &options);
                });
        }
        self
    }

    /// Adds an axis line along the edge `loc`, displaced by `offset`
    /// points, under `name` in place of any artist there (matplotlib's
    /// `new_fixed_axis(loc, offset)` stored as `ax.axis[name]`). Its tick
    /// labels and label face `loc`.
    pub fn new_fixed_axis(&self, name: &str, loc: SpineSide, offset: (f64, f64)) -> AxisArtist {
        let (helper, clipped) = match self.grid_helper() {
            SharedGridHelper::Rectilinear => (AxisHelper::Fixed(loc), false),
            SharedGridHelper::CurveLinear(grid) => (
                AxisHelper::CurveFixed {
                    grid,
                    side: loc,
                    nth_ticks: loc.axis(),
                },
                false,
            ),
            SharedGridHelper::Floating(grid) => {
                (AxisHelper::FloatingFixed { grid, side: loc }, true)
            }
        };
        let artist = self.insert(name, helper, (loc, offset));
        artist.update(|state| state.line.clipped = clipped);
        artist
    }

    /// Adds an axis line where coordinate `nth` varies and the other one
    /// equals `value`, under `name` in place of any artist there
    /// (`ax.axis[name] = ax.new_floating_axis(nth_coord, value)`); see
    /// [`AxisArtistAxes::new_floating_axis_facing`].
    pub fn new_floating_axis(&self, name: &str, nth: AxisName, value: f64) -> AxisArtist {
        self.new_floating_axis_facing(name, nth, value, SpineSide::Bottom)
    }

    /// [`AxisArtistAxes::new_floating_axis`] with the tick labels and label facing
    /// `axis_direction` (matplotlib's `axis_direction` argument, which
    /// curvilinear grids ignore). The coordinates are data coordinates, or
    /// curvilinear ones with a curvilinear grid; the line is clipped to
    /// the axes.
    pub fn new_floating_axis_facing(
        &self,
        name: &str,
        nth: AxisName,
        value: f64,
        axis_direction: SpineSide,
    ) -> AxisArtist {
        let line = FloatingLine {
            nth,
            value,
            extremes: (f64::NEG_INFINITY, f64::INFINITY),
        };
        let (helper, direction) = match self.grid_helper() {
            SharedGridHelper::Rectilinear => (AxisHelper::Floating { nth, value }, axis_direction),
            SharedGridHelper::CurveLinear(grid) => {
                (AxisHelper::CurveFloating { grid, line }, SpineSide::Bottom)
            }
            SharedGridHelper::Floating(grid) => (
                AxisHelper::CurveFloating {
                    grid: Rc::clone(grid.curvelinear()),
                    line,
                },
                SpineSide::Bottom,
            ),
        };
        let artist = self.insert(name, helper, (direction, (0.0, 0.0)));
        artist.update(|state| state.line.clipped = true);
        artist
    }

    /// Adds a parasite axes whose data coordinates map to this axes' data
    /// coordinates through `transform`: what is plotted on it lines up
    /// with the ticks and grid of this axes, and is drawn among this
    /// axes' artists in z-order (matplotlib's `HostAxes.get_aux_axes(tr)`
    /// with `viewlim_mode="equal"`). Its own patch, spines and axes are
    /// hidden.
    pub fn aux_axes(&self, transform: &AuxTransform) -> ParasiteAxes {
        let aux = HostAxes::new(self.ax.clone())
            .aux_axes(ParasiteLink::Equal)
            .expect("parasites with equal view limits need no inverse transform");
        aux.raw()
            .with_mut::<AxesState, _>(|state| state.aux_transform = Some(transform.resolved()));
        aux
    }

    /// Adds an axis artist under `name`, created facing `direction` and
    /// displaced by `offset` points; an artist already there is replaced
    /// in place, keeping its rank in the axis dictionary.
    fn insert(
        &self,
        name: &str,
        helper: AxisHelper,
        (direction, offset): (SpineSide, (f64, f64)),
    ) -> AxisArtist {
        let existing = self.axis(name);
        let rank = existing.as_ref().and_then(|artist| {
            self.children()
                .iter()
                .position(|&id| id == artist.raw().id())
        });
        if let Some(existing) = existing {
            existing.remove();
        }
        let artist = adopt_axis_artist(&self.ax, name, helper, (direction, offset));
        if let Some(rank) = rank {
            let id = artist.raw().id();
            self.ax.raw().with_mut::<AxesState, _>(|state| {
                state.children.retain(|&child| child != id);
                state.children.insert(rank.min(state.children.len()), id);
            });
        }
        artist
    }
}

/// The rcParams `ax` was made under.
fn axes_rc(ax: &crate::axes::Axes) -> Rc<crate::rc::RcParams> {
    ax.raw().with::<AxesState, _>(|state| Rc::clone(&state.rc))
}

/// Adds an axis artist of `ax` under `name` along `helper`, created facing
/// `direction` and displaced by `offset` points, styled from the axes'
/// rcParams.
pub(super) fn adopt_axis_artist(
    ax: &crate::axes::Axes,
    name: &str,
    helper: AxisHelper,
    (direction, offset): (SpineSide, (f64, f64)),
) -> AxisArtist {
    let figure = ax.raw().figure_state();
    let artist = AxisArtistState::new(name, ax.id(), helper, (direction, offset), axes_rc(ax));
    let id: ArtistId = figure.borrow_mut().adopt(ax.id(), Box::new(artist));
    AxisArtist::from_raw(RawHandle::new(Rc::clone(figure), id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    #[test]
    fn axis_artists_replace_spines_and_axes() {
        let fig = Figure::new();
        let ax = AxisArtistAxes::new(fig.subplot());
        assert!(!ax.ax().spines().left.visible() && !ax.ax().xaxis().visible());
        let names: Vec<String> = ax.axis_artists().iter().map(AxisArtist::name).collect();
        assert_eq!(names, ["bottom", "top", "left", "right"]);
        let top = ax.axis("top").expect("an edge");
        assert!(!top.major_ticklabels().visible() && !top.label().visible());
        assert!(top.major_ticks().visible());
        assert!(ax.axis("bottom").and_then(|a| a.window_extent()).is_some());
        let gridlines = ax.gridlines().expect("gridlines");
        assert!(!gridlines.visible());
        ax.grid(true, GridOptions::default());
        assert!(gridlines.visible() && ax.ax().has_grid(AxisName::Y, true));
    }

    #[test]
    fn floating_axes_replace_entries_of_the_same_name_in_place() {
        let fig = Figure::new();
        let ax = AxisArtistAxes::zero(fig.subplot());
        let xzero = ax.axis("xzero").expect("an axis along y = 0");
        assert!(!xzero.visible());
        let replacement = ax.new_floating_axis("xzero", AxisName::X, 0.5);
        assert!(!xzero.raw().is_attached());
        assert_eq!(ax.axis("xzero"), Some(replacement.clone()));
        let names: Vec<String> = ax.axis_artists().iter().map(AxisArtist::name).collect();
        assert_eq!(names[4], "xzero", "the replacement keeps the rank");
    }

    #[test]
    fn clearing_starts_over_with_the_edges_of_the_grid_helper() {
        let fig = Figure::new();
        let ax = AxisArtistAxes::new(fig.subplot());
        ax.new_floating_axis("y=0", AxisName::X, 0.0);
        ax.ax().plot([0.0, 1.0], [0.0, 2.0]).expect("a line");
        ax.clear();
        let names: Vec<String> = ax.axis_artists().iter().map(AxisArtist::name).collect();
        assert_eq!(names, ["bottom", "top", "left", "right"]);
        assert!(ax.ax().lines().is_empty() && ax.gridlines().is_some());
        assert!(!ax.ax().spines().left.visible() && !ax.ax().xaxis().visible());
        let right = ax.axis("right").expect("an edge");
        assert!(!right.major_ticklabels().visible() && right.major_ticks().visible());
    }

    #[test]
    fn floating_lines_of_curvilinear_grids_take_extremes() {
        let fig = Figure::new();
        let polar = GridHelperCurveLinear::new(
            AuxTransform::polar(),
            crate::axisartist::grid_finder::GridFinderOptions::default(),
        );
        let ax = AxisArtistAxes::with_grid_helper(fig.subplot(), polar);
        ax.ax()
            .set_xlim(-1.0, 1.0)
            .and_then(|ax| ax.set_ylim(-1.0, 1.0))
            .expect("finite limits");
        // The ray at angle 0 from the centre to the right edge.
        let ray = ax.new_floating_axis("ray", AxisName::X, 0.0);
        ray.toggle(Toggle {
            all: Some(false),
            ..Toggle::default()
        });
        let width = |artist: &AxisArtist| artist.window_extent().expect("drawn").width();
        let whole = width(&ray);
        ray.set_extremes(0.5, 2.0);
        assert!(width(&ray) < whole, "from r = 0.5 on");
        ray.set_extremes(f64::NEG_INFINITY, f64::INFINITY);
        assert_eq!(width(&ray), whole);
        let bottom = ax.axis("bottom").expect("an edge");
        let before = bottom.window_extent();
        bottom.set_extremes(0.5, 2.0);
        assert_eq!(bottom.window_extent(), before, "edges ignore extremes");
    }

    #[test]
    fn fixed_artists_join_plain_axes() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let left = AxisArtist::add_fixed(&ax, SpineSide::Left);
        assert!(ax.spines().left.visible() && ax.yaxis().visible());
        assert!(left.window_extent().is_some());
        assert!(left.major_ticklabels().visible() && left.label().visible());
    }

    #[test]
    fn auxiliary_axes_live_in_their_host() {
        let fig = Figure::new();
        let host = fig.subplot();
        let ax = AxisArtistAxes::new(host.clone());
        let aux = ax.aux_axes(&AuxTransform::identity());
        assert_eq!(fig.gca(), host);
        assert_eq!(aux.host().axes(), &host);
        assert!(aux.plot([0.0, 1.0], [0.0, 1.0]).is_ok());
        assert_eq!(fig.axes().len(), 1);
    }
}
