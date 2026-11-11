//! One axis of an axes: its scale, tick helpers and computed ticks
//! (matplotlib's `Axis`); the tick computation is in `tick_compute`.

use std::cell::Cell;
use std::rc::Rc;

use pltrs_core::Point;

use crate::artist::{AggFilter, ArtistId};

use super::axis_artists::AxisGids;
use super::axis_label::{AxisLabel, label_font};
use super::axis_pick::AxisPicking;
use super::tick_set::TickSet;
use super::tick_updates::ProgramTicked;
use super::ticks::{Sides, TickStyle};
use super::{TickLabelProps, TickLineProps};
use crate::dates::YearLocator;
use crate::polar::{RadialLocator, theta_tickers};
use crate::rc::RcParams;
use crate::scale::{AxisTickers, LinearScale, Scale};
use crate::spines::SpineSide;
use crate::ticker::{
    AutolimitMode, AxisName, Formatter, Locator, TickSpace, TickerError, TickerSettings,
};
use crate::transforms::{AxisMap, program_calls};
use crate::units::{AxisUnits, Categories, UnitAxisInfo, UnitData, UnitState};

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

pltrs_core::archive_struct!(AutoscaleState {
    enabled,
    margin,
    stale
});

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
    pub(crate) tick_space: Option<TickSpace>,
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

pltrs_core::archive_enum!(AxisRole {
    Cartesian,
    Theta,
    Radius,
});

/// The tick helpers of an axis, shared by axes sharing that axis.
#[derive(Clone, Debug)]
pub(crate) struct Tickers {
    pub(crate) major_locator: Rc<dyn Locator>,
    pub(crate) major_formatter: Rc<dyn Formatter>,
    pub(crate) minor_locator: Rc<dyn Locator>,
    pub(crate) minor_formatter: Rc<dyn Formatter>,
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

pltrs_core::archive_struct!(TickerDefaults { major, minor });

impl TickerDefaults {
    /// Counts the major (or `minor`) `helper` as the scale's own or not.
    pub(crate) fn set(&mut self, minor: bool, helper: TickHelperKind, default: bool) {
        let level = if minor {
            &mut self.minor
        } else {
            &mut self.major
        };
        match helper {
            TickHelperKind::Locator => level.locator = default,
            TickHelperKind::Formatter => level.formatter = default,
        }
    }
}

/// A tick helper of an axis level: the locator placing the ticks or the
/// formatter labelling them (what matplotlib's `isDefault_majloc`,
/// `isDefault_majfmt`, `isDefault_minloc` and `isDefault_minfmt` tell of).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TickHelperKind {
    /// The locator.
    Locator,
    /// The formatter.
    Formatter,
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

pltrs_core::archive_struct!(LevelDefaults { locator, formatter });

impl Default for LevelDefaults {
    fn default() -> Self {
        Self {
            locator: true,
            formatter: true,
        }
    }
}

/// The properties single ticks of one level have of their own, on top
/// of the style of their level, by tick index (matplotlib's `Tick`
/// artists changed one by one).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct TickOverrides {
    /// Properties of the labels on the first side (matplotlib's
    /// `Tick.label1`).
    pub(crate) labels: Vec<TickLabelProps>,
    /// Properties of the labels on the second side (matplotlib's
    /// `Tick.label2`, a text of its own).
    pub(crate) second_labels: Vec<TickLabelProps>,
    /// Mark and gridline properties.
    pub(crate) lines: Vec<TickLineProps>,
    /// Gaps between the marks and the labels, in points (matplotlib's
    /// `Tick.set_pad`).
    pub(crate) pads: Vec<Option<f64>>,
    /// Whether the ticks are drawn, `None` for a tick that is
    /// (matplotlib's `Tick.set_visible`).
    pub(crate) visible: Vec<Option<bool>>,
    /// The image filters the ticks are drawn through (matplotlib's
    /// `Tick.set_agg_filter`), which ticks made later do not take.
    pub(crate) agg_filters: Vec<Option<AggFilter>>,
    /// How many ticks of the level exist, as matplotlib's tick lists
    /// count them: ticks are made when the axis places them or a program
    /// lists them (`get_major_ticks(n)`), and never unmade but by a reset.
    pub(crate) made: usize,
}

pltrs_core::archive_struct!(TickOverrides {
    labels,
    second_labels,
    lines,
    pads,
    visible,
    agg_filters,
    made
});

impl TickOverrides {
    /// Makes the first `count` ticks of the level exist: those made now
    /// take what the first tick has of its own, as matplotlib copies the
    /// first tick's properties to every tick it adds
    /// (`Axis._copy_tick_props`), unless they were given properties of
    /// their own before.
    pub(crate) fn make_ticks(&mut self, count: usize) {
        if count <= self.made {
            return;
        }
        copy_first(&mut self.labels, self.made, count);
        copy_first(&mut self.second_labels, self.made, count);
        copy_first(&mut self.lines, self.made, count);
        copy_first(&mut self.pads, self.made, count);
        copy_first(&mut self.visible, self.made, count);
        self.made = count;
    }
}

/// Gives the ticks being made (`made..count`) that have no entry of
/// `list` yet its first entry, when the first tick has one: the ticks
/// made before, and those given an entry already, keep what they have
/// (the level's style when they have no entry).
fn copy_first<T: Clone + Default>(list: &mut Vec<T>, made: usize, count: usize) {
    let Some(first) = list.first().cloned() else {
        return;
    };
    list.resize_with(made.max(list.len()), T::default);
    list.resize(count.max(list.len()), first);
}

/// State of one axis.
///
/// The scale and tick helpers are reference counted because axes sharing
/// an axis share them (matplotlib shares the `Ticker` and scale objects).
#[derive(Clone, Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "mirrors matplotlib's independent boolean Axis attributes"
)]
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
    /// The `axes.autolimit_mode` set on the axes
    /// ([`Axes::set_autolimit_mode`](super::Axes::set_autolimit_mode));
    /// `None` reads the rcParam whenever the axis autoscales, as
    /// matplotlib does.
    pub(crate) autolimit_mode: Option<AutolimitMode>,
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
    /// Properties of single major ticks, on top of `major_style`.
    pub(crate) major_overrides: TickOverrides,
    /// Properties of single minor ticks, on top of `minor_style`.
    pub(crate) minor_overrides: TickOverrides,
    /// The axis label.
    pub(crate) label: AxisLabel,
    /// Whether ticks, tick labels, the label and the offset text are drawn.
    pub(crate) visible: bool,
    /// Whether the axis is animated: its axes leaves it out of the draws of
    /// its figure but printing, as matplotlib's `Axes.draw` leaves out its
    /// animated children.
    pub(crate) animated: bool,
    /// The image filter the axis is drawn through (matplotlib's
    /// `allow_rasterization` around `Axis.draw`).
    pub(crate) agg_filter: Option<AggFilter>,
    /// Whether layouts make room for the axis' texts (matplotlib's
    /// `Axis.set_in_layout`).
    pub(crate) in_layout: bool,
    /// The side the offset text sits on (matplotlib's tick position for
    /// x, offset position for y).
    pub(crate) offset_side: SpineSide,
    /// Whether the offset text is drawn.
    pub(crate) offset_visible: bool,
    /// The font size of the offset text in points: the tick label size
    /// when the axis was made or cleared (matplotlib's `xtick.labelsize`
    /// or `ytick.labelsize` then), which the tick parameters leave.
    pub(crate) offset_size: f64,
    /// Whether minor ticks on major ticks are dropped.
    pub(crate) remove_overlapping_locs: bool,
    /// Whether, and how, presses pick the axis.
    pub(crate) picking: AxisPicking,
    /// The gids of the axis, its texts and its ticks in SVG output.
    pub(crate) gids: AxisGids,
    /// Whether the ticks were ever placed (drawn or measured while shown,
    /// see [`super::Axis::ticks_placed`]); set while drawing, hence a
    /// cell.
    pub(crate) ticks_placed: Cell<bool>,
    /// Where the last layout of the axis' texts (a draw, or a measure of
    /// them) anchored the label in display pixels (see
    /// [`super::Axis::label_placed`]); set while drawing, hence a cell.
    pub(crate) label_placed: Cell<Option<Point>>,
    /// Where the last layout of the axis' texts anchored the offset text
    /// in display pixels (see [`super::Axis::offset_text_placed`]); set
    /// while drawing, hence a cell.
    pub(crate) offset_placed: Cell<Option<Point>>,
    /// Whether the axes follows a change of the axis' units (recomputing
    /// its data limits and autoscaling): matplotlib connects its
    /// `_unit_change_handler` to the axis' callbacks once the axes is
    /// made, and the first `Axis.clear` after that (the axes' own clear
    /// included) disconnects it with the other callbacks.
    pub(crate) follows_units: bool,
    /// The ends of the view interval last mapped through the scale (their
    /// bits) and where they went (see [`AxisState::scaled_ends`]).
    scaled_view: Cell<Option<([u64; 2], [f64; 2])>>,
    /// What the program last ticked the axis for, when its helpers or
    /// scale call the program (see [`TickUpdater`](super::TickUpdater)).
    pub(crate) program_ticked: Option<ProgramTicked>,
    /// Whether the program's class of the axis answers its view interval
    /// itself (overrides matplotlib's `get_view_interval`), which
    /// matplotlib's tick updates ask: the program's tick updater ticks the
    /// axis then. Set by the program's class, again whenever it is made.
    pub(crate) program_view: bool,
}

pltrs_core::archive_struct!(AxisState {
    name,
    role,
    scale,
    map,
    major_locator,
    major_formatter,
    minor_locator,
    minor_formatter,
    ticker_owner,
    defaults,
    categories,
    units,
    autolimit_mode,
    autoscale,
    label_size,
    ticks,
    major_style,
    minor_style,
    major_overrides,
    minor_overrides,
    label,
    visible,
    animated,
    agg_filter,
    in_layout,
    offset_side,
    offset_visible,
    offset_size,
    remove_overlapping_locs,
    picking,
    gids,
    ticks_placed,
    label_placed,
    offset_placed,
    follows_units,
    scaled_view
} skip { program_ticked, program_view });

impl AxisState {
    /// A linear axis configured from `rc`.
    pub(crate) fn new(name: AxisName, rc: &RcParams) -> Self {
        let margin = match name {
            AxisName::X => rc.axes_xmargin,
            AxisName::Y => rc.axes_ymargin,
        };
        let scale: Rc<dyn Scale> = Rc::new(LinearScale);
        let tickers = scale.default_tickers(name, &TickerSettings::from(rc));
        let major_style = TickStyle::from_rc(name, true, rc);
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
            autolimit_mode: None,
            ticks: Ok(TickSet::default()),
            offset_size: major_style.label_size,
            major_style,
            minor_style: TickStyle::from_rc(name, false, rc),
            major_overrides: TickOverrides::default(),
            minor_overrides: TickOverrides::default(),
            label: AxisLabel {
                text: String::new(),
                is_default: true,
                font: label_font(rc),
                color: rc.axes_labelcolor.clone(),
                pad: rc.axes_labelpad,
                side: SpineSide::of(name, false),
                loc: match name {
                    AxisName::X => rc.xaxis_labellocation.into(),
                    AxisName::Y => rc.yaxis_labellocation.into(),
                },
                halign: None,
                multialign: None,
                visible: true,
                rotation: None,
                valign: None,
                coords: None,
                bbox: None,
                usetex: rc.text_usetex,
            },
            visible: true,
            animated: false,
            agg_filter: None,
            in_layout: true,
            offset_side: SpineSide::of(name, false),
            offset_visible: true,
            remove_overlapping_locs: true,
            picking: AxisPicking::default(),
            gids: AxisGids::default(),
            ticks_placed: Cell::new(false),
            label_placed: Cell::new(None),
            offset_placed: Cell::new(None),
            follows_units: true,
            scaled_view: Cell::new(None),
            program_ticked: None,
            program_view: false,
        }
    }

    /// The properties single major (or `minor`) ticks have of their own.
    pub(crate) fn overrides(&self, minor: bool) -> &TickOverrides {
        if minor {
            &self.minor_overrides
        } else {
            &self.major_overrides
        }
    }

    /// The properties single major (or `minor`) ticks have of their own,
    /// to change.
    pub(crate) fn overrides_mut(&mut self, minor: bool) -> &mut TickOverrides {
        if minor {
            &mut self.minor_overrides
        } else {
            &mut self.major_overrides
        }
    }

    /// The scale.
    pub(crate) fn scale(&self) -> &dyn Scale {
        self.scale.as_ref()
    }

    /// The rcParams the tick helpers read now: the current ones, since
    /// matplotlib reads them when it makes its locators and formatters
    /// and whenever it autoscales and places ticks, with the autolimit
    /// mode set on the axes.
    pub(crate) fn settings(&self) -> TickerSettings {
        let mut settings = TickerSettings::current();
        if let Some(mode) = self.autolimit_mode {
            settings.axes_autolimit_mode = mode;
        }
        settings
    }

    /// Replaces the scale and installs its default tick helpers, made
    /// under the current rcParams (matplotlib's `Axis._set_scale`).
    pub(crate) fn set_scale(&mut self, scale: Rc<dyn Scale>) {
        let settings = self.settings();
        let AxisTickers {
            major_locator,
            minor_locator,
            major_formatter,
            minor_formatter,
        } = scale.default_tickers(self.name, &settings);
        self.major_locator = Rc::from(major_locator);
        self.major_formatter = Rc::from(major_formatter);
        self.minor_locator = Rc::from(minor_locator);
        self.minor_formatter = Rc::from(minor_formatter);
        self.defaults = TickerDefaults::default();
        self.adopt_scale(scale);
        match self.role {
            AxisRole::Cartesian => {}
            AxisRole::Theta => {
                (self.major_locator, self.major_formatter) = theta_tickers(&settings);
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
    pub(super) fn for_role(&self, locator: Rc<dyn Locator>) -> Rc<dyn Locator> {
        match self.role {
            AxisRole::Radius => RadialLocator::wrap(locator, self.scale.as_ref()),
            AxisRole::Cartesian | AxisRole::Theta => locator,
        }
    }

    /// Uses `scale` (shared with another axis) without touching the tick
    /// helpers (matplotlib's `sharex` assigning `_scale`).
    pub(crate) fn adopt_scale(&mut self, scale: Rc<dyn Scale>) {
        self.map = scale.axis_map();
        self.scaled_view.set(None);
        self.scale = scale;
    }

    /// Has the scale's transform map nothing, the scale kept (matplotlib's
    /// `transScale` made anew as an identity by `set_figure`), until the
    /// scale is set again.
    pub(crate) fn unmap_scale(&mut self) {
        self.map = None;
        self.scaled_view.set(None);
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

    /// Takes over the [`unit_data`](UnitState::unit_data) of another
    /// axis.
    pub(crate) fn set_unit_data(&mut self, (units, categories): UnitData) {
        self.units = units;
        self.categories = categories;
    }

    /// The non-linear part of the scale (`None` for linear axes).
    pub(crate) fn map(&self) -> Option<&Rc<dyn AxisMap>> {
        self.map.as_ref()
    }

    /// Maps a value through the scale.
    pub(crate) fn forward(&self, value: f64) -> f64 {
        self.map.as_ref().map_or(value, |m| m.forward(value))
    }

    /// The ends of a view interval through the scale, its map called once
    /// for both and not again while they and the scale stay the same
    /// (matplotlib's `TransformedBbox` of the view limits keeps its points
    /// until they change).
    pub(crate) fn scaled_ends(&self, ends: [f64; 2]) -> [f64; 2] {
        let bits = ends.map(f64::to_bits);
        if let Some((given, scaled)) = self.scaled_view.get()
            && given == bits
        {
            return scaled;
        }
        let mut scaled = ends;
        if let Some(map) = &self.map {
            // matplotlib maps the view where its transforms first need it
            // (see `transform_sites`).
            program_calls::quietly(|| program_calls::forward_all(map, &mut scaled));
        }
        self.scaled_view.set(Some((bits, scaled)));
        scaled
    }

    /// The ends of a view interval through the scale as
    /// [`scaled_ends`](Self::scaled_ends) last gave them, if it was asked
    /// for these `ends` with this scale; a scale that does not call the
    /// program is asked now.
    pub(crate) fn known_scaled_ends(&self, ends: [f64; 2]) -> Option<[f64; 2]> {
        match &self.map {
            None => Some(ends),
            Some(map) if !map.calls_program() => Some(ends.map(|end| map.forward(end))),
            Some(_) => (self.scaled_view.get())
                .filter(|&(given, _)| given == ends.map(f64::to_bits))
                .map(|(_, scaled)| scaled),
        }
    }

    /// Maps a scaled value back to data.
    pub(crate) fn inverse(&self, value: f64) -> f64 {
        self.map.as_ref().map_or(value, |m| m.inverse(value))
    }

    /// Maps every value of `values` in place through the scale as
    /// matplotlib's `axis.get_transform().transform(...)` does (see
    /// [`AxisMap::forward_column`]).
    pub(crate) fn forward_column(&self, values: &mut [f64]) {
        if let Some(map) = &self.map {
            map.forward_column(values);
        }
    }

    /// Maps every scaled value of `values` back to data in place as
    /// matplotlib's `get_transform().inverted().transform(...)` does (see
    /// [`AxisMap::inverse_column`]).
    pub(crate) fn inverse_column(&self, values: &mut [f64]) {
        if let Some(map) = &self.map {
            map.inverse_column(values);
        }
    }
}

/// `formatter`, a converter's, as its locator leaves it when the locator
/// set on the axis keeps that one from ticking: matplotlib's converter
/// hands its formatter its own locator, which then tells the unit it starts
/// with, a year (the date formatters' choice of format).
fn idle_locator_formatter(formatter: &Rc<dyn Formatter>) -> Rc<dyn Formatter> {
    let idle: Rc<dyn Locator> = Rc::new(YearLocator::new(1, 1, 1).expect("a valid yearly rule"));
    formatter
        .following(&idle)
        .unwrap_or_else(|| Rc::clone(formatter))
}

impl UnitState for AxisState {
    fn units(&self) -> &AxisUnits {
        &self.units
    }

    fn units_mut(&mut self) -> &mut AxisUnits {
        &mut self.units
    }

    fn categories(&self) -> Option<&Categories> {
        self.categories.as_ref()
    }

    fn categories_mut(&mut self) -> &mut Option<Categories> {
        &mut self.categories
    }

    fn use_unit_tickers(&mut self, locator: Rc<dyn Locator>, formatter: Rc<dyn Formatter>) {
        let locates = self.defaults.major.locator;
        if locates {
            self.major_locator = self.for_role(locator);
        }
        if self.defaults.major.formatter {
            self.major_formatter = if locates {
                formatter
            } else {
                idle_locator_formatter(&formatter)
            };
        }
    }

    fn use_axis_info_tickers(&mut self, info: &UnitAxisInfo) {
        let locates = info.majloc.is_some() && self.defaults.major.locator;
        if let Some(locator) = &info.majloc
            && self.defaults.major.locator
        {
            self.set_major_locator(Rc::clone(locator));
            self.defaults.major.locator = true;
        }
        if let Some(locator) = &info.minloc
            && self.defaults.minor.locator
        {
            self.set_minor_locator(Rc::clone(locator));
            self.defaults.minor.locator = true;
        }
        if let Some(formatter) = &info.majfmt
            && self.defaults.major.formatter
        {
            let formatter = if locates || info.majloc.is_none() {
                Rc::clone(formatter)
            } else {
                idle_locator_formatter(formatter)
            };
            self.set_major_formatter(formatter);
            self.defaults.major.formatter = true;
        }
        if let Some(formatter) = &info.minfmt
            && self.defaults.minor.formatter
        {
            self.set_minor_formatter(Rc::clone(formatter));
            self.defaults.minor.formatter = true;
        }
    }

    fn use_default_label(&mut self, label: &str) {
        if self.label.is_default {
            label.clone_into(&mut self.label.text);
        }
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
