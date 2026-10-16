//! One axis of an axes: its scale, tick helpers and computed ticks
//! (matplotlib's `Axis`).

use std::rc::Rc;

use super::ticks::TickStyle;
use crate::colors::Color;
use crate::fonts::FontProperties;
use crate::rc::RcParams;
use crate::scale::{LinearScale, Scale};
use crate::ticker::{AxisInfo, AxisName, Formatter, Locator, TickContext, TickerSettings};
use crate::transforms::AxisMap;

/// A tick with its label text.
#[derive(Clone, Debug, PartialEq)]
pub struct Tick {
    /// Location in data coordinates.
    pub loc: f64,
    /// Label text (possibly empty).
    pub label: String,
}

/// The ticks an axis draws in the current view.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TickSet {
    /// Major ticks inside the view.
    pub major: Vec<Tick>,
    /// Minor ticks inside the view (excluding those on major ticks).
    pub minor: Vec<Tick>,
    /// Offset or scale text shown at the end of the axis (e.g. `1e6`).
    pub offset: String,
}

/// The label of an axis (matplotlib's `Axis.label`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AxisLabel {
    /// Label text; empty labels are not drawn.
    pub(crate) text: String,
    /// Font (`axes.labelsize`, `axes.labelweight`).
    pub(crate) font: FontProperties,
    /// Colour (`axes.labelcolor`).
    pub(crate) color: Color,
    /// Gap to the tick labels in points (`axes.labelpad`).
    pub(crate) pad: f64,
}

/// Autoscaling configuration and state of one axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AutoscaleState {
    /// Whether the view follows the data.
    pub(crate) enabled: bool,
    /// Padding added around the data, as a fraction of its span.
    pub(crate) margin: f64,
    /// Set when data changed and the view must be recomputed before use.
    pub(crate) stale: bool,
}

/// The geometry an axis needs to place its ticks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AxisGeometry {
    /// Visible interval (may be decreasing).
    pub(crate) view: (f64, f64),
    /// Interval spanned by the data.
    pub(crate) data: (f64, f64),
    /// Smallest positive data value.
    pub(crate) minpos: f64,
    /// Length of the axis in points.
    pub(crate) length_points: f64,
}

/// State of one axis.
#[derive(Debug)]
pub(crate) struct AxisState {
    pub(crate) name: AxisName,
    scale: Box<dyn Scale>,
    map: Option<Rc<dyn AxisMap>>,
    pub(crate) major_locator: Box<dyn Locator>,
    pub(crate) major_formatter: Box<dyn Formatter>,
    pub(crate) minor_locator: Box<dyn Locator>,
    pub(crate) minor_formatter: Box<dyn Formatter>,
    pub(crate) settings: TickerSettings,
    pub(crate) autoscale: AutoscaleState,
    /// Size of tick labels in points (drives the tick-space heuristic).
    pub(crate) label_size: f64,
    /// Ticks computed by the last prepare pass, or why they could not be.
    pub(crate) ticks: Result<TickSet, String>,
    /// Appearance of the major ticks.
    pub(crate) major_style: TickStyle,
    /// Appearance of the minor ticks.
    pub(crate) minor_style: TickStyle,
    /// The axis label.
    pub(crate) label: AxisLabel,
}

impl AxisState {
    /// A linear axis configured from `rc`.
    pub(crate) fn new(name: AxisName, rc: &RcParams) -> Self {
        let settings = TickerSettings::from(rc);
        let (margin, label_size) = match name {
            AxisName::X => (rc.axes_xmargin, rc.xtick_labelsize),
            AxisName::Y => (rc.axes_ymargin, rc.ytick_labelsize),
        };
        let scale: Box<dyn Scale> = Box::new(LinearScale);
        let tickers = scale.default_tickers(name, &settings);
        Self {
            name,
            map: scale.axis_map(),
            scale,
            major_locator: tickers.major_locator,
            major_formatter: tickers.major_formatter,
            minor_locator: tickers.minor_locator,
            minor_formatter: tickers.minor_formatter,
            autoscale: AutoscaleState {
                enabled: true,
                margin,
                stale: false,
            },
            label_size: label_size.to_points(rc.font_size),
            settings,
            ticks: Ok(TickSet::default()),
            major_style: TickStyle::from_rc(name, true, rc),
            minor_style: TickStyle::from_rc(name, false, rc),
            label: AxisLabel {
                text: String::new(),
                font: FontProperties::from_rc(rc).with_size(rc.axes_labelsize, rc.font_size),
                color: rc.axes_labelcolor.clone(),
                pad: rc.axes_labelpad,
            },
        }
    }

    /// The scale.
    pub(crate) fn scale(&self) -> &dyn Scale {
        self.scale.as_ref()
    }

    /// Replaces the scale and installs its default tick helpers
    /// (matplotlib's `Axis._set_scale`).
    pub(crate) fn set_scale(&mut self, scale: Box<dyn Scale>) {
        let tickers = scale.default_tickers(self.name, &self.settings);
        self.major_locator = tickers.major_locator;
        self.major_formatter = tickers.major_formatter;
        self.minor_locator = tickers.minor_locator;
        self.minor_formatter = tickers.minor_formatter;
        self.map = scale.axis_map();
        self.scale = scale;
    }

    /// The non-linear part of the scale (`None` for linear axes).
    pub(crate) fn map(&self) -> Option<&Rc<dyn AxisMap>> {
        self.map.as_ref()
    }

    /// Maps a value through the scale.
    pub(crate) fn forward(&self, value: f64) -> f64 {
        self.map.as_ref().map_or(value, |m| m.forward(value))
    }

    /// What tick helpers know about this axis.
    pub(crate) fn info(&self, geometry: &AxisGeometry) -> AxisInfo {
        AxisInfo {
            name: self.name,
            view_interval: geometry.view,
            data_interval: geometry.data,
            minpos: geometry.minpos,
            tick_space: self.tick_space(geometry.length_points),
            major_ticks: Vec::new(),
            scale: self.scale.hint(),
        }
    }

    /// How many ticks fit (matplotlib's `get_tick_space`): tick labels are
    /// assumed to be at most 3:1 (x) or 2:1 (y) wide.
    fn tick_space(&self, length_points: f64) -> usize {
        let ratio = match self.name {
            AxisName::X => 3.0,
            AxisName::Y => 2.0,
        };
        let size = self.label_size * ratio;
        if size > 0.0 {
            (length_points / size).floor().max(0.0) as usize
        } else {
            usize::MAX >> 1
        }
    }

    /// Computes the ticks to draw for the current view (matplotlib's
    /// `Axis._update_ticks`).
    pub(crate) fn update_ticks(&mut self, geometry: &AxisGeometry) {
        self.ticks = self.compute_ticks(geometry).map_err(|err| err.to_string());
    }

    fn compute_ticks(
        &self,
        geometry: &AxisGeometry,
    ) -> Result<TickSet, crate::ticker::TickerError> {
        let mut info = self.info(geometry);
        let major_locs = {
            let ctx = TickContext::new(&info, &self.settings);
            self.major_locator.locate(&ctx)?
        };
        info.major_ticks.clone_from(&major_locs);
        let ctx = TickContext::new(&info, &self.settings);
        let major_labels = self.major_formatter.format_ticks(&major_locs, &ctx)?;
        let minor_locs =
            self.without_overlaps(self.minor_locator.locate(&ctx)?, &major_locs, geometry.view);
        let minor_labels = self.minor_formatter.format_ticks(&minor_locs, &ctx)?;

        let (lo, hi) = (self.forward(geometry.view.0), self.forward(geometry.view.1));
        let visible = |loc: f64| interval_contains_close((lo, hi), self.forward(loc), 1e-10);
        let collect = |locs: &[f64], labels: Vec<String>| {
            locs.iter()
                .zip(labels)
                .filter(|(loc, _)| visible(**loc))
                .map(|(&loc, label)| Tick { loc, label })
                .collect()
        };
        Ok(TickSet {
            major: collect(&major_locs, major_labels.labels),
            minor: collect(&minor_locs, minor_labels.labels),
            offset: major_labels.offset,
        })
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

/// matplotlib's `_interval_contains_close`: `val` within the sorted
/// interval widened by `rtol` times its length.
pub(crate) fn interval_contains_close((a, b): (f64, f64), val: f64, rtol: f64) -> bool {
    let (a, b) = if a > b { (b, a) } else { (a, b) };
    let r = (b - a) * rtol;
    a - r <= val && val <= b + r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry(view: (f64, f64)) -> AxisGeometry {
        AxisGeometry {
            view,
            data: view,
            minpos: f64::INFINITY,
            length_points: 357.12,
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

    #[test]
    fn interval_tolerance() {
        assert!(interval_contains_close((0.0, 1.0), 1.0 + 1e-12, 1e-10));
        assert!(interval_contains_close((1.0, 0.0), 0.5, 1e-10));
        assert!(!interval_contains_close((0.0, 1.0), 1.1, 1e-10));
    }
}
