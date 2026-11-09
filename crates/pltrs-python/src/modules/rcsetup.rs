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

use std::path::PathBuf;

use pltrs::colors::Color;
use pltrs::rc::{
    ColorOrAuto, ColorOrEdge, ColorOrInherit, LabelColor, RcError, RcFile, RcParams, RcWarningKind,
    parse_rc_text,
};

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
            RcError::Unreadable { .. } => PyErr::new(ExceptionKind::OSError, err.to_string()),
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
    Ints(Vec<i64>),
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
        } else if let Some(pair) = value.downcast_ref::<[i64; 2]>() {
            Self::Ints(pair.to_vec())
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
            Self::Ints(numbers) => numbers.into_py(py),
            Self::Texts(texts) => texts.into_py(py),
        }
    }
}

/// A Python value spelled as rc-file text: strings as they are, `True`,
/// `False` and `None`, numbers as Python prints them (numpy's as the
/// Python numbers they hold), sequences joined by commas (a sequence inside
/// one in parentheses, as a dash pattern's `(0, (3, 1))`), anything else (a
/// `Cycler`) as its `repr`.
#[derive(Clone, Debug, PartialEq)]
struct RcText(String);

impl FromPy<'_> for RcText {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        rc_text(object).map(Self)
    }
}

/// Whether `object` is spelled as a sequence of its items.
fn list_like(object: &Bound<'_>) -> bool {
    ["list", "tuple", "ndarray"].contains(&object.type_name().as_str())
}

/// The rc-file texts of the items of the sequence `object`, those that are
/// sequences themselves as Python's tuples.
fn item_texts(object: &Bound<'_>) -> PyResult<Vec<String>> {
    object
        .try_iter()?
        .map(|item| {
            let item = item?;
            if !list_like(&item) {
                return rc_text(&item);
            }
            Ok(match item_texts(&item)?.as_slice() {
                [single] => format!("({single},)"),
                items => format!("({})", items.join(", ")),
            })
        })
        .collect()
}

/// The rc-file text of `object` (see [`RcText`]).
fn rc_text(object: &Bound<'_>) -> PyResult<String> {
    if object.is_none() {
        return Ok("None".to_owned());
    }
    if let Ok(text) = object.extract::<String>() {
        return Ok(text);
    }
    if list_like(object) {
        return Ok(item_texts(object)?.join(", "));
    }
    if object.getattr("dtype").is_ok()
        && let Ok(number) = object.call_method("item", &[])
    {
        // A scalar of numpy's (`np.float64(8.0)`), whose repr no rc
        // text spells.
        return rc_text(&number);
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

/// `rcParams[key]`, typed as [`RcValue`] (pltrs (Rust)'s own `pltrs.*`
/// parameters, which matplotlib's validators do not know).
fn get(store: Option<Native<RcParams>>, key: String) -> PyResult<RcValue> {
    read(store.as_ref(), |params| RcValue::of(params, &key))
}

/// The rc-file text of `key`'s value, which matplotlib's validator of the
/// parameter turns into matplotlib's Python value.
fn text(store: Option<Native<RcParams>>, key: String) -> PyResult<String> {
    read(store.as_ref(), |params| {
        params
            .get(&key)
            .ok_or_else(|| PyErr::from(RcError::UnknownKey(key)))
    })
}

/// The generation of the thread's current parameters (`pltrs::rc::generation`):
/// values read from the parameters stay valid while it is the same.
fn generation() -> PyResult<u64> {
    Ok(pltrs::rc::generation())
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

/// `rcParams[key] = color` for a colour rc text cannot spell, a tuple
/// matplotlib's validator of `key` took (`legend.labelcolor`'s, which
/// rejects tuple strings from rc files, or one of numpy's numbers, as
/// colormaps give them, for any colour key).
fn set_color(store: Option<Native<RcParams>>, key: String, color: Color) -> PyResult<()> {
    let assign = |params: &mut RcParams| {
        let field = params
            .value_mut(&key)
            .ok_or_else(|| PyErr::from(RcError::UnknownKey(key.clone())))?;
        if let Some(field) = field.downcast_mut::<Color>() {
            *field = color.clone();
        } else if let Some(field) = field.downcast_mut::<LabelColor>() {
            *field = LabelColor::Color(color.clone());
        } else if let Some(field) = field.downcast_mut::<ColorOrAuto>() {
            *field = ColorOrAuto::Color(color.clone());
        } else if let Some(field) = field.downcast_mut::<ColorOrEdge>() {
            *field = ColorOrEdge::Color(color.clone());
        } else if let Some(field) = field.downcast_mut::<ColorOrInherit>() {
            *field = ColorOrInherit::Color(color.clone());
        } else {
            return Err(PyErr::value_error(format!("Key {key}: not a colour")));
        }
        Ok(())
    };
    if let Some(params) = store {
        assign(&mut params.borrow_mut())
    } else {
        let mut assigned = Ok(());
        pltrs::rc::update(|params| assigned = assign(params));
        assigned
    }
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

/// Pairs of strings: rc settings `(key, value)`.
type Pairs = Vec<(String, String)>;

/// A line of an rc file matplotlib reports: its kind (`"missing colon"`,
/// `"duplicate key"`, `"unknown key"` or `"invalid value"`), number and
/// text, and for an unknown key or an invalid value the key and the value
/// it assigns.
type Skipped = (String, usize, String, Option<String>, Option<String>);

/// The assignments of the rc file at `path`, `(key, value)` in the order
/// the keys first appear (each with its last valid value), and the lines
/// matplotlib reports (see [`Skipped`]), in matplotlib's order, which
/// `_rc_params_in_file` logs with matplotlib's messages.
fn read_file(path: PathBuf) -> PyResult<(Pairs, Vec<Skipped>)> {
    let file = RcFile::read(path)?;
    let entries = file
        .entries
        .into_iter()
        .map(|entry| (entry.key, entry.value))
        .collect();
    let skipped = file
        .warnings
        .into_iter()
        .map(|warning| {
            let kind = match warning.kind {
                RcWarningKind::MissingColon => "missing colon",
                RcWarningKind::DuplicateKey => "duplicate key",
                RcWarningKind::UnknownKey(_) => "unknown key",
                RcWarningKind::InvalidValue(_) => "invalid value",
            };
            let assigned = match warning.kind {
                RcWarningKind::UnknownKey(_) | RcWarningKind::InvalidValue(_) => {
                    parse_rc_text(&warning.text)
                        .ok()
                        .and_then(|entries| entries.into_iter().next())
                }
                RcWarningKind::MissingColon | RcWarningKind::DuplicateKey => None,
            };
            let (key, value) =
                assigned.map_or((None, None), |entry| (Some(entry.key), Some(entry.value)));
            (kind.to_owned(), warning.line, warning.text, key, value)
        })
        .collect();
    Ok((entries, skipped))
}

/// The style sheets bundled with pltrs (matplotlib's `stylelib`), each as
/// its name and its `(key, value)` settings in file order.
fn bundled_styles() -> PyResult<Vec<(String, Pairs)>> {
    pltrs::style::bundled_sheets()
        .into_iter()
        .map(|sheet| {
            let settings = sheet
                .entries()?
                .into_iter()
                .map(|entry| (entry.key, entry.value))
                .collect();
            Ok((sheet.name().to_owned(), settings))
        })
        .collect()
}

/// Copies the parameters `keys` of the store `source` into the store
/// `target` as they are (`rcParams.update(other_rcparams)`, `style.use`).
fn update_from(
    target: Option<Native<RcParams>>,
    source: Option<Native<RcParams>>,
    keys: Vec<String>,
) -> PyResult<()> {
    let texts = read(source.as_ref(), |params| {
        keys.iter()
            .map(|key| {
                params
                    .get(key)
                    .map(|text| (key.as_str(), text))
                    .ok_or_else(|| RcError::UnknownKey(key.clone()))
            })
            .collect::<Result<Vec<_>, _>>()
    })?;
    let copy = |params: &mut RcParams| -> Result<(), RcError> {
        texts
            .iter()
            .try_for_each(|(key, text)| params.set(key, text))
    };
    if let Some(params) = target {
        copy(&mut params.borrow_mut())?;
    } else {
        let mut updated = (*pltrs::rc::params()).clone();
        let copied = copy(&mut updated);
        pltrs::rc::replace(updated);
        copied?;
    }
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("read_file", read_file)?;
    module.add("bundled_styles", bundled_styles)?;
    module.add("update_from", update_from)?;
    module.add("keys", keys)?;
    module.add("get", get)?;
    module.add("text", text)?;
    module.add("generation", generation)?;
    module.add("set", set)?;
    module.add("set_color", set_color)?;
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
            // A dash pattern keeps its pattern apart from its offset.
            assert_eq!(text("(0, (3, 1, 1, 1))"), "0, (3, 1, 1, 1)");
            assert_eq!(text("[[1, 2], (3,), ()]"), "(1, 2), (3,), ()");
            // numpy's scalars as the Python numbers they hold.
            let scalar = "class Scalar:\n    dtype = 'float64'\n    def item(self):\n        \
                          return 8.0\n    def __repr__(self):\n        return 'np.float64(8.0)'\n";
            let numbers = eval(py, scalar, "[Scalar(), 5.5]").extract::<RcText>();
            assert_eq!(numbers.unwrap().0, "8.0, 5.5");
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
