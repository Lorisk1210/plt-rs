//! Axis modes (matplotlib's `Axes.axis(...)` with a string): switching
//! the decorations off and on, and the aspect/limit presets.

use super::aspect::check_adjustable;
use super::{Adjustable, AspectError, Axes, AxesState, AxisName};
use crate::rc::Aspect;
use crate::transforms::Anchor;

/// A preset of [`Axes::axis_mode`] (matplotlib's `axis('equal')` and
/// friends).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AxisMode {
    /// Show the patch, spines and axes (`'on'`).
    On,
    /// Hide the patch, spines and axes (`'off'`).
    Off,
    /// Equal scaling by changing the data limits (`'equal'`).
    Equal,
    /// Equal scaling by shrinking the axes box (`'scaled'`).
    Scaled,
    /// Limits just large enough for the data (`'tight'`).
    Tight,
    /// Automatic scaling and aspect (`'auto'`).
    Auto,
    /// `'scaled'` with tight limits (`'image'`).
    Image,
    /// A square box with equal x and y ranges (`'square'`).
    Square,
}

impl Axes {
    /// Applies an axis preset (matplotlib's `Axes.axis(mode)`).
    ///
    /// ```
    /// use pltrs::axes::{AspectError, AxisMode};
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// fig.subplot().axis_mode(AxisMode::Equal)?;
    /// let polar = fig.polar_subplot();
    /// assert!(matches!(
    ///     polar.axis_mode(AxisMode::Equal),
    ///     Err(AspectError::FixedDataRatio(_))
    /// ));
    /// # Ok::<(), AspectError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`AxisMode::Equal`] on axes of a curved projection (polar, the map
    /// projections): it equalises the scales by changing the data limits,
    /// which those axes cannot do since their data ratio is fixed
    /// (matplotlib refuses their `adjustable="datalim"`). The axes are
    /// left unchanged.
    pub fn axis_mode(&self, mode: AxisMode) -> Result<&Self, AspectError> {
        match mode {
            AxisMode::On => return Ok(self.set_axis_on()),
            AxisMode::Off => return Ok(self.set_axis_off()),
            AxisMode::Equal => {
                self.raw
                    .with::<AxesState, _>(|axes| check_adjustable(axes, Adjustable::Datalim))?;
            }
            _ => {}
        }
        self.set_autoscale_on(true);
        self.set_aspect(Aspect::Auto)
            .expect("an automatic aspect is always valid");
        self.autoscale_view(Some(false));
        let equal_box = || {
            self.set_equal_aspect(Adjustable::Box);
            self.set_anchor(Anchor::C);
        };
        match mode {
            AxisMode::Equal => self.set_equal_aspect(Adjustable::Datalim),
            AxisMode::Scaled => {
                equal_box();
                self.set_autoscale_on(false);
            }
            AxisMode::Tight => {
                self.autoscale_view(Some(true));
                self.set_autoscale_on(false);
            }
            AxisMode::Image => {
                self.autoscale_view(Some(true));
                self.set_autoscale_on(false);
                equal_box();
            }
            AxisMode::Square => {
                equal_box();
                self.set_autoscale_on(false);
                let (x0, x1) = self.xlim();
                let (y0, y1) = self.ylim();
                let edge = (x1 - x0).max(y1 - y0);
                // A view too wide for a square of finite limits stays,
                // where matplotlib raises.
                let _ = self
                    .set_xlim(x0, x0 + edge)
                    .and_then(|ax| ax.set_ylim(y0, y0 + edge));
            }
            AxisMode::On | AxisMode::Off | AxisMode::Auto => {}
        }
        Ok(self)
    }

    /// Autoscales both axes now rather than at the next layout
    /// (matplotlib's `autoscale_view`); `tight` replaces the tight
    /// setting when given.
    pub fn autoscale_view(&self, tight: Option<bool>) -> &Self {
        self.autoscale_view_along(tight, (true, true))
    }

    /// [`Axes::autoscale_view`] along the axes `(scalex, scaley)` asks for
    /// (matplotlib's `autoscale_view(tight, scalex, scaley)`); with neither
    /// axis asked only `tight` changes. An autoscale still waiting runs
    /// first, its limit callbacks with it, when an axis asked follows the
    /// data: matplotlib's `autoscale_view` reads the bounds of the axis it
    /// sets, which brings a stale view up to date, and then sets them
    /// again, calling the callbacks a second time.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.plot([0.0, 10.0], [0.0, 5.0])?;
    /// ax.set_xlim(0.0, 1.0)?.set_ylim(0.0, 1.0)?;
    /// ax.set_autoscale_on(true);
    /// ax.autoscale_view_along(None, (false, true));
    /// assert_eq!(ax.xlim(), (0.0, 1.0));
    /// assert_eq!(ax.ylim(), (-0.25, 5.25));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn autoscale_view_along(
        &self,
        tight: Option<bool>,
        (scalex, scaley): (bool, bool),
    ) -> &Self {
        let sets_bounds = [(scalex, AxisName::X), (scaley, AxisName::Y)]
            .into_iter()
            .any(|(scale, name)| scale && self.autoscale_on(Some(name)));
        if sets_bounds {
            self.unstale_view();
        }
        self.raw.with_mut::<AxesState, _>(|axes| {
            if let Some(tight) = tight {
                axes.tight = tight;
            }
            axes.request_autoscale(scalex, scaley);
        });
        if scalex || scaley {
            self.unstale_view();
        }
        self
    }

    /// Whether autoscaling leaves the view limits where the margins put
    /// them, unrounded by `axes.autolimit_mode` (matplotlib's `_tight`),
    /// as [`Axes::autoscale_view`] and the other autoscaling calls last
    /// set it; `false` at first.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// assert!(!ax.autoscales_tight());
    /// ax.autoscale_view_along(Some(true), (false, false));
    /// assert!(ax.autoscales_tight());
    /// ```
    pub fn autoscales_tight(&self) -> bool {
        self.raw.with::<AxesState, _>(|axes| axes.tight)
    }

    /// An equal aspect honoured by `adjustable`, which the axes take
    /// ([`Axes::axis_mode`] checks the data limits are adjustable).
    fn set_equal_aspect(&self, adjustable: Adjustable) {
        self.set_aspect(Aspect::Equal)
            .expect("an equal aspect is always valid");
        self.store_adjustable(adjustable);
    }
}
