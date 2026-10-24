//! The aspect ratio of an axes (matplotlib's `set_aspect`,
//! `set_adjustable`, `set_anchor`, `set_box_aspect` and `apply_aspect`).
//!
//! A fixed aspect is honoured during layout, before ticks are placed: with
//! [`Adjustable::Box`] the axes shrinks inside its original position and
//! is placed by its [`Anchor`]; with [`Adjustable::Datalim`] the view
//! limits of one axis grow instead.

use std::fmt;
use std::rc::Rc;

use plt_core::{Anchor, Bbox};

use super::{Axes, AxesState, ProjectionKind};
use crate::artist::{Artist, ArtistHandle, ArtistId, DrawError};
use crate::axes_grid1::{DividerCell, InsetLocatorId};
use crate::figure::FigureState;
use crate::rc::Aspect;
use crate::ticker::AxisName;
use crate::transforms::TransformContext;

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

/// Refuses [`Adjustable::Datalim`] for axes of a curved projection, whose
/// data ratio is fixed.
fn check_adjustable(axes: &AxesState, adjustable: Adjustable) -> Result<(), AspectError> {
    if adjustable == Adjustable::Datalim && !axes.projection.is_rectilinear() {
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

/// Places an axes inside part of its original position before the aspect
/// is applied, like the axes locator matplotlib gives colorbars to make
/// room for their extensions (`_ColorbarAxesLocator`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AxesLocator {
    /// The factors the original width and height are scaled by.
    pub(crate) shrink: (f64, f64),
    /// The shift of the scaled box, in fractions of the original width
    /// and height.
    pub(crate) offset: (f64, f64),
    /// The box aspect to use instead of the axes' own.
    pub(crate) box_aspect: Option<f64>,
}

/// A function placing an axes at layout time (matplotlib's axes locator,
/// a callable of the axes and the renderer): given the position the axes
/// was given, in fractions of its figure, it returns the position to
/// place the axes at before its aspect is applied. It runs inside the
/// layout pass, so it must not call methods of the figure's handles.
/// Clones share the function.
#[derive(Clone)]
pub struct AxesLocatorFn(Rc<dyn Fn(Bbox) -> Bbox>);

impl AxesLocatorFn {
    /// A locator calling `locate`.
    pub fn new(locate: impl Fn(Bbox) -> Bbox + 'static) -> Self {
        Self(Rc::new(locate))
    }

    /// Where the locator places an axes given `original`.
    pub fn locate(&self, original: Bbox) -> Bbox {
        (self.0)(original)
    }
}

impl fmt::Debug for AxesLocatorFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AxesLocatorFn").finish_non_exhaustive()
    }
}

impl PartialEq for AxesLocatorFn {
    /// Locators are equal when they share the function.
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl AxesLocator {
    /// The part of `original` used, computed exactly as matplotlib's
    /// `pos.shrunk(sx, sy).translated(ox * pos.width, oy * pos.height)`.
    fn locate(&self, original: &Bbox) -> Bbox {
        let (x0, y0, width, height) = original.bounds();
        let (tx, ty) = (self.offset.0 * width, self.offset.1 * height);
        Bbox::from_extents(
            x0 + tx,
            y0 + ty,
            x0 + self.shrink.0 * width + tx,
            y0 + self.shrink.1 * height + ty,
        )
    }
}

impl AspectState {
    /// The position the axes is drawn at: the one computed by the last
    /// [`FigureState::apply_aspect`] for the current `original` position,
    /// or `original` itself.
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
    /// use plt::prelude::*;
    /// use plt::rc::Aspect;
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
            if let Some(axes) = state.store.get_as::<AxesState>(id) {
                check_adjustable(axes, adjustable)?;
            }
        }
        for id in group {
            if let Some(axes) = state.store.get_as_mut::<AxesState>(id) {
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
    /// and geographic axes, whose data ratio is fixed.
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

    /// Places the axes with `locator` at every layout, in place of its
    /// original position, before its aspect is applied; `None` removes it
    /// (matplotlib's `set_axes_locator`). An axes has one locator: this
    /// replaces the placement by a divider, an inset locator or a
    /// colorbar.
    ///
    /// ```
    /// use plt::axes::AxesLocatorFn;
    /// use plt::prelude::*;
    /// use plt::transforms::Bbox;
    /// let ax = Figure::new().add_axes([0.1, 0.1, 0.8, 0.8])?;
    /// ax.set_axes_locator(Some(AxesLocatorFn::new(|original: Bbox| {
    ///     let (x0, y0, width, height) = original.bounds();
    ///     Bbox::from_bounds(x0, y0, width / 2.0, height)
    /// })));
    /// let (_, _, width, _) = ax.position().bounds();
    /// assert!((width - 0.4).abs() < 1e-12);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_axes_locator(&self, locator: Option<AxesLocatorFn>) -> &Self {
        self.raw.with_mut::<AxesState, _>(|axes| {
            axes.axes_locator = locator;
            axes.aspect.divider = None;
            axes.aspect.inset = None;
            axes.aspect.locator = None;
        });
        self.raw.figure_state().borrow_mut().layout.pending = true;
        self
    }

    /// The locator set with [`set_axes_locator`](Self::set_axes_locator),
    /// if any (matplotlib's `get_axes_locator`).
    pub fn axes_locator(&self) -> Option<AxesLocatorFn> {
        self.raw
            .with::<AxesState, _>(|axes| axes.axes_locator.clone())
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

impl AxesState {
    /// The `(lower, upper)` view bound of one axis in scaled coordinates.
    fn scaled_bound(&self, name: AxisName) -> (f64, f64) {
        let (v0, v1) = self.view_interval(name);
        let axis = self.axis(name);
        (axis.forward(v0.min(v1)), axis.forward(v0.max(v1)))
    }

    /// matplotlib's `get_data_ratio` (always 1 for polar and geographic
    /// axes).
    pub(crate) fn data_ratio(&self) -> f64 {
        if !self.projection.is_rectilinear() {
            return 1.0;
        }
        let (x0, x1) = self.scaled_bound(AxisName::X);
        let (y0, y1) = self.scaled_bound(AxisName::Y);
        let xsize = (x1 - x0).abs().max(1e-30);
        let ysize = (y1 - y0).abs().max(1e-30);
        ysize / xsize
    }
}

/// Where an axes is placed before its aspect is applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Placement {
    /// Where its divider, inset locator or axes locator puts it.
    Located,
    /// At its original position, without calling its locators
    /// (matplotlib's `get_tightbbox(call_axes_locator=False)`, which
    /// sizes measuring the axes for its own divider use).
    Original,
}

impl FigureState {
    /// Brings one axes up to date for layout: a due layout-engine pass,
    /// pending autoscaling, then the aspect.
    pub(crate) fn lay_out_axes(&mut self, axes: ArtistId) {
        self.lay_out_axes_at(axes, Placement::Located);
    }

    /// [`lay_out_axes`](Self::lay_out_axes) with the axes placed by
    /// `placement`.
    pub(crate) fn lay_out_axes_at(&mut self, axes: ArtistId, placement: Placement) {
        self.run_pending_layout();
        self.unstale_axes(axes);
        self.apply_aspect_at(axes, placement);
    }

    /// matplotlib's `apply_aspect`: sets the active position of `axes`
    /// and, for [`Adjustable::Datalim`], widens one view interval.
    pub(crate) fn apply_aspect(&mut self, axes: ArtistId) {
        self.apply_aspect_at(axes, Placement::Located);
    }

    /// [`apply_aspect`](Self::apply_aspect) with the axes placed by
    /// `placement`.
    fn apply_aspect_at(&mut self, axes: ArtistId, placement: Placement) {
        let Some(state) = self.store.get_as::<AxesState>(axes) else {
            return;
        };
        let original = state.position;
        let settings = state.aspect;
        let custom = state.axes_locator.clone();
        let located = match (placement, custom) {
            (Placement::Located, Some(locator)) => locator.locate(original),
            (Placement::Located, None) => {
                let divided = settings
                    .divider
                    .and_then(|cell| self.locate_in_divider(cell))
                    .or_else(|| settings.inset.and_then(|id| self.locate_inset(axes, id)))
                    .unwrap_or(original);
                settings
                    .locator
                    .map_or(divided, |locator| locator.locate(&divided))
            }
            (Placement::Original, _) => original,
        };
        let Some(state) = self.store.get_as::<AxesState>(axes) else {
            return;
        };
        let box_aspect = settings
            .locator
            .and_then(|locator| locator.box_aspect)
            .or(settings.box_aspect);
        let data_ratio = state.data_ratio();
        // Insets and secondary axes are placed in their host axes.
        let frame = state
            .base()
            .parent
            .and_then(|f| self.frame_bbox(f).or_else(|| self.axes_bbox(f)));
        let active = if settings.ratio.is_none() && box_aspect.is_none() {
            Some(located)
        } else if let Some(frame) = frame {
            let fig_aspect = frame.height() / frame.width();
            let shrunk = |aspect: f64| {
                located
                    .shrunk_to_aspect(aspect, Some(&located), fig_aspect)
                    .map(|b| b.anchored(settings.anchor(), &located))
            };
            match (settings.adjustable, box_aspect) {
                (Adjustable::Box, _) => shrunk(settings.ratio.unwrap_or(1.0) * data_ratio),
                (Adjustable::Datalim, Some(box_aspect)) => shrunk(box_aspect),
                (Adjustable::Datalim, None) => Some(located),
            }
            .inspect(|position| {
                if let (Adjustable::Datalim, Some(ratio)) = (settings.adjustable, settings.ratio) {
                    let box_aspect = fig_aspect * (position.height() / position.width());
                    self.adjust_datalim(axes, ratio, box_aspect);
                }
            })
        } else {
            None
        };
        if let (Some(active), Some(state)) = (active, self.store.get_as_mut::<AxesState>(axes)) {
            state.aspect.active = Some((original, active));
        }
    }

    /// The `adjustable='datalim'` half of `apply_aspect`: widens the x or
    /// the y view interval about its centre to reach `box_aspect / ratio`.
    fn adjust_datalim(&mut self, axes: ArtistId, ratio: f64, box_aspect: f64) {
        let Some(state) = self.store.get_as::<AxesState>(axes) else {
            return;
        };
        let (xmin, xmax) = state.scaled_bound(AxisName::X);
        let (ymin, ymax) = state.scaled_bound(AxisName::Y);
        let xsize = (xmax - xmin).abs().max(1e-30);
        let ysize = (ymax - ymin).abs().max(1e-30);
        let data_ratio = box_aspect / ratio;
        let y_expander = data_ratio * xsize / ysize - 1.0;
        if y_expander.abs() < 0.005 {
            return;
        }
        let limits = state.data_lim.bbox;
        let (x0, x1) = (
            state.xaxis.forward(limits.x0()),
            state.xaxis.forward(limits.x1()),
        );
        let (y0, y1) = (
            state.yaxis.forward(limits.y0()),
            state.yaxis.forward(limits.y1()),
        );
        let xr = 1.05 * (x1 - x0);
        let yr = 1.05 * (y1 - y0);
        let xmarg = xsize - xr;
        let ymarg = ysize - yr;
        let ysize_target = data_ratio * xsize;
        let xsize_target = ysize / data_ratio;
        let shared_x = self.siblings(AxisName::X, axes).len() > 1;
        let shared_y = self.siblings(AxisName::Y, axes).len() > 1;
        if shared_x && shared_y {
            self.pending_errors.push(DrawError {
                artist: axes,
                message: "set_aspect(..., adjustable='datalim') is not allowed when both axes \
                          are shared; use adjustable='box'"
                    .to_owned(),
            });
            return;
        }
        let adjust_y = !shared_y && {
            let adjy = if xmarg > 0.0 && ymarg > 0.0 {
                (ysize_target - yr > 0.0 && y_expander < 0.0)
                    || (xsize_target - xr < 0.0 && y_expander > 0.0)
            } else {
                y_expander > 0.0
            };
            shared_x || adjy
        };
        let (name, centre, size) = if adjust_y {
            (AxisName::Y, f64::midpoint(ymin, ymax), ysize_target)
        } else {
            (AxisName::X, f64::midpoint(xmin, xmax), xsize_target)
        };
        let axis = state.axis(name);
        let inverse = |v: f64| axis.map().map_or(v, |m| m.inverse(v));
        let (low, high) = (inverse(centre - size / 2.0), inverse(centre + size / 2.0));
        let (v0, v1) = state.view_interval(name);
        let bound = if v0 > v1 { (high, low) } else { (low, high) };
        self.set_axis_limits(axes, name, bound, None, true);
    }
}

#[cfg(test)]
mod tests;
