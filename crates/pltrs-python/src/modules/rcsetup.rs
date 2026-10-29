//! `_native.rcsetup`: rcParams, validated and stored by pltrs (Rust)'s `rc`
//! module (matplotlib's `rcsetup` and the `RcParams` of its
//! `__init__`).
//!
//! A store is `None`, the current parameters of the thread (what
//! `matplotlib.rcParams` is), or a native `RcParams` value (a copy, such
//! as `rcParamsDefault` or an `RcParams(...)` made in Python). Values
//! cross as rc-file text one way (the Python value is spelled as an rc
//! file would, then validated by pltrs (Rust) exactly as matplotlib validates
//! it) and as typed values the other.

use pltrs::rc::{RcError, RcParams};

use crate::binding::prelude::*;

native_value!(RcParams);

impl From<RcError> for PyErr {
    /// matplotlib's `KeyError` for unknown keys and `ValueError` for
    /// invalid values.
    fn from(err: RcError) -> Self {
        match err {
            RcError::UnknownKey(key) => PyErr::key_error(format!(
                "{key} is not a valid rc parameter (see rcParams.keys() for a list of valid \
                 parameters)"
            )),
            RcError::InvalidValue { key, reason, .. } => {
                PyErr::value_error(format!("Key {key}: {reason}"))
            }
            other => PyErr::value_error(other.to_string()),
        }
    }
}

/// An rcParam value, as Python gets it.
#[derive(Clone, Debug, PartialEq)]
enum RcValue {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    Floats(Vec<f64>),
    Texts(Vec<String>),
}

impl RcValue {
    /// The value of `key` in `params`: numbers, flags, strings and lists
    /// of them as such, anything else (colours, keywords, cyclers) as its
    /// rc-file text.
    fn of(params: &RcParams, key: &str) -> PyResult<Self> {
        let value = params
            .value(key)
            .ok_or_else(|| PyErr::from(RcError::UnknownKey(key.to_owned())))?;
        let typed = if let Some(&flag) = value.downcast_ref::<bool>() {
            Self::Bool(flag)
        } else if let Some(&number) = value.downcast_ref::<i64>() {
            Self::Int(number)
        } else if let Some(&number) = value.downcast_ref::<f64>() {
            Self::Float(number)
        } else if let Some(text) = value.downcast_ref::<String>() {
            Self::Text(text.clone())
        } else if let Some(numbers) = value.downcast_ref::<Vec<f64>>() {
            Self::Floats(numbers.clone())
        } else if let Some(pair) = value.downcast_ref::<[f64; 2]>() {
            Self::Floats(pair.to_vec())
        } else if let Some(texts) = value.downcast_ref::<Vec<String>>() {
            Self::Texts(texts.clone())
        } else if let Some(number) = value.downcast_ref::<Option<f64>>() {
            number.map_or(Self::None, Self::Float)
        } else if let Some(text) = value.downcast_ref::<Option<String>>() {
            text.clone().map_or(Self::None, Self::Text)
        } else if let Some(&pltrs::rc::Dpi::Value(dpi)) = value.downcast_ref::<pltrs::rc::Dpi>() {
            Self::Float(dpi)
        } else {
            let text = params.get(key).unwrap_or_default();
            if text == "None" {
                Self::None
            } else {
                Self::Text(text)
            }
        };
        Ok(typed)
    }
}

impl IntoPy for RcValue {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        match self {
            Self::None => ().into_py(py),
            Self::Bool(flag) => flag.into_py(py),
            Self::Int(number) => number.into_py(py),
            Self::Float(number) => number.into_py(py),
            Self::Text(text) => text.into_py(py),
            Self::Floats(numbers) => numbers.into_py(py),
            Self::Texts(texts) => texts.into_py(py),
        }
    }
}

/// A Python value spelled as rc-file text: strings as they are, `True`,
/// `False` and `None`, numbers as Python prints them, sequences joined by
/// commas, anything else (a `Cycler`) as its `repr`.
#[derive(Clone, Debug, PartialEq)]
struct RcText(String);

impl FromPy<'_> for RcText {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        rc_text(object).map(Self)
    }
}

/// The rc-file text of `object` (see [`RcText`]).
fn rc_text(object: &Bound<'_>) -> PyResult<String> {
    if object.is_none() {
        return Ok("None".to_owned());
    }
    if let Ok(text) = object.extract::<String>() {
        return Ok(text);
    }
    let list_like = ["list", "tuple", "ndarray"].contains(&object.type_name().as_str());
    if list_like {
        let parts: Vec<String> = object
            .try_iter()?
            .map(|item| rc_text(&item?))
            .collect::<PyResult<_>>()?;
        return Ok(parts.join(", "));
    }
    object.repr()
}

/// Runs `read` on the parameters of `store`.
fn read<T>(store: Option<&Native<RcParams>>, read: impl FnOnce(&RcParams) -> T) -> T {
    match store {
        Some(params) => read(&params.borrow()),
        None => read(&pltrs::rc::params()),
    }
}

/// `rcParams[key]`.
fn get(store: Option<Native<RcParams>>, key: String) -> PyResult<RcValue> {
    read(store.as_ref(), |params| RcValue::of(params, &key))
}

/// The rc-file text of `text` (an [`RcText`]) for `key`: Python's `None`
/// for `savefig.bbox` is `standard`, which matplotlib's `validate_bbox`
/// takes as the object `None` though not as the text.
fn for_key<'a>(key: &str, text: &'a str) -> &'a str {
    if key == "savefig.bbox" && text == "None" {
        "standard"
    } else {
        text
    }
}

/// `rcParams[key] = value`, validated.
fn set(store: Option<Native<RcParams>>, key: String, value: RcText) -> PyResult<()> {
    let text = for_key(&key, &value.0);
    match store {
        Some(params) => params.borrow_mut().set(&key, text)?,
        None => pltrs::rc::set(&key, text)?,
    }
    Ok(())
}

/// Every rcParam's name, in matplotlib's order: pltrs (Rust)'s own (`pltrs.*`)
/// are left out, as matplotlib code iterating over rcParams expects only
/// matplotlib's, but they can still be read and set by name.
fn keys() -> PyResult<Vec<&'static str>> {
    Ok(RcParams::KEYS
        .iter()
        .copied()
        .filter(|key| !key.starts_with("pltrs."))
        .collect())
}

/// A copy of a store's parameters (the current ones for `None`).
fn copy(store: Option<Native<RcParams>>) -> PyResult<RcParams> {
    Ok(read(store.as_ref(), Clone::clone))
}

/// matplotlib's defaults (`rcParamsDefault`).
fn defaults() -> PyResult<RcParams> {
    Ok(RcParams::matplotlib())
}

/// Makes `params` the current parameters (`rc_context`'s restore,
/// `rcdefaults`).
fn replace(params: RcParams) -> PyResult<()> {
    pltrs::rc::replace(params);
    Ok(())
}

/// Applies the style sheets `names`, in order (`style.use`).
fn use_styles(names: Vec<String>) -> PyResult<()> {
    for name in names {
        pltrs::style::use_style(&name)?;
    }
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("keys", keys)?;
    module.add("get", get)?;
    module.add("set", set)?;
    module.add("copy", copy)?;
    module.add("defaults", defaults)?;
    module.add("replace", replace)?;
    module.add("use_styles", use_styles)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{eval, extension, with_python};

    #[test]
    fn values_come_back_in_python_types() {
        let defaults = RcParams::matplotlib();
        let value = |key: &str| RcValue::of(&defaults, key).unwrap();
        assert_eq!(value("lines.linewidth"), RcValue::Float(1.5));
        assert_eq!(value("image.lut"), RcValue::Int(256));
        assert_eq!(value("axes.grid"), RcValue::Bool(false));
        assert_eq!(value("figure.figsize"), RcValue::Floats(vec![6.4, 4.8]));
        assert_eq!(
            value("font.family"),
            RcValue::Texts(vec!["sans-serif".into()])
        );
        assert_eq!(value("svg.hashsalt"), RcValue::None);
        assert_eq!(value("lines.linestyle"), RcValue::Text("-".into()));
        assert!(RcValue::of(&defaults, "no.such.key").is_err());
    }

    #[test]
    fn python_values_are_spelled_as_rc_text() {
        with_python(|py| {
            let text = |expression: &str| eval(py, "", expression).extract::<RcText>().unwrap().0;
            assert_eq!(text("2"), "2");
            assert_eq!(text("None"), "None");
            assert_eq!(text("True"), "True");
            assert_eq!(text("'serif'"), "serif");
            assert_eq!(text("[3.5, 42]"), "3.5, 42");
            assert_eq!(text("('a', 'b')"), "a, b");
        });
    }

    #[test]
    fn stores_validate_with_matplotlibs_exceptions() {
        with_python(|py| {
            extension(py);
            let object = RcParams::matplotlib().into_py(py).unwrap();
            let store = || object.extract::<Native<RcParams>>().unwrap();
            let current = get(None, "lines.linewidth".into()).unwrap();
            set(Some(store()), "lines.linewidth".into(), RcText("3".into())).unwrap();
            let width = get(Some(store()), "lines.linewidth".into()).unwrap();
            assert_eq!(width, RcValue::Float(3.0));
            assert_eq!(
                get(None, "lines.linewidth".into()).unwrap(),
                current,
                "untouched"
            );
            let err = set(None, "lines.linewidth".into(), RcText("wide".into())).unwrap_err();
            assert_eq!(err.class_name(py), "ValueError");
            assert!(err.message(py).starts_with("Key lines.linewidth: "));
            let err = get(None, "no.such.key".into()).unwrap_err();
            assert_eq!(err.class_name(py), "KeyError");
            assert!(err.message(py).contains("is not a valid rc parameter"));
            assert!(keys().unwrap().iter().all(|key| !key.starts_with("pltrs.")));
        });
    }
}
