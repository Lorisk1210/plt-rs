//! Configuring ticks: tick parameters, minor ticks, locator parameters,
//! axis z-order and tick label formats (matplotlib's `tick_params`,
//! `minorticks_on`, `locator_params`, `set_axisbelow`,
//! `ticklabel_format`).

use std::any::Any;
use std::borrow::Cow;
use std::rc::Rc;

use plt_text::{HAlign, RotationMode, VAlign};

use super::axis::AxisState;
use super::axis_handle::Axis;
use super::tick_lines::TickLineProps;
use super::ticks::{GridOptions, TickDirection, TickLevel, TickStyle};
use super::{Axes, AxesState, AxisName, PlotError};
use crate::colors::Color;
use crate::fonts::{FontSize, FontStyle, FontWeight};
use crate::lines::LineStyle;
use crate::polar::ThetaLocator;
use crate::rc::{AxisBelow, RcParams};
use crate::scale::{ScaleError, ScaleOptions, scale_factory};
use crate::ticker::{
    AutoLocator, AutoMinorLocator, FixedLocator, Formatter, Locator, MaxNLocator, Nbins,
    NullLocator, OffsetMode, ScalarFormatter,
};
use crate::units::AxisValues;

/// Options of [`Axes::ticklabel_format`]; unset fields keep the current
/// setting of the formatter.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TicklabelFormat {
    /// Only this axis (`None` = both).
    pub axis: Option<AxisName>,
    /// Scientific notation (`style='sci'`) or plain numbers (`'plain'`).
    pub scientific: Option<bool>,
    /// Exponents outside `[low, high]` use scientific notation
    /// (`scilimits`; `(0, 0)` always does).
    pub scilimits: Option<(i32, i32)>,
    /// Whether and which offset is subtracted (`useOffset`).
    pub use_offset: Option<OffsetMode>,
    /// Format the offset and scale texts as math text (`useMathText`).
    pub use_mathtext: Option<bool>,
}

impl TicklabelFormat {
    /// `formatter` with the settings this format sets changed.
    pub(crate) fn applied_to(&self, mut formatter: ScalarFormatter) -> ScalarFormatter {
        if let Some(scientific) = self.scientific {
            formatter = formatter.with_scientific(scientific);
        }
        if let Some(limits) = self.scilimits {
            formatter = formatter.with_powerlimits(limits);
        }
        if let Some(mode) = self.use_offset {
            formatter = formatter.with_use_offset(mode);
        }
        if let Some(mathtext) = self.use_mathtext {
            formatter = formatter.with_use_mathtext(mathtext);
        }
        formatter
    }
}

impl Axis {
    /// Turns minor ticks on with the locator matplotlib picks for the
    /// scale: the scale's own minor locator for log, symlog, asinh and
    /// logit axes, otherwise an `AutoMinorLocator` (matplotlib's
    /// `Axis.minorticks_on`).
    pub fn minorticks_on(&self) -> &Self {
        let scale_minor = self.read(|axis| {
            matches!(axis.scale().name(), "log" | "symlog" | "asinh" | "logit")
                .then(|| axis.scale().default_tickers(axis.name, &axis.settings))
        });
        let locator: Rc<dyn Locator> = match scale_minor {
            Some(tickers) => Rc::from(tickers.minor_locator),
            None => Rc::new(AutoMinorLocator::new(None)),
        };
        self.update_shared(|axis| axis.set_minor_locator(Rc::clone(&locator)));
        self
    }

    /// Removes the minor ticks (matplotlib's `Axis.minorticks_off`).
    pub fn minorticks_off(&self) -> &Self {
        let locator: Rc<dyn Locator> = Rc::new(NullLocator);
        self.update_shared(|axis| axis.set_minor_locator(Rc::clone(&locator)));
        self
    }
}

impl Axes {
    /// Turns minor ticks on for both axes (matplotlib's `minorticks_on`).
    pub fn minorticks_on(&self) -> &Self {
        self.xaxis().minorticks_on();
        self.yaxis().minorticks_on();
        self
    }

    /// Removes the minor ticks of both axes (matplotlib's `minorticks_off`).
    pub fn minorticks_off(&self) -> &Self {
        self.xaxis().minorticks_off();
        self.yaxis().minorticks_off();
        self
    }

    /// Draws ticks and gridlines below or above the plotted artists
    /// (matplotlib's `set_axisbelow`).
    pub fn set_axisbelow(&self, below: AxisBelow) -> &Self {
        self.raw.with_mut::<AxesState, _>(|a| a.axisbelow = below);
        self
    }

    /// Where ticks and gridlines sit among the plotted artists
    /// (matplotlib's `get_axisbelow`).
    pub fn axisbelow(&self) -> AxisBelow {
        self.raw.with::<AxesState, _>(|a| a.axisbelow)
    }

    /// Sets the scale of one axis by name with options (matplotlib's
    /// `set_xscale(name, **kwargs)`).
    pub fn set_scale_with(
        &self,
        axis: AxisName,
        name: &str,
        options: ScaleOptions,
    ) -> Result<&Self, ScaleError> {
        Ok(self.set_scale(axis, scale_factory(name, options)?))
    }

    /// Changes how the default major tick labels are formatted
    /// (matplotlib's `ticklabel_format`); the axes sharing an axis follow.
    ///
    /// # Errors
    ///
    /// When an affected axis does not label its ticks with a
    /// `ScalarFormatter`.
    pub fn ticklabel_format(&self, format: &TicklabelFormat) -> Result<&Self, PlotError> {
        for name in [AxisName::X, AxisName::Y] {
            if format.axis.is_some_and(|only| only != name) {
                continue;
            }
            let axis = self.axis_of(name);
            let current = axis.read(|state| {
                let formatter: &dyn Any = state.major_formatter.as_ref();
                formatter.downcast_ref::<ScalarFormatter>().cloned()
            });
            let formatter = current.ok_or(PlotError::NotScalarFormatter(name))?;
            let formatter: Rc<dyn Formatter> = Rc::new(format.applied_to(formatter));
            axis.update_shared(|state| state.major_formatter = Rc::clone(&formatter));
        }
        Ok(self)
    }
}

/// Options of [`Axes::tick_params`] (matplotlib's `tick_params`); unset
/// fields keep their current value. The side switches only apply to the
/// axis drawn on that side (`bottom` and `top` to x, `left` and `right`
/// to y).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TickParams {
    /// Only this axis (`None` = both).
    pub axis: Option<AxisName>,
    /// Which tick levels change.
    pub which: TickLevel,
    /// Restore the rc defaults before applying the rest.
    pub reset: bool,
    /// Tick direction.
    pub direction: Option<TickDirection>,
    /// Tick length in points.
    pub length: Option<f64>,
    /// Tick width in points.
    pub width: Option<f64>,
    /// Tick colour.
    pub color: Option<Color>,
    /// Gap between tick and label in points.
    pub pad: Option<f64>,
    /// Label font size.
    pub labelsize: Option<FontSize>,
    /// Label colour.
    pub labelcolor: Option<Color>,
    /// Tick and label colour (overrides `color` and `labelcolor`).
    pub colors: Option<Color>,
    /// Label rotation in degrees.
    pub labelrotation: Option<f64>,
    /// Whether the labels of polar axes turn with their ticks, with
    /// `labelrotation` (default 0) added: matplotlib's
    /// `labelrotation="auto"` (or `("auto", angle)`) for `Some(true)` and
    /// `"default"` for `Some(false)`. A `labelrotation` given alone is a
    /// fixed rotation.
    pub labelrotation_auto: Option<bool>,
    /// How rotated labels are aligned.
    pub labelrotation_mode: Option<RotationMode>,
    /// Label font family (`labelfontfamily`).
    pub labelfontfamily: Option<String>,
    /// Ticks on the bottom side.
    pub bottom: Option<bool>,
    /// Ticks on the top side.
    pub top: Option<bool>,
    /// Ticks on the left side.
    pub left: Option<bool>,
    /// Ticks on the right side.
    pub right: Option<bool>,
    /// Labels on the bottom side.
    pub labelbottom: Option<bool>,
    /// Labels on the top side.
    pub labeltop: Option<bool>,
    /// Labels on the left side.
    pub labelleft: Option<bool>,
    /// Labels on the right side.
    pub labelright: Option<bool>,
    /// Gridline colour (`grid_color`).
    pub grid_color: Option<Color>,
    /// Gridline opacity (`grid_alpha`).
    pub grid_alpha: Option<f64>,
    /// Gridline width in points (`grid_linewidth`).
    pub grid_linewidth: Option<f64>,
    /// Gridline style (`grid_linestyle`).
    pub grid_linestyle: Option<LineStyle>,
}

impl TickParams {
    /// Applies the parameters to the style of one tick level of axis
    /// `name`.
    pub(crate) fn apply(&self, name: AxisName, major: bool, style: &mut TickStyle, rc: &RcParams) {
        if self.reset {
            *style = TickStyle::from_rc(name, major, rc);
        }
        let set = |slot: &mut f64, value: Option<f64>| {
            if let Some(value) = value {
                *slot = value;
            }
        };
        set(&mut style.size, self.length);
        set(&mut style.width, self.width);
        set(&mut style.pad, self.pad);
        if self.labelrotation.is_some() || self.labelrotation_auto.is_some() {
            style.label_rotation = self.labelrotation.unwrap_or(0.0);
            style.label_rotation_auto = self.labelrotation_auto.unwrap_or(false);
        }
        if let Some(direction) = self.direction {
            style.direction = direction;
        }
        if let Some(size) = self.labelsize {
            style.label_size = size.to_points(rc.font_size);
        }
        if let Some(mode) = self.labelrotation_mode {
            style.label_rotation_mode = mode;
        }
        if let Some(family) = &self.labelfontfamily {
            style.label_family = Some(family.clone());
        }
        let (color, labelcolor) = match &self.colors {
            Some(both) => (Some(both), Some(both)),
            None => (self.color.as_ref(), self.labelcolor.as_ref()),
        };
        if let Some(color) = color {
            style.color = color.clone();
        }
        if let Some(color) = labelcolor {
            style.label_color = color.clone();
        }
        GridOptions {
            color: self.grid_color.clone(),
            alpha: self.grid_alpha,
            linewidth: self.grid_linewidth,
            linestyle: self.grid_linestyle.clone(),
            ..GridOptions::default()
        }
        .restyle(&mut style.grid);
        let (low, high, label_low, label_high) = match name {
            AxisName::X => (self.bottom, self.top, self.labelbottom, self.labeltop),
            AxisName::Y => (self.left, self.right, self.labelleft, self.labelright),
        };
        for (slot, value) in [
            (&mut style.ticks.first, low),
            (&mut style.ticks.second, high),
            (&mut style.labels.first, label_low),
            (&mut style.labels.second, label_high),
        ] {
            if let Some(value) = value {
                *slot = value;
            }
        }
    }
}

/// Text properties of tick labels (the keyword arguments of matplotlib's
/// `set_ticklabels`); unset fields keep their current value.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TickLabelProps {
    /// Rotation in degrees.
    pub rotation: Option<f64>,
    /// How rotated labels are aligned.
    pub rotation_mode: Option<RotationMode>,
    /// Horizontal alignment.
    pub halign: Option<HAlign>,
    /// Vertical alignment.
    pub valign: Option<VAlign>,
    /// Alignment of the lines of multi-line labels (`multialignment`).
    pub multialign: Option<HAlign>,
    /// Text colour.
    pub color: Option<Color>,
    /// Font size.
    pub fontsize: Option<FontSize>,
    /// Font weight.
    pub fontweight: Option<FontWeight>,
    /// Font style (upright, italic or oblique).
    pub fontstyle: Option<FontStyle>,
    /// Whether the labels count in layouts and push the axis label and
    /// the offset text away (matplotlib's `set_in_layout`).
    pub in_layout: Option<bool>,
}

impl TickLabelProps {
    /// The same properties with a relative font size turned into points
    /// against `base_size` (font sizes are resolved when they are set).
    fn with_absolute_size(&self, base_size: f64) -> Self {
        Self {
            fontsize: (self.fontsize).map(|size| FontSize::Points(size.to_points(base_size))),
            ..self.clone()
        }
    }

    /// `self` with every property set in `newer` replaced.
    fn overlay(&self, newer: &Self) -> Self {
        Self {
            rotation: newer.rotation.or(self.rotation),
            rotation_mode: newer.rotation_mode.or(self.rotation_mode),
            halign: newer.halign.or(self.halign),
            valign: newer.valign.or(self.valign),
            multialign: newer.multialign.or(self.multialign),
            color: newer.color.clone().or_else(|| self.color.clone()),
            fontsize: newer.fontsize.or(self.fontsize),
            fontweight: newer.fontweight.or(self.fontweight),
            fontstyle: newer.fontstyle.or(self.fontstyle),
            in_layout: newer.in_layout.or(self.in_layout),
        }
    }

    /// Sets the label properties of `style` that are set here; font sizes
    /// must be absolute ([`Self::with_absolute_size`]).
    fn apply(&self, style: &mut TickStyle) {
        if let Some(rotation) = self.rotation {
            style.label_rotation = rotation;
        }
        if let Some(mode) = self.rotation_mode {
            style.label_rotation_mode = mode;
        }
        if self.halign.is_some() {
            style.label_halign = self.halign;
        }
        if self.valign.is_some() {
            style.label_valign = self.valign;
        }
        if self.multialign.is_some() {
            style.label_multialign = self.multialign;
        }
        if let Some(color) = &self.color {
            style.label_color = color.clone();
        }
        if let Some(size) = self.fontsize {
            style.label_size = size.to_points(style.label_size);
        }
        if self.fontweight.is_some() {
            style.label_weight = self.fontweight;
        }
        if self.fontstyle.is_some() {
            style.label_style = self.fontstyle;
        }
        if let Some(in_layout) = self.in_layout {
            style.label_in_layout = in_layout;
        }
    }
}

impl AxisState {
    /// The style of the major tick at `index` (counting every tick the
    /// major locator places): the major style with that tick's label,
    /// mark and gridline properties on top.
    pub(crate) fn major_tick_style(&self, index: usize) -> Cow<'_, TickStyle> {
        let label = (self.major_label_overrides.get(index))
            .filter(|props| **props != TickLabelProps::default());
        let line = (self.major_line_overrides.get(index))
            .filter(|props| **props != TickLineProps::default());
        if label.is_none() && line.is_none() {
            return Cow::Borrowed(&self.major_style);
        }
        let mut style = self.major_style.clone();
        if let Some(props) = label {
            props.apply(&mut style);
        }
        if let Some(props) = line {
            props.apply(&mut style);
        }
        Cow::Owned(style)
    }
}

/// Options of [`Axes::locator_params`]; unset fields keep the locator's
/// current setting.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LocatorParams {
    /// Only this axis (`None` = both).
    pub axis: Option<AxisName>,
    /// Maximum number of intervals.
    pub nbins: Option<Nbins>,
    /// Acceptable tick multiples within `[1, 10]`.
    pub steps: Option<Vec<f64>>,
    /// Integer ticks only.
    pub integer: Option<bool>,
    /// Symmetric about zero.
    pub symmetric: Option<bool>,
}

impl Axis {
    /// Changes the tick parameters of this axis (matplotlib's
    /// `Axis.set_tick_params`); `params.axis` is ignored.
    pub fn set_tick_params(&self, params: &TickParams) -> &Self {
        let rc = crate::rc::params();
        let name = self.name();
        self.update(|axis| {
            if params.which.major() {
                params.apply(name, true, &mut axis.major_style, &rc);
                // The tick-space heuristic follows the major label size of
                // the tick parameters (matplotlib's `_get_tick_label_size`).
                if params.reset {
                    axis.label_size = None;
                }
                if params.labelsize.is_some() {
                    axis.label_size = Some(axis.major_style.label_size);
                }
            }
            if params.which.minor() {
                params.apply(name, false, &mut axis.minor_style, &rc);
            }
        });
        self
    }

    /// Changes the text properties of the labels of one tick level.
    pub fn set_ticklabel_props(&self, which: TickLevel, props: TickLabelProps) -> &Self {
        let props = props.with_absolute_size(crate::rc::params().font_size);
        self.update_labels(|axis| {
            if which.major() {
                props.apply(&mut axis.major_style);
            }
            if which.minor() {
                props.apply(&mut axis.minor_style);
            }
        });
        self
    }

    /// Changes the text properties of the labels of the major tick at
    /// `index` among those the major locator places, in view or not
    /// (matplotlib's `axis.get_major_ticks()[index].label1.set(...)` and
    /// the same on `label2`). They apply on top of the properties of the
    /// major tick labels, to that tick only.
    ///
    /// ```
    /// use plt::axes::TickLabelProps;
    /// use plt::figure::Figure;
    /// let ax = Figure::new().subplot();
    /// ax.set_xticks([0.0, 0.5, 1.0]);
    /// let tilted = TickLabelProps { rotation: Some(45.0), ..TickLabelProps::default() };
    /// ax.xaxis().set_major_ticklabel_props(1, tilted);
    /// ```
    pub fn set_major_ticklabel_props(&self, index: usize, props: TickLabelProps) -> &Self {
        let props = props.with_absolute_size(crate::rc::params().font_size);
        self.update_labels(|axis| {
            let overrides = &mut axis.major_label_overrides;
            if overrides.len() <= index {
                overrides.resize_with(index + 1, TickLabelProps::default);
            }
            overrides[index] = overrides[index].overlay(&props);
        });
        self
    }

    /// Puts the minor ticks exactly at `ticks` (matplotlib's
    /// `set_ticks(..., minor=True)`), widening the view to include them.
    pub fn set_minor_ticks(&self, ticks: impl Into<AxisValues>) -> &Self {
        let ticks = self.convert_units(ticks);
        self.include_ticks_in_view(&ticks);
        let locator: Rc<dyn Locator> = Rc::new(FixedLocator::new(ticks));
        self.update_shared(|axis| axis.set_minor_locator(Rc::clone(&locator)));
        self
    }
}

impl Axes {
    /// Changes the tick parameters of one or both axes (matplotlib's
    /// `tick_params`).
    pub fn tick_params(&self, params: &TickParams) -> &Self {
        for name in [AxisName::X, AxisName::Y] {
            if params.axis.is_none_or(|only| only == name) {
                self.axis_of(name).set_tick_params(params);
            }
        }
        self
    }

    /// Changes the parameters of the major locators (matplotlib's
    /// `locator_params`) and re-autoscales the affected axes. A
    /// `MaxNLocator` (or `AutoLocator`) takes every parameter, a
    /// `FixedLocator` (from [`Axis::set_ticks`]) only a fixed `nbins`,
    /// which subsamples its ticks. The default θ locator of polar axes
    /// passes them to the locator it uses for sectors (matplotlib's
    /// `ThetaLocator.base.set_params`).
    ///
    /// # Errors
    ///
    /// When an affected axis places its major ticks with another locator,
    /// its locator does not take a given parameter, or `steps` are
    /// invalid.
    pub fn locator_params(&self, params: &LocatorParams) -> Result<&Self, PlotError> {
        for name in [AxisName::X, AxisName::Y] {
            if params.axis.is_some_and(|only| only != name) {
                continue;
            }
            let axis = self.axis_of(name);
            axis.set_major_locator_params(params)?;
            axis.update_shared(|state| state.autoscale.stale = true);
        }
        Ok(self)
    }
}

impl Axis {
    /// Changes the parameters of this axis' major locator as
    /// [`Axes::locator_params`] does, but leaves the view limits alone
    /// (matplotlib's `axis.get_major_locator().set_params(...)`);
    /// `params.axis` is ignored.
    ///
    /// ```
    /// use plt::axes::LocatorParams;
    /// use plt::prelude::*;
    /// use plt::ticker::Nbins;
    /// let ax = Figure::new().subplot();
    /// let params = LocatorParams { nbins: Some(Nbins::Fixed(7)), ..LocatorParams::default() };
    /// ax.xaxis().set_major_locator_params(&params)?;
    /// assert_eq!(ax.xlim(), (0.0, 1.0), "an empty axes keeps its limits");
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// As [`Axes::locator_params`].
    pub fn set_major_locator_params(&self, params: &LocatorParams) -> Result<&Self, PlotError> {
        let locator = self.read(|state| with_params(state.major_locator.as_ref(), params));
        let locator = locator.ok_or(PlotError::NotMaxNLocator(self.name()))??;
        self.update_shared(|state| state.major_locator = Rc::clone(&locator));
        Ok(self)
    }
}

/// `locator` with `params` applied (its `set_params`), `None` when it
/// does not take them.
fn with_params(
    locator: &dyn Locator,
    params: &LocatorParams,
) -> Option<Result<Rc<dyn Locator>, PlotError>> {
    let any: &dyn Any = locator;
    if let Some(theta) = any.downcast_ref::<ThetaLocator>() {
        return with_params(theta.base(), params)
            .map(|base| base.map(|base| Rc::new(ThetaLocator::wrapping(base)) as Rc<dyn Locator>));
    }
    if let Some(fixed) = any.downcast_ref::<FixedLocator>() {
        let only_nbins =
            params.steps.is_none() && params.integer.is_none() && params.symmetric.is_none();
        return match (params.nbins, only_nbins) {
            (Some(Nbins::Fixed(nbins)), true) => Some(Ok(Rc::new(fixed.clone().with_nbins(nbins)))),
            (None, true) => Some(Ok(Rc::new(fixed.clone()))),
            _ => None,
        };
    }
    let mut locator = any
        .downcast_ref::<AutoLocator>()
        .map(|auto| auto.as_max_n().clone())
        .or_else(|| any.downcast_ref::<MaxNLocator>().cloned())?;
    if let Some(nbins) = params.nbins {
        locator = locator.with_nbins(nbins);
    }
    if let Some(steps) = &params.steps {
        locator = match locator.with_steps(steps) {
            Ok(locator) => locator,
            Err(err) => return Some(Err(PlotError::Ticker(err))),
        };
    }
    if let Some(integer) = params.integer {
        locator = locator.with_integer(integer);
    }
    if let Some(symmetric) = params.symmetric {
        locator = locator.with_symmetric(symmetric);
    }
    Some(Ok(Rc::new(locator)))
}
