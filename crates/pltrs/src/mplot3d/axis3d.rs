//! One axis of a 3-D axes: its scale, tick helpers, tick styles, label,
//! pane and appearance (the state of matplotlib's `axis3d.Axis`).

use std::rc::Rc;

use pltrs_core::Rgba;

use crate::axes::{Tick, TickSet, TickStyle, TickerDefaults, grid_level_on};
use crate::colors::Color;
use crate::fonts::FontProperties;
use crate::lines::LineStyle;
use crate::rc::RcParams;
use crate::scale::{AxisTickers, Scale};
use crate::ticker::{
    AxisInfo, AxisName, Formatter, Locator, NullLocator, TickContext, TickerError, TickerSettings,
    drawable_ticks,
};
use crate::transforms::interval_contains_close;
use crate::units::{AxisUnits, Categories};

use super::coords::Axis3;
use super::geometry::AxisPosition;
use super::scaling::AxisScale;

/// The view inset matplotlib checks tick visibility against with
/// `axes3d.automargin` (`1/48 * 23/24`, in `Axis._update_ticks`).
const AUTOMARGIN_TICK_INSET: f64 = 0.019_965_277_777_777_776;

/// The rc tick parameters of the major (`major = true`) or minor ticks of
/// a 3-D axis: those of an `XTick`, with gridlines following `axes.grid`
/// and `axes.grid.which` whatever `axes.grid.axis` says (`Axes3D.grid`
/// does not take the 2-D axes' `axis` argument).
pub(crate) fn tick_style(major: bool, rc: &RcParams) -> TickStyle {
    TickStyle {
        grid_on: grid_level_on(major, rc),
        ..TickStyle::from_rc(AxisName::X, major, rc)
    }
}

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
    /// Whether the label was never set, so that a unit converter may
    /// give it (matplotlib's `isDefault_label`).
    pub(crate) is_default: bool,
}

pltrs_core::archive_struct!(AxisLabel3D {
    text,
    font,
    color,
    pad,
    rotate,
    is_default
});

impl AxisLabel3D {
    /// Whether the label is drawn parallel to its axis (matplotlib's
    /// `get_rotate_label`).
    pub(crate) fn rotated(&self) -> bool {
        self.rotate.unwrap_or_else(|| self.text.chars().count() > 4)
    }
}

/// How the axis line, the gridlines and the tick marks of a 3-D axis are
/// drawn whatever the tick parameters say (matplotlib's `_axinfo`).
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
    /// Width of the major tick marks in points (`xtick.major.width` for x
    /// and z, `ytick.major.width` for y).
    pub(crate) major_tick_width: f64,
    /// Width of the minor tick marks in points (`xtick.minor.width` for x
    /// and z, `ytick.minor.width` for y).
    pub(crate) minor_tick_width: f64,
}

pltrs_core::archive_struct!(AxisStyle {
    line_width,
    line_color,
    grid_color,
    grid_width,
    grid_style,
    major_tick_width,
    minor_tick_width
});

impl AxisStyle {
    /// matplotlib's defaults for axis `dir` under `rc`.
    fn from_rc(dir: Axis3, rc: &RcParams) -> Self {
        if rc.internal_classic_mode {
            // Fixed thin black axis lines, light solid gridlines and tick
            // marks as wide as lines.
            return Self {
                line_width: 0.75,
                line_color: Rgba::BLACK.into(),
                grid_color: Rgba::opaque(0.9, 0.9, 0.9).into(),
                grid_width: 1.0,
                grid_style: LineStyle::Solid,
                major_tick_width: rc.lines_linewidth,
                minor_tick_width: rc.lines_linewidth,
            };
        }
        let (major_tick_width, minor_tick_width) = if dir == Axis3::Y {
            (rc.ytick_major_width, rc.ytick_minor_width)
        } else {
            (rc.xtick_major_width, rc.xtick_minor_width)
        };
        Self {
            line_width: rc.axes_linewidth,
            line_color: rc.axes_edgecolor.clone(),
            grid_color: rc.grid_color.clone(),
            grid_width: rc.grid_linewidth,
            grid_style: rc.grid_linestyle.clone(),
            major_tick_width,
            minor_tick_width,
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

/// A tick to draw with the style of its level.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DrawnTick<'a> {
    pub(crate) tick: &'a Tick,
    /// The tick parameters of its level (colour, label font).
    pub(crate) style: &'a TickStyle,
    /// The width of its mark in points.
    pub(crate) width: f64,
    /// The distance of its label in points (its own pad).
    pub(crate) pad: f64,
}

/// The tick locators and formatters of an axis, shared with the axes
/// sharing it (matplotlib's `major` and `minor` `Ticker`s).
#[derive(Clone, Debug)]
pub(crate) struct Tickers3D {
    pub(crate) major_locator: Rc<dyn Locator>,
    pub(crate) major_formatter: Rc<dyn Formatter>,
    pub(crate) minor_locator: Rc<dyn Locator>,
    pub(crate) minor_formatter: Rc<dyn Formatter>,
    /// Which tick helpers are still the defaults a unit converter may
    /// replace (matplotlib's `isDefault_majloc` and friends).
    pub(crate) defaults: TickerDefaults,
}

pltrs_core::archive_struct!(Tickers3D {
    major_locator,
    major_formatter,
    minor_locator,
    minor_formatter,
    defaults
});

impl Tickers3D {
    /// The default tick helpers of `scale` on axis `dir` (matplotlib's
    /// `set_default_locators_and_formatters`; its linear scale only turns
    /// minor ticks on for axes named x or y).
    fn defaults(scale: &dyn Scale, dir: Axis3, settings: &TickerSettings) -> Self {
        let AxisTickers {
            major_locator,
            minor_locator,
            major_formatter,
            minor_formatter,
        } = match dir {
            Axis3::X => scale.default_tickers(AxisName::X, settings),
            Axis3::Y => scale.default_tickers(AxisName::Y, settings),
            Axis3::Z => scale.default_tickers(
                AxisName::X,
                &TickerSettings {
                    xtick_minor_visible: false,
                    ..settings.clone()
                },
            ),
        };
        Self {
            major_locator: Rc::from(major_locator),
            major_formatter: Rc::from(major_formatter),
            minor_locator: Rc::from(minor_locator),
            minor_formatter: Rc::from(minor_formatter),
            defaults: TickerDefaults::default(),
        }
    }
}

/// State of one axis of a 3-D axes.
#[derive(Debug)]
pub(crate) struct Axis3DState {
    pub(crate) dir: Axis3,
    pub(crate) scale: AxisScale,
    pub(crate) tickers: Tickers3D,
    /// The unit converter, how it was chosen, and the unit.
    pub(crate) units: AxisUnits,
    /// Category names, once categorical values were plotted.
    pub(crate) categories: Option<Categories>,
    pub(crate) label: AxisLabel3D,
    /// Fill and edge colour of the pane behind the data.
    pub(crate) pane_color: Color,
    pub(crate) style: AxisStyle,
    /// The tick parameters of the major ticks (every 3-D tick is an
    /// `XTick`).
    pub(crate) major_style: TickStyle,
    /// The tick parameters of the minor ticks.
    pub(crate) minor_style: TickStyle,
    /// Colour of the offset text (`xtick.color`, then the tick label
    /// colour set by tick parameters).
    pub(crate) offset_color: Color,
    /// Ticks computed by the last layout pass, or why they could not be.
    pub(crate) ticks: Result<TickSet, TickerError>,
    /// Whether the axis line is drawn (matplotlib's `axis.line` visibility).
    pub(crate) line_visible: bool,
    /// The edges carrying the line, tick marks, tick labels and offset
    /// text.
    pub(crate) tick_position: AxisPosition,
    /// The edges carrying the label.
    pub(crate) label_position: AxisPosition,
    /// The label pads of the first major ticks in the order the major
    /// locator places them: matplotlib's `Tick` objects keep their own
    /// pads, and the ticks it adds later copy the first one. Empty when
    /// every major tick has the pad of [`Self::major_style`].
    pub(crate) major_pads: Vec<f64>,
}

pltrs_core::archive_struct!(Axis3DState {
    dir,
    scale,
    tickers,
    units,
    categories,
    label,
    pane_color,
    style,
    major_style,
    minor_style,
    offset_color,
    ticks,
    line_visible,
    tick_position,
    label_position,
    major_pads
});

impl Axis3DState {
    /// A linear axis configured from `rc`.
    pub(crate) fn new(dir: Axis3, rc: &RcParams) -> Self {
        let scale = AxisScale::default();
        let pane_color = match dir {
            Axis3::X => &rc.axes3d_xaxis_panecolor,
            Axis3::Y => &rc.axes3d_yaxis_panecolor,
            Axis3::Z => &rc.axes3d_zaxis_panecolor,
        };
        Self {
            dir,
            tickers: Tickers3D::defaults(scale.scale().as_ref(), dir, &TickerSettings::from(rc)),
            scale,
            units: AxisUnits::default(),
            categories: None,
            label: AxisLabel3D {
                text: String::new(),
                font: crate::axes::axis_label::label_font(rc),
                color: rc.axes_labelcolor.clone(),
                pad: rc.axes_labelpad,
                rotate: None,
                is_default: true,
            },
            pane_color: pane_color.clone(),
            style: AxisStyle::from_rc(dir, rc),
            major_style: tick_style(true, rc),
            minor_style: tick_style(false, rc),
            offset_color: rc.xtick_color.clone(),
            ticks: Ok(TickSet::default()),
            line_visible: true,
            tick_position: AxisPosition::Default,
            label_position: AxisPosition::Default,
            major_pads: Vec::new(),
        }
    }

    /// Whether the offset text is drawn: with the major tick labels
    /// (matplotlib's offset text visibility following `label1On` and
    /// `label2On`).
    pub(crate) fn offset_visible(&self) -> bool {
        self.major_style.labels.first || self.major_style.labels.second
    }

    /// Replaces the scale and installs its default tick helpers, made
    /// under the current rcParams (matplotlib's `Axis._set_scale`).
    pub(crate) fn set_scale(&mut self, scale: AxisScale) {
        let settings = TickerSettings::current();
        self.tickers = Tickers3D::defaults(scale.scale().as_ref(), self.dir, &settings);
        self.scale = scale;
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
            scale: self.scale.scale().hint(),
            three_d: true,
        }
    }

    /// How many ticks fit along an axis `length_points` long (matplotlib's
    /// `XAxis.get_tick_space`, which every 3-D axis inherits).
    fn tick_space(&self, length_points: f64) -> usize {
        let size = self.major_style.label_size * 3.0;
        if size > 0.0 {
            (length_points / size).floor().max(0.0) as usize
        } else {
            usize::MAX >> 1
        }
    }

    /// `(v0, v1)` expanded when singular by the major locator.
    pub(crate) fn nonsingular(&self, span: &AxisSpan, v0: f64, v1: f64) -> (f64, f64) {
        let info = self.info(span, 0.0);
        let settings = TickerSettings::current();
        let ctx = TickContext::new(&info, &settings);
        self.tickers.major_locator.nonsingular(v0, v1, &ctx)
    }

    /// The major locator's view limits for data spanning `(v0, v1)`.
    pub(crate) fn view_limits(
        &self,
        span: &AxisSpan,
        v0: f64,
        v1: f64,
    ) -> Result<(f64, f64), TickerError> {
        let info = self.info(span, 0.0);
        let settings = TickerSettings::current();
        let ctx = TickContext::new(&info, &settings);
        self.tickers.major_locator.view_limits(v0, v1, &ctx)
    }

    /// Recomputes the major and minor ticks inside the view (matplotlib's
    /// `Axis._update_ticks`) for an axes `length_points` wide; with
    /// `automargin` the view is inset as `axes3d.automargin` asks.
    pub(crate) fn update_ticks(&mut self, span: &AxisSpan, length_points: f64, automargin: bool) {
        self.ticks = self.compute_ticks(span, length_points, automargin);
    }

    /// The label pad of the major tick at `slot` in the locator's order.
    fn major_pad(&self, slot: usize) -> f64 {
        (self.major_pads.get(slot))
            .or(self.major_pads.first())
            .copied()
            .unwrap_or(self.major_style.pad)
    }

    /// The label pads of the major ticks the last layout pass placed,
    /// inside the view or not (matplotlib's `get_pad` of every tick of
    /// `get_major_ticks()`).
    pub(crate) fn major_tick_pads(&self) -> Vec<f64> {
        let located = self
            .ticks
            .as_ref()
            .map_or(0, |ticks| ticks.major_unclipped.len());
        (0..located).map(|slot| self.major_pad(slot)).collect()
    }

    /// The ticks of the view, inside it or not, with the position among
    /// those of each one inside.
    fn compute_ticks(
        &self,
        span: &AxisSpan,
        length_points: f64,
        automargin: bool,
    ) -> Result<TickSet, TickerError> {
        let mut info = self.info(span, length_points);
        let settings = TickerSettings::current();
        let major_locs = {
            let ctx = TickContext::new(&info, &settings);
            drawable_ticks(self.tickers.major_locator.locate(&ctx)?)?
        };
        info.major_ticks.clone_from(&major_locs);
        let ctx = TickContext::new(&info, &settings);
        let visible = self.visibility(span.view, automargin);
        let unclipped = |locs: &[f64], labels: Vec<String>| -> Vec<Tick> {
            locs.iter()
                .zip(labels)
                .map(|(&loc, label)| Tick { loc, label })
                .collect()
        };
        let inside = |ticks: &[Tick]| -> (Vec<Tick>, Vec<usize>) {
            (ticks.iter().enumerate())
                .filter(|(_, tick)| visible(tick.loc))
                .map(|(index, tick)| (tick.clone(), index))
                .unzip()
        };
        let mut ticks = TickSet::default();
        if Self::level_computed(&self.major_style, self.tickers.major_locator.as_ref()) {
            let labels = self
                .tickers
                .major_formatter
                .format_ticks(&major_locs, &ctx)?;
            ticks.major_unclipped = unclipped(&major_locs, labels.labels);
            (ticks.major, ticks.major_index) = inside(&ticks.major_unclipped);
            ticks.offset = labels.offset;
        }
        if Self::level_computed(&self.minor_style, self.tickers.minor_locator.as_ref()) {
            let minor = drawable_ticks(self.tickers.minor_locator.locate(&ctx)?)?;
            let minor_locs = self.without_overlaps(minor, &major_locs, span.view);
            let labels = self
                .tickers
                .minor_formatter
                .format_ticks(&minor_locs, &ctx)?;
            ticks.minor_unclipped = unclipped(&minor_locs, labels.labels);
            (ticks.minor, ticks.minor_index) = inside(&ticks.minor_unclipped);
        }
        Ok(ticks)
    }

    /// Whether a tick level is computed: something of it is shown and its
    /// locator places ticks (matplotlib's `_tick_group_visible` and
    /// `NullLocator` check).
    fn level_computed(style: &TickStyle, locator: &dyn Locator) -> bool {
        let any: &dyn std::any::Any = locator;
        style.is_drawn() && !any.is::<NullLocator>()
    }

    /// Whether a tick at a location is inside the view, compared in
    /// transformed coordinates.
    fn visibility(&self, view: (f64, f64), automargin: bool) -> impl Fn(f64) -> bool + '_ {
        let (mut lo, mut hi) = if view.0 > view.1 {
            (view.1, view.0)
        } else {
            view
        };
        if automargin {
            let delta = hi - lo;
            (lo, hi) = (
                lo + delta * AUTOMARGIN_TICK_INSET,
                hi - delta * AUTOMARGIN_TICK_INSET,
            );
        }
        let interval = (self.scale.forward(lo), self.scale.forward(hi));
        move |loc| interval_contains_close(interval, self.scale.forward(loc), 1e-10)
    }

    /// Drops minor ticks that coincide with major ones (matplotlib's
    /// `get_minorticklocs` with `np.isclose(atol=span * 1e-5)` in
    /// transformed coordinates).
    fn without_overlaps(&self, minor: Vec<f64>, major: &[f64], view: (f64, f64)) -> Vec<f64> {
        let (lo, hi) = (self.scale.forward(view.0), self.scale.forward(view.1));
        let tolerance = (hi - lo).abs() * 1e-5;
        let major: Vec<f64> = major.iter().map(|&m| self.scale.forward(m)).collect();
        minor
            .into_iter()
            .filter(|&m| {
                let t = self.scale.forward(m);
                !major.iter().any(|&j| (t - j).abs() <= tolerance)
            })
            .collect()
    }

    /// The ticks drawn, major ones first; none when they could not be
    /// computed.
    pub(crate) fn drawn_ticks(&self) -> impl Iterator<Item = DrawnTick<'_>> {
        let (major, slots, minor): (&[Tick], &[usize], &[Tick]) = match &self.ticks {
            Ok(ticks) => (&ticks.major, &ticks.major_index, &ticks.minor),
            Err(_) => (&[], &[], &[]),
        };
        let (style, width) = (&self.major_style, self.style.major_tick_width);
        let majors = (major.iter().zip(slots)).map(move |(tick, &slot)| DrawnTick {
            tick,
            style,
            width,
            pad: self.major_pad(slot),
        });
        let (style, width) = (&self.minor_style, self.style.minor_tick_width);
        majors.chain(minor.iter().map(move |tick| DrawnTick {
            tick,
            style,
            width,
            pad: style.pad,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::LogScale;

    fn span(view: (f64, f64)) -> AxisSpan {
        AxisSpan {
            view,
            data: view,
            minpos: f64::INFINITY,
        }
    }

    fn labels(axis: &Axis3DState) -> Vec<&str> {
        axis.drawn_ticks().map(|t| t.tick.label.as_str()).collect()
    }

    #[test]
    fn default_ticks_match_matplotlib() {
        // An empty 3-D axes: limits (-1/48, 1 + 1/48), axes 266.112 pt wide.
        let mut axis = Axis3DState::new(Axis3::Z, &RcParams::matplotlib());
        axis.update_ticks(&span((-1.0 / 48.0, 1.0 + 1.0 / 48.0)), 266.112, false);
        assert_eq!(labels(&axis), ["0.0", "0.2", "0.4", "0.6", "0.8", "1.0"]);
    }

    #[test]
    fn tick_gridlines_follow_axes_grid_whatever_axis_it_names() {
        let mut rc = RcParams::matplotlib();
        assert!(!Axis3DState::new(Axis3::Z, &rc).major_style.grid_on);
        rc.set("axes.grid", "True").unwrap();
        rc.set("axes.grid.axis", "y").unwrap();
        for dir in Axis3::ALL {
            let axis = Axis3DState::new(dir, &rc);
            assert!(axis.major_style.grid_on, "{dir:?}");
            assert!(!axis.minor_style.grid_on, "axes.grid.which is major");
        }
    }

    #[test]
    fn classic_mode_draws_thin_axis_lines_and_light_solid_grids() {
        let mut rc = RcParams::matplotlib();
        rc.set("_internal.classic_mode", "True").unwrap();
        rc.set("lines.linewidth", "1.5").unwrap();
        let style = Axis3DState::new(Axis3::Y, &rc).style;
        assert_eq!(style.line_width, 0.75);
        assert_eq!(style.grid_style, LineStyle::Solid);
        assert_eq!(style.grid_color, Rgba::opaque(0.9, 0.9, 0.9).into());
        assert_eq!((style.major_tick_width, style.minor_tick_width), (1.5, 1.5));
        let default = Axis3DState::new(Axis3::Y, &RcParams::matplotlib()).style;
        assert_eq!(default.line_width, RcParams::matplotlib().axes_linewidth);
    }

    #[test]
    fn labels_rotate_when_long() {
        let mut label = Axis3DState::new(Axis3::X, &RcParams::matplotlib()).label;
        label.text = "abcd".into();
        assert!(!label.rotated());
        label.text = "X axis".into();
        assert!(label.rotated());
        label.rotate = Some(false);
        assert!(!label.rotated());
    }

    #[test]
    fn inverted_views_keep_their_ticks() {
        let mut axis = Axis3DState::new(Axis3::X, &RcParams::matplotlib());
        axis.update_ticks(&span((1.0, 0.0)), 266.112, false);
        assert_eq!(axis.drawn_ticks().count(), 6);
    }

    #[test]
    fn automargin_insets_the_view() {
        let mut axis = Axis3DState::new(Axis3::X, &RcParams::matplotlib());
        axis.update_ticks(&span((0.0, 1.0)), 266.112, true);
        assert_eq!(labels(&axis), ["0.2", "0.4", "0.6", "0.8"]);
    }

    #[test]
    fn steps_are_a_little_finer_than_in_2d() {
        // matplotlib's z ticks for zlim (-0.1458, 7.1458) on a 4 in axes
        // (tick space 7): a step of 1, where a 2-D axis would take 2.
        let mut axis = Axis3DState::new(Axis3::Z, &RcParams::matplotlib());
        axis.update_ticks(&span((-7.0 / 48.0, 7.0 + 7.0 / 48.0)), 221.76, false);
        let locs: Vec<f64> = axis.drawn_ticks().map(|t| t.tick.loc).collect();
        assert_eq!(locs, [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]);
    }

    #[test]
    fn log_axes_draw_minor_ticks_after_the_major_ones() {
        let mut axis = Axis3DState::new(Axis3::Z, &RcParams::matplotlib());
        axis.set_scale(AxisScale::new(Rc::new(LogScale::new(10.0).expect("base"))));
        // matplotlib's z view after plotting z from 1 to 1000.
        let view = (0.604_296_390_238_132_8, 1_654.817_099_943_182);
        axis.update_ticks(&span(view), 266.112, false);
        let ticks: Vec<(f64, bool)> = axis
            .drawn_ticks()
            .map(|t| (t.tick.loc, t.tick.label.is_empty()))
            .collect();
        assert_eq!(
            &ticks[..4],
            [(1.0, false), (10.0, false), (100.0, false), (1000.0, false)]
        );
        assert_eq!(ticks.len(), 31, "27 unlabelled minor ticks follow");
        assert!(ticks[4..].iter().all(|&(_, empty)| empty));
    }
}
