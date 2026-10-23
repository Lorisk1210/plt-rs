//! Handles to the x and y axis of an axes (matplotlib's `XAxis` and
//! `YAxis`): tick helpers, tick positions and labels, unit conversion.

use std::any::Any;
use std::rc::Rc;

use super::axis::AxisState;
use super::axis_label::LabelLoc;
use super::axis_layout::{AxisPainter, AxisTexts, PlacedText};
use super::ticks::{GridOptions, Sides, TickPosition, TickStyle};
use super::{Axes, AxesState, PlotError};
use crate::artist::{ArtistHandle, LayoutContext};
use crate::colors::Color;
use crate::fonts::{FontProperties, FontSize, FontStyle, FontWeight};
use crate::spines::SpineSide;
use crate::text::{TextBox, TextBoxProps, VerticalAlignment};
use crate::ticker::{
    AxisName, FixedFormatter, FixedLocator, Formatter, FuncFormatter, Locator, NullFormatter,
};
use crate::transforms::{Bbox, Point};
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

    /// Changes the styles of this axis's tick labels; the layout engine
    /// runs again before positions are next used, as their extents may
    /// have changed (matplotlib lays out at every draw).
    pub(super) fn update_labels(&self, f: impl FnOnce(&mut AxisState)) {
        self.update(f);
        self.axes.raw().figure_state().borrow_mut().layout.pending = true;
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
        let owner = self.axes.id();
        self.update_shared(|axis| {
            axis.set_major_locator(Rc::clone(locator));
            axis.ticker_owner = Some(owner);
        });
        self
    }

    /// Places the minor ticks with `locator`.
    pub fn set_minor_locator(&self, locator: impl Locator) -> &Self {
        self.set_shared_minor_locator(&(Rc::new(locator) as Rc<dyn Locator>))
    }

    /// Places the minor ticks with a locator that may be shared.
    pub(crate) fn set_shared_minor_locator(&self, locator: &Rc<dyn Locator>) -> &Self {
        let owner = self.axes.id();
        self.update_shared(|axis| {
            axis.minor_locator = Rc::clone(locator);
            axis.ticker_owner = Some(owner);
        });
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
        self.set_shared_major_locator(&locator)
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

    /// Labels the minor ticks with `labels`, like
    /// [`set_ticklabels`](Self::set_ticklabels) does the major ones
    /// (matplotlib's `Axis.set_ticklabels(..., minor=True)`).
    ///
    /// # Errors
    ///
    /// When the minor ticks were set explicitly and their number differs
    /// from the number of labels.
    pub fn set_minor_ticklabels<S: Into<String>>(
        &self,
        labels: impl IntoIterator<Item = S>,
    ) -> Result<&Self, PlotError> {
        let labels: Vec<String> = labels.into_iter().map(Into::into).collect();
        let formatter =
            self.read(|axis| tick_label_formatter(axis.minor_locator.as_ref(), labels))?;
        self.update_shared(|axis| axis.minor_formatter = Rc::clone(&formatter));
        Ok(self)
    }

    /// Whether minor ticks falling on major ticks are dropped (the
    /// default) or drawn too (matplotlib's `Axis.remove_overlapping_locs`).
    pub fn set_remove_overlapping_locs(&self, remove: bool) -> &Self {
        self.update(|axis| axis.remove_overlapping_locs = remove);
        self
    }

    /// Shows (`visible`) or hides the gridlines of `options.which`,
    /// restyled by `options` (matplotlib's `Axis.grid`; `options.axis` is
    /// ignored here).
    pub fn grid(&self, visible: bool, options: &GridOptions) -> &Self {
        self.update(|axis| {
            for (style, on) in [
                (&mut axis.major_style, options.which.major()),
                (&mut axis.minor_style, options.which.minor()),
            ] {
                if on {
                    style.grid_on = visible;
                    options.restyle(&mut style.grid);
                }
            }
        });
        self
    }

    /// Every major (or minor) tick location of the current view, inside
    /// it or not; minor locations on major ticks are left out unless
    /// overlapping locations are kept (matplotlib's `Axis.get_ticklocs`).
    ///
    /// # Errors
    ///
    /// When the locator fails.
    pub fn ticklocs(&self, minor: bool) -> Result<Vec<f64>, String> {
        Ok(self.axes.ticks(self.name)?.unclipped_locs(minor))
    }

    /// The labels of the [`ticklocs`](Self::ticklocs): those on the first
    /// side (bottom or left), then those on the second, for the sides that
    /// show labels (matplotlib's `Axis.get_ticklabels`).
    ///
    /// # Errors
    ///
    /// When the locator or the formatter fails.
    pub fn ticklabels(&self, minor: bool) -> Result<Vec<String>, String> {
        let ticks = self.axes.ticks(self.name)?;
        let sides = self.read(|axis| {
            if minor {
                axis.minor_style.labels
            } else {
                axis.major_style.labels
            }
        });
        Ok(ticks.unclipped_labels(minor, sides))
    }

    /// The appearance of the major (or minor) ticks: sides with marks and
    /// labels, sizes, colours and gridlines (matplotlib's
    /// `get_tick_params`).
    pub fn tick_style(&self, minor: bool) -> TickStyle {
        self.read(|axis| {
            if minor {
                axis.minor_style.clone()
            } else {
                axis.major_style.clone()
            }
        })
    }

    /// The display boxes of the shown major tick labels with text in the
    /// figure's current layout (laid out first): those on the first side
    /// (bottom or left), then those on the second, in tick order
    /// (matplotlib's `[label.get_window_extent() for label in
    /// axis.get_ticklabels()]`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.set_yticks([1.0, 2.0]).set_yticklabels(["short", "a longer label"])?;
    /// let boxes = ax.yaxis().ticklabel_extents();
    /// assert_eq!(boxes.len(), 2);
    /// assert!(boxes[1].width() > boxes[0].width());
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    pub fn ticklabel_extents(&self) -> Vec<Bbox> {
        self.with_texts(|texts, axis| {
            let majors = axis.ticks.as_ref().map_or(0, |ticks| ticks.major.len());
            let on_side = |first_side: bool| {
                (texts.tick_labels.iter())
                    .filter(move |label| label.tick < majors && label.first_side == first_side)
                    .map(|label| label.text.bbox())
            };
            on_side(true).chain(on_side(false)).collect()
        })
        .unwrap_or_default()
    }

    /// Runs `f` on this axis and its texts laid out in the figure's
    /// current layout (laid out first).
    fn with_texts<R>(&self, f: impl FnOnce(&AxisTexts, &AxisState) -> R) -> Option<R> {
        let mut state = self.axes.raw().figure_state().borrow_mut();
        state.prepare();
        let state = &*state;
        let axes = state.store.get_as::<AxesState>(self.axes.id())?;
        let texts = AxisPainter::new(self.axes.id(), axes, self.name)
            .texts(LayoutContext::new(state), &axes.rc);
        Some(f(&texts, axes.axis(self.name)))
    }

    /// Whether the offset (or scale) text is shown (matplotlib's
    /// `offsetText.get_visible`); [`Axes::label_outer`] hides it on inner
    /// edges.
    pub fn offset_text_visible(&self) -> bool {
        self.read(|axis| axis.offset_visible)
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

    /// Puts the axis label on `side` (matplotlib's `set_label_position`),
    /// with the vertical alignment of that side.
    ///
    /// # Errors
    ///
    /// When `side` is not a side of this axis (e.g. `Left` for x).
    pub fn set_label_position(&self, side: SpineSide) -> Result<&Self, PlotError> {
        self.check_side(side)?;
        self.update(|axis| axis.label.move_to(side));
        Ok(self)
    }

    /// Rotates the axis label by `degrees` counter-clockwise, reduced to
    /// `[0, 360)` (matplotlib's `set_ylabel(..., rotation=...)`);
    /// unrotated, x labels are horizontal and y labels vertical.
    pub fn set_label_rotation(&self, degrees: f64) -> &Self {
        self.update(|axis| axis.label.rotation = Some(degrees.rem_euclid(360.0)));
        self
    }

    /// Aligns the axis label vertically on its anchor (matplotlib's
    /// `set_ylabel(..., va=...)`) until the label moves to another side.
    /// By default x labels hang from their anchor below the axes (`Top`)
    /// and sit on it above (`Baseline`); y labels sit on it left of the
    /// axes (`Bottom`) and hang from it on the right (`Top`), in their
    /// rotated frame.
    pub fn set_label_valign(&self, valign: VerticalAlignment) -> &Self {
        self.update(|axis| axis.label.valign = Some(valign));
        self
    }

    /// Colours the tick lines of both levels without their marks' edges
    /// (matplotlib's `set_color` on the `Line2D`s of the ticks, which
    /// `Artist.findobj` reaches): the marks are open paths whose edge
    /// keeps its colour, so only the face, seen in vector output, changes.
    pub fn set_tick_line_color(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.update(|axis| {
            axis.restyle_ticks(|style| {
                style.edgecolor.get_or_insert_with(|| style.color.clone());
                style.color = color.clone();
            });
        });
        self
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

    /// Where along the axis its label sits.
    pub fn label_loc(&self) -> LabelLoc {
        self.read(|axis| axis.label.loc)
    }

    /// Sets the gap between the tick labels and the axis label in points
    /// (matplotlib's `labelpad`).
    pub fn set_label_pad(&self, pad: f64) -> &Self {
        self.update(|axis| axis.label.pad = pad);
        self
    }

    /// Anchors the axis label at `(x, y)` in axes fractions instead of
    /// beyond the tick labels, keeping its alignment (matplotlib's
    /// `set_label_coords`).
    ///
    /// ```
    /// use plt::prelude::*;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// ax.set_ylabel("depth");
    /// ax.yaxis().set_label_coords(-0.3, 0.5);
    /// let label = ax.yaxis().label_extent().expect("a label");
    /// let (x0, _, width, _) = ax.position().bounds();
    /// let anchor = (x0 - 0.3 * width) * fig.size_pixels().0;
    /// assert!((label.x1() - anchor).abs() < 1e-9);
    /// ```
    pub fn set_label_coords(&self, x: f64, y: f64) -> &Self {
        self.update_labels(|axis| axis.label.coords = Some(Point::new(x, y)));
        self
    }

    /// Draws a box behind the axis label, or none (matplotlib's
    /// `set_xlabel(..., bbox=...)`); its padding in font sizes is fixed
    /// for the label's current font size, as for any text.
    pub fn set_label_bbox(&self, props: Option<TextBoxProps>) -> &Self {
        self.axes.raw().with_mut::<AxesState, _>(|axes| {
            let rc = Rc::clone(&axes.rc);
            let label = &mut axes.axis_mut(self.name).label;
            label.bbox = props.map(|props| TextBox::new(props, label.font.size, &rc));
        });
        self
    }

    /// The display box of the axis label in the figure's current layout
    /// (laid out first), if it is shown (matplotlib's
    /// `axis.label.get_window_extent()`).
    pub fn label_extent(&self) -> Option<Bbox> {
        self.with_texts(|texts, _| texts.label.as_ref().map(PlacedText::bbox))
            .flatten()
    }

    /// Sets the font size of the axis label.
    pub fn set_label_fontsize(&self, size: FontSize) -> &Self {
        let base = crate::rc::params().font_size;
        self.update(|axis| axis.label.font = axis.label.font.clone().with_size(size, base));
        self
    }

    /// Sets the font weight of the axis label (matplotlib's
    /// `set_xlabel(..., fontweight=...)`).
    pub fn set_label_weight(&self, weight: FontWeight) -> &Self {
        self.update(|axis| axis.label.font.weight = weight);
        self
    }

    /// Sets the font style of the axis label (matplotlib's
    /// `set_xlabel(..., fontstyle=...)`).
    pub fn set_label_style(&self, style: FontStyle) -> &Self {
        self.update(|axis| axis.label.font.style = style);
        self
    }

    /// Sets every font property of the axis label at once, its size
    /// included (matplotlib's `set_xlabel(..., fontproperties=...)`).
    pub fn set_label_font(&self, font: FontProperties) -> &Self {
        self.update(|axis| axis.label.font = font);
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
pub(crate) fn tick_label_formatter(
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
            style.grid_on
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

#[cfg(test)]
mod tests {
    use crate::artist::ArtistHandle;
    use crate::axes::AxesState;
    use crate::figure::Figure;
    use crate::fonts::{FontProperties, FontSize, FontStyle, FontWeight};

    #[test]
    fn label_weight_changes_only_that_axis_label() {
        let ax = Figure::new().subplot();
        ax.yaxis().set_label_weight(FontWeight::BOLD);
        let weights = ax.raw().with::<AxesState, _>(|axes| {
            (axes.xaxis.label.font.weight, axes.yaxis.label.font.weight)
        });
        assert_eq!(weights, (FontWeight::NORMAL, FontWeight::BOLD));
    }

    #[test]
    fn label_font_replaces_every_property_of_that_axis_label() {
        let ax = Figure::new().subplot();
        let font = FontProperties {
            families: vec!["Times New Roman".into()],
            style: FontStyle::Italic,
            ..FontProperties::from_rc(&crate::rc::params())
        };
        ax.xaxis().set_label_fontsize(FontSize::Points(20.0));
        ax.yaxis().set_label_font(font.clone());
        let fonts = ax.raw().with::<AxesState, _>(|axes| {
            (axes.xaxis.label.font.clone(), axes.yaxis.label.font.clone())
        });
        assert_eq!(fonts.0.size, 20.0);
        assert_eq!(fonts.1, font);
    }

    #[test]
    fn label_rotations_are_reduced_to_a_turn() {
        let ax = Figure::new().subplot();
        let rotation = || {
            ax.raw()
                .with::<AxesState, _>(|axes| axes.yaxis.label.rotation)
        };
        assert_eq!(rotation(), None);
        ax.yaxis().set_label_rotation(-90.0);
        assert_eq!(rotation(), Some(270.0));
        ax.yaxis().set_label_rotation(450.0);
        assert_eq!(rotation(), Some(90.0));
    }
}
