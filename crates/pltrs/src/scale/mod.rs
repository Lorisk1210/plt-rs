//! Axis scales (matplotlib's `scale`): linear, log, symlog, logit, asinh
//! and user functions.
//!
//! A [`Scale`] maps data values to the scaled coordinates an axis is drawn
//! in (via an [`AxisMap`] that plugs into the axes' data transform),
//! restricts autoscaled limits to its domain, and supplies the default
//! tick locators and formatters for an axis using it.
//!
//! ```
//! use pltrs::scale::{LogScale, Scale};
//! let log = LogScale::new(10.0).unwrap();
//! assert_eq!(log.forward(&[0.001, 10.0, 0.0]), [-3.0, 1.0, -1000.0]);
//! assert_eq!(log.limit_range_for_scale(-5.0, 10.0, 0.5), (0.5, 10.0));
//! ```
//!
//! Scales can also be created by name, a [`ScaleName`] or its string, like
//! `set_xscale("log", base=2)`:
//!
//! ```
//! use pltrs::scale::{ScaleName, ScaleOptions, scale_factory};
//! let scale = scale_factory("log", ScaleOptions { base: Some(2.0), ..ScaleOptions::default() })
//!     .unwrap();
//! assert_eq!(scale.name(), ScaleName::Log);
//! ```
#![expect(
    clippy::float_cmp,
    reason = "matplotlib's scale transforms select code paths by exact float comparisons"
)]

mod asinh;
mod linear;
mod log;
mod logit;
mod name;
mod registry;
mod symlog;
mod transforms;

use std::any::Any;
use std::fmt;
use std::rc::Rc;

pub use crate::ticker::LogBase;
pub use asinh::AsinhScale;
pub use linear::{FuncScale, LinearScale};
pub use log::{FuncScaleLog, LogScale};
pub use logit::LogitScale;
pub use name::ScaleName;
pub use registry::{ScaleConstructor, ScaleOptions, ScaleRegistry, scale_factory, scale_names};
pub use symlog::SymmetricalLogScale;
pub(crate) use transforms::ScaleFunction;
pub use transforms::{
    AsinhTransform, FuncLogTransform, FuncTransform, LogTransform, LogitTransform,
    SymmetricalLogTransform,
};

use crate::ticker::{
    AutoLocator, AutoMinorLocator, AxisName, Formatter, Locator, NullFormatter, NullLocator,
    ScalarFormatter, ScaleHint, TickerSettings,
};
use crate::transforms::AxisMap;

/// What a log or logit scale does with values outside its domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NonPositive {
    /// Map them to a very large negative (or positive) scaled value.
    Clip,
    /// Map them to NaN/infinity so they are not drawn.
    Mask,
}

pltrs_core::archive_enum!(NonPositive { Clip, Mask });

/// The tick helpers a scale installs on an axis
/// (matplotlib's `set_default_locators_and_formatters`).
#[derive(Debug)]
pub struct AxisTickers {
    /// Major tick locator.
    pub major_locator: Box<dyn Locator>,
    /// Minor tick locator.
    pub minor_locator: Box<dyn Locator>,
    /// Major tick formatter.
    pub major_formatter: Box<dyn Formatter>,
    /// Minor tick formatter.
    pub minor_formatter: Box<dyn Formatter>,
}

/// A data-to-scaled-coordinate mapping for one axis (matplotlib's
/// `ScaleBase`).
pub trait Scale: fmt::Debug + Any {
    /// The name the scale goes by (matplotlib's `name`), e.g.
    /// [`ScaleName::Log`].
    fn name(&self) -> ScaleName;

    /// The map from data to scaled coordinates, or `None` for the
    /// identity (linear) map.
    fn axis_map(&self) -> Option<Rc<dyn AxisMap>>;

    /// Restricts `[vmin, vmax]` to the scale's domain; `minpos` is the
    /// smallest positive data value. Used when autoscaling.
    fn limit_range_for_scale(&self, vmin: f64, vmax: f64, _minpos: f64) -> (f64, f64) {
        (vmin, vmax)
    }

    /// Whether `value` can be shown on this scale (finite and in the
    /// domain).
    fn val_in_range(&self, value: f64) -> bool {
        value.is_finite()
    }

    /// How tick helpers should treat an axis with this scale.
    fn hint(&self) -> ScaleHint {
        ScaleHint::Other
    }

    /// The default locators and formatters for `axis` under `settings`.
    fn default_tickers(&self, axis: AxisName, settings: &TickerSettings) -> AxisTickers;

    /// Whether the scale runs code of the program's that observes its
    /// calls (see [`AxisMap::calls_program`]): by default, whether its
    /// map does.
    fn calls_program(&self) -> bool {
        self.axis_map().is_some_and(|map| map.calls_program())
    }

    /// Applies the forward map to every value.
    fn forward(&self, values: &[f64]) -> Vec<f64> {
        match self.axis_map() {
            Some(map) => values.iter().map(|&v| map.forward(v)).collect(),
            None => values.to_vec(),
        }
    }

    /// Applies the inverse map to every value.
    fn inverse(&self, values: &[f64]) -> Vec<f64> {
        match self.axis_map() {
            Some(map) => values.iter().map(|&v| map.inverse(v)).collect(),
            None => values.to_vec(),
        }
    }
}

/// Why a scale could not be created.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScaleError {
    /// A parameter is outside its valid domain.
    InvalidParameter {
        /// The parameter's name.
        name: &'static str,
        /// What is wrong with it.
        reason: &'static str,
    },
    /// No scale is registered under this name.
    UnknownScale(String),
    /// The scale does not accept this option.
    UnexpectedOption {
        /// The scale's name.
        scale: String,
        /// The option.
        option: &'static str,
    },
    /// A required option is missing.
    MissingOption {
        /// The scale's name.
        scale: String,
        /// The option.
        option: &'static str,
    },
}

impl ScaleError {
    pub(crate) fn invalid(name: &'static str, reason: &'static str) -> Self {
        Self::InvalidParameter { name, reason }
    }
}

impl fmt::Display for ScaleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParameter { name, reason } => write!(f, "invalid {name}: {reason}"),
            Self::UnknownScale(name) => write!(f, "{name:?} is not a registered scale"),
            Self::UnexpectedOption { scale, option } => {
                write!(f, "the {scale:?} scale does not accept {option:?}")
            }
            Self::MissingOption { scale, option } => {
                write!(f, "the {scale:?} scale requires {option:?}")
            }
        }
    }
}

impl std::error::Error for ScaleError {}

/// The tick helpers of linear-looking scales (linear and function
/// scales): `AutoLocator`, `ScalarFormatter`, and minor ticks only when
/// `[xy]tick.minor.visible` is set.
fn linear_tickers(axis: AxisName, settings: &TickerSettings) -> AxisTickers {
    let minor_visible = match axis {
        AxisName::X => settings.xtick_minor_visible,
        AxisName::Y => settings.ytick_minor_visible,
    };
    AxisTickers {
        major_locator: Box::new(AutoLocator::from_settings(settings)),
        minor_locator: if minor_visible {
            Box::new(AutoMinorLocator::new(None))
        } else {
            Box::new(NullLocator)
        },
        major_formatter: Box::new(ScalarFormatter::from_settings(settings)),
        minor_formatter: Box::new(NullFormatter),
    }
}

pltrs_core::archive_kinds!(dyn Scale {
    "linear" => LinearScale,
    "func" => FuncScale,
    "log" => LogScale,
    "func-log" => FuncScaleLog,
    "symlog" => SymmetricalLogScale,
    "logit" => LogitScale,
    "asinh" => AsinhScale,
});
