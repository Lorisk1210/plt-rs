//! The ticks of an axis: where they go, their labels and styles, which
//! sides show them and the gridlines at them (matplotlib's `Axis.set_ticks`,
//! `get_ticklocs`, `get_ticklabels`, `set_ticks_position`, `grid`, ...).

use std::any::Any;
use std::rc::Rc;

use super::axis::AxisState;
use super::axis_handle::Axis;
use super::axis_layout::{AxisPainter, AxisTexts};
use super::ticks::{GridOptions, Sides, TickPosition, TickStyle};
use super::{Axes, AxesState, PlotError, TickLabelProps};
use crate::artist::{ArtistHandle, LayoutContext};
use crate::figure::FigureState;
use crate::polar::RadialLocator;
use crate::spines::SpineSide;
use crate::ticker::{AxisName, FixedFormatter, FixedLocator, Formatter, FuncFormatter, Locator};
use crate::ticker::{NullFormatter, NullLocator, TickerError};
use crate::transforms::Bbox;
use crate::units::AxisValues;

impl Axis {
    /// Puts the major ticks exactly at `ticks`, widening the view to
    /// include them (matplotlib's `Axis.set_ticks`).
    pub fn set_ticks(&self, ticks: impl Into<AxisValues>) -> &Self {
        let ticks = self.convert_units(ticks);
        self.include_ticks_in_view(&ticks);
        let locator: Rc<dyn Locator> = Rc::new(FixedLocator::new(ticks));
        self.set_shared_major_locator(&locator)
    }

    /// Widens the view of this axis and its siblings to include `ticks`,
    /// running this axes' limit callbacks when its view changed
    /// (matplotlib's `_set_tick_locations`).
    pub(super) fn include_ticks_in_view(&self, ticks: &[f64]) {
        if ticks.is_empty() {
            return;
        }
        let lowest = ticks.iter().copied().fold(f64::INFINITY, f64::min);
        let highest = ticks.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        self.axes.unstale_view();
        let figure = self.axes.raw().figure_state();
        {
            let mut state = figure.borrow_mut();
            let (own, name) = (self.axes.id(), self.name);
            let view = |state: &mut FigureState| {
                state
                    .artist_as_mut::<AxesState>(own)
                    .map(|axes| axes.view_interval(name))
            };
            let before = view(&mut state);
            for id in state.siblings(name, own) {
                if let Some(axes) = state.artist_as_mut::<AxesState>(id) {
                    axes.include_in_view(name, lowest, highest);
                }
            }
            if view(&mut state) != before {
                state.limit_callbacks.notify(own, name);
            }
        }
        super::run_limit_callbacks(figure);
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
        self.update_shared(|axis| axis.set_minor_formatter(Rc::clone(&formatter)));
        Ok(self)
    }

    /// Whether minor ticks falling on major ticks are dropped (the
    /// default) or drawn too (matplotlib's `Axis.remove_overlapping_locs`).
    pub fn set_remove_overlapping_locs(&self, remove: bool) -> &Self {
        self.update(|axis| axis.remove_overlapping_locs = remove);
        self
    }

    /// Whether minor ticks falling on major ticks are dropped
    /// (matplotlib's `get_remove_overlapping_locs`).
    pub fn remove_overlapping_locs(&self) -> bool {
        self.read(|axis| axis.remove_overlapping_locs)
    }

    /// The text properties given to the labels of the major tick at
    /// `index` with
    /// [`set_major_ticklabel_props`](Self::set_major_ticklabel_props),
    /// unset fields following the major tick labels.
    pub fn major_ticklabel_props(&self, index: usize) -> TickLabelProps {
        self.tick_label_props(false, index)
    }

    /// Shows (`visible`) or hides the gridlines of `options.which`,
    /// restyled by `options` (matplotlib's `Axis.grid`; `options.axis` is
    /// ignored here).
    pub fn grid(&self, visible: bool, options: GridOptions) -> &Self {
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
    /// overlapping locations are kept (matplotlib's `Axis.get_ticklocs`,
    /// `get_majorticklocs` and `get_minorticklocs`).
    ///
    /// # Errors
    ///
    /// When the locator fails, as for [`Axes::ticks`].
    pub fn ticklocs(&self, minor: bool) -> Result<Vec<f64>, TickerError> {
        Ok(self.axes.ticks(self.name)?.unclipped_locs(minor))
    }

    /// The labels of the [`ticklocs`](Self::ticklocs): those on the first
    /// side (bottom or left), then those on the second, for the sides that
    /// show labels (matplotlib's `Axis.get_ticklabels`).
    ///
    /// # Errors
    ///
    /// When the locator or the formatter fails, as for [`Axes::ticks`].
    pub fn ticklabels(&self, minor: bool) -> Result<Vec<String>, TickerError> {
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

    /// The appearance of the major (or `minor`) tick at `index` among
    /// those the locator places: the style of its level with what that
    /// tick has of its own on top (matplotlib's `Tick` artists, changed one
    /// by one).
    ///
    /// ```
    /// use pltrs::axes::TickLabelProps;
    /// let ax = pltrs::figure::Figure::new().subplot();
    /// ax.set_xticks([0.0, 0.5, 1.0]);
    /// let red = TickLabelProps { color: Some("red".into()), ..TickLabelProps::default() };
    /// ax.xaxis().set_major_ticklabel_props(1, red);
    /// assert_eq!(ax.xaxis().tick_style_at(false, 1).label_color, "red".into());
    /// assert_eq!(ax.xaxis().tick_style_at(false, 0), ax.xaxis().tick_style(false));
    /// ```
    pub fn tick_style_at(&self, minor: bool, index: usize) -> TickStyle {
        self.read(|axis| axis.tick_style_at(minor, index).into_owned())
    }

    /// The display box of the label on the first (bottom or left) or the
    /// second side of the major (or `minor`) tick at `index` among those
    /// the locator places, in the figure's current layout (laid out
    /// first), if it is drawn (matplotlib's `tick.label1.get_window_extent()`).
    pub fn tick_label_extent(&self, minor: bool, index: usize, first_side: bool) -> Option<Bbox> {
        self.with_texts(|texts, axis| {
            let ticks = axis.ticks.as_ref().ok()?;
            let position = if minor {
                let in_view = ticks.minor_index.iter().position(|&i| i == index)?;
                ticks.major.len() + in_view
            } else {
                ticks.major_index.iter().position(|&i| i == index)?
            };
            (texts.tick_labels.iter())
                .find(|label| label.tick == position && label.first_side == first_side)
                .map(|label| label.text.bbox())
        })
        .flatten()
    }

    /// The appearance of the major (or minor) ticks: sides with marks and
    /// labels, sizes, colours, direction and gridlines (matplotlib's
    /// `get_tick_params`; `style.direction` is `get_ticks_direction`).
    pub fn tick_style(&self, minor: bool) -> TickStyle {
        self.read(|axis| {
            if minor {
                axis.minor_style.clone()
            } else {
                axis.major_style.clone()
            }
        })
    }

    /// How far the tick marks reach out of the axes, in points: the
    /// longer outward part of a major and a minor mark, whether or not
    /// the axis has ticks of either level (matplotlib's
    /// `Axis.get_tick_padding`, which measures the first tick of each
    /// level, made on demand).
    pub fn tick_padding(&self) -> f64 {
        self.read(|axis| {
            [&axis.major_style, &axis.minor_style]
                .into_iter()
                .map(|style| style.size * style.direction.outward_fraction())
                .fold(0.0, f64::max)
        })
    }

    /// The display boxes of the shown major tick labels with text in the
    /// figure's current layout (laid out first): those on the first side
    /// (bottom or left), then those on the second, in tick order
    /// (matplotlib's `[label.get_window_extent() for label in
    /// axis.get_ticklabels()]`).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.set_yticks([1.0, 2.0]).set_yticklabels(["short", "a longer label"])?;
    /// let boxes = ax.yaxis().ticklabel_extents();
    /// assert_eq!(boxes.len(), 2);
    /// assert!(boxes[1].width() > boxes[0].width());
    /// # Ok::<(), pltrs::axes::PlotError>(())
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
    pub(super) fn with_texts<R>(&self, f: impl FnOnce(&AxisTexts, &AxisState) -> R) -> Option<R> {
        let mut state = self.axes.raw().figure_state().borrow_mut();
        state.prepare();
        let state = &*state;
        let axes = state.store.get_as::<AxesState>(self.axes.id())?;
        let texts = AxisPainter::new(self.axes.id(), axes, self.name)
            .texts(LayoutContext::new(state), &axes.rc);
        Some(f(&texts, axes.axis(self.name)))
    }

    /// Fails unless `side` is one this axis can be drawn on.
    pub(super) fn check_side(&self, side: SpineSide) -> Result<(), PlotError> {
        if side.axis() == self.name {
            Ok(())
        } else {
            Err(PlotError::WrongSide {
                axis: self.name,
                side,
            })
        }
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

    /// Where the ticks and their labels are (matplotlib's
    /// `get_ticks_position`): marks and labels on one side only
    /// ([`TickPosition::Side`]), or marks on both sides labelled on the
    /// first ([`TickPosition::Default`]), for every tick level whose
    /// locator places ticks (not a `NullLocator`); `None` for anything
    /// else (matplotlib's `"unknown"`). Without such a level, the ticks
    /// count as on the first side, as in matplotlib.
    pub fn ticks_position(&self) -> Option<TickPosition> {
        let levels: Vec<Option<LevelPosition>> = self.read(|axis| {
            [
                (&axis.major_locator, &axis.major_style),
                (&axis.minor_locator, &axis.minor_style),
            ]
            .into_iter()
            .filter(|(locator, _)| !(locator.as_ref() as &dyn Any).is::<NullLocator>())
            .map(|(_, style)| level_position(style))
            .collect()
        });
        let position = [
            LevelPosition::Low,
            LevelPosition::High,
            LevelPosition::Default,
        ]
        .into_iter()
        .find(|&position| levels.iter().all(|&level| level == Some(position)))?;
        Some(match position {
            LevelPosition::Low => TickPosition::Side(SpineSide::of(self.name, false)),
            LevelPosition::High => TickPosition::Side(SpineSide::of(self.name, true)),
            LevelPosition::Default => TickPosition::Default,
        })
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

/// Where the marks and labels of one tick level are, among the positions
/// matplotlib's `_get_ticks_position` recognises.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LevelPosition {
    /// Marks and labels on the first side only.
    Low,
    /// Marks and labels on the second side only.
    High,
    /// Marks on both sides, labels on the first.
    Default,
}

/// The position of the marks and labels of `style`, if recognised.
fn level_position(style: &TickStyle) -> Option<LevelPosition> {
    let (ticks, labels) = (style.ticks, style.labels);
    match (ticks.first, ticks.second, labels.first, labels.second) {
        (true, false, true, false) => Some(LevelPosition::Low),
        (false, true, false, true) => Some(LevelPosition::High),
        (true, true, true, false) => Some(LevelPosition::Default),
        _ => None,
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
    // A radial axis wraps its locator; matplotlib looks through wrapping
    // locators to their `base`.
    let fixed = any.downcast_ref::<FixedLocator>().or_else(|| {
        let radial = any.downcast_ref::<RadialLocator>()?;
        (radial.base() as &dyn Any).downcast_ref::<FixedLocator>()
    });
    let Some(fixed) = fixed else {
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
    /// Shows or hides the major gridlines of both axes (matplotlib's
    /// `grid`).
    pub fn grid(&self, visible: bool) -> &Self {
        self.grid_with(visible, GridOptions::default())
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
    pub fn grid_with(&self, visible: bool, options: GridOptions) -> &Self {
        for name in [AxisName::X, AxisName::Y] {
            if options.axis.is_none_or(|only| only == name) {
                self.axis_of(name).grid(visible, options.clone());
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
    use super::*;
    use crate::axes::{TickDirection, TickLevel, TickParams};
    use crate::figure::Figure;

    #[test]
    fn tick_positions_read_back_as_set() {
        let ax = Figure::new().subplot();
        let (xaxis, yaxis) = (ax.xaxis(), ax.yaxis());
        let bottom = Some(TickPosition::Side(SpineSide::Bottom));
        assert_eq!(xaxis.ticks_position(), bottom);
        for (position, read) in [
            (
                TickPosition::Side(SpineSide::Top),
                Some(TickPosition::Side(SpineSide::Top)),
            ),
            (TickPosition::Default, Some(TickPosition::Default)),
            (TickPosition::None, None),
            (TickPosition::Both, Some(TickPosition::Default)),
        ] {
            xaxis.set_ticks_position(position).expect("an x side");
            assert_eq!(xaxis.ticks_position(), read, "{position:?}");
        }
        yaxis.tick_to(SpineSide::Right).expect("a y side");
        let right = Some(TickPosition::Side(SpineSide::Right));
        assert_eq!(yaxis.ticks_position(), right);
        ax.tick_params(TickParams {
            labelleft: Some(true),
            ..TickParams::default()
        });
        assert_eq!(yaxis.ticks_position(), None, "matplotlib's \"unknown\"");
    }

    #[test]
    fn levels_without_ticks_leave_the_position_alone() {
        // matplotlib's test_get_tick_position_tick_params: the major ticks
        // move, the minor ones place nothing.
        let ax = Figure::new().subplot();
        ax.tick_params(TickParams {
            top: Some(true),
            labeltop: Some(true),
            bottom: Some(false),
            labelbottom: Some(false),
            ..TickParams::default()
        });
        let top = Some(TickPosition::Side(SpineSide::Top));
        assert_eq!(ax.xaxis().ticks_position(), top);
        ax.xaxis().minorticks_on();
        assert_eq!(
            ax.xaxis().ticks_position(),
            None,
            "the minor ticks stay below"
        );
        ax.xaxis().minorticks_off().set_major_locator(NullLocator);
        let bottom = Some(TickPosition::Side(SpineSide::Bottom));
        assert_eq!(ax.xaxis().ticks_position(), bottom, "no ticks at all");
    }

    #[test]
    fn tick_padding_is_the_longer_outward_part() {
        // Values from matplotlib: major marks 3.5 points long, minor 2.
        let ax = Figure::new().subplot();
        let xaxis = ax.xaxis();
        assert_eq!(xaxis.tick_padding(), 3.5);
        ax.tick_params(TickParams {
            direction: Some(TickDirection::InOut),
            ..TickParams::default()
        });
        assert_eq!(xaxis.tick_padding(), 2.0, "the minor marks still point out");
        xaxis.set_ticks(Vec::<f64>::new());
        assert_eq!(xaxis.tick_padding(), 2.0, "with or without ticks");
        ax.tick_params(TickParams {
            which: TickLevel::Both,
            direction: Some(TickDirection::In),
            ..TickParams::default()
        });
        assert_eq!(xaxis.tick_padding(), 0.0);
    }

    #[test]
    fn per_tick_label_properties_read_back() {
        let ax = Figure::new().subplot();
        let tilted = TickLabelProps {
            rotation: Some(45.0),
            ..TickLabelProps::default()
        };
        ax.xaxis().set_major_ticklabel_props(1, tilted);
        assert_eq!(ax.xaxis().major_ticklabel_props(1).rotation, Some(45.0));
        assert_eq!(
            ax.xaxis().major_ticklabel_props(0),
            TickLabelProps::default()
        );
    }

    #[test]
    fn overlapping_locations_are_removed_by_default() {
        let ax = Figure::new().subplot();
        assert!(ax.xaxis().remove_overlapping_locs());
        ax.xaxis().set_remove_overlapping_locs(false);
        assert!(!ax.xaxis().remove_overlapping_locs());
        assert!(ax.yaxis().remove_overlapping_locs());
    }
}
