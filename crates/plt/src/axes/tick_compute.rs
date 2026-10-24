//! Computing the ticks of an axis for its view (matplotlib's
//! `Axis._update_ticks` and `get_tick_space`): the locators' locations,
//! the formatters' labels, and the minor ticks that do not fall on major
//! ones.

use super::axis::{AxisGeometry, AxisState};
use super::tick_set::{Tick, TickSet};
use crate::ticker::{AxisInfo, AxisName, TickContext, TickerError, drawable_ticks};
use crate::transforms::interval_contains_close;

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
    /// `Axis._update_ticks`).
    pub(crate) fn update_ticks(&mut self, geometry: &AxisGeometry) {
        self.ticks = self.compute_ticks(geometry);
    }

    fn compute_ticks(&self, geometry: &AxisGeometry) -> Result<TickSet, TickerError> {
        let mut info = self.info(geometry);
        let mut major_locs = {
            let ctx = TickContext::new(&info, &self.settings);
            drawable_ticks(self.major_locator.locate(&ctx)?)?
        };
        if let Some(floor) = geometry.tick_floor {
            major_locs.retain(|&loc| loc > floor);
        }
        info.major_ticks.clone_from(&major_locs);
        let ctx = TickContext::new(&info, &self.settings);
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
        if self.major_style.is_drawn() {
            let labels = self.major_formatter.format_ticks(&major_locs, &ctx)?;
            ticks.major_unclipped = unclipped(&major_locs, labels.labels);
            (ticks.major_index, ticks.major) = in_view(&ticks.major_unclipped);
            ticks.offset = labels.offset;
            ticks.major_locs.clone_from(&major_locs);
        }
        if self.minor_style.is_drawn() {
            let all_locs = drawable_ticks(self.minor_locator.locate(&ctx)?)?;
            let minor_locs = if self.remove_overlapping_locs {
                self.without_overlaps(all_locs.clone(), &major_locs, geometry.view)
            } else {
                all_locs.clone()
            };
            let labels = self.minor_formatter.format_ticks(&minor_locs, &ctx)?;
            ticks.minor_unclipped = unclipped(&minor_locs, labels.labels);
            (ticks.minor_index, ticks.minor) = in_view(&ticks.minor_unclipped);
            let labels = self.minor_formatter.format_ticks(&all_locs, &ctx)?;
            ticks.minor_all = collect(&unclipped(&all_locs, labels.labels));
            ticks.minor_locs = all_locs;
        }
        Ok(ticks)
    }

    /// Drops minor ticks that coincide with major ones (matplotlib's
    /// `get_minorticklocs` with `np.isclose(atol=span * 1e-5)` in scale space).
    fn without_overlaps(&self, minor: Vec<f64>, major: &[f64], view: (f64, f64)) -> Vec<f64> {
        let (lo, hi) = (self.forward(view.0), self.forward(view.1));
        let tolerance = (hi - lo).abs() * 1e-5;
        let major: Vec<f64> = major.iter().map(|&m| self.forward(m)).collect();
        minor
            .into_iter()
            .filter(|&m| {
                let t = self.forward(m);
                !major.iter().any(|&j| (t - j).abs() <= tolerance)
            })
            .collect()
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
