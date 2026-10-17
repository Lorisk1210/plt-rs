//! The aspect ratio of an axes (matplotlib's `set_aspect`,
//! `set_adjustable`, `set_anchor`, `set_box_aspect` and `apply_aspect`).
//!
//! A fixed aspect is honoured during layout, before ticks are placed: with
//! [`Adjustable::Box`] the axes shrinks inside its original position and
//! is placed by its [`Anchor`]; with [`Adjustable::Datalim`] the view
//! limits of one axis grow instead.

use std::fmt;

use plt_core::{Anchor, Bbox};

use super::{Axes, AxesState};
use crate::artist::{Artist, ArtistHandle, ArtistId, DrawError};
use crate::axes_grid1::DividerCell;
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

/// An invalid aspect ratio.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AspectError {
    /// The ratio is zero, negative or not finite.
    NotPositive(f64),
}

impl fmt::Display for AspectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotPositive(value) => {
                write!(f, "aspect must be finite and positive, got {value}")
            }
        }
    }
}

impl std::error::Error for AspectError {}

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
    /// The part of the original position used, in fractions of it.
    pub(crate) fraction: Bbox,
    /// The box aspect to use instead of the axes' own.
    pub(crate) box_aspect: Option<f64>,
}

impl AxesLocator {
    /// `fraction` of `original`, computed like matplotlib's
    /// `shrunk(...).translated(...)`.
    fn locate(&self, original: &Bbox) -> Bbox {
        let (x0, y0, width, height) = original.bounds();
        let (fx, fy, fw, fh) = self.fraction.bounds();
        Bbox::from_bounds(x0 + fx * width, y0 + fy * height, width * fw, height * fh)
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
    /// let fig = Figure::new();
    /// let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8]);
    /// ax.set_xlim(0.0, 2.0).set_ylim(0.0, 1.0);
    /// ax.set_aspect(Aspect::Equal)?;
    /// let (_, _, width, height) = ax.position().bounds();
    /// assert!((width * 6.4 - 2.0 * height * 4.8).abs() < 1e-9);
    /// # Ok::<(), plt::axes::AspectError>(())
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

    /// The aspect: [`Aspect::Auto`] or the fixed [`Aspect::Ratio`].
    pub fn aspect(&self) -> Aspect {
        self.raw
            .with::<AxesState, _>(|axes| axes.aspect.ratio.map_or(Aspect::Auto, Aspect::Ratio))
    }

    /// Chooses what changes to honour the aspect (matplotlib's
    /// `set_adjustable`).
    pub fn set_adjustable(&self, adjustable: Adjustable) -> &Self {
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

    /// Fixes the height/width ratio of the axes box itself, independent of
    /// the data (matplotlib's `set_box_aspect`); a ratio switches the
    /// adjustable to [`Adjustable::Datalim`].
    ///
    /// # Errors
    ///
    /// When the ratio is not finite and positive.
    pub fn set_box_aspect(&self, ratio: Option<f64>) -> Result<&Self, AspectError> {
        if let Some(ratio) = ratio {
            ratio_of(Aspect::Ratio(ratio))?;
        }
        self.raw.with_mut::<AxesState, _>(|axes| {
            if ratio.is_some() {
                axes.aspect.adjustable = Adjustable::Datalim;
            }
            axes.aspect.box_aspect = ratio;
        });
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

    /// matplotlib's `get_data_ratio` (always 1 for polar axes).
    pub(crate) fn data_ratio(&self) -> f64 {
        if self.polar().is_some() {
            return 1.0;
        }
        let (x0, x1) = self.scaled_bound(AxisName::X);
        let (y0, y1) = self.scaled_bound(AxisName::Y);
        let xsize = (x1 - x0).abs().max(1e-30);
        let ysize = (y1 - y0).abs().max(1e-30);
        ysize / xsize
    }
}

impl FigureState {
    /// Brings one axes up to date for layout: a due layout-engine pass,
    /// pending autoscaling, then the aspect.
    pub(crate) fn lay_out_axes(&mut self, axes: ArtistId) {
        self.run_pending_layout();
        self.unstale_axes(axes);
        self.apply_aspect(axes);
    }

    /// matplotlib's `apply_aspect`: sets the active position of `axes`
    /// and, for [`Adjustable::Datalim`], widens one view interval.
    pub(crate) fn apply_aspect(&mut self, axes: ArtistId) {
        let Some(state) = self.store.get_as::<AxesState>(axes) else {
            return;
        };
        let original = state.position;
        let settings = state.aspect;
        let divided = settings
            .divider
            .and_then(|cell| self.locate_in_divider(cell))
            .unwrap_or(original);
        let Some(state) = self.store.get_as::<AxesState>(axes) else {
            return;
        };
        let located = settings
            .locator
            .map_or(divided, |locator| locator.locate(&divided));
        let box_aspect = settings
            .locator
            .and_then(|locator| locator.box_aspect)
            .or(settings.box_aspect);
        let data_ratio = state.data_ratio();
        let frame = state.base().parent.and_then(|f| self.frame_bbox(f));
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
mod tests {
    use super::*;
    use crate::figure::Figure;

    fn axes_with_limits(x: (f64, f64), y: (f64, f64)) -> (Figure, Axes) {
        let fig = Figure::new();
        let ax = fig.add_axes([0.125, 0.11, 0.775, 0.77]);
        ax.set_xlim(x.0, x.1).set_ylim(y.0, y.1);
        (fig, ax)
    }

    #[test]
    fn box_adjustable_shrinks_and_centres_the_axes() {
        let (_fig, ax) = axes_with_limits((0.0, 10.0), (0.0, 10.0));
        ax.set_aspect(Aspect::Equal).unwrap();
        let (x, y, w, h) = ax.position().bounds();
        assert!((w * 640.0 - h * 480.0).abs() < 1e-9, "square on screen");
        assert!((h - 0.77).abs() < 1e-12);
        assert!((x + w / 2.0 - (0.125 + 0.775 / 2.0)).abs() < 1e-12);
        assert_eq!(y, 0.11);
        assert_eq!(ax.original_position().bounds(), (0.125, 0.11, 0.775, 0.77));
    }

    #[test]
    fn anchors_place_the_shrunken_box() {
        let (_fig, ax) = axes_with_limits((0.0, 10.0), (0.0, 10.0));
        ax.set_aspect(Aspect::Equal).unwrap().set_anchor(Anchor::W);
        assert_eq!(ax.position().x0(), 0.125);
        ax.set_anchor(Anchor::E);
        assert!((ax.position().x1() - 0.9).abs() < 1e-12);
    }

    #[test]
    fn datalim_widens_the_view() {
        let (_fig, ax) = axes_with_limits((0.0, 1.0), (0.0, 1.0));
        ax.set_aspect(Aspect::Equal)
            .unwrap()
            .set_adjustable(Adjustable::Datalim)
            .apply_aspect();
        // Values from matplotlib: without data, y shrinks about its centre.
        assert_eq!(ax.xlim(), (0.0, 1.0));
        assert_eq!(
            ax.ylim(),
            (0.127_419_354_838_709_7, 0.872_580_645_161_290_3)
        );
        assert_eq!(ax.position(), ax.original_position());
        let (_fig, ax) = axes_with_limits((0.0, 1.0), (0.0, 1.0));
        ax.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
        ax.set_autoscale_on(true);
        ax.set_aspect(Aspect::Equal)
            .unwrap()
            .set_adjustable(Adjustable::Datalim);
        let (x0, x1) = ax.apply_aspect().xlim();
        assert!((x0 + 0.238_095_238_095_238_25).abs() < 1e-12);
        assert!((x1 - 1.238_095_238_095_238_1).abs() < 1e-12);
        assert_eq!(ax.ylim(), (-0.05, 1.05));
    }

    #[test]
    fn box_aspect_fixes_the_box_shape() {
        let (_fig, ax) = axes_with_limits((0.0, 3.0), (0.0, 1.0));
        ax.set_box_aspect(Some(1.0)).unwrap();
        assert_eq!(ax.adjustable(), Adjustable::Datalim);
        let (_, _, w, h) = ax.position().bounds();
        assert!((w * 640.0 - h * 480.0).abs() < 1e-9);
        assert_eq!(ax.xlim(), (0.0, 3.0), "auto aspect leaves the limits");
    }

    #[test]
    fn auto_restores_the_original_position() {
        let (_fig, ax) = axes_with_limits((0.0, 10.0), (0.0, 1.0));
        ax.set_aspect(Aspect::Ratio(2.0)).unwrap();
        assert_ne!(ax.position(), ax.original_position());
        ax.set_aspect(Aspect::Auto).unwrap();
        assert_eq!(ax.position(), ax.original_position());
        assert_eq!(ax.aspect(), Aspect::Auto);
        ax.set_position([0.2, 0.2, 0.5, 0.5]);
        assert_eq!(ax.position(), Bbox::from_bounds(0.2, 0.2, 0.5, 0.5));
    }

    #[test]
    fn invalid_ratios_are_rejected() {
        let (_fig, ax) = axes_with_limits((0.0, 1.0), (0.0, 1.0));
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(ax.set_aspect(Aspect::Ratio(bad)).is_err(), "{bad}");
            assert!(ax.set_box_aspect(Some(bad)).is_err(), "{bad}");
        }
        assert_eq!(ax.aspect(), Aspect::Auto);
        assert!(
            AspectError::NotPositive(0.0)
                .to_string()
                .contains("positive")
        );
    }

    #[test]
    fn data_ratio_uses_scaled_bounds() {
        let (_fig, ax) = axes_with_limits((0.0, 4.0), (2.0, 0.0));
        assert_eq!(ax.data_ratio(), 0.5);
    }
}
