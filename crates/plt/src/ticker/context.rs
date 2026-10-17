//! What locators and formatters may know about the axis they serve.
//!
//! matplotlib's tick helpers hold a back-pointer to their `Axis` and query
//! it for limits, the minimum positive datum, the available tick space, ...
//! plt-rs passes that information explicitly as an [`AxisInfo`] inside a
//! [`TickContext`], together with the [`TickerSettings`] in effect.

use super::settings::TickerSettings;

/// Which axis of an axes a tick helper serves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AxisName {
    /// The horizontal axis.
    #[default]
    X,
    /// The vertical axis.
    Y,
}

impl AxisName {
    /// matplotlib's name for the axis (`"x"` or `"y"`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Y => "y",
        }
    }
}

/// The kind of scale an axis uses, as far as tick helpers care.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ScaleHint {
    /// A linear axis.
    #[default]
    Linear,
    /// A logarithmic axis (`AutoMinorLocator` refuses to place ticks).
    Log,
    /// A symmetric log axis; log formatters only count decades beyond
    /// `linthresh`.
    SymLog {
        /// Extent of the linear region around zero.
        linthresh: f64,
    },
    /// Any other scale.
    Other,
}

/// The axis state a locator or formatter consults (matplotlib's `Axis`
/// as seen through `_DummyAxis`).
#[derive(Clone, Debug, PartialEq)]
pub struct AxisInfo {
    /// Which axis this is.
    pub name: AxisName,
    /// Visible range `(vmin, vmax)`; may be decreasing.
    pub view_interval: (f64, f64),
    /// Range spanned by the data.
    pub data_interval: (f64, f64),
    /// Smallest positive data value (used by log and logit locators).
    pub minpos: f64,
    /// Number of ticks that fit along the axis (`Axis.get_tick_space`).
    pub tick_space: usize,
    /// Current major tick locations (consulted by `AutoMinorLocator`).
    pub major_ticks: Vec<f64>,
    /// The axis' scale.
    pub scale: ScaleHint,
    /// Whether the axis belongs to a 3-D axes, where `MaxNLocator` takes
    /// slightly smaller steps (matplotlib 3.9's automargin compensation).
    pub three_d: bool,
}

impl AxisInfo {
    /// matplotlib's `_DummyAxis()`: unit view and data intervals,
    /// `minpos = 0` and room for 9 ticks.
    pub const DUMMY: Self = Self {
        name: AxisName::X,
        view_interval: (0.0, 1.0),
        data_interval: (0.0, 1.0),
        minpos: 0.0,
        tick_space: 9,
        major_ticks: Vec::new(),
        scale: ScaleHint::Linear,
        three_d: false,
    };

    /// A dummy axis whose view *and* data intervals are `(vmin, vmax)`.
    pub fn with_view(vmin: f64, vmax: f64) -> Self {
        Self {
            view_interval: (vmin, vmax),
            data_interval: (vmin, vmax),
            ..Self::DUMMY
        }
    }
}

impl Default for AxisInfo {
    fn default() -> Self {
        Self::DUMMY
    }
}

static DEFAULT_SETTINGS: TickerSettings = TickerSettings::DEFAULT;
static DUMMY_AXIS: AxisInfo = AxisInfo::DUMMY;

/// Everything a locator or formatter may consult besides its own
/// parameters: the axis (if the helper is attached to one) and the
/// rcParams in effect.
#[derive(Clone, Copy, Debug)]
pub struct TickContext<'a> {
    axis: Option<&'a AxisInfo>,
    settings: &'a TickerSettings,
}

impl<'a> TickContext<'a> {
    /// A context for a helper attached to `axis`.
    pub fn new(axis: &'a AxisInfo, settings: &'a TickerSettings) -> Self {
        Self {
            axis: Some(axis),
            settings,
        }
    }

    /// A context for a helper used without an axis (matplotlib's
    /// `locator.tick_values(...)` on a fresh locator).
    pub fn detached(settings: &'a TickerSettings) -> Self {
        Self {
            axis: None,
            settings,
        }
    }

    /// The attached axis, if any.
    pub fn axis(&self) -> Option<&'a AxisInfo> {
        self.axis
    }

    /// The attached axis, or [`AxisInfo::DUMMY`] (what matplotlib's
    /// `create_dummy_axis` provides).
    pub fn axis_or_dummy(&self) -> &'a AxisInfo {
        self.axis.unwrap_or(&DUMMY_AXIS)
    }

    /// The view interval of the axis (dummy: `(0, 1)`).
    pub fn view_interval(&self) -> (f64, f64) {
        self.axis_or_dummy().view_interval
    }

    /// The rcParams in effect.
    pub fn settings(&self) -> &'a TickerSettings {
        self.settings
    }
}

impl Default for TickContext<'static> {
    /// No axis and matplotlib's default rcParams.
    fn default() -> Self {
        Self::detached(&DEFAULT_SETTINGS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detached_context_falls_back_to_dummy_axis() {
        let ctx = TickContext::default();
        assert!(ctx.axis().is_none());
        assert_eq!(ctx.view_interval(), (0.0, 1.0));
        assert_eq!(ctx.axis_or_dummy().tick_space, 9);
        let axis = AxisInfo::with_view(3.0, -1.0);
        let settings = TickerSettings::default();
        let ctx = TickContext::new(&axis, &settings);
        assert_eq!(ctx.view_interval(), (3.0, -1.0));
        assert_eq!(ctx.axis_or_dummy().data_interval, (3.0, -1.0));
    }
}
