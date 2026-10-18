//! One axis of an axes: its scale, tick helpers and computed ticks
//! (matplotlib's `Axis`).

use std::rc::Rc;

use crate::artist::ArtistId;

use super::ticks::{Sides, TickStyle};
use crate::colors::Color;
use crate::dates::{AutoDateFormatter, AutoDateLocator, ConciseDateFormatter};
use crate::fonts::FontProperties;
use crate::polar::{RadialLocator, theta_tickers};
use crate::rc::RcParams;
use crate::scale::{AxisTickers, LinearScale, Scale};
use crate::spines::SpineSide;
use crate::ticker::{AxisInfo, AxisName, Formatter, Locator, TickContext, TickerSettings};
use crate::transforms::{AxisMap, interval_contains_close};
use crate::units::{AxisValues, Categories, CategoryFormatter, CategoryLocator};

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
    /// Every minor tick inside the view, those on major ticks included,
    /// labelled together (what axisartist draws from matplotlib's
    /// `minor.locator()`).
    pub minor_all: Vec<Tick>,
    /// Every location the major locator gave, inside the view or not
    /// (axisartist draws its gridlines at matplotlib's `major.locator()`).
    pub major_locs: Vec<f64>,
    /// Every location the minor locator gave, inside the view or not.
    pub minor_locs: Vec<f64>,
    /// Offset or scale text shown at the end of the axis (e.g. `1e6`).
    pub offset: String,
}

/// Where an axis label sits along its axis (matplotlib's `loc` of
/// `set_xlabel`/`set_ylabel`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LabelLoc {
    /// At the left (x) or bottom (y) end, aligned to it.
    Start,
    /// Centred.
    #[default]
    Center,
    /// At the right (x) or top (y) end, aligned to it.
    End,
}

impl LabelLoc {
    /// The rc name (`"left"`, `"bottom"`, `"center"`, ...) for axis `name`.
    pub(crate) fn from_rc(name: &str) -> Self {
        match name {
            "left" | "bottom" => Self::Start,
            "right" | "top" => Self::End,
            _ => Self::Center,
        }
    }

    /// The position along the axis as a fraction of its length.
    pub(crate) fn fraction(self) -> f64 {
        match self {
            Self::Start => 0.0,
            Self::Center => 0.5,
            Self::End => 1.0,
        }
    }
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
    /// The side of the axes the label sits on.
    pub(crate) side: SpineSide,
    /// Where along the axis the label sits.
    pub(crate) loc: LabelLoc,
    /// Whether the label is drawn (matplotlib's `label.set_visible`).
    pub(crate) visible: bool,
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
    /// How many ticks fit along the axis of another axes the locators are
    /// bound to (`None` measures this axis).
    pub(crate) tick_space: Option<usize>,
    /// Major ticks at or below this value are dropped before labelling
    /// (the origin of full-circle polar axes).
    pub(crate) tick_floor: Option<f64>,
}

/// The part an axis plays in its projection, which decides how its tick
/// helpers are adapted (matplotlib's `ThetaAxis` and `RadialAxis`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) enum AxisRole {
    /// An x or y axis of rectilinear axes.
    #[default]
    Cartesian,
    /// The angle of polar axes: ticks at degree multiples, labelled in
    /// degrees.
    Theta,
    /// The radius of polar axes: major locators are wrapped in a
    /// [`RadialLocator`].
    Radius,
}

/// The tick helpers of an axis, shared by axes sharing that axis.
#[derive(Clone, Debug)]
pub(crate) struct Tickers {
    major_locator: Rc<dyn Locator>,
    major_formatter: Rc<dyn Formatter>,
    minor_locator: Rc<dyn Locator>,
    minor_formatter: Rc<dyn Formatter>,
}

/// State of one axis.
///
/// The scale and tick helpers are reference counted because axes sharing
/// an axis share them (matplotlib shares the `Ticker` and scale objects).
#[derive(Debug)]
pub(crate) struct AxisState {
    pub(crate) name: AxisName,
    /// The part the axis plays in its projection.
    pub(crate) role: AxisRole,
    scale: Rc<dyn Scale>,
    map: Option<Rc<dyn AxisMap>>,
    pub(crate) major_locator: Rc<dyn Locator>,
    pub(crate) major_formatter: Rc<dyn Formatter>,
    pub(crate) minor_locator: Rc<dyn Locator>,
    pub(crate) minor_formatter: Rc<dyn Formatter>,
    /// The axes whose axis the locators measure when it is not this one:
    /// matplotlib binds a locator to the axis that set it, and axes
    /// sharing an axis share their locators (`locator.axis`).
    pub(crate) ticker_owner: Option<ArtistId>,
    /// Whether the major locator and formatter are still the defaults,
    /// which units may replace (matplotlib's `isDefault_majloc`/`_majfmt`).
    default_major: (bool, bool),
    /// Category names, once categorical values were plotted.
    categories: Option<Categories>,
    pub(crate) settings: TickerSettings,
    pub(crate) autoscale: AutoscaleState,
    /// Size of the major tick labels in points as set by the rc and the
    /// tick parameters (drives the tick-space heuristic).
    pub(crate) label_size: f64,
    /// Ticks computed by the last prepare pass, or why they could not be.
    pub(crate) ticks: Result<TickSet, String>,
    /// Appearance of the major ticks.
    pub(crate) major_style: TickStyle,
    /// Appearance of the minor ticks.
    pub(crate) minor_style: TickStyle,
    /// The axis label.
    pub(crate) label: AxisLabel,
    /// Whether ticks, tick labels, the label and the offset text are drawn.
    pub(crate) visible: bool,
    /// The side the offset text sits on (matplotlib's tick position for
    /// x, offset position for y).
    pub(crate) offset_side: SpineSide,
    /// Whether the offset text is drawn.
    pub(crate) offset_visible: bool,
    /// Whether minor ticks on major ticks are dropped.
    pub(crate) remove_overlapping_locs: bool,
}

impl AxisState {
    /// A linear axis configured from `rc`.
    pub(crate) fn new(name: AxisName, rc: &RcParams) -> Self {
        let settings = TickerSettings::from(rc);
        let (margin, label_size) = match name {
            AxisName::X => (rc.axes_xmargin, rc.xtick_labelsize),
            AxisName::Y => (rc.axes_ymargin, rc.ytick_labelsize),
        };
        let scale: Rc<dyn Scale> = Rc::new(LinearScale);
        let tickers = scale.default_tickers(name, &settings);
        Self {
            name,
            role: AxisRole::Cartesian,
            map: scale.axis_map(),
            scale,
            major_locator: Rc::from(tickers.major_locator),
            major_formatter: Rc::from(tickers.major_formatter),
            minor_locator: Rc::from(tickers.minor_locator),
            minor_formatter: Rc::from(tickers.minor_formatter),
            ticker_owner: None,
            default_major: (true, true),
            categories: None,
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
                side: SpineSide::of(name, false),
                loc: LabelLoc::from_rc(match name {
                    AxisName::X => &rc.xaxis_labellocation,
                    AxisName::Y => &rc.yaxis_labellocation,
                }),
                visible: true,
            },
            visible: true,
            offset_side: SpineSide::of(name, false),
            offset_visible: true,
            remove_overlapping_locs: true,
        }
    }

    /// The scale.
    pub(crate) fn scale(&self) -> &dyn Scale {
        self.scale.as_ref()
    }

    /// Replaces the scale and installs its default tick helpers
    /// (matplotlib's `Axis._set_scale`).
    pub(crate) fn set_scale(&mut self, scale: Rc<dyn Scale>) {
        let AxisTickers {
            major_locator,
            minor_locator,
            major_formatter,
            minor_formatter,
        } = scale.default_tickers(self.name, &self.settings);
        self.major_locator = Rc::from(major_locator);
        self.major_formatter = Rc::from(major_formatter);
        self.minor_locator = Rc::from(minor_locator);
        self.minor_formatter = Rc::from(minor_formatter);
        self.default_major = (true, true);
        self.adopt_scale(scale);
        match self.role {
            AxisRole::Cartesian => {}
            AxisRole::Theta => {
                (self.major_locator, self.major_formatter) = theta_tickers(&self.settings);
            }
            AxisRole::Radius => {
                self.major_locator = self.for_role(Rc::clone(&self.major_locator));
            }
        }
    }

    /// The axis' own sticky edges (matplotlib's `Axis.sticky_edges`): a
    /// radial axis sticks to zero.
    pub(crate) fn sticky_edges(&self) -> &'static [f64] {
        match self.role {
            AxisRole::Radius => &[0.0],
            AxisRole::Cartesian | AxisRole::Theta => &[],
        }
    }

    /// `locator` as this axis uses it: wrapped in a [`RadialLocator`] on
    /// a radial axis (matplotlib's `RadialAxis.set_major_locator`).
    fn for_role(&self, locator: Rc<dyn Locator>) -> Rc<dyn Locator> {
        match self.role {
            AxisRole::Radius => RadialLocator::wrap(locator, self.scale.as_ref()),
            AxisRole::Cartesian | AxisRole::Theta => locator,
        }
    }

    /// Uses `scale` (shared with another axis) without touching the tick
    /// helpers (matplotlib's `sharex` assigning `_scale`).
    pub(crate) fn adopt_scale(&mut self, scale: Rc<dyn Scale>) {
        self.map = scale.axis_map();
        self.scale = scale;
    }

    /// The tick helpers, to share with another axis.
    pub(crate) fn tickers(&self) -> Tickers {
        Tickers {
            major_locator: Rc::clone(&self.major_locator),
            major_formatter: Rc::clone(&self.major_formatter),
            minor_locator: Rc::clone(&self.minor_locator),
            minor_formatter: Rc::clone(&self.minor_formatter),
        }
    }

    /// Uses tick helpers shared with another axis (matplotlib's `sharex`
    /// assigning the `major` and `minor` tickers).
    pub(crate) fn use_tickers(&mut self, tickers: Tickers) {
        self.major_locator = tickers.major_locator;
        self.major_formatter = tickers.major_formatter;
        self.minor_locator = tickers.minor_locator;
        self.minor_formatter = tickers.minor_formatter;
    }

    /// The shared scale object.
    pub(crate) fn scale_rc(&self) -> Rc<dyn Scale> {
        Rc::clone(&self.scale)
    }

    /// Applies `update` to the styles of both tick levels (matplotlib's
    /// `set_tick_params(which="both", ...)`).
    pub(crate) fn restyle_ticks(&mut self, mut update: impl FnMut(&mut TickStyle)) {
        update(&mut self.major_style);
        update(&mut self.minor_style);
    }

    /// Moves ticks and labels to the side at the high (`true`) or low end
    /// of the axis, offset text included (matplotlib's
    /// `set_ticks_position('top')` and friends).
    pub(crate) fn ticks_to_side(&mut self, high: bool) {
        let only = Sides {
            first: !high,
            second: high,
        };
        self.restyle_ticks(|style| {
            style.ticks = only;
            style.labels = only;
        });
        self.offset_side = SpineSide::of(self.name, high);
    }

    /// matplotlib's `tick_top`/`tick_right` (`high`) and
    /// `tick_bottom`/`tick_left`: ticks move to one side, and the labels
    /// with them unless both label sides were off.
    pub(crate) fn tick_to(&mut self, high: bool) {
        let labelled = self.major_style.labels.first || self.major_style.labels.second;
        self.ticks_to_side(high);
        if !labelled {
            self.restyle_ticks(|style| style.labels = Sides::default());
        }
    }

    /// Replaces the major locator.
    pub(crate) fn set_major_locator(&mut self, locator: Rc<dyn Locator>) {
        self.major_locator = self.for_role(locator);
        self.default_major.0 = false;
    }

    /// Replaces the major formatter.
    pub(crate) fn set_major_formatter(&mut self, formatter: Rc<dyn Formatter>) {
        self.major_formatter = formatter;
        self.default_major.1 = false;
    }

    /// Converts plotting values to numbers, registering new category names
    /// and labelling the ticks with them unless tick helpers were chosen
    /// explicitly (matplotlib's `update_units` + `convert_units`).
    pub(crate) fn convert_units(&mut self, values: AxisValues) -> Vec<f64> {
        let names = match values {
            AxisValues::Numbers(values) => return values,
            AxisValues::Dates(numbers) => {
                // matplotlib's DateConverter (or ConciseDateConverter, as
                // `date.converter` says) axisinfo.
                let rc = crate::rc::params();
                let locator =
                    || AutoDateLocator::new().with_interval_multiples(rc.date_interval_multiples);
                if self.default_major.0 {
                    self.major_locator = self.for_role(Rc::new(locator()));
                }
                if self.default_major.1 {
                    self.major_formatter = if rc.date_converter == "concise" {
                        Rc::new(ConciseDateFormatter::new())
                    } else {
                        Rc::new(AutoDateFormatter::new(locator()))
                    };
                }
                return numbers;
            }
            AxisValues::Categories(names) => names,
        };
        let categories = self.categories.get_or_insert_with(Categories::default);
        let positions = categories.convert(&names);
        let (locator, formatter) = (
            CategoryLocator::new(categories),
            CategoryFormatter::new(categories),
        );
        if self.default_major.0 {
            self.major_locator = self.for_role(Rc::new(locator));
        }
        if self.default_major.1 {
            self.major_formatter = Rc::new(formatter);
        }
        positions
    }

    /// The non-linear part of the scale (`None` for linear axes).
    pub(crate) fn map(&self) -> Option<&Rc<dyn AxisMap>> {
        self.map.as_ref()
    }

    /// Maps a value through the scale.
    pub(crate) fn forward(&self, value: f64) -> f64 {
        self.map.as_ref().map_or(value, |m| m.forward(value))
    }

    /// Maps a scaled value back to data.
    pub(crate) fn inverse(&self, value: f64) -> f64 {
        self.map.as_ref().map_or(value, |m| m.inverse(value))
    }

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
            scale: self.scale.hint(),
            three_d: false,
        }
    }

    /// How many ticks fit (matplotlib's `get_tick_space`): tick labels are
    /// assumed to be at most 3:1 (x) or 2:1 (y) wide.
    pub(crate) fn tick_space(&self, length_points: f64) -> usize {
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
        let mut major_locs = {
            let ctx = TickContext::new(&info, &self.settings);
            self.major_locator.locate(&ctx)?
        };
        if let Some(floor) = geometry.tick_floor {
            major_locs.retain(|&loc| loc > floor);
        }
        info.major_ticks.clone_from(&major_locs);
        let ctx = TickContext::new(&info, &self.settings);
        let (lo, hi) = (self.forward(geometry.view.0), self.forward(geometry.view.1));
        let visible = |loc: f64| interval_contains_close((lo, hi), self.forward(loc), 1e-10);
        let collect = |locs: &[f64], labels: Vec<String>| {
            locs.iter()
                .zip(labels)
                .filter(|(loc, _)| visible(**loc))
                .map(|(&loc, label)| Tick { loc, label })
                .collect()
        };
        // Like matplotlib's `_tick_group_visible`, a tick level whose
        // marks, labels and gridlines are all off is not computed.
        let mut ticks = TickSet::default();
        if self.major_style.is_drawn() {
            let labels = self.major_formatter.format_ticks(&major_locs, &ctx)?;
            ticks.major = collect(&major_locs, labels.labels);
            ticks.offset = labels.offset;
            ticks.major_locs.clone_from(&major_locs);
        }
        if self.minor_style.is_drawn() {
            let all_locs = self.minor_locator.locate(&ctx)?;
            let minor_locs = if self.remove_overlapping_locs {
                self.without_overlaps(all_locs.clone(), &major_locs, geometry.view)
            } else {
                all_locs.clone()
            };
            let labels = self.minor_formatter.format_ticks(&minor_locs, &ctx)?;
            ticks.minor = collect(&minor_locs, labels.labels);
            let labels = self.minor_formatter.format_ticks(&all_locs, &ctx)?;
            ticks.minor_all = collect(&all_locs, labels.labels);
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
    fn date_axes_follow_the_date_converter() {
        let formatter = |converter: &str| {
            crate::rc::context(|| {
                crate::rc::set("date.converter", converter).unwrap();
                let mut axis = AxisState::new(AxisName::X, &RcParams::matplotlib());
                axis.convert_units(AxisValues::Dates(vec![18_262.0, 18_300.0]));
                let formatter: &dyn std::any::Any = axis.major_formatter.as_ref();
                (
                    formatter.is::<ConciseDateFormatter>(),
                    formatter.is::<AutoDateFormatter>(),
                )
            })
        };
        assert_eq!(formatter("auto"), (false, true));
        assert_eq!(formatter("concise"), (true, false));
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
