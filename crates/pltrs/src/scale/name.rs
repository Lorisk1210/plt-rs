//! The names of scales, as `set_xscale` takes them and `get_xscale`
//! returns them.

use std::convert::Infallible;
use std::fmt;
use std::str::FromStr;

/// The name of a scale: one of matplotlib's built-in scales, or a custom
/// one (matplotlib's scale names).
///
/// Names convert from strings, with [`From`] or [`str::parse`]: a
/// built-in name becomes its variant and any other name a
/// [`ScaleName::Custom`]. Creating a scale checks the name: by name, only
/// the scales a [`ScaleRegistry`](super::ScaleRegistry) holds can be made,
/// so a setter given an unknown name returns
/// [`ScaleError::UnknownScale`](super::ScaleError::UnknownScale).
///
/// ```
/// use pltrs::scale::ScaleName;
/// assert_eq!(ScaleName::from("symlog"), ScaleName::Symlog);
/// assert_eq!("mercator".parse(), Ok(ScaleName::Custom("mercator".into())));
/// assert_eq!(ScaleName::FunctionLog.to_string(), "functionlog");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ScaleName {
    /// `"linear"`.
    Linear,
    /// `"log"`.
    Log,
    /// `"symlog"`.
    Symlog,
    /// `"logit"`.
    Logit,
    /// `"asinh"`.
    Asinh,
    /// `"function"`.
    Function,
    /// `"functionlog"`.
    FunctionLog,
    /// Another name: a scale added to a
    /// [`ScaleRegistry`](super::ScaleRegistry), or the name a
    /// [`Scale`](super::Scale) implemented outside pltrs gives itself.
    Custom(String),
}

impl ScaleName {
    /// The built-in names, sorted (matplotlib's `get_scale_names`).
    pub const BUILT_IN: [Self; 7] = [
        Self::Asinh,
        Self::Function,
        Self::FunctionLog,
        Self::Linear,
        Self::Log,
        Self::Logit,
        Self::Symlog,
    ];

    /// matplotlib's spelling of the name.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Linear => "linear",
            Self::Log => "log",
            Self::Symlog => "symlog",
            Self::Logit => "logit",
            Self::Asinh => "asinh",
            Self::Function => "function",
            Self::FunctionLog => "functionlog",
            Self::Custom(name) => name,
        }
    }

    /// Whether `minorticks_on` uses the scale's own minor locator rather
    /// than an `AutoMinorLocator`: log, symlog, asinh and logit scales.
    pub(crate) fn has_own_minor_locator(&self) -> bool {
        matches!(self, Self::Log | Self::Symlog | Self::Asinh | Self::Logit)
    }
}

impl fmt::Display for ScaleName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for ScaleName {
    fn from(name: &str) -> Self {
        Self::BUILT_IN
            .into_iter()
            .find(|builtin| builtin.as_str() == name)
            .unwrap_or_else(|| Self::Custom(name.to_owned()))
    }
}

impl From<String> for ScaleName {
    fn from(name: String) -> Self {
        match Self::from(name.as_str()) {
            Self::Custom(_) => Self::Custom(name),
            builtin => builtin,
        }
    }
}

impl FromStr for ScaleName {
    type Err = Infallible;

    /// The name for `name`; never fails, as any name may be registered.
    fn from_str(name: &str) -> Result<Self, Infallible> {
        Ok(Self::from(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_names_round_trip() {
        for name in ScaleName::BUILT_IN {
            assert_eq!(ScaleName::from(name.as_str()), name);
            assert_eq!(ScaleName::from(name.to_string()), name);
        }
        let mut sorted = ScaleName::BUILT_IN.map(|name| name.to_string());
        sorted.sort();
        assert_eq!(sorted, ScaleName::BUILT_IN.map(|name| name.to_string()));
    }

    #[test]
    fn other_names_are_custom() {
        assert_eq!(ScaleName::from("Log"), ScaleName::Custom("Log".into()));
        assert_eq!(
            ScaleName::from(String::new()),
            ScaleName::Custom(String::new())
        );
        assert_eq!(ScaleName::Custom("mercator".into()).as_str(), "mercator");
        assert!(ScaleName::Asinh.has_own_minor_locator());
        assert!(!ScaleName::Function.has_own_minor_locator());
    }
}
