//! The gridlines of an axisartist axes (matplotlib's
//! `GridlinesCollection`): a line collection whose segments come from the
//! grid helper at every draw, since some grid lines cross no axis.

use std::any::Any;
use std::rc::Rc;

use pltrs_core::{Path, Point};

use super::artist::AxisArtistState;
use super::floating_axes;
use super::grid_helper_curvelinear::{GridHelperCurveLinear, info_gridlines};
use crate::artist::ArtistHandle;
use crate::artist::{Artist, ArtistId, ArtistStore, DrawErrorKind, LayoutContext};
use crate::axes::{AxesState, GridOptions, TickLevel};
use crate::collections::{
    Collection, CollectionArtist, CollectionKind, DrawTimeShapes, EdgeColors,
};
use crate::rc::RcParams;
use crate::ticker::AxisName;
use crate::transforms::Transform;

/// The grid helper an axisartist axes shares with its axis artists and
/// gridlines.
#[derive(Clone, Debug)]
pub(crate) enum SharedGridHelper {
    /// The axes' own ticks (matplotlib's `GridHelperRectlinear`).
    Rectilinear,
    /// A curvilinear grid.
    CurveLinear(Rc<GridHelperCurveLinear>),
    /// A curvilinear grid over fixed extremes.
    Floating(Rc<floating_axes::FloatingGridHelper>),
}

pltrs_core::archive_enum!(SharedGridHelper {
    Rectilinear,
    CurveLinear(v0),
    Floating(v0),
});

/// A setting of the gridlines as the program gave it (matplotlib's
/// `GridlinesCollection.set_which` and `set_axis`, which keep any value):
/// one pltrs knows, or another, on which a draw through the rectilinear
/// grid helper fails as matplotlib's `get_gridlines` does (a curvilinear
/// one shows no lines for an unknown axis and takes no level).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GridlinesChoice<T> {
    /// A value pltrs knows.
    Known(T),
    /// Another value, as Python's `repr` spells it.
    Unknown(String),
}

pltrs_core::archive_enum!(GridlinesChoice<T> { Known(v0), Unknown(v0) });

/// Chooses the level (`which`, when given) and the axis (`axis`, when
/// given; `None` inside for both) the gridlines `gridlines` of an
/// axisartist axes show (matplotlib's `GridlinesCollection.set_which` and
/// `set_axis`; see [`GridlinesChoice`]); `false` when `gridlines` draws no
/// gridlines of an axes.
///
/// ```
/// use pltrs::axes::TickLevel;
/// use pltrs::axisartist::{AxisArtistAxes, GridlinesChoice, choose_gridlines};
/// use pltrs::prelude::*;
/// let fig = Figure::new();
/// let ax = AxisArtistAxes::new(fig.subplot());
/// let gridlines = ax.gridlines().expect("made with the axes");
/// assert!(choose_gridlines(&gridlines, Some(GridlinesChoice::Known(TickLevel::Both)), None));
/// ax.grid(true, Default::default());
/// choose_gridlines(&gridlines, None, Some(GridlinesChoice::Unknown("'z'".into())));
/// let error = fig.canvas().draw().unwrap_err();
/// assert!(error.to_string().contains("'z' is not a valid value for axis"));
/// ```
pub fn choose_gridlines(
    gridlines: &Collection,
    which: Option<GridlinesChoice<TickLevel>>,
    axis: Option<GridlinesChoice<Option<AxisName>>>,
) -> bool {
    let chosen = gridlines
        .raw()
        .with_mut::<CollectionArtist, _>(|collection| Gridlines::choose(collection, which, axis));
    if chosen {
        gridlines.raw().set_stale(true);
    }
    chosen
}

/// The segments of the gridlines: which levels and axes they show
/// (matplotlib's `GridlinesCollection` `which` and `axis`).
#[derive(Clone, Debug)]
pub(crate) struct Gridlines {
    axes: ArtistId,
    pub(crate) helper: SharedGridHelper,
    which: GridlinesChoice<TickLevel>,
    axis: GridlinesChoice<Option<AxisName>>,
}

pltrs_core::archive_struct!(Gridlines {
    axes,
    helper,
    which,
    axis
});

/// Whether `axes` is drawn along a curvilinear grid: its gridlines and the
/// axis artists of its grid helper place their lines and ticks from the
/// grid finder, never reading the ticks of the axes' own, hidden axes
/// (which matplotlib's rectilinear grid helper asks for).
pub(crate) fn on_curvilinear_grid(store: &ArtistStore, axes: ArtistId) -> bool {
    store.get_as::<AxesState>(axes).is_some_and(|state| {
        state.children.iter().any(|&id| {
            store
                .get_as::<CollectionArtist>(id)
                .and_then(CollectionArtist::draw_time_state::<Gridlines>)
                .is_some_and(|gridlines| !matches!(gridlines.helper, SharedGridHelper::Rectilinear))
        })
    })
}

/// Whether the axis artists of `axes` read the ticks of its own axes,
/// hidden as they are: matplotlib's rectilinear grid helper asks their
/// locators where its axis artists place their ticks.
pub(crate) fn reads_hidden_axes(store: &ArtistStore, axes: ArtistId) -> bool {
    let has_axis_artists = store.get_as::<AxesState>(axes).is_some_and(|state| {
        (state.children.iter()).any(|&id| store.get_as::<AxisArtistState>(id).is_some())
    });
    has_axis_artists && !on_curvilinear_grid(store, axes)
}

impl Gridlines {
    /// The gridlines collection of `axes` through `helper`, hidden, styled
    /// by the `grid.*` rcParams (matplotlib's `Axes.clear`).
    pub(crate) fn collection(
        axes: ArtistId,
        helper: SharedGridHelper,
        rc: &Rc<RcParams>,
    ) -> CollectionArtist {
        let mut collection = CollectionArtist::new(CollectionKind::Line, Vec::new(), Rc::clone(rc));
        collection.class_name = Some("GridlinesCollection");
        collection.edgecolors = EdgeColors::Colors(vec![rc.grid_color.clone()]);
        collection.linewidths = Some(vec![rc.grid_linewidth]);
        collection.linestyles = vec![rc.grid_linestyle.clone()];
        collection.base_mut().visible = false;
        collection.base_mut().clip_box = Some(axes);
        collection.draw_time = Some(Rc::new(Self {
            axes,
            helper,
            which: GridlinesChoice::Known(TickLevel::Major),
            axis: GridlinesChoice::Known(None),
        }));
        collection
    }

    /// Replaces the level (`which`, when given) and the axis (`axis`)
    /// the gridlines of `collection` show; `false` when `collection`
    /// draws no gridlines of an axes.
    pub(crate) fn choose(
        collection: &mut CollectionArtist,
        which: Option<GridlinesChoice<TickLevel>>,
        axis: Option<GridlinesChoice<Option<AxisName>>>,
    ) -> bool {
        let Some(lines) = collection.draw_time_state::<Self>().cloned() else {
            return false;
        };
        collection.draw_time = Some(Rc::new(Self {
            which: which.unwrap_or(lines.which.clone()),
            axis: axis.unwrap_or(lines.axis.clone()),
            ..lines
        }));
        true
    }

    /// Shows or hides the gridlines of `options.which` and
    /// `options.axis`, restyled by `options` (matplotlib's
    /// `axislines.Axes.grid` on the gridlines).
    pub(crate) fn apply(collection: &mut CollectionArtist, visible: bool, options: &GridOptions) {
        if let Some(lines) = collection.draw_time_state::<Self>().cloned() {
            collection.draw_time = Some(Rc::new(Self {
                which: GridlinesChoice::Known(options.which),
                axis: GridlinesChoice::Known(options.axis),
                ..lines
            }));
        }
        if let Some(color) = &options.color {
            collection.edgecolors = EdgeColors::Colors(vec![color.clone()]);
        }
        if let Some(width) = options.linewidth {
            collection.linewidths = Some(vec![width]);
        }
        if let Some(style) = &options.linestyle {
            collection.linestyles = vec![style.clone()];
        }
        let base = collection.base_mut();
        base.visible = visible;
        if options.alpha.is_some() {
            base.alpha = options.alpha;
        }
    }

    /// The lines of the rectilinear grid: verticals at the x locator
    /// positions of `which`, horizontals at the y ones whose gridlines are
    /// on, including positions beyond the view that the clip hides
    /// (matplotlib's `GridHelperRectlinear.get_gridlines`).
    fn rectilinear(&self, axes: &AxesState) -> Vec<Vec<Point>> {
        let (GridlinesChoice::Known(which), GridlinesChoice::Known(axis)) =
            (&self.which, &self.axis)
        else {
            return Vec::new();
        };
        let ticks = |name: AxisName, major: bool, minor: bool| -> Vec<f64> {
            let Ok(set) = axes.axis(name).ticks.as_ref() else {
                return Vec::new();
            };
            let mut values = Vec::new();
            if major {
                values.extend(&set.major_locs);
            }
            if minor {
                values.extend(&set.minor_locs);
            }
            values
        };
        let (x1, x2) = axes.view_interval(AxisName::X);
        let (y1, y2) = axes.view_interval(AxisName::Y);
        let mut lines = Vec::new();
        if *axis != Some(AxisName::Y) {
            let xs = ticks(AxisName::X, which.major(), which.minor());
            lines.extend(
                xs.into_iter()
                    .map(|x| vec![Point::new(x, y1), Point::new(x, y2)]),
            );
        }
        if *axis != Some(AxisName::X) {
            let yaxis = axes.axis(AxisName::Y);
            let ys = ticks(
                AxisName::Y,
                yaxis.major_style.grid_on,
                yaxis.minor_style.grid_on,
            );
            lines.extend(
                ys.into_iter()
                    .map(|y| vec![Point::new(x1, y), Point::new(x2, y)]),
            );
        }
        lines
    }
}

impl DrawTimeShapes for Gridlines {
    fn shapes(&self, ctx: LayoutContext<'_>) -> Result<(Vec<Path>, Transform), String> {
        let axes = ctx
            .store()
            .get_as::<AxesState>(self.axes)
            .ok_or("the axes of the gridlines is gone")?;
        let view = axes.view_lim;
        let lines = match &self.helper {
            SharedGridHelper::Rectilinear => self.rectilinear(axes),
            SharedGridHelper::CurveLinear(_) | SharedGridHelper::Floating(_)
                if matches!(self.axis, GridlinesChoice::Unknown(_)) =>
            {
                Vec::new()
            }
            SharedGridHelper::CurveLinear(grid) => {
                let info = grid
                    .grid_finder()
                    .grid_info(view)
                    .map_err(|e| e.to_string())?;
                info_gridlines(&info, self.known_axis())
            }
            SharedGridHelper::Floating(grid) => {
                let info = grid.grid_info(view).map_err(|e| e.to_string())?;
                info_gridlines(&info.lines, self.known_axis())
            }
        };
        Ok((
            lines.into_iter().map(Path::polyline).collect(),
            Transform::Data(self.axes),
        ))
    }

    /// The setting matplotlib's rectilinear grid helper refuses where it
    /// draws the gridlines (its `_api.check_in_list` of the level, then of
    /// the axis).
    fn problem(&self) -> Option<(DrawErrorKind, String)> {
        if !matches!(self.helper, SharedGridHelper::Rectilinear) {
            return None;
        }
        let refused = |spelled: &str, key: &str, values: &str| {
            format!("{spelled} is not a valid value for {key}. Supported values are {values}")
        };
        let message = match (&self.which, &self.axis) {
            (GridlinesChoice::Unknown(which), _) => {
                refused(which, "which", "'both', 'major', 'minor'")
            }
            (_, GridlinesChoice::Unknown(axis)) => refused(axis, "axis", "'both', 'x', 'y'"),
            _ => return None,
        };
        Some((DrawErrorKind::InvalidValue, message))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Gridlines {
    /// The axis the gridlines show, `None` for both (and for an unknown
    /// one, which the curvilinear helpers do not reach).
    fn known_axis(&self) -> Option<AxisName> {
        match self.axis {
            GridlinesChoice::Known(axis) => axis,
            GridlinesChoice::Unknown(_) => None,
        }
    }
}
