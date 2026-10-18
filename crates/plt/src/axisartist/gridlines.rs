//! The gridlines of an axisartist axes (matplotlib's
//! `GridlinesCollection`): a line collection whose segments come from the
//! grid helper at every draw, since some grid lines cross no axis.

use std::any::Any;
use std::rc::Rc;

use plt_core::{Path, Point};

use super::floating_axes;
use super::grid_helper_curvelinear::{GridHelperCurveLinear, info_gridlines};
use crate::artist::{Artist, ArtistId, LayoutContext};
use crate::axes::{AxesState, GridOptions, Tick, TickLevel};
use crate::collections::{CollectionArtist, CollectionKind, DrawTimeShapes, EdgeColors};
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
    Floating(Rc<floating_axes::GridHelperCurveLinear>),
}

/// The segments of the gridlines: which levels and axes they show
/// (matplotlib's `GridlinesCollection` `which` and `axis`).
#[derive(Clone, Debug)]
pub(crate) struct Gridlines {
    axes: ArtistId,
    pub(crate) helper: SharedGridHelper,
    which: TickLevel,
    axis: Option<AxisName>,
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
        collection.edgecolors = EdgeColors::Colors(vec![rc.grid_color.clone()]);
        collection.linewidths = Some(vec![rc.grid_linewidth]);
        collection.linestyles = vec![rc.grid_linestyle.clone()];
        collection.base_mut().visible = false;
        collection.base_mut().clip_box = Some(axes);
        collection.draw_time = Some(Box::new(Self {
            axes,
            helper,
            which: TickLevel::Major,
            axis: None,
        }));
        collection
    }

    /// Shows or hides the gridlines of `options.which` and
    /// `options.axis`, restyled by `options` (matplotlib's
    /// `axislines.Axes.grid` on the gridlines).
    pub(crate) fn apply(collection: &mut CollectionArtist, visible: bool, options: &GridOptions) {
        if let Some(lines) = collection.draw_time_state::<Self>().cloned() {
            collection.draw_time = Some(Box::new(Self {
                which: options.which,
                axis: options.axis,
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

    /// The lines of the rectilinear grid: verticals at the x ticks of
    /// `which`, horizontals at the y ticks whose gridlines are on
    /// (matplotlib's `GridHelperRectlinear.get_gridlines`).
    fn rectilinear(&self, axes: &AxesState) -> Vec<Vec<Point>> {
        let ticks = |name: AxisName, major: bool, minor: bool| -> Vec<f64> {
            let Ok(set) = axes.axis(name).ticks.as_ref() else {
                return Vec::new();
            };
            let locs = |ticks: &[Tick]| ticks.iter().map(|t| t.loc).collect::<Vec<_>>();
            let mut values = Vec::new();
            if major {
                values.extend(locs(&set.major));
            }
            if minor {
                values.extend(locs(&set.minor_all));
            }
            values
        };
        let (x1, x2) = axes.view_interval(AxisName::X);
        let (y1, y2) = axes.view_interval(AxisName::Y);
        let mut lines = Vec::new();
        if self.axis != Some(AxisName::Y) {
            let xs = ticks(AxisName::X, self.which.major(), self.which.minor());
            lines.extend(
                xs.into_iter()
                    .map(|x| vec![Point::new(x, y1), Point::new(x, y2)]),
            );
        }
        if self.axis != Some(AxisName::X) {
            let yaxis = axes.axis(AxisName::Y);
            let ys = ticks(
                AxisName::Y,
                yaxis.major_style.grid.is_some(),
                yaxis.minor_style.grid.is_some(),
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
            SharedGridHelper::CurveLinear(grid) => {
                let info = grid
                    .grid_finder()
                    .get_grid_info(view)
                    .map_err(|e| e.to_string())?;
                info_gridlines(&info, self.axis)
            }
            SharedGridHelper::Floating(grid) => {
                let info = grid.grid_info(view).map_err(|e| e.to_string())?;
                info_gridlines(&info.lines, self.axis)
            }
        };
        Ok((
            lines.into_iter().map(Path::polyline).collect(),
            Transform::Data(self.axes),
        ))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
