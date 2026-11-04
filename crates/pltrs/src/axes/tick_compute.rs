//! Computing the ticks of an axis for its view (matplotlib's
//! `Axis._update_ticks` and `get_tick_space`): the locators' locations,
//! the formatters' labels, and the minor ticks that do not fall on major
//! ones.

use std::any::Any;

use super::axis::{AxisGeometry, AxisRole, AxisState};
use super::axis_handle::Axis;
use super::tick_set::{Tick, TickSet};
use super::{AxesState, TickLevels};
use crate::artist::ArtistHandle;
use crate::figure::FigureState;
use crate::ticker::{
    AxisInfo, AxisName, LogitLocator, OverlapTolerance, TickContext, TickerError, TickerSettings,
    drawable_ticks,
};
use crate::transforms::{TransformContext, interval_contains_close};

impl Axis {
    /// Every major (or minor) tick location of the current view, as
    /// matplotlib computes them on demand: in the axes' geometry as it
    /// is, without laying the figure out (no aspect or axes locator
    /// applied, no layout engine), the view limits autoscaled first
    /// (matplotlib's `Axis.get_ticklocs` read before a draw; after one, the
    /// geometry is the drawn one). [`Axis::ticklocs`] lays the figure out.
    ///
    /// ```
    /// let ax = pltrs::figure::Figure::new().subplot();
    /// ax.set_xlim(0.0, 1.0)?;
    /// assert_eq!(ax.xaxis().ticklocs_as_is(false)?, ax.xaxis().ticklocs(false)?);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the locator fails.
    pub fn ticklocs_as_is(&self, minor: bool) -> Result<Vec<f64>, TickerError> {
        let id = self.axes().id();
        let name = self.name();
        let ticks = self.axes().raw().figure_state().work_on(|state| {
            state.unstale_axes(id);
            let extent = state.axes_bbox(id)?;
            let tick_space = state.shared_tick_spaces(id)[usize::from(name == AxisName::Y)];
            let dpi = state.dpi();
            let locate = |state: &FigureState| {
                (state.store.get_as::<AxesState>(id))
                    .map(|axes| axes.level_locs_for(name, &extent, dpi, tick_space, minor))
            };
            // The locators run with the figure free to read, as the
            // program's may.
            state
                .lend(|cell| locate(&cell.borrow()))
                .unwrap_or_else(|| locate(state))
        });
        ticks.unwrap_or_else(|| self.ticklocs(minor))
    }
}

impl AxisState {
    /// What tick helpers know about this axis.
    pub(crate) fn info(&self, geometry: &AxisGeometry) -> AxisInfo {
        AxisInfo {
            name: self.name,
            view_interval: geometry.view,
            data_interval: geometry.data,
            minpos: geometry.minpos,
            tick_space: geometry
                .tick_space
                .unwrap_or_else(|| self.tick_space(geometry.length_points)),
            major_ticks: Vec::new(),
            scale: self.scale().hint(),
            three_d: false,
        }
    }

    /// How many ticks fit (matplotlib's `get_tick_space`): tick labels are
    /// assumed to be at most 3:1 (x) or 2:1 (y) wide. Without a label size
    /// from the tick parameters, the size is the rc's when the ticks are
    /// laid out, as matplotlib reads it at draw time
    /// (`_get_tick_label_size`).
    pub(crate) fn tick_space(&self, length_points: f64) -> usize {
        let label_size = self.label_size.unwrap_or_else(|| {
            let rc = crate::rc::params();
            let size = match self.name {
                AxisName::X => rc.xtick_labelsize,
                AxisName::Y => rc.ytick_labelsize,
            };
            size.to_points(rc.font_size)
        });
        let ratio = match self.name {
            AxisName::X => 3.0,
            AxisName::Y => 2.0,
        };
        let size = label_size * ratio;
        if size > 0.0 {
            (length_points / size).floor().max(0.0) as usize
        } else {
            usize::MAX >> 1
        }
    }

    /// Computes the ticks to draw for the current view (matplotlib's
    /// `Axis._update_ticks`), making the ticks of every location; ticks a
    /// level cannot make (see [`GridLineProps::refused`]) fail.
    ///
    /// [`GridLineProps::refused`]: crate::axes::GridLineProps::refused
    pub(crate) fn update_ticks(&mut self, geometry: &AxisGeometry) {
        let ticks = self.compute_ticks(geometry);
        self.install_ticks(ticks);
    }

    /// [`update_ticks`](Self::update_ticks) for the `minor` level alone (or
    /// the major one), the ticks of the other level kept as they were:
    /// matplotlib's `set_ticklabels` places the ticks of its level only
    /// (`get_majorticklocs` or `get_minorticklocs`).
    pub(crate) fn update_level_ticks(&mut self, geometry: &AxisGeometry, minor: bool) {
        let levels = TickLevels {
            major: !minor && self.major_style.is_drawn(),
            minor: minor && self.minor_style.is_drawn(),
        };
        let mut ticks = self.compute_levels(geometry, levels);
        if let (Ok(ticks), Ok(before)) = (&mut ticks, &self.ticks) {
            ticks.keep_level(before, !minor);
        }
        self.install_ticks(ticks);
    }

    /// Makes `ticks` the axis' ticks, making the ticks of every location;
    /// ticks a level cannot make (see [`GridLineProps::refused`]) fail.
    ///
    /// [`GridLineProps::refused`]: crate::axes::GridLineProps::refused
    pub(crate) fn install_ticks(&mut self, ticks: Result<TickSet, TickerError>) {
        self.ticks = ticks;
        let Ok(ticks) = &self.ticks else {
            return;
        };
        let counts = [
            (false, ticks.major_unclipped.len()),
            (true, ticks.minor_unclipped.len()),
        ];
        for (minor, count) in counts {
            let style = if minor {
                &self.minor_style
            } else {
                &self.major_style
            };
            match style.grid.line.refused {
                Some(keyword) if count > self.overrides(minor).made => {
                    self.ticks = Err(TickerError::TickKeywordTwice(keyword));
                    return;
                }
                _ => self.overrides_mut(minor).make_ticks(count),
            }
        }
    }

    /// The major tick locations for the view of `geometry`, and what tick
    /// helpers know about the axis once they are placed.
    fn major_locs(
        &self,
        geometry: &AxisGeometry,
        settings: &TickerSettings,
    ) -> Result<(AxisInfo, Vec<f64>), TickerError> {
        let mut info = self.info(geometry);
        let ctx = TickContext::new(&info, settings);
        let mut locs = drawable_ticks(self.major_locator.locate(&ctx)?)?;
        if let Some(floor) = geometry.tick_floor {
            locs.retain(|&loc| loc > floor);
        }
        info.major_ticks.clone_from(&locs);
        Ok((info, locs))
    }

    /// Whether the minor locator places logit ticks on the axis of a polar
    /// projection, which matplotlib's `LogitLocator` refuses.
    fn minor_refuses_polar(&self) -> bool {
        let locator: &dyn Any = self.minor_locator.as_ref();
        self.role != AxisRole::Cartesian && locator.is::<LogitLocator>()
    }

    /// Every minor tick location for the view `ctx` knows; a logit
    /// locator of a polar axis refuses to place any, as matplotlib's
    /// `LogitLocator.tick_values` does (so drawing or measuring the axis
    /// fails while its minor ticks are shown).
    fn all_minor_locs(&self, ctx: &TickContext<'_>) -> Result<Vec<f64>, TickerError> {
        if self.minor_refuses_polar() {
            return Err(TickerError::PolarLogit);
        }
        self.minor_locator.locate(ctx).and_then(drawable_ticks)
    }

    /// The minor tick locations among `all` that are shown: those not on
    /// a `major` one, unless the axis keeps overlapping ones.
    fn minor_locs(&self, all: &[f64], major: &[f64], view: (f64, f64)) -> Vec<f64> {
        if self.remove_overlapping_locs {
            self.without_overlaps(all.to_vec(), major, view)
        } else {
            all.to_vec()
        }
    }

    /// The major (or `minor`) tick locations for the view of `geometry`,
    /// whether their ticks are drawn or not (matplotlib's
    /// `get_majorticklocs`, which places the major ticks alone, and
    /// `get_minorticklocs`, which a logit locator refuses on a polar axis).
    pub(crate) fn level_locs(
        &self,
        geometry: &AxisGeometry,
        minor: bool,
    ) -> Result<Vec<f64>, TickerError> {
        let settings = self.settings();
        let (info, major) = self.major_locs(geometry, &settings)?;
        if !minor {
            return Ok(major);
        }
        if self.minor_refuses_polar() {
            return Err(TickerError::PolarLogit);
        }
        let all = self.all_minor_locs(&TickContext::new(&info, &settings))?;
        Ok(self.minor_locs(&all, &major, geometry.view))
    }

    /// The ticks to draw for the view of `geometry`.
    pub(crate) fn compute_ticks(&self, geometry: &AxisGeometry) -> Result<TickSet, TickerError> {
        self.compute_levels(geometry, self.levels())
    }

    /// The ticks of `levels` for the view of `geometry`, those of the
    /// other levels left empty.
    fn compute_levels(
        &self,
        geometry: &AxisGeometry,
        levels: TickLevels,
    ) -> Result<TickSet, TickerError> {
        let settings = self.settings();
        let (info, major_locs) = self.major_locs(geometry, &settings)?;
        let ctx = TickContext::new(&info, &settings);
        let (lo, hi) = (self.forward(geometry.view.0), self.forward(geometry.view.1));
        let visible = |loc: f64| interval_contains_close((lo, hi), self.forward(loc), 1e-10);
        let unclipped = |locs: &[f64], labels: Vec<String>| -> Vec<Tick> {
            locs.iter()
                .zip(labels)
                .map(|(&loc, label)| Tick { loc, label })
                .collect()
        };
        let collect = |ticks: &[Tick]| {
            ticks
                .iter()
                .filter(|tick| visible(tick.loc))
                .cloned()
                .collect()
        };
        let in_view = |ticks: &[Tick]| -> (Vec<usize>, Vec<Tick>) {
            (ticks.iter().enumerate())
                .filter(|(_, tick)| visible(tick.loc))
                .map(|(index, tick)| (index, tick.clone()))
                .unzip()
        };
        // Like matplotlib's `_tick_group_visible`, a tick level whose
        // marks, labels and gridlines are all off is not computed.
        let mut ticks = TickSet::default();
        if levels.major {
            let labels = self.major_formatter.format_ticks(&major_locs, &ctx)?;
            ticks.major_unclipped = unclipped(&major_locs, labels.labels);
            (ticks.major_index, ticks.major) = in_view(&ticks.major_unclipped);
            ticks.offset = labels.offset;
            ticks.major_locs.clone_from(&major_locs);
        }
        if levels.minor {
            let all_locs = self.all_minor_locs(&ctx)?;
            let minor_locs = self.minor_locs(&all_locs, &major_locs, geometry.view);
            let labels = self.minor_formatter.format_ticks(&minor_locs, &ctx)?;
            ticks.minor_unclipped = unclipped(&minor_locs, labels.labels);
            (ticks.minor_index, ticks.minor) = in_view(&ticks.minor_unclipped);
            let labels = self.minor_formatter.format_ticks(&all_locs, &ctx)?;
            ticks.minor_all = collect(&unclipped(&all_locs, labels.labels));
            ticks.minor_locs = all_locs;
        }
        Ok(ticks)
    }

    /// Drops minor ticks that coincide with major ones (see
    /// [`OverlapTolerance`]), warning as numpy does about a tolerance that
    /// is not finite.
    fn without_overlaps(&self, minor: Vec<f64>, major: &[f64], view: (f64, f64)) -> Vec<f64> {
        let tolerance = OverlapTolerance::of_view(view, |value| self.forward(value));
        tolerance.warn_if_invalid();
        tolerance.remove_overlaps(minor, major, |value| self.forward(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rc::RcParams;

    fn geometry(view: (f64, f64)) -> AxisGeometry {
        AxisGeometry {
            view,
            data: view,
            minpos: f64::INFINITY,
            length_points: 357.12,
            tick_space: None,
            tick_floor: None,
        }
    }

    #[test]
    fn default_ticks_match_matplotlib() {
        // spec/goldens/figures/basic/line.json: xlim (0.85, 4.15)
        let mut axis = AxisState::new(AxisName::X, &RcParams::matplotlib());
        axis.update_ticks(&geometry((0.85, 4.15)));
        let ticks = axis.ticks.expect("valid ticks");
        let summary: Vec<(f64, &str)> = ticks
            .major
            .iter()
            .map(|t| (t.loc, t.label.as_str()))
            .collect();
        assert_eq!(
            summary,
            [
                (1.0, "1.0"),
                (1.5, "1.5"),
                (2.0, "2.0"),
                (2.5, "2.5"),
                (3.0, "3.0"),
                (3.5, "3.5"),
                (4.0, "4.0")
            ]
        );
        assert!(ticks.minor.is_empty(), "minor ticks are off by default");
    }

    #[test]
    fn tick_space_heuristic() {
        let axis = AxisState::new(AxisName::X, &RcParams::matplotlib());
        assert_eq!(axis.tick_space(357.12), 11);
        let y = AxisState::new(AxisName::Y, &RcParams::matplotlib());
        assert_eq!(y.tick_space(266.112), 13);
    }
}
