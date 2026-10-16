//! Handles to the x and y axis of an axes (matplotlib's `XAxis` and
//! `YAxis`): tick helpers, tick positions and labels, unit conversion.

use std::any::Any;

use super::axis::AxisState;
use super::{Axes, AxesState, PlotError};
use crate::artist::ArtistHandle;
use crate::ticker::{
    AxisName, FixedFormatter, FixedLocator, Formatter, FuncFormatter, Locator, NullFormatter,
};
use crate::units::AxisValues;

/// One axis of an axes, from [`Axes::xaxis`], [`Axes::yaxis`] or
/// [`Axes::axis_of`].
///
/// ```
/// use plt::prelude::*;
/// let ax = Figure::new().subplot();
/// ax.xaxis().set_ticks([1.0, 2.0, 3.0]).set_ticklabels(["one", "two", "three"])?;
/// # Ok::<(), plt::axes::PlotError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Axis {
    axes: Axes,
    name: AxisName,
}

impl Axis {
    /// Which axis this is.
    pub fn name(&self) -> AxisName {
        self.name
    }

    fn update<R>(&self, f: impl FnOnce(&mut AxisState) -> R) -> R {
        self.axes
            .raw()
            .with_mut::<AxesState, _>(|axes| f(axes.axis_mut(self.name)))
    }

    /// Places the major ticks with `locator`.
    pub fn set_major_locator(&self, locator: impl Locator) -> &Self {
        self.update(|axis| axis.set_major_locator(Box::new(locator)));
        self
    }

    /// Places the minor ticks with `locator`.
    pub fn set_minor_locator(&self, locator: impl Locator) -> &Self {
        self.update(|axis| axis.minor_locator = Box::new(locator));
        self
    }

    /// Labels the major ticks with `formatter`.
    pub fn set_major_formatter(&self, formatter: impl Formatter) -> &Self {
        self.update(|axis| axis.set_major_formatter(Box::new(formatter)));
        self
    }

    /// Labels the minor ticks with `formatter`.
    pub fn set_minor_formatter(&self, formatter: impl Formatter) -> &Self {
        self.update(|axis| axis.minor_formatter = Box::new(formatter));
        self
    }

    /// Converts plotting values to axis numbers, registering new category
    /// names (matplotlib's `update_units` + `convert_units`).
    pub fn convert_units(&self, values: impl Into<AxisValues>) -> Vec<f64> {
        let values = values.into();
        self.update(|axis| axis.convert_units(values))
    }

    /// Puts the major ticks exactly at `ticks`, widening the view to
    /// include them (matplotlib's `Axis.set_ticks`).
    pub fn set_ticks(&self, ticks: impl Into<AxisValues>) -> &Self {
        let ticks = self.convert_units(ticks);
        let lowest = ticks.iter().copied().fold(f64::INFINITY, f64::min);
        let highest = ticks.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        if !ticks.is_empty() {
            self.axes.unstale_view();
        }
        self.axes.raw().with_mut::<AxesState, _>(|axes| {
            if !ticks.is_empty() {
                axes.include_in_view(self.name, lowest, highest);
            }
            axes.axis_mut(self.name)
                .set_major_locator(Box::new(FixedLocator::new(ticks)));
        });
        self
    }

    /// Labels the major ticks with `labels`: one per fixed tick when the
    /// ticks were set explicitly, otherwise in order; no labels hide them
    /// (matplotlib's `Axis.set_ticklabels`).
    ///
    /// # Errors
    ///
    /// When the ticks were set explicitly and their number differs from
    /// the number of labels.
    pub fn set_ticklabels<S: Into<String>>(
        &self,
        labels: impl IntoIterator<Item = S>,
    ) -> Result<&Self, PlotError> {
        let labels: Vec<String> = labels.into_iter().map(Into::into).collect();
        self.update(|axis| {
            tick_label_formatter(axis.major_locator.as_ref(), labels)
                .map(|formatter| axis.set_major_formatter(formatter))
        })?;
        Ok(self)
    }

    /// Whether the axis runs from high to low values (matplotlib's
    /// `Axis.get_inverted`).
    pub fn inverted(&self) -> bool {
        let (low, high) = match self.name {
            AxisName::X => self.axes.xlim(),
            AxisName::Y => self.axes.ylim(),
        };
        high < low
    }
}

/// The formatter showing `labels` at the ticks `locator` places.
fn tick_label_formatter(
    locator: &dyn Locator,
    labels: Vec<String>,
) -> Result<Box<dyn Formatter>, PlotError> {
    if labels.is_empty() {
        return Ok(Box::new(NullFormatter));
    }
    let any: &dyn Any = locator;
    let Some(fixed) = any.downcast_ref::<FixedLocator>() else {
        return Ok(Box::new(FixedFormatter::new(labels)));
    };
    if fixed.locs().len() != labels.len() {
        return Err(PlotError::TickLabelCount {
            ticks: fixed.locs().len(),
            labels: labels.len(),
        });
    }
    let table: Vec<(f64, String)> = fixed.locs().iter().copied().zip(labels).collect();
    Ok(Box::new(FuncFormatter::new(move |value, _| {
        label_at(&table, value)
    })))
}

/// The label of the tick exactly at `value`, or nothing (matplotlib's
/// `_format_with_dict`).
#[expect(
    clippy::float_cmp,
    reason = "labels belong to exact tick positions, as in matplotlib's dict lookup"
)]
fn label_at(table: &[(f64, String)], value: f64) -> String {
    table
        .iter()
        .find(|(loc, _)| *loc == value)
        .map(|(_, label)| label.clone())
        .unwrap_or_default()
}

impl Axes {
    /// The x axis.
    pub fn xaxis(&self) -> Axis {
        self.axis_of(AxisName::X)
    }

    /// The y axis.
    pub fn yaxis(&self) -> Axis {
        self.axis_of(AxisName::Y)
    }

    /// The axis called `name`.
    pub fn axis_of(&self, name: AxisName) -> Axis {
        Axis {
            axes: self.clone(),
            name,
        }
    }

    /// Puts the x ticks exactly at `ticks` (matplotlib's `set_xticks`).
    pub fn set_xticks(&self, ticks: impl Into<AxisValues>) -> &Self {
        self.xaxis().set_ticks(ticks);
        self
    }

    /// Puts the y ticks exactly at `ticks` (matplotlib's `set_yticks`).
    pub fn set_yticks(&self, ticks: impl Into<AxisValues>) -> &Self {
        self.yaxis().set_ticks(ticks);
        self
    }

    /// Labels the x ticks (matplotlib's `set_xticklabels`).
    ///
    /// # Errors
    ///
    /// See [`Axis::set_ticklabels`].
    pub fn set_xticklabels<S: Into<String>>(
        &self,
        labels: impl IntoIterator<Item = S>,
    ) -> Result<&Self, PlotError> {
        self.xaxis().set_ticklabels(labels)?;
        Ok(self)
    }

    /// Labels the y ticks (matplotlib's `set_yticklabels`).
    ///
    /// # Errors
    ///
    /// See [`Axis::set_ticklabels`].
    pub fn set_yticklabels<S: Into<String>>(
        &self,
        labels: impl IntoIterator<Item = S>,
    ) -> Result<&Self, PlotError> {
        self.yaxis().set_ticklabels(labels)?;
        Ok(self)
    }
}
