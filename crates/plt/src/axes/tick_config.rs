//! Configuring ticks: tick parameters, minor ticks, locator parameters,
//! axis z-order and tick label formats (matplotlib's `tick_params`,
//! `minorticks_on`, `locator_params`, `set_axisbelow`,
//! `ticklabel_format`).

use std::any::Any;
use std::rc::Rc;

use plt_text::{HAlign, RotationMode, VAlign};

use super::axis_handle::Axis;
use super::ticks::{GridOptions, TickDirection, TickLevel, TickStyle};
use super::{Axes, AxesState, AxisName, PlotError};
use crate::colors::Color;
use crate::fonts::FontSize;
use crate::lines::LineStyle;
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
        self.update_shared(|axis| axis.minor_locator = Rc::clone(&locator));
        self
    }

    /// Removes the minor ticks (matplotlib's `Axis.minorticks_off`).
    pub fn minorticks_off(&self) -> &Self {
        let locator: Rc<dyn Locator> = Rc::new(NullLocator);
        self.update_shared(|axis| axis.minor_locator = Rc::clone(&locator));
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
        self.raw
            .with_mut::<AxesState, _>(|a| a.axis_zorder = below.zorder());
        self
    }

    /// Sets the scale of one axis by name with options (matplotlib's
    /// `set_xscale(name, **kwargs)`).
    pub fn set_scale_with(
        &self,
        axis: AxisName,
        name: &str,
        options: &ScaleOptions,
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
            let mut formatter = current.ok_or(PlotError::NotScalarFormatter(name))?;
            if let Some(scientific) = format.scientific {
                formatter = formatter.with_scientific(scientific);
            }
            if let Some(limits) = format.scilimits {
                formatter = formatter.with_powerlimits(limits);
            }
            if let Some(mode) = format.use_offset {
                formatter = formatter.with_use_offset(mode);
            }
            if let Some(mathtext) = format.use_mathtext {
                formatter = formatter.with_use_mathtext(mathtext);
            }
            let formatter: Rc<dyn Formatter> = Rc::new(formatter);
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
    /// How rotated labels are aligned.
    pub labelrotation_mode: Option<RotationMode>,
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
        set(&mut style.label_rotation, self.labelrotation);
        if let Some(direction) = self.direction {
            style.direction = direction;
        }
        if let Some(size) = self.labelsize {
            style.label_size = size.to_points(rc.font_size);
        }
        if let Some(mode) = self.labelrotation_mode {
            style.label_rotation_mode = mode;
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
                if params.reset || params.labelsize.is_some() {
                    axis.label_size = axis.major_style.label_size;
                }
            }
            if params.which.minor() {
                params.apply(name, false, &mut axis.minor_style, &rc);
            }
        });
        self
    }

    /// Changes the text properties of the labels of one tick level.
    pub fn set_ticklabel_props(&self, which: TickLevel, props: &TickLabelProps) -> &Self {
        let base_size = crate::rc::params().font_size;
        let apply = |style: &mut TickStyle| {
            if let Some(rotation) = props.rotation {
                style.label_rotation = rotation;
            }
            if let Some(mode) = props.rotation_mode {
                style.label_rotation_mode = mode;
            }
            if props.halign.is_some() {
                style.label_halign = props.halign;
            }
            if props.valign.is_some() {
                style.label_valign = props.valign;
            }
            if props.multialign.is_some() {
                style.label_multialign = props.multialign;
            }
            if let Some(color) = &props.color {
                style.label_color = color.clone();
            }
            if let Some(size) = props.fontsize {
                style.label_size = size.to_points(base_size);
            }
        };
        self.update(|axis| {
            if which.major() {
                apply(&mut axis.major_style);
            }
            if which.minor() {
                apply(&mut axis.minor_style);
            }
        });
        self
    }

    /// Puts the minor ticks exactly at `ticks` (matplotlib's
    /// `set_ticks(..., minor=True)`), widening the view to include them.
    pub fn set_minor_ticks(&self, ticks: impl Into<AxisValues>) -> &Self {
        let ticks = self.convert_units(ticks);
        self.include_ticks_in_view(&ticks);
        let locator: Rc<dyn Locator> = Rc::new(FixedLocator::new(ticks));
        self.update_shared(|axis| axis.minor_locator = Rc::clone(&locator));
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
    /// which subsamples its ticks.
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
            let locator = axis.read(|state| with_params(state.major_locator.as_ref(), params));
            let locator = locator.ok_or(PlotError::NotMaxNLocator(name))??;
            axis.update_shared(|state| {
                state.major_locator = Rc::clone(&locator);
                state.autoscale.stale = true;
            });
        }
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
