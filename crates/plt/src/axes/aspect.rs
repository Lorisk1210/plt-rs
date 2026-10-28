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

use plt_core::{Anchor, Bbox};

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

/// An aspect setting an axes refuses.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AspectError {
    /// The ratio is zero, negative or not finite.
    NotPositive(f64),
    /// Axes of a curved projection keep a data ratio of 1, so they cannot
    /// adjust their data limits to an aspect (matplotlib refuses
    /// `datalim` for axes overriding `get_data_ratio`).
    FixedDataRatio(ProjectionKind),
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
    if adjustable == Adjustable::Datalim && axes.projection.fixed_data_ratio().is_some() {
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
}

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
    /// use plt::axes::Aspect;
    /// use plt::prelude::*;
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
    /// use plt::axes::{Adjustable, AspectError, ProjectionKind};
    /// use plt::prelude::*;
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
    /// ratio switches their adjustable to [`Adjustable::Datalim`].
    ///
    /// # Errors
    ///
    /// When the ratio is not finite and positive.
    pub fn set_box_aspect(&self, ratio: Option<f64>) -> Result<&Self, AspectError> {
        if let Some(ratio) = ratio {
            ratio_of(Aspect::Ratio(ratio))?;
        }
        for twin in self.twins() {
            twin.raw.with_mut::<AxesState, _>(|axes| {
                if ratio.is_some() {
                    axes.aspect.adjustable = Adjustable::Datalim;
                }
                axes.aspect.box_aspect = ratio;
            });
        }
        Ok(self)
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
    /// than at the next layout (matplotlib's `apply_aspect`).
    pub fn apply_aspect(&self) -> &Self {
        self.raw.figure_state().borrow_mut().lay_out_axes(self.id());
        self
    }
}

#[cfg(test)]
mod tests;
