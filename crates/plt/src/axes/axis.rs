//! One axis of an axes: its scale, tick helpers and computed ticks
//! (matplotlib's `Axis`); the tick computation is in `tick_compute`.

use std::rc::Rc;

use crate::artist::ArtistId;

use super::axis_label::{AxisLabel, LabelLoc, label_font};
use super::axis_pick::AxisPicking;
use super::axis_units::AxisUnits;
use super::tick_set::TickSet;
use super::ticks::{Sides, TickStyle};
use super::{TickLabelProps, TickLineProps};
use crate::dates::TimeZone;
use crate::polar::{RadialLocator, theta_tickers};
use crate::rc::RcParams;
use crate::scale::{AxisTickers, LinearScale, Scale};
use crate::spines::SpineSide;
use crate::ticker::{AxisName, Formatter, Locator, TickerError, TickerSettings};
use crate::transforms::AxisMap;
use crate::units::{Categories, CategoryFormatter, CategoryLocator, UnitConverter, date_tickers};

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

/// Which tick helpers of an axis are still the ones its scale installed
/// (matplotlib's `isDefault_majloc`, `isDefault_majfmt`, `isDefault_minloc`
/// and `isDefault_minfmt`): setting a helper clears its flag, setting a
/// scale sets them all again.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TickerDefaults {
    /// The major ticks' helpers.
    pub(crate) major: LevelDefaults,
    /// The minor ticks' helpers.
    pub(crate) minor: LevelDefaults,
}

/// Whether the locator and the formatter of one tick level are still the
/// scale's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LevelDefaults {
    /// The locator.
    pub(crate) locator: bool,
    /// The formatter.
    pub(crate) formatter: bool,
}

impl Default for LevelDefaults {
    fn default() -> Self {
        Self {
            locator: true,
            formatter: true,
        }
    }
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
    /// Which tick helpers are still the defaults; units replace only
    /// those.
    pub(crate) defaults: TickerDefaults,
    /// Category names, once categorical values were plotted.
    categories: Option<Categories>,
    /// The unit converter and how it was chosen.
    pub(crate) units: AxisUnits,
    pub(crate) settings: TickerSettings,
    pub(crate) autoscale: AutoscaleState,
    /// Size of the major tick labels in points as set by the tick
    /// parameters (matplotlib's `_major_tick_kw['labelsize']`), which
    /// drives the tick-space heuristic; `None` follows the current
    /// `xtick.labelsize` or `ytick.labelsize`.
    pub(crate) label_size: Option<f64>,
    /// Ticks computed by the last prepare pass, or why they could not be.
    pub(crate) ticks: Result<TickSet, TickerError>,
    /// Appearance of the major ticks.
    pub(crate) major_style: TickStyle,
    /// Appearance of the minor ticks.
    pub(crate) minor_style: TickStyle,
    /// Label properties of single major ticks, by tick index, on top of
    /// `major_style`.
    pub(crate) major_label_overrides: Vec<TickLabelProps>,
    /// Mark and gridline properties of single major ticks, by tick index,
    /// on top of `major_style`.
    pub(crate) major_line_overrides: Vec<TickLineProps>,
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
    /// Whether, and how, presses pick the axis.
    pub(crate) picking: AxisPicking,
}

impl AxisState {
    /// A linear axis configured from `rc`.
    pub(crate) fn new(name: AxisName, rc: &RcParams) -> Self {
        let settings = TickerSettings::from(rc);
        let margin = match name {
            AxisName::X => rc.axes_xmargin,
            AxisName::Y => rc.axes_ymargin,
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
            defaults: TickerDefaults::default(),
            categories: None,
            units: AxisUnits::default(),
            autoscale: AutoscaleState {
                enabled: true,
                margin,
                stale: false,
            },
            label_size: None,
            settings,
            ticks: Ok(TickSet::default()),
            major_style: TickStyle::from_rc(name, true, rc),
            minor_style: TickStyle::from_rc(name, false, rc),
            major_label_overrides: Vec::new(),
            major_line_overrides: Vec::new(),
            label: AxisLabel {
                text: String::new(),
                is_default: true,
                font: label_font(rc),
                color: rc.axes_labelcolor.clone(),
                pad: rc.axes_labelpad,
                side: SpineSide::of(name, false),
                loc: LabelLoc::from_rc(match name {
                    AxisName::X => &rc.xaxis_labellocation,
                    AxisName::Y => &rc.yaxis_labellocation,
                }),
                halign: None,
                multialign: None,
                visible: true,
                rotation: None,
                valign: None,
                coords: None,
                bbox: None,
            },
            visible: true,
            offset_side: SpineSide::of(name, false),
            offset_visible: true,
            remove_overlapping_locs: true,
            picking: AxisPicking::default(),
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
        self.defaults = TickerDefaults::default();
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

    /// The category names plotted on the axis, if any.
    pub(crate) fn categories(&self) -> Option<&Categories> {
        self.categories.as_ref()
    }

    /// Replaces the major locator and hands it to the major formatter
    /// (matplotlib's `Axis.set_major_locator`).
    pub(crate) fn set_major_locator(&mut self, locator: Rc<dyn Locator>) {
        self.major_locator = self.for_role(locator);
        self.defaults.major.locator = false;
        if let Some(formatter) = self.major_formatter.following(&self.major_locator) {
            self.major_formatter = formatter;
        }
    }

    /// Replaces the minor locator and hands it to the minor formatter
    /// (matplotlib's `Axis.set_minor_locator`).
    pub(crate) fn set_minor_locator(&mut self, locator: Rc<dyn Locator>) {
        if let Some(formatter) = self.minor_formatter.following(&locator) {
            self.minor_formatter = formatter;
        }
        self.minor_locator = locator;
        self.defaults.minor.locator = false;
    }

    /// Replaces the major formatter.
    pub(crate) fn set_major_formatter(&mut self, formatter: Rc<dyn Formatter>) {
        self.major_formatter = formatter;
        self.defaults.major.formatter = false;
    }

    /// Replaces the minor formatter.
    pub(crate) fn set_minor_formatter(&mut self, formatter: Rc<dyn Formatter>) {
        self.minor_formatter = formatter;
        self.defaults.minor.formatter = false;
    }

    /// Takes date numbers, giving the axis the date converter and its
    /// date ticks, in the time zone of the axis unit, unless tick helpers
    /// were chosen explicitly (matplotlib's `update_units` +
    /// `convert_units` for dates).
    pub(crate) fn convert_dates(&mut self, numbers: Vec<f64>) -> Vec<f64> {
        self.units.adopt(UnitConverter::Dates);
        self.use_date_tickers(&self.units.time_zone());
        numbers
    }

    /// Converts category names to positions, registering new names and
    /// labelling the ticks with them unless tick helpers were chosen
    /// explicitly (matplotlib's `update_units` + `convert_units` for
    /// strings).
    pub(crate) fn convert_categories(&mut self, names: &[String]) -> Vec<f64> {
        self.units.adopt(UnitConverter::Categories);
        let positions = self
            .categories
            .get_or_insert_with(Categories::default)
            .convert(names);
        self.use_category_tickers();
        positions
    }

    /// Installs the tick helpers of the category names where the defaults
    /// are still in place (matplotlib's `_update_axisinfo` with the
    /// category converter's `axisinfo`).
    fn use_category_tickers(&mut self) {
        let Some(categories) = &self.categories else {
            return;
        };
        let (locator, formatter) = (
            CategoryLocator::new(categories),
            CategoryFormatter::new(categories),
        );
        if self.defaults.major.locator {
            self.major_locator = self.for_role(Rc::new(locator));
        }
        if self.defaults.major.formatter {
            self.major_formatter = Rc::new(formatter);
        }
    }

    /// Takes over the [`unit_data`](Self::unit_data) of an axis it is
    /// shared with, after values were converted there, with the tick
    /// helpers they bring where the defaults are in place: matplotlib's
    /// shared axes hold one `units` object (category names placed on one
    /// are placed on all) and share their tickers.
    /// Converters of user values are not shared: matplotlib shares their
    /// units through `set_units` alone.
    pub(crate) fn share_units(&mut self, (units, categories): &(AxisUnits, Option<Categories>)) {
        match &units.converter {
            Some(UnitConverter::Dates) => {
                self.units.adopt(UnitConverter::Dates);
                self.use_date_tickers(&self.units.time_zone());
            }
            Some(UnitConverter::Categories) => {
                self.units.adopt(UnitConverter::Categories);
                self.categories.clone_from(categories);
                self.use_category_tickers();
            }
            Some(UnitConverter::Custom(_)) | None => {}
        }
    }

    /// The unit converter and category names, which a twin takes over
    /// (matplotlib's `twinx` assigning `ax2.xaxis.units = self.xaxis.units`).
    pub(crate) fn unit_data(&self) -> (AxisUnits, Option<Categories>) {
        (self.units.clone(), self.categories.clone())
    }

    /// Takes over the [`unit_data`](Self::unit_data) of another axis.
    pub(crate) fn set_unit_data(&mut self, (units, categories): (AxisUnits, Option<Categories>)) {
        self.units = units;
        self.categories = categories;
    }

    /// Installs the tick helpers of dates in `tz` where the defaults are
    /// still in place (matplotlib's `_update_axisinfo` with the date
    /// converter's `axisinfo`).
    pub(crate) fn use_date_tickers(&mut self, tz: &TimeZone) {
        let (locator, formatter) = date_tickers(tz);
        if self.defaults.major.locator {
            self.major_locator = self.for_role(locator);
        }
        if self.defaults.major.formatter {
            self.major_formatter = formatter;
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::{AutoDateFormatter, ConciseDateFormatter};

    #[test]
    fn date_axes_follow_the_date_converter() {
        let formatter = |converter: &str| {
            crate::rc::context(|| {
                crate::rc::set("date.converter", converter).unwrap();
                let mut axis = AxisState::new(AxisName::X, &RcParams::matplotlib());
                axis.convert_dates(vec![18_262.0, 18_300.0]);
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
    fn axis_labels_take_the_rc_size_and_weight() {
        let mut rc = RcParams::matplotlib();
        rc.set("axes.labelsize", "large").expect("valid");
        rc.set("axes.labelweight", "bold").expect("valid");
        let font = AxisState::new(AxisName::Y, &rc).label.font;
        assert_eq!(font.size, 12.0);
        assert_eq!(font.weight, crate::fonts::FontWeight::BOLD);
    }
}
