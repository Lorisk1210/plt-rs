//! A Python formatter labelling an axis pltrs (Rust) draws, asked the way
//! matplotlib's `Axis` asks its formatter: for all the labels of the
//! ticks at once and then for the offset text (`format_ticks`,
//! `get_offset`), so that formatters keeping state between the two (an
//! offset, an order of magnitude) see every tick, and for single values
//! with `format_data` and `format_data_short`.
//!
//! While the figure is drawn its axes cannot be asked about themselves,
//! so the formatter is handed the description pltrs (Rust) has of the
//! axis (an [`AxisInfo`], which its `_axis_info()` gives back to the native
//! functions it calls).

use pltrs::ticker::{AxisInfo, FormattedTicks, Formatter, TickContext, TickerError};

use crate::binding::prelude::*;

native_value!(AxisInfo);

/// The methods of a Python `Formatter` an axis calls.
#[derive(Debug)]
pub(super) struct AxisFormatter {
    /// `formatter._label_axis(locs, axis)`: `(labels, offset)`.
    label_axis: Callback,
    /// `formatter.format_data(value)`.
    format_data: Callback,
    /// `formatter.format_data_short(value)`.
    format_data_short: Callback,
}

impl AxisFormatter {
    /// The formatter `formatter`, if it has the methods of a `Formatter`.
    pub(super) fn of(formatter: &Bound<'_>) -> PyResult<Self> {
        let method = |name: &str| -> PyResult<Callback> { formatter.getattr(name)?.extract() };
        Ok(Self {
            label_axis: method("_label_axis")?,
            format_data: method("format_data")?,
            format_data_short: method("format_data_short")?,
        })
    }
}

impl Formatter for AxisFormatter {
    /// The formatter's labels and offset text; none when it raised (the
    /// exception surfaces from the native call that drew).
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
}
