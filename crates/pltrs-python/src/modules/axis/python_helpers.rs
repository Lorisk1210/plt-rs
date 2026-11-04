//! Python locators and formatters driving the ticks of an axis pltrs
//! (Rust) draws: the adapters `_tick_helpers` makes for a helper of the
//! program's (a `Locator` or `Formatter` subclass, a `FuncFormatter`, the
//! date and category helpers), asked the way matplotlib's `Axis` asks its
//! helpers. A formatter is asked for all the labels of the ticks at once
//! and then for the offset text, so that formatters keeping state between
//! the two (an offset, an order of magnitude) see every tick.
//!
//! While the figure is laid out or drawn its axes cannot be asked about
//! themselves, so every call hands the helper the description pltrs
//! (Rust) has of the axis (an [`AxisInfo`]), which the helper's axis
//! answers from meanwhile. A helper that raises returns nothing (no ticks,
//! no labels) and its exception surfaces from the native call that drew.

use pltrs::ticker::{AxisInfo, FormattedTicks, Formatter, Locator, TickContext, TickerError};

use crate::binding::prelude::*;

native_value!(AxisInfo);

/// A method of the Python adapter `adapter`.
fn method(adapter: &Bound<'_>, name: &str) -> PyResult<Callback> {
    adapter.getattr(name)?.extract()
}

/// The Python locator behind an adapter with the methods `locate(info)`,
/// `tick_values(vmin, vmax, info)`, `nonsingular(v0, v1, info)`,
/// `view_limits(dmin, dmax, info)` and `date_unit(info)` (the days per
/// tick unit of a date locator, `None` for others).
#[derive(Debug)]
pub(crate) struct PyLocator {
    /// The adapter, which also gives the helper back (`helper()`).
    pub(crate) adapter: Owned,
    locate: Callback,
    tick_values: Callback,
    nonsingular: Callback,
    view_limits: Callback,
    date_unit: Callback,
}

impl PyLocator {
    /// The locator behind `adapter`.
    pub(crate) fn of(adapter: &Bound<'_>) -> PyResult<Self> {
        Ok(Self {
            adapter: adapter.clone().unbind(),
            locate: method(adapter, "locate")?,
            tick_values: method(adapter, "tick_values")?,
            nonsingular: method(adapter, "nonsingular")?,
            view_limits: method(adapter, "view_limits")?,
            date_unit: method(adapter, "date_unit")?,
        })
    }
}

impl Locator for PyLocator {
    fn tick_values(
        &self,
        vmin: f64,
        vmax: f64,
        ctx: &TickContext<'_>,
    ) -> Result<Vec<f64>, TickerError> {
        Ok(self
            .tick_values
            .call((vmin, vmax, ctx.axis().cloned()))
            .unwrap_or_default())
    }

    fn locate(&self, ctx: &TickContext<'_>) -> Result<Vec<f64>, TickerError> {
        Ok(self.locate.call((ctx.axis().cloned(),)).unwrap_or_default())
    }

    fn nonsingular(&self, v0: f64, v1: f64, ctx: &TickContext<'_>) -> (f64, f64) {
        self.nonsingular
            .call((v0, v1, ctx.axis().cloned()))
            .unwrap_or((v0, v1))
    }

    fn view_limits(
        &self,
        dmin: f64,
        dmax: f64,
        ctx: &TickContext<'_>,
    ) -> Result<(f64, f64), TickerError> {
        Ok(self
            .view_limits
            .call((dmin, dmax, ctx.axis().cloned()))
            .unwrap_or((dmin, dmax)))
    }

    fn date_unit(&self, ctx: &TickContext<'_>) -> Option<f64> {
        self.date_unit.call((ctx.axis().cloned(),)).flatten()
    }

    fn calls_program(&self) -> bool {
        true
    }
}

/// The Python formatter behind an adapter with the methods
/// `label_axis(locs, info)` (the labels and the offset text),
/// `format_data(value)` and `format_data_short(value)`.
#[derive(Debug)]
pub(crate) struct PyFormatter {
    /// The adapter, which also gives the helper back (`helper()`).
    pub(crate) adapter: Owned,
    label_axis: Callback,
    format_data: Callback,
    format_data_short: Callback,
}

impl PyFormatter {
    /// The formatter behind `adapter`.
    pub(crate) fn of(adapter: &Bound<'_>) -> PyResult<Self> {
        Ok(Self {
            adapter: adapter.clone().unbind(),
            label_axis: method(adapter, "label_axis")?,
            format_data: method(adapter, "format_data")?,
            format_data_short: method(adapter, "format_data_short")?,
        })
    }
}

impl Formatter for PyFormatter {
    /// The formatter's labels and offset text; none when it raised.
    fn format_ticks(
        &self,
        locs: &[f64],
        ctx: &TickContext<'_>,
    ) -> Result<FormattedTicks, TickerError> {
        let Some((labels, offset)) = self
            .label_axis
            .call::<(Vec<String>, String)>((NdArray::from_vec(locs.to_vec()), ctx.axis().cloned()))
        else {
            return Ok(FormattedTicks::default());
        };
        if labels.len() != locs.len() {
            return Err(TickerError::InvalidParameter {
                name: "labels",
                reason: format!("{} labels for {} ticks", labels.len(), locs.len()),
            });
        }
        Ok(FormattedTicks { labels, offset })
    }

    fn format_data(&self, value: f64, _: &TickContext<'_>) -> Result<String, TickerError> {
        Ok(self.format_data.call((value,)).unwrap_or_default())
    }

    fn format_data_short(&self, value: f64, _: &TickContext<'_>) -> Result<String, TickerError> {
        Ok(self.format_data_short.call((value,)).unwrap_or_default())
    }

    fn calls_program(&self) -> bool {
        true
    }
}
