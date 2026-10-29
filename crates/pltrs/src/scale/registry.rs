//! Creating scales by name (matplotlib's `scale_factory` and
//! `register_scale`).

use std::collections::BTreeMap;
use std::fmt;

use super::{
    AsinhScale, FuncScale, FuncScaleLog, FuncTransform, LinearScale, LogScale, LogitScale,
    NonPositive, Scale, ScaleError, ScaleName, SymmetricalLogScale,
};

/// Keyword options for a scale, as passed to `set_xscale(name, **kwargs)`.
///
/// Each scale accepts a subset; passing another option is an error, like
/// matplotlib's `TypeError` for unexpected keyword arguments.
#[derive(Clone, Debug, Default)]
pub struct ScaleOptions {
    /// Logarithm base (`log`, `symlog`, `asinh`, `functionlog`).
    pub base: Option<f64>,
    /// Minor tick multiples (`log`, `symlog`, `asinh`).
    pub subs: Option<Vec<f64>>,
    /// Treatment of out-of-domain values (`log`, `logit`).
    pub nonpositive: Option<NonPositive>,
    /// Extent of the linear region (`symlog`).
    pub linthresh: Option<f64>,
    /// Decades per half of the linear region (`symlog`).
    pub linscale: Option<f64>,
    /// Width of the quasi-linear region (`asinh`).
    pub linear_width: Option<f64>,
    /// Label for one half (`logit`).
    pub one_half: Option<String>,
    /// Overline notation above one half (`logit`).
    pub use_overline: Option<bool>,
    /// Forward and inverse functions (`function`, `functionlog`).
    pub functions: Option<FuncTransform>,
}

impl ScaleOptions {
    /// Fails if an option outside `accepted` is set.
    fn only(&self, scale: &str, accepted: &[&str]) -> Result<(), ScaleError> {
        let given = [
            ("base", self.base.is_some()),
            ("subs", self.subs.is_some()),
            ("nonpositive", self.nonpositive.is_some()),
            ("linthresh", self.linthresh.is_some()),
            ("linscale", self.linscale.is_some()),
            ("linear_width", self.linear_width.is_some()),
            ("one_half", self.one_half.is_some()),
            ("use_overline", self.use_overline.is_some()),
            ("functions", self.functions.is_some()),
        ];
        match given
            .iter()
            .find(|(name, set)| *set && !accepted.contains(name))
        {
            Some(&(option, _)) => Err(ScaleError::UnexpectedOption {
                scale: scale.to_owned(),
                option,
            }),
            None => Ok(()),
        }
    }

    fn functions(&self, scale: &str) -> Result<FuncTransform, ScaleError> {
        self.functions
            .clone()
            .ok_or_else(|| ScaleError::MissingOption {
                scale: scale.to_owned(),
                option: "functions",
            })
    }
}

/// Builds a scale from options.
pub type ScaleConstructor = Box<dyn Fn(&ScaleOptions) -> Result<Box<dyn Scale>, ScaleError>>;

/// Scales available by name; [`ScaleRegistry::default`] holds the
/// built-in ones and custom scales can be added.
pub struct ScaleRegistry {
    constructors: BTreeMap<String, ScaleConstructor>,
}

impl fmt::Debug for ScaleRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScaleRegistry")
            .field("names", &self.names())
            .finish()
    }
}

fn boxed(scale: impl Scale) -> Box<dyn Scale> {
    Box::new(scale)
}

impl Default for ScaleRegistry {
    fn default() -> Self {
        let mut registry = Self {
            constructors: BTreeMap::new(),
        };
        registry.register("linear", |o| {
            o.only("linear", &[])?;
            Ok(boxed(LinearScale))
        });
        registry.register("log", |o| {
            o.only("log", &["base", "subs", "nonpositive"])?;
            let mut scale = LogScale::new(o.base.unwrap_or(10.0))?
                .with_nonpositive(o.nonpositive.unwrap_or(NonPositive::Clip));
            if let Some(subs) = &o.subs {
                scale = scale.with_subs(subs.clone());
            }
            Ok(boxed(scale))
        });
        registry.register("symlog", |o| {
            o.only("symlog", &["base", "linthresh", "subs", "linscale"])?;
            let mut scale = SymmetricalLogScale::new(
                o.base.unwrap_or(10.0),
                o.linthresh.unwrap_or(2.0),
                o.linscale.unwrap_or(1.0),
            )?;
            if let Some(subs) = &o.subs {
                scale = scale.with_subs(subs.clone());
            }
            Ok(boxed(scale))
        });
        registry.register("logit", |o| {
            o.only("logit", &["nonpositive", "one_half", "use_overline"])?;
            let mut scale = LogitScale::new(o.nonpositive.unwrap_or(NonPositive::Mask))
                .with_use_overline(o.use_overline.unwrap_or(false));
            if let Some(one_half) = &o.one_half {
                scale = scale.with_one_half(one_half.clone());
            }
            Ok(boxed(scale))
        });
        registry.register("asinh", |o| {
            o.only("asinh", &["linear_width", "base", "subs"])?;
            let mut scale = AsinhScale::new(o.linear_width.unwrap_or(1.0), o.base.unwrap_or(10.0))?;
            if let Some(subs) = &o.subs {
                scale = scale.with_subs(subs.clone());
            }
            Ok(boxed(scale))
        });
        registry.register("function", |o| {
            o.only("function", &["functions"])?;
            Ok(boxed(FuncScale::from_transform(o.functions("function")?)))
        });
        registry.register("functionlog", |o| {
            o.only("functionlog", &["functions", "base"])?;
            Ok(boxed(FuncScaleLog::new(
                o.functions("functionlog")?,
                o.base.unwrap_or(10.0),
            )?))
        });
        registry
    }
}

impl ScaleRegistry {
    /// Registers (or replaces) the scale called `name`.
    pub fn register(
        &mut self,
        name: impl Into<ScaleName>,
        constructor: impl Fn(&ScaleOptions) -> Result<Box<dyn Scale>, ScaleError> + 'static,
    ) {
        self.constructors
            .insert(name.into().to_string(), Box::new(constructor));
    }

    /// The registered names, sorted.
    pub fn names(&self) -> Vec<ScaleName> {
        self.constructors
            .keys()
            .map(|name| ScaleName::from(name.as_str()))
            .collect()
    }

    /// Creates the scale called `name` from `options`.
    ///
    /// # Errors
    ///
    /// When no scale is registered under `name`, or the scale refuses the
    /// options.
    pub fn create(
        &self,
        name: impl Into<ScaleName>,
        options: ScaleOptions,
    ) -> Result<Box<dyn Scale>, ScaleError> {
        let name = name.into();
        let constructor = self
            .constructors
            .get(name.as_str())
            .ok_or_else(|| ScaleError::UnknownScale(name.to_string()))?;
        constructor(&options)
    }
}

/// The names of the built-in scales, sorted (matplotlib's
/// `get_scale_names`).
pub fn scale_names() -> Vec<ScaleName> {
    ScaleRegistry::default().names()
}

/// Creates a built-in scale by name (matplotlib's `scale_factory`).
///
/// # Errors
///
/// When `name` is not a built-in scale, or the scale refuses the options.
pub fn scale_factory(
    name: impl Into<ScaleName>,
    options: ScaleOptions,
) -> Result<Box<dyn Scale>, ScaleError> {
    ScaleRegistry::default().create(name, options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_names_are_sorted() {
        assert_eq!(scale_names(), ScaleName::BUILT_IN);
    }

    #[test]
    fn factory_validates_options() {
        assert_eq!(
            scale_factory("linear", ScaleOptions::default())
                .unwrap()
                .name(),
            ScaleName::Linear
        );
        assert!(matches!(
            scale_factory("bogus", ScaleOptions::default()),
            Err(ScaleError::UnknownScale(_))
        ));
        let linthresh = ScaleOptions {
            linthresh: Some(1.0),
            ..ScaleOptions::default()
        };
        assert!(matches!(
            scale_factory("log", linthresh.clone()),
            Err(ScaleError::UnexpectedOption { .. })
        ));
        assert_eq!(
            scale_factory(ScaleName::Symlog, linthresh).unwrap().name(),
            ScaleName::Symlog
        );
        assert!(matches!(
            scale_factory("function", ScaleOptions::default()),
            Err(ScaleError::MissingOption { .. })
        ));
    }

    #[test]
    fn custom_scales_can_be_registered() {
        let mut registry = ScaleRegistry::default();
        registry.register("reciprocal", |_| {
            Ok(boxed(FuncScale::new(|x| 1.0 / x, |y| 1.0 / y)))
        });
        let reciprocal = ScaleName::Custom("reciprocal".into());
        assert!(registry.names().contains(&reciprocal));
        let scale = registry
            .create(reciprocal, ScaleOptions::default())
            .unwrap();
        assert_eq!(scale.forward(&[4.0]), [0.25]);
        assert!(matches!(
            ScaleRegistry::default().create("reciprocal", ScaleOptions::default()),
            Err(ScaleError::UnknownScale(name)) if name == "reciprocal"
        ));
    }
}
