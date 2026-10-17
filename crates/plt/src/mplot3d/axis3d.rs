//! One axis of a 3-D axes: its tick helpers, label, pane and appearance
//! (the state of matplotlib's `axis3d.Axis`).

use crate::axes::{Tick, TickSet};
use crate::colors::Color;
use crate::fonts::FontProperties;
use crate::lines::LineStyle;
use crate::rc::{ColorOrInherit, RcParams};
use crate::scale::{AxisTickers, LinearScale, Scale};
use crate::ticker::{
    AxisInfo, AxisName, Formatter, Locator, ScaleHint, TickContext, TickerSettings,
};
use crate::transforms::interval_contains_close;

use super::coords::Axis3;

/// The label of a 3-D axis.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AxisLabel3D {
    /// The text; empty labels are not drawn.
    pub(crate) text: String,
    /// Font (`axes.labelsize`).
    pub(crate) font: FontProperties,
    /// Colour (`axes.labelcolor`).
    pub(crate) color: Color,
    /// Extra distance from the axis in points (`axes.labelpad`).
    pub(crate) pad: f64,
    /// Whether the label follows the axis direction; `None` rotates
    /// labels longer than four characters.
    pub(crate) rotate: Option<bool>,
}

impl AxisLabel3D {
    /// Whether the label is drawn parallel to its axis (matplotlib's
    /// `get_rotate_label`).
    pub(crate) fn rotated(&self) -> bool {
        self.rotate.unwrap_or_else(|| self.text.chars().count() > 4)
    }
}

/// How the lines and texts of a 3-D axis look (matplotlib's `_axinfo`
/// plus the tick properties).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AxisStyle {
    /// Width of the axis line in points (`axes.linewidth`).
    pub(crate) line_width: f64,
    /// Colour of the axis line (`axes.edgecolor`).
    pub(crate) line_color: Color,
    /// Gridline colour (`grid.color`).
    pub(crate) grid_color: Color,
    /// Gridline width in points (`grid.linewidth`).
    pub(crate) grid_width: f64,
    /// Gridline style (`grid.linestyle`).
    pub(crate) grid_style: LineStyle,
    /// Width of the tick marks in points (`xtick.major.width` for x and
    /// z, `ytick.major.width` for y).
    pub(crate) tick_width: f64,
    /// Colour of the tick marks (`xtick.color`).
    pub(crate) tick_color: Color,
    /// Distance between tick marks and labels in points
    /// (`xtick.major.pad`).
    pub(crate) tick_pad: f64,
    /// Font of the tick labels (`xtick.labelsize`).
    pub(crate) tick_font: FontProperties,
    /// Colour of the tick labels (`xtick.labelcolor`).
    pub(crate) tick_label_color: Color,
}

impl AxisStyle {
    /// matplotlib's defaults for axis `dir` under `rc` (every 3-D axis
    /// is an `XAxis` with `XTick`s).
    fn from_rc(dir: Axis3, rc: &RcParams) -> Self {
        let tick_label_color = match &rc.xtick_labelcolor {
            ColorOrInherit::Inherit => rc.xtick_color.clone(),
            ColorOrInherit::Color(color) => color.clone(),
        };
        Self {
            line_width: rc.axes_linewidth,
            line_color: rc.axes_edgecolor.clone(),
            grid_color: rc.grid_color.clone(),
            grid_width: rc.grid_linewidth,
            grid_style: rc.grid_linestyle.clone(),
            tick_width: if dir == Axis3::Y {
                rc.ytick_major_width
            } else {
                rc.xtick_major_width
            },
            tick_color: rc.xtick_color.clone(),
            tick_pad: rc.xtick_major_pad,
            tick_font: FontProperties::from_rc(rc).with_size(rc.xtick_labelsize, rc.font_size),
            tick_label_color,
        }
    }
}

/// The limits an axis places its ticks for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AxisSpan {
    /// The view interval (decreasing when inverted).
    pub(crate) view: (f64, f64),
    /// The data interval.
    pub(crate) data: (f64, f64),
    /// The smallest positive datum.
    pub(crate) minpos: f64,
}

/// State of one axis of a 3-D axes.
#[derive(Debug)]
pub(crate) struct Axis3D {
    pub(crate) dir: Axis3,
    pub(crate) major_locator: Box<dyn Locator>,
    pub(crate) major_formatter: Box<dyn Formatter>,
    pub(crate) settings: TickerSettings,
    pub(crate) label: AxisLabel3D,
    /// Fill and edge colour of the pane behind the data.
    pub(crate) pane_color: Color,
    pub(crate) style: AxisStyle,
    /// Size of the tick labels in points (drives the tick-space
    /// heuristic).
    tick_label_size: f64,
    /// Ticks computed by the last layout pass, or why they could not be.
    pub(crate) ticks: Result<TickSet, String>,
}

impl Axis3D {
    /// A linear axis configured from `rc`.
    pub(crate) fn new(dir: Axis3, rc: &RcParams) -> Self {
        let settings = TickerSettings::from(rc);
        let AxisTickers {
            major_locator,
            major_formatter,
            ..
        } = LinearScale.default_tickers(AxisName::X, &settings);
        let pane_color = match dir {
            Axis3::X => &rc.axes3d_xaxis_panecolor,
            Axis3::Y => &rc.axes3d_yaxis_panecolor,
            Axis3::Z => &rc.axes3d_zaxis_panecolor,
        };
        Self {
            dir,
            major_locator,
            major_formatter,
            settings,
            label: AxisLabel3D {
                text: String::new(),
                font: FontProperties::from_rc(rc).with_size(rc.axes_labelsize, rc.font_size),
                color: rc.axes_labelcolor.clone(),
                pad: rc.axes_labelpad,
                rotate: None,
            },
            pane_color: pane_color.clone(),
            style: AxisStyle::from_rc(dir, rc),
            tick_label_size: rc.xtick_labelsize.to_points(rc.font_size),
            ticks: Ok(TickSet::default()),
        }
    }

    /// What tick helpers know about the axis.
    pub(crate) fn info(&self, span: &AxisSpan, length_points: f64) -> AxisInfo {
        AxisInfo {
            name: AxisName::X,
            view_interval: span.view,
            data_interval: span.data,
            minpos: span.minpos,
            tick_space: self.tick_space(length_points),
            major_ticks: Vec::new(),
            scale: ScaleHint::Linear,
            three_d: true,
        }
    }

    /// How many ticks fit along an axis `length_points` long (matplotlib's
    /// `XAxis.get_tick_space`, which every 3-D axis inherits).
    fn tick_space(&self, length_points: f64) -> usize {
        let size = self.tick_label_size * 3.0;
        if size > 0.0 {
            (length_points / size).floor().max(0.0) as usize
        } else {
            usize::MAX >> 1
        }
    }

    /// `(v0, v1)` expanded when singular by the major locator.
    pub(crate) fn nonsingular(&self, span: &AxisSpan, v0: f64, v1: f64) -> (f64, f64) {
        let info = self.info(span, 0.0);
        let ctx = TickContext::new(&info, &self.settings);
        self.major_locator.nonsingular(v0, v1, &ctx)
    }

    /// The major locator's view limits for data spanning `(v0, v1)`.
    pub(crate) fn view_limits(
        &self,
        span: &AxisSpan,
        v0: f64,
        v1: f64,
    ) -> Result<(f64, f64), String> {
        let info = self.info(span, 0.0);
        let ctx = TickContext::new(&info, &self.settings);
        self.major_locator
            .view_limits(v0, v1, &ctx)
            .map_err(|e| e.to_string())
    }

    /// Recomputes the major ticks inside the view (matplotlib's
    /// `Axis._update_ticks`) for an axes `length_points` wide.
    pub(crate) fn update_ticks(&mut self, span: &AxisSpan, length_points: f64) {
        self.ticks = self.compute_ticks(span, length_points);
    }

    fn compute_ticks(&self, span: &AxisSpan, length_points: f64) -> Result<TickSet, String> {
        let mut info = self.info(span, length_points);
        let locs = {
            let ctx = TickContext::new(&info, &self.settings);
            self.major_locator.locate(&ctx).map_err(|e| e.to_string())?
        };
        info.major_ticks.clone_from(&locs);
        let ctx = TickContext::new(&info, &self.settings);
        let labels = self
            .major_formatter
            .format_ticks(&locs, &ctx)
            .map_err(|e| e.to_string())?;
        let major = locs
            .iter()
            .zip(labels.labels)
            .filter(|(loc, _)| interval_contains_close(span.view, **loc, 1e-10))
            .map(|(&loc, label)| Tick { loc, label })
            .collect();
        Ok(TickSet {
            major,
            minor: Vec::new(),
            offset: labels.offset,
        })
    }

    /// The major ticks drawn, empty when they could not be computed.
    pub(crate) fn drawn_ticks(&self) -> &[Tick] {
        self.ticks.as_ref().map_or(&[], |ticks| &ticks.major)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(view: (f64, f64)) -> AxisSpan {
        AxisSpan {
            view,
            data: view,
            minpos: f64::INFINITY,
        }
    }

    #[test]
    fn default_ticks_match_matplotlib() {
        // An empty 3-D axes: limits (-1/48, 1 + 1/48), axes 266.112 pt wide.
        let mut axis = Axis3D::new(Axis3::Z, &RcParams::matplotlib());
        axis.update_ticks(&span((-1.0 / 48.0, 1.0 + 1.0 / 48.0)), 266.112);
        let labels: Vec<&str> = axis
            .drawn_ticks()
            .iter()
            .map(|t| t.label.as_str())
            .collect();
        assert_eq!(labels, ["0.0", "0.2", "0.4", "0.6", "0.8", "1.0"]);
    }

    #[test]
    fn labels_rotate_when_long() {
        let mut label = Axis3D::new(Axis3::X, &RcParams::matplotlib()).label;
        label.text = "abcd".into();
        assert!(!label.rotated());
        label.text = "X axis".into();
        assert!(label.rotated());
        label.rotate = Some(false);
        assert!(!label.rotated());
    }

    #[test]
    fn inverted_views_keep_their_ticks() {
        let mut axis = Axis3D::new(Axis3::X, &RcParams::matplotlib());
        axis.update_ticks(&span((1.0, 0.0)), 266.112);
        assert_eq!(axis.drawn_ticks().len(), 6);
    }

    #[test]
    fn steps_are_a_little_finer_than_in_2d() {
        // matplotlib's z ticks for zlim (-0.1458, 7.1458) on a 4 in axes
        // (tick space 7): a step of 1, where a 2-D axis would take 2.
        let mut axis = Axis3D::new(Axis3::Z, &RcParams::matplotlib());
        axis.update_ticks(&span((-7.0 / 48.0, 7.0 + 7.0 / 48.0)), 221.76);
        let locs: Vec<f64> = axis.drawn_ticks().iter().map(|t| t.loc).collect();
        assert_eq!(locs, [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]);
    }
}
