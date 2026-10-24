//! Locator parameters (matplotlib's `locator_params`): the number of
//! intervals and the steps, integer and symmetry settings of the major
//! locators.

use std::any::Any;
use std::rc::Rc;

use crate::axes::{Axes, Axis, AxisName, PlotError};
use crate::polar::ThetaLocator;
use crate::ticker::{AutoLocator, FixedLocator, Locator, MaxNLocator, Nbins};

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

impl Axes {
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
