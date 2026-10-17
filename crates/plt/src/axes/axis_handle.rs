//! Handles to the x and y axis of an axes (matplotlib's `XAxis` and
//! `YAxis`): tick helpers, tick positions and labels, unit conversion.

use std::any::Any;
use std::rc::Rc;

use super::axis::{AxisState, LabelLoc};
use super::ticks::{GridOptions, GridStyle, Sides, TickPosition};
use super::{Axes, AxesState, PlotError};
use crate::artist::ArtistHandle;
use crate::colors::Color;
use crate::fonts::FontSize;
use crate::spines::SpineSide;
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

    /// Reads this axis's state.
    pub(super) fn read<R>(&self, f: impl FnOnce(&AxisState) -> R) -> R {
        self.axes
            .raw()
            .with::<AxesState, _>(|axes| f(axes.axis(self.name)))
    }

    /// Changes this axis's state.
    pub(super) fn update<R>(&self, f: impl FnOnce(&mut AxisState) -> R) -> R {
        self.axes
            .raw()
            .with_mut::<AxesState, _>(|axes| f(axes.axis_mut(self.name)))
    }

    /// Applies `f` to this axis and to the same axis of every axes
    /// sharing it (matplotlib shares the tickers between them).
    pub(super) fn update_shared(&self, f: impl FnMut(&mut AxisState)) {
        self.axes
            .raw()
            .figure_state()
            .borrow_mut()
            .update_shared_axis(self.axes.id(), self.name, f);
    }

    /// Places the major ticks with `locator`.
    pub fn set_major_locator(&self, locator: impl Locator) -> &Self {
        self.set_shared_major_locator(&(Rc::new(locator) as Rc<dyn Locator>))
    }

    /// Places the major ticks with a locator that may be shared.
    pub(crate) fn set_shared_major_locator(&self, locator: &Rc<dyn Locator>) -> &Self {
        self.update_shared(|axis| axis.set_major_locator(Rc::clone(locator)));
        self
    }

    /// Places the minor ticks with `locator`.
    pub fn set_minor_locator(&self, locator: impl Locator) -> &Self {
        self.set_shared_minor_locator(&(Rc::new(locator) as Rc<dyn Locator>))
    }

    /// Places the minor ticks with a locator that may be shared.
    pub(crate) fn set_shared_minor_locator(&self, locator: &Rc<dyn Locator>) -> &Self {
        self.update_shared(|axis| axis.minor_locator = Rc::clone(locator));
        self
    }

    /// Labels the major ticks with `formatter`.
    pub fn set_major_formatter(&self, formatter: impl Formatter) -> &Self {
        self.set_shared_major_formatter(&(Rc::new(formatter) as Rc<dyn Formatter>))
    }

    /// Labels the major ticks with a formatter that may be shared.
    pub(crate) fn set_shared_major_formatter(&self, formatter: &Rc<dyn Formatter>) -> &Self {
        self.update_shared(|axis| axis.set_major_formatter(Rc::clone(formatter)));
        self
    }

    /// Labels the minor ticks with `formatter`.
    pub fn set_minor_formatter(&self, formatter: impl Formatter) -> &Self {
        let formatter: Rc<dyn Formatter> = Rc::new(formatter);
        self.update_shared(|axis| axis.minor_formatter = Rc::clone(&formatter));
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
        self.include_ticks_in_view(&ticks);
        let locator: Rc<dyn Locator> = Rc::new(FixedLocator::new(ticks));
        self.update_shared(|axis| axis.set_major_locator(Rc::clone(&locator)));
        self
    }

    /// Widens the view of this axis and its siblings to include `ticks`
    /// (matplotlib's `_set_tick_locations`).
    pub(super) fn include_ticks_in_view(&self, ticks: &[f64]) {
        if ticks.is_empty() {
            return;
        }
        let lowest = ticks.iter().copied().fold(f64::INFINITY, f64::min);
        let highest = ticks.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        self.axes.unstale_view();
        let figure = self.axes.raw().figure_state();
        let mut state = figure.borrow_mut();
        for id in state.siblings(self.name, self.axes.id()) {
            if let Some(axes) = state.store.get_as_mut::<AxesState>(id) {
                axes.include_in_view(self.name, lowest, highest);
            }
        }
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
        let formatter =
            self.update(|axis| tick_label_formatter(axis.major_locator.as_ref(), labels))?;
        self.update_shared(|axis| axis.set_major_formatter(Rc::clone(&formatter)));
        Ok(self)
    }

    /// Shows (`visible`) or hides the gridlines of `options.which`,
    /// restyled by `options` (matplotlib's `Axis.grid`; `options.axis` is
    /// ignored here).
    pub fn grid(&self, visible: bool, options: &GridOptions) -> &Self {
        let rc = crate::rc::params();
        self.update(|axis| {
            for (major, on) in [
                (true, options.which.major()),
                (false, options.which.minor()),
            ] {
                if !on {
                    continue;
                }
                let style = if major {
                    &mut axis.major_style
                } else {
                    &mut axis.minor_style
                };
                let current = style.grid.take();
                style.grid = visible.then(|| {
                    options.restyle(current.unwrap_or_else(|| GridStyle::from_rc(major, &rc)))
                });
            }
        });
        self
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

    /// Makes the axis run from high to low values or back, keeping its
    /// range (matplotlib's `Axis.set_inverted`).
    pub fn set_inverted(&self, inverted: bool) -> &Self {
        let (low, high) = match self.name {
            AxisName::X => self.axes.xlim(),
            AxisName::Y => self.axes.ylim(),
        };
        let (low, high) = if low <= high {
            (low, high)
        } else {
            (high, low)
        };
        let limits = if inverted { (high, low) } else { (low, high) };
        self.axes.raw().figure_state().borrow_mut().set_axis_limits(
            self.axes.id(),
            self.name,
            limits,
            None,
            true,
        );
        self
    }

    /// Shows or hides the whole axis: ticks, tick labels, label and offset
    /// text (matplotlib's `Axis.set_visible`).
    pub fn set_visible(&self, visible: bool) -> &Self {
        self.update(|axis| axis.visible = visible);
        self
    }

    /// Whether the axis is drawn.
    pub fn visible(&self) -> bool {
        self.axes
            .raw()
            .with::<AxesState, _>(|axes| axes.axis(self.name).visible)
    }

    /// Fails unless `side` is one this axis can be drawn on.
    fn check_side(&self, side: SpineSide) -> Result<(), PlotError> {
        if side.axis() == self.name {
            Ok(())
        } else {
            Err(PlotError::WrongSide {
                axis: self.name,
                side,
            })
        }
    }

    /// Puts the axis label on `side` (matplotlib's `set_label_position`).
    ///
    /// # Errors
    ///
    /// When `side` is not a side of this axis (e.g. `Left` for x).
    pub fn set_label_position(&self, side: SpineSide) -> Result<&Self, PlotError> {
        self.check_side(side)?;
        self.update(|axis| axis.label.side = side);
        Ok(self)
    }

    /// Colours the axis label (matplotlib's `set_xlabel(..., color=...)`).
    pub fn set_label_color(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.update(|axis| axis.label.color = color);
        self
    }

    /// Places the axis label along the axis (matplotlib's `loc` of
    /// `set_xlabel`).
    pub fn set_label_loc(&self, loc: LabelLoc) -> &Self {
        self.update(|axis| axis.label.loc = loc);
        self
    }

    /// Sets the gap between the tick labels and the axis label in points
    /// (matplotlib's `labelpad`).
    pub fn set_label_pad(&self, pad: f64) -> &Self {
        self.update(|axis| axis.label.pad = pad);
        self
    }

    /// Sets the font size of the axis label.
    pub fn set_label_fontsize(&self, size: FontSize) -> &Self {
        let base = crate::rc::params().font_size;
        self.update(|axis| axis.label.font = axis.label.font.clone().with_size(size, base));
        self
    }

    /// The side the axis label sits on.
    pub fn label_position(&self) -> SpineSide {
        self.axes
            .raw()
            .with::<AxesState, _>(|axes| axes.axis(self.name).label.side)
    }

    /// Chooses which sides get ticks and tick labels (matplotlib's
    /// `set_ticks_position`).
    ///
    /// # Errors
    ///
    /// When a [`TickPosition::Side`] is not a side of this axis.
    pub fn set_ticks_position(&self, position: TickPosition) -> Result<&Self, PlotError> {
        let both = Sides {
            first: true,
            second: true,
        };
        match position {
            TickPosition::Side(side) => {
                self.check_side(side)?;
                self.update(|axis| axis.ticks_to_side(side.is_high()));
            }
            TickPosition::Both => self.update(|axis| axis.restyle_ticks(|s| s.ticks = both)),
            TickPosition::None => {
                self.update(|axis| axis.restyle_ticks(|s| s.ticks = Sides::default()));
            }
            TickPosition::Default => self.update(|axis| {
                axis.restyle_ticks(|s| {
                    s.ticks = both;
                    s.labels = Sides {
                        first: true,
                        second: false,
                    };
                });
            }),
        }
        Ok(self)
    }

    /// Moves the ticks, and the tick labels unless both were hidden, to
    /// `side` (matplotlib's `tick_top`, `tick_bottom`, `tick_left` and
    /// `tick_right`).
    ///
    /// # Errors
    ///
    /// When `side` is not a side of this axis.
    pub fn tick_to(&self, side: SpineSide) -> Result<&Self, PlotError> {
        self.check_side(side)?;
        self.update(|axis| axis.tick_to(side.is_high()));
        Ok(self)
    }
}

/// The formatter showing `labels` at the ticks `locator` places.
fn tick_label_formatter(
    locator: &dyn Locator,
    labels: Vec<String>,
) -> Result<Rc<dyn Formatter>, PlotError> {
    if labels.is_empty() {
        return Ok(Rc::new(NullFormatter));
    }
    let any: &dyn Any = locator;
    let Some(fixed) = any.downcast_ref::<FixedLocator>() else {
        return Ok(Rc::new(FixedFormatter::new(labels)));
    };
    if fixed.locs().len() != labels.len() {
        return Err(PlotError::TickLabelCount {
            ticks: fixed.locs().len(),
            labels: labels.len(),
        });
    }
    let table: Vec<(f64, String)> = fixed.locs().iter().copied().zip(labels).collect();
    Ok(Rc::new(FuncFormatter::new(move |value, _| {
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

    /// Shows or hides the major gridlines of both axes (matplotlib's
    /// `grid`).
    pub fn grid(&self, visible: bool) -> &Self {
        self.grid_with(visible, &GridOptions::default())
    }

    /// Whether the major (`major`) or minor gridlines of one axis are
    /// shown.
    pub fn has_grid(&self, axis: AxisName, major: bool) -> bool {
        self.raw.with::<AxesState, _>(|state| {
            let axis = state.axis(axis);
            let style = if major {
                &axis.major_style
            } else {
                &axis.minor_style
            };
            style.grid.is_some()
        })
    }

    /// Shows or hides gridlines of the axes and tick levels in `options`,
    /// restyled by `options`.
    pub fn grid_with(&self, visible: bool, options: &GridOptions) -> &Self {
        for name in [AxisName::X, AxisName::Y] {
            if options.axis.is_none_or(|only| only == name) {
                self.axis_of(name).grid(visible, options);
            }
        }
        self
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
