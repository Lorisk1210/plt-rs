//! Configuring ticks: minor ticks, axis z-order and tick label formats
//! (matplotlib's `minorticks_on`, `set_axisbelow`, `ticklabel_format`).

use std::any::Any;
use std::rc::Rc;

use super::axis_handle::Axis;
use super::{Axes, AxesState, AxisName, PlotError};
use crate::rc::AxisBelow;
use crate::scale::{ScaleError, ScaleOptions, scale_factory};
use crate::ticker::{
    AutoMinorLocator, Formatter, Locator, NullLocator, OffsetMode, ScalarFormatter,
};

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
