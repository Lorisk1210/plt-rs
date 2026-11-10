//! The aspect ratio of an axes (matplotlib's `set_aspect`,
//! `set_adjustable`, `set_anchor`, `set_box_aspect` and `apply_aspect`).
//!
//! A fixed aspect is honoured during layout, before ticks are placed: with
//! [`Adjustable::Box`] the axes shrinks inside its original position and
//! is placed by its [`Anchor`]; with [`Adjustable::Datalim`] the view
//! limits of one axis grow instead. The aspect is applied in `apply`;
//! axes locators, which place the axes before that, are in `locator`.

mod apply;
mod locator;

pub(crate) use apply::Placement;
pub(crate) use locator::AxesLocator;
pub use locator::AxesLocatorFn;

use std::fmt;

use pltrs_core::{Anchor, Bbox};

use super::{Axes, AxesState, ProjectionKind};
use crate::artist::{ArtistHandle, ArtistLookup};
use crate::axes_grid1::{DividerCell, InsetLocatorId};
use crate::rc::Aspect;
use crate::ticker::AxisName;

/// What an axes changes to honour a fixed aspect ratio (matplotlib's
/// `adjustable`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Adjustable {
    /// Shrink the axes box inside its original position.
    #[default]
    Box,
    /// Widen the data limits of one axis.
    Datalim,
}

pltrs_core::archive_enum!(Adjustable { Box, Datalim });

/// An aspect setting an axes refuses.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AspectError {
    /// The ratio is zero, negative or not finite.
    NotPositive(f64),
    /// Axes of a curved projection keep a data ratio of 1, so they cannot
    /// adjust their data limits to an aspect (matplotlib refuses
    /// `datalim` for axes overriding `get_data_ratio`).
    FixedDataRatio(ProjectionKind),
    /// Adjusting the data limits to the aspect would make them NaN or
    /// infinite: a scale cannot map the view limits (a logit axis
    /// beyond `(0, 1)`), so the data ratio is not finite (matplotlib's
    /// `set_xbound` refuses the limits).
    NonFiniteLimits,
    /// The box cannot be shrunk to the aspect: it or the figure's is not
    /// positive (matplotlib's `Bbox.shrunk_to_aspect` refuses it).
    BoxNotPositive,
}

impl fmt::Display for AspectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotPositive(value) => {
                write!(f, "aspect must be finite and positive, got {value}")
            }
            Self::FixedDataRatio(projection) => write!(
                f,
                "cannot adjust the data limits of {} axes: their data ratio is fixed",
                projection.name()
            ),
            Self::NonFiniteLimits => f.write_str("Axis limits cannot be NaN or Inf"),
            Self::BoxNotPositive => f.write_str("'box_aspect' and 'fig_aspect' must be positive"),
        }
    }
}

impl std::error::Error for AspectError {}

/// Refuses [`Adjustable::Datalim`] for axes whose projection fixes their
/// data ratio (curved projections, and custom ones fixing it).
pub(super) fn check_adjustable(
    axes: &AxesState,
    adjustable: Adjustable,
) -> Result<(), AspectError> {
    if adjustable == Adjustable::Datalim && axes.projection.fixes_data_ratio() {
        return Err(AspectError::FixedDataRatio(axes.projection.kind()));
    }
    Ok(())
}

/// The aspect settings of one axes and the position they produced.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct AspectState {
    /// Height/width ratio of data units; `None` fills the position.
    ratio: Option<f64>,
    adjustable: Adjustable,
    anchor: Option<Anchor>,
    /// Height/width ratio of the axes box itself.
    box_aspect: Option<f64>,
    /// Places the axes in the cells of a divider instead of its original
    /// position (`axes_grid1`'s axes locators).
    pub(crate) divider: Option<DividerCell>,
    /// Anchors the axes in a box of its parent instead of its original
    /// position (`axes_grid1.inset_locator`).
    pub(crate) inset: Option<InsetLocatorId>,
    /// Narrows the (divided) original position before the aspect is
    /// applied.
    pub(crate) locator: Option<AxesLocator>,
    /// The active position and the original position it derives from.
    active: Option<(Bbox, Bbox)>,
    /// The active position of an axes placed in a host axes, in the
    /// fractions of the host's (sub)figure, where matplotlib computes it.
    in_figure: Option<InFigure>,
}

pltrs_core::archive_struct!(AspectState {
    ratio,
    adjustable,
    anchor,
    box_aspect,
    divider,
    inset,
    locator,
    active,
    in_figure
});

/// The active position of an axes placed in a host axes (an inset, a
/// secondary axes) in the fractions of the host's (sub)figure, with what
/// it was computed from: the axes' original position (in the host's
/// fractions) and the display boxes of the host and of the figure.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct InFigure {
    original: Bbox,
    host: Bbox,
    figure: Bbox,
    active: Bbox,
}

pltrs_core::archive_struct!(InFigure {
    original,
    host,
    figure,
    active
});

impl AspectState {
    /// The position the axes is drawn at: the one computed by the last
    /// [`FigureState::apply_aspect`](crate::figure::FigureState::apply_aspect)
    /// for the current `original` position, or `original` itself.
    pub(crate) fn active_position(&self, original: &Bbox) -> Bbox {
        match self.active {
            Some((from, active)) if from == *original => active,
            _ => *original,
        }
    }

    /// Draws the axes at `active` until the aspect is next applied to the
    /// `original` position.
    pub(crate) fn set_active(&mut self, original: Bbox, active: Bbox) {
        self.active = Some((original, active));
        self.in_figure = None;
    }

    /// Draws the axes, placed in its host, at `active` (in the fractions of
    /// the host's figure) while it keeps its `original` position and its
    /// host and figure stay at `host` and `figure`; `in_host` is the same
    /// position in the host's fractions.
    pub(crate) fn set_active_in_figure(
        &mut self,
        original: Bbox,
        (host, figure): (Bbox, Bbox),
        active: Bbox,
        in_host: Bbox,
    ) {
        self.active = Some((original, in_host));
        self.in_figure = Some(InFigure {
            original,
            host,
            figure,
            active,
        });
    }

    /// The active position of an axes placed in its host, in the fractions
    /// of the host's figure, if it was computed for the `original`
    /// position with the host and the figure at `host` and `figure`.
    pub(crate) fn active_in_figure(
        &self,
        original: &Bbox,
        host: &Bbox,
        figure: &Bbox,
    ) -> Option<Bbox> {
        self.in_figure
            .filter(|placed| {
                placed.original == *original && placed.host == *host && placed.figure == *figure
            })
            .map(|placed| placed.active)
    }

    fn anchor(&self) -> Anchor {
        self.anchor.unwrap_or(Anchor::C)
    }

    /// The fixed height/width ratio of data units, `None` for automatic.
    pub(crate) fn ratio(&self) -> Option<f64> {
        self.ratio
    }

    /// Anchors the axes and fixes the shape of its box with a free data
    /// aspect: `set_anchor`, `set_box_aspect` and `set_aspect('auto')` in
    /// one, as layout engines reshape colorbars.
    pub(crate) fn fix_box(&mut self, anchor: Anchor, box_aspect: f64) {
        self.anchor = Some(anchor);
        self.box_aspect = Some(box_aspect);
        self.adjustable = Adjustable::Datalim;
        self.ratio = None;
    }
}

/// Validates an aspect into a ratio (`None` for [`Aspect::Auto`]).
fn ratio_of(aspect: Aspect) -> Result<Option<f64>, AspectError> {
    let ratio = match aspect {
        Aspect::Auto => return Ok(None),
        Aspect::Equal => 1.0,
        Aspect::Ratio(ratio) => ratio,
    };
    if ratio > 0.0 && ratio.is_finite() {
        Ok(Some(ratio))
    } else {
        Err(AspectError::NotPositive(ratio))
    }
}

impl Axes {
    /// Fixes the ratio of y to x data units on screen (matplotlib's
    /// `set_aspect`): [`Aspect::Equal`] draws one unit of x as long as one
    /// unit of y, [`Aspect::Auto`] fills the axes position.
    ///
    /// ```
    /// use pltrs::axes::Aspect;
    /// use pltrs::prelude::*;
    /// let fig = Figure::with_size(6.4, 4.8, 100.0);
    /// let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8])?;
    /// ax.set_xlim(0.0, 2.0)?.set_ylim(0.0, 1.0)?;
    /// ax.set_aspect(Aspect::Equal)?;
    /// let (_, _, width, height) = ax.position().bounds();
    /// assert!((width * 6.4 - 2.0 * height * 4.8).abs() < 1e-9);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a numeric ratio is not finite and positive.
    pub fn set_aspect(&self, aspect: Aspect) -> Result<&Self, AspectError> {
        let ratio = ratio_of(aspect)?;
        self.raw
            .with_mut::<AxesState, _>(|axes| axes.aspect.ratio = ratio);
        Ok(self)
    }

    /// Sets the aspect and the adjustable of this axes and of every axes
    /// sharing its x or y axis (matplotlib's `set_aspect(...,
    /// adjustable=..., share=True)`).
    ///
    /// # Errors
    ///
    /// When a numeric ratio is not finite and positive, or for
    /// [`Adjustable::Datalim`] when an axes of the group has a fixed data
    /// ratio (see [`set_adjustable`](Self::set_adjustable)).
    pub fn set_shared_aspect(
        &self,
        aspect: Aspect,
        adjustable: Adjustable,
    ) -> Result<&Self, AspectError> {
        let ratio = ratio_of(aspect)?;
        let figure = self.raw.figure_state();
        let mut state = figure.borrow_mut();
        let mut group = state.siblings(AxisName::X, self.id());
        group.extend(state.siblings(AxisName::Y, self.id()));
        for &id in &group {
            if let Some(axes) = state.artist_as::<AxesState>(id) {
                check_adjustable(axes, adjustable)?;
            }
        }
        for id in group {
            if let Some(axes) = state.artist_as_mut::<AxesState>(id) {
                axes.aspect.ratio = ratio;
                axes.aspect.adjustable = adjustable;
            }
        }
        state.stale = true;
        Ok(self)
    }

    /// The aspect: [`Aspect::Auto`] or the fixed [`Aspect::Ratio`].
    pub fn aspect(&self) -> Aspect {
        self.raw
            .with::<AxesState, _>(|axes| axes.aspect.ratio.map_or(Aspect::Auto, Aspect::Ratio))
    }

    /// Chooses what changes to honour the aspect (matplotlib's
    /// `set_adjustable`).
    ///
    /// ```
    /// use pltrs::axes::{Adjustable, AspectError, ProjectionKind};
    /// use pltrs::prelude::*;
    /// let fig = Figure::new();
    /// fig.subplot().set_adjustable(Adjustable::Datalim)?;
    /// let polar = fig.polar_subplot();
    /// assert_eq!(
    ///     polar.set_adjustable(Adjustable::Datalim).err(),
    ///     Some(AspectError::FixedDataRatio(ProjectionKind::Polar))
    /// );
    /// # Ok::<(), AspectError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`AspectError::FixedDataRatio`] for [`Adjustable::Datalim`] on polar
    /// and geographic axes, and those of custom projections fixing their
    /// data ratio.
    pub fn set_adjustable(&self, adjustable: Adjustable) -> Result<&Self, AspectError> {
        self.raw
            .with::<AxesState, _>(|axes| check_adjustable(axes, adjustable))?;
        Ok(self.store_adjustable(adjustable))
    }

    /// Sets the adjustable unchecked, for axes known to take it.
    pub(crate) fn store_adjustable(&self, adjustable: Adjustable) -> &Self {
        self.raw
            .with_mut::<AxesState, _>(|axes| axes.aspect.adjustable = adjustable);
        self
    }

    /// What changes to honour the aspect.
    pub fn adjustable(&self) -> Adjustable {
        self.raw.with::<AxesState, _>(|axes| axes.aspect.adjustable)
    }

    /// Where a shrunken axes sits inside its original position
    /// (matplotlib's `set_anchor`).
    pub fn set_anchor(&self, anchor: Anchor) -> &Self {
        self.raw
            .with_mut::<AxesState, _>(|axes| axes.aspect.anchor = Some(anchor));
        self
    }

    /// The anchor of a shrunken axes ([`Anchor::C`] by default).
    pub fn anchor(&self) -> Anchor {
        self.raw.with::<AxesState, _>(|axes| axes.aspect.anchor())
    }

    /// Fixes the height/width ratio of the box of this axes and of its
    /// twins, independent of the data (matplotlib's `set_box_aspect`); a
    /// ratio switches their adjustable to [`Adjustable::Datalim`]. The
    /// ratio is kept as given: a ratio that is not positive fails when the
    /// aspect is applied ([`AspectError::BoxNotPositive`]), one that is NaN
    /// leaves the box no size.
    ///
    /// ```
    /// let ax = pltrs::figure::Figure::new().subplot();
    /// ax.set_box_aspect(Some(-1.0));
    /// assert_eq!(ax.box_aspect(), Some(-1.0));
    /// assert!(ax.apply_aspect().is_err());
    /// ```
    pub fn set_box_aspect(&self, ratio: Option<f64>) -> &Self {
        for twin in self.twins() {
            twin.raw.with_mut::<AxesState, _>(|axes| {
                if ratio.is_some() {
                    axes.aspect.adjustable = Adjustable::Datalim;
                }
                axes.aspect.box_aspect = ratio;
            });
        }
        self
    }

    /// The fixed height/width ratio of the axes box, if any.
    pub fn box_aspect(&self) -> Option<f64> {
        self.raw.with::<AxesState, _>(|axes| axes.aspect.box_aspect)
    }

    /// The position the axes was given, before any aspect adjustment
    /// (matplotlib's `get_position(original=True)`).
    pub fn original_position(&self) -> Bbox {
        self.raw.with::<AxesState, _>(|axes| axes.position)
    }

    /// Draws this axes and its twins at their original positions again,
    /// undoing the aspect adjustment until the aspect is next applied;
    /// like [`Axes::set_position`], this takes them out of the layout
    /// engine's control (matplotlib's `reset_position`).
    pub fn reset_position(&self) -> &Self {
        for twin in self.twins() {
            twin.set_in_layout(false);
            twin.raw.with_mut::<AxesState, _>(|axes| {
                axes.aspect.set_active(axes.position, axes.position);
            });
        }
        self
    }

    /// The ratio of the y to the x view span in scaled coordinates
    /// (matplotlib's `get_data_ratio`).
    pub fn data_ratio(&self) -> f64 {
        self.unstale_view();
        self.raw.with::<AxesState, _>(AxesState::data_ratio)
    }

    /// Adjusts the position or the view limits for the aspect now rather
    /// than at the next layout (matplotlib's `apply_aspect`). A pending
    /// layout-engine pass waits for the next layout, as in matplotlib,
    /// where layout engines run when the figure is drawn.
    ///
    /// ```
    /// use pltrs::axes::{Adjustable, Aspect, AspectError};
    /// use pltrs::prelude::*;
    /// use pltrs::scale::ScaleName;
    /// let ax = Figure::new().subplot();
    /// ax.set_adjustable(Adjustable::Datalim)?;
    /// ax.set_aspect(Aspect::Ratio(2.0))?;
    /// ax.set_ylim(-1.0, 1.0)?;
    /// ax.set_yscale(ScaleName::Logit)?;
    /// // The logit scale cannot map -1: no limits fit the aspect.
    /// assert_eq!(ax.apply_aspect().err(), Some(AspectError::NonFiniteLimits));
    /// assert_eq!(ax.ylim(), (-1.0, 1.0));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When adjusting the data limits would make them NaN or infinite
    /// (they are left as they are).
    pub fn apply_aspect(&self) -> Result<&Self, AspectError> {
        // Worked on apart from the figure, which a projection of the
        // program's asked for its data ratio finds free.
        let id = self.id();
        (self.raw.figure_state())
            .work_on(|state| state.apply_aspect_now(id, Placement::Located))?;
        Ok(self)
    }

    /// [`Axes::apply_aspect`] before the axes is measured with
    /// [`Axes::tight_bbox`], which brings a secondary axis to its host's
    /// limits itself: matplotlib's `get_tightbbox` applies the aspect once
    /// (and a secondary axis takes its host's limits, warning about
    /// identical ones, once).
    ///
    /// # Errors
    ///
    /// As [`Axes::apply_aspect`].
    pub fn apply_aspect_to_measure(&self) -> Result<&Self, AspectError> {
        let id = self.id();
        (self.raw.figure_state())
            .work_on(|state| state.apply_aspect_unsynced(id, Placement::Located))?;
        Ok(self)
    }

    /// [`Axes::apply_aspect`] with the axes placed at `position`, in
    /// figure fractions, instead of where its locators put it
    /// (matplotlib's `apply_aspect(position)`): the axes is drawn at
    /// `position` adjusted for the aspect until the aspect is applied
    /// again (as [`Axes::position`] and every layout do).
    ///
    /// # Errors
    ///
    /// As [`Axes::apply_aspect`].
    pub fn apply_aspect_at(&self, position: Bbox) -> Result<&Self, AspectError> {
        let id = self.id();
        let placement = Placement::Given(position);
        self.raw
            .figure_state()
            .work_on(|state| state.apply_aspect_now(id, placement))?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests;
