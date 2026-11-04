//! Conversions between Python objects and Rust values: [`FromPy`] for
//! arguments, [`IntoPy`] for results.
//!
//! The conversions follow Python's own rules, and raise its messages:
//! a `bool` is any object's truth value; an integer is whatever has
//! `__index__` (Python and numpy integers, not floats); a float whatever
//! `float()` takes without parsing (numbers and numpy scalars, not
//! strings); a `Vec` any iterable but a string. Numbers in bulk go
//! through the buffer protocol instead ([`NdArray`](super::NdArray)).

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::containers::{list, new_dict, tuple};
use super::error::{ExceptionKind, PyErr, PyResult};
use super::gil::Python;
use super::object::{Bound, Owned};
use crate::ffi;

/// A Rust value made from a Python object (a native function's argument).
pub(crate) trait FromPy<'py>: Sized {
    /// Converts `object`, raising `TypeError` or `ValueError` when it does
    /// not fit.
    fn from_py(object: &Bound<'py>) -> PyResult<Self>;
}

/// A Rust value that becomes a Python object (a native function's result).
pub(crate) trait IntoPy {
    /// Whether every value converts to `None`: what a native function
    /// returning nothing returns.
    const IS_NONE: bool = false;

    /// Converts `self` into a new Python object.
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>>;
}

impl<'py> FromPy<'py> for Bound<'py> {
    fn from_py(object: &Bound<'py>) -> PyResult<Self> {
        Ok(object.clone())
    }
}

impl IntoPy for Bound<'_> {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        Ok(self.unbind().bind(py))
    }
}

impl IntoPy for &Bound<'_> {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        self.clone().into_py(py)
    }
}

impl FromPy<'_> for Owned {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        Ok(object.clone().unbind())
    }
}

impl IntoPy for Owned {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        Ok(self.bind(py))
    }
}

impl IntoPy for &Owned {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        Ok(self.bind(py))
    }
}

impl IntoPy for () {
    const IS_NONE: bool = true;

    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        Ok(Bound::none(py))
    }
}

impl FromPy<'_> for bool {
    /// The truth value, `bool(object)`.
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        object.is_truthy()
    }
}

impl IntoPy for bool {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        // SAFETY: the GIL is held; returns a new reference to a singleton.
        unsafe { Bound::from_owned_ptr(py, ffi::PyBool_FromLong(self.into())) }
    }
}

/// `operator.index(object)` as an `i64`.
fn index(object: &Bound<'_>) -> PyResult<i64> {
    let py = object.py();
    // SAFETY: a live object; a new reference or NULL with an exception.
    let integer = unsafe { Bound::from_owned_ptr(py, ffi::PyNumber_Index(object.as_ptr())) }?;
    // SAFETY: `integer` is an `int`; -1 may also mean an overflow.
    let value = unsafe { ffi::PyLong_AsLongLong(integer.as_ptr()) };
    match (value, PyErr::take(py)) {
        (-1, Some(err)) => Err(err),
        _ => Ok(value),
    }
}

/// Integer types convert through `__index__`, refusing values out of
/// their range with Python's `OverflowError`.
macro_rules! integers {
    ($($ty:ty),*) => {$(
        impl FromPy<'_> for $ty {
            fn from_py(object: &Bound<'_>) -> PyResult<Self> {
                <$ty>::try_from(index(object)?).map_err(|_| {
                    PyErr::new(
                        ExceptionKind::OverflowError,
                        concat!("Python int too large to convert to ", stringify!($ty)),
                    )
                })
            }
        }

        impl IntoPy for $ty {
            #[allow(
                clippy::cast_lossless,
                reason = "the macro serves every integer type; only u64 and usize reach the cast"
            )]
            fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
                let value = i64::try_from(self);
                // SAFETY: the GIL is held; the calls return a new
                // reference or NULL with an exception.
                unsafe {
                    Bound::from_owned_ptr(py, match value {
                        Ok(value) => ffi::PyLong_FromLongLong(value),
                        Err(_) => ffi::PyLong_FromUnsignedLongLong(self as u64),
                    })
                }
            }
        }
    )*};
}

integers!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

impl FromPy<'_> for f64 {
    /// `float(object)` for numbers (Python's and numpy's).
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        // SAFETY: a live object; -1.0 with an exception set on failure.
        let value = unsafe { ffi::PyFloat_AsDouble(object.as_ptr()) };
        #[expect(clippy::float_cmp, reason = "exactly -1.0 is CPython's error marker")]
        let marker = value == -1.0;
        if marker && let Some(err) = PyErr::take(object.py()) {
            return Err(err);
        }
        Ok(value)
    }
}

impl IntoPy for f64 {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        // SAFETY: the GIL is held; a new reference or NULL.
        unsafe { Bound::from_owned_ptr(py, ffi::PyFloat_FromDouble(self)) }
    }
}

impl FromPy<'_> for f32 {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        f64::from_py(object).map(|value| value as f32)
    }
}

impl IntoPy for f32 {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        f64::from(self).into_py(py)
    }
}

impl FromPy<'_> for String {
    /// A `str` (and only a `str`).
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if !object.has_type_flag(ffi::Py_TPFLAGS_UNICODE_SUBCLASS) {
            return Err(PyErr::type_error(format!(
                "expected str, got {}",
                object.type_name()
            )));
        }
        let mut size: ffi::Py_ssize_t = 0;
        // SAFETY: `object` is a `str`; the UTF-8 buffer it returns lives
        // as long as the string, which `object` keeps alive while it is
        // copied below.
        let data = unsafe { ffi::PyUnicode_AsUTF8AndSize(object.as_ptr(), &raw mut size) };
        if data.is_null() {
            return Err(PyErr::fetch(object.py()));
        }
        let length = usize::try_from(size).expect("a string's size is not negative");
        // SAFETY: CPython guarantees `size` valid UTF-8 bytes at `data`.
        let bytes = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), length) };
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }
}

impl IntoPy for &str {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        let length = ffi::Py_ssize_t::try_from(self.len())
            .map_err(|_| PyErr::new(ExceptionKind::OverflowError, "string too long"))?;
        // SAFETY: `length` UTF-8 bytes at the pointer; a new reference.
        unsafe {
            Bound::from_owned_ptr(
                py,
                ffi::PyUnicode_FromStringAndSize(self.as_ptr().cast(), length),
            )
        }
    }
}

impl IntoPy for String {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        self.as_str().into_py(py)
    }
}

impl IntoPy for &String {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        self.as_str().into_py(py)
    }
}

impl<'py, T: FromPy<'py>> FromPy<'py> for Option<T> {
    /// `None`, or a `T`.
    fn from_py(object: &Bound<'py>) -> PyResult<Self> {
        if object.is_none() {
            Ok(None)
        } else {
            T::from_py(object).map(Some)
        }
    }
}

impl<T: IntoPy> IntoPy for Option<T> {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        match self {
            Some(value) => value.into_py(py),
            None => Ok(Bound::none(py)),
        }
    }
}

/// The items of an iterable that is not a string.
pub(crate) fn items<'py>(object: &Bound<'py>) -> PyResult<Vec<Bound<'py>>> {
    let text = ffi::Py_TPFLAGS_UNICODE_SUBCLASS | ffi::Py_TPFLAGS_BYTES_SUBCLASS;
    if object.has_type_flag(text) {
        return Err(PyErr::type_error(format!(
            "expected a sequence, got {}",
            object.type_name()
        )));
    }
    object.try_iter()?.collect()
}

impl<'py, T: FromPy<'py>> FromPy<'py> for Vec<T> {
    /// The items of any iterable but a string.
    fn from_py(object: &Bound<'py>) -> PyResult<Self> {
        items(object)?.iter().map(T::from_py).collect()
    }
}

impl<T: IntoPy> IntoPy for Vec<T> {
    /// A `list`.
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        let items = self
            .into_iter()
            .map(|item| item.into_py(py))
            .collect::<PyResult<_>>()?;
        list(py, items)
    }
}

/// Tuples convert from iterables of exactly their length, and into
/// Python tuples.
macro_rules! tuples {
    ($($length:literal: ($($name:ident: $ty:ident),+);)*) => {$(
        impl<'py, $($ty: FromPy<'py>),+> FromPy<'py> for ($($ty,)+) {
            fn from_py(object: &Bound<'py>) -> PyResult<Self> {
                let items = items(object)?;
                let [$($name),+] = <[Bound<'py>; $length]>::try_from(items).map_err(|items| {
                    PyErr::value_error(format!(
                        "expected {} values to unpack, got {}",
                        $length,
                        items.len()
                    ))
                })?;
                Ok(($($ty::from_py(&$name)?,)+))
            }
        }

        impl<$($ty: IntoPy),+> IntoPy for ($($ty,)+) {
            fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
                let ($($name,)+) = self;
                tuple(py, vec![$($name.into_py(py)?),+])
            }
        }
    )*};
}

tuples! {
    1: (a: A);
    2: (a: A, b: B);
    3: (a: A, b: B, c: C);
    4: (a: A, b: B, c: C, d: D);
    5: (a: A, b: B, c: C, d: D, e: E);
    6: (a: A, b: B, c: C, d: D, e: E, f: F);
    7: (a: A, b: B, c: C, d: D, e: E, f: F, g: G);
}

impl<'py, V: FromPy<'py>> FromPy<'py> for BTreeMap<String, V> {
    /// A mapping with string keys (anything with `items()`).
    fn from_py(object: &Bound<'py>) -> PyResult<Self> {
        let pairs = object.call_method("items", &[])?;
        pairs
            .try_iter()?
            .map(|pair| {
                let (key, value): (String, V) = pair?.extract()?;
                Ok((key, value))
            })
            .collect()
    }
}

impl<V: IntoPy> IntoPy for BTreeMap<String, V> {
    /// A `dict`.
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        let dict = new_dict(py)?;
        for (key, value) in self {
            dict.set_item(key, value)?;
        }
        Ok(dict)
    }
}

impl FromPy<'_> for PathBuf {
    /// A path: a `str` or an `os.PathLike` giving one.
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        // SAFETY: a live object; a new reference or NULL with an exception.
        let path =
            unsafe { Bound::from_owned_ptr(object.py(), ffi::PyOS_FSPath(object.as_ptr())) }?;
        path.extract::<String>().map(PathBuf::from)
    }
}

/// Bytes, converting from and to Python's `bytes` (a `Vec<u8>` is a
/// list of integers instead).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Bytes(pub(crate) Vec<u8>);

impl FromPy<'_> for Bytes {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if !object.has_type_flag(ffi::Py_TPFLAGS_BYTES_SUBCLASS) {
            return Err(PyErr::type_error(format!(
                "expected bytes, got {}",
                object.type_name()
            )));
        }
        // SAFETY: `object` is a `bytes`, which owns its buffer while
        // `object` keeps it alive during the copy.
        let bytes = unsafe {
            let data = ffi::PyBytes_AsString(object.as_ptr());
            let length = usize::try_from(ffi::PyBytes_Size(object.as_ptr()))
                .expect("a bytes object's size is not negative");
            std::slice::from_raw_parts(data.cast::<u8>(), length).to_vec()
        };
        Ok(Self(bytes))
    }
}

impl IntoPy for Bytes {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        let length = ffi::Py_ssize_t::try_from(self.0.len())
            .map_err(|_| PyErr::new(ExceptionKind::OverflowError, "too many bytes"))?;
        // SAFETY: `length` bytes at the pointer are copied; a new reference.
        unsafe {
            Bound::from_owned_ptr(
                py,
                ffi::PyBytes_FromStringAndSize(self.0.as_ptr().cast(), length),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{eval, str, with_python};

    #[test]
    fn scalars_round_trip() {
        with_python(|py| {
            assert!(true.into_py(py).unwrap().extract::<bool>().unwrap());
            assert_eq!((-7_i64).into_py(py).unwrap().extract::<i64>().unwrap(), -7);
            assert_eq!(
                u32::MAX.into_py(py).unwrap().extract::<u32>().unwrap(),
                u32::MAX
            );
            assert_eq!(2.5_f64.into_py(py).unwrap().extract::<f64>().unwrap(), 2.5);
            assert_eq!(
                "héllo".into_py(py).unwrap().extract::<String>().unwrap(),
                "héllo"
            );
            assert!(().into_py(py).unwrap().is_none());
            assert_eq!(eval(py, "", "7").extract::<f64>().unwrap(), 7.0);
            assert_eq!(eval(py, "", "True").extract::<i64>().unwrap(), 1);
            assert!(
                !eval(py, "", "[]").extract::<bool>().unwrap(),
                "truth values"
            );
        });
    }

    #[test]
    fn conversions_raise_pythons_errors() {
        with_python(|py| {
            let err = str(py, "x").extract::<f64>().unwrap_err();
            assert_eq!(err.class_name(py), "TypeError");
            assert!(err.message(py).contains("must be real number, not str"));
            let err = eval(py, "", "1.5").extract::<i64>().unwrap_err();
            assert!(
                err.message(py)
                    .contains("cannot be interpreted as an integer")
            );
            let err = eval(py, "", "300").extract::<u8>().unwrap_err();
            assert_eq!(err.class_name(py), "OverflowError");
            let err = eval(py, "", "3").extract::<String>().unwrap_err();
            assert_eq!(err.message(py), "expected str, got int");
        });
    }

    #[test]
    fn containers_round_trip() {
        with_python(|py| {
            let numbers = vec![1.0, 2.0].into_py(py).unwrap();
            assert_eq!(numbers.type_name(), "list");
            assert_eq!(numbers.extract::<Vec<f64>>().unwrap(), [1.0, 2.0]);
            let pair = (1_i64, "a").into_py(py).unwrap();
            assert_eq!(pair.type_name(), "tuple");
            assert_eq!(
                pair.extract::<(i64, String)>().unwrap(),
                (1, "a".to_owned())
            );
            let err = eval(py, "", "(1, 2, 3)")
                .extract::<(i64, i64)>()
                .unwrap_err();
            assert!(
                err.message(py)
                    .contains("expected 2 values to unpack, got 3")
            );
            let map = BTreeMap::from([("k".to_owned(), 1_i64)])
                .into_py(py)
                .unwrap();
            assert_eq!(map.extract::<BTreeMap<String, i64>>().unwrap()["k"], 1);
            let some = Some(3_i64).into_py(py).unwrap();
            assert_eq!(some.extract::<Option<i64>>().unwrap(), Some(3));
            assert_eq!(Bound::none(py).extract::<Option<i64>>().unwrap(), None);
            let err = str(py, "ab").extract::<Vec<String>>().unwrap_err();
            assert_eq!(err.message(py), "expected a sequence, got str");
        });
    }

    #[test]
    fn paths_and_bytes_convert() {
        with_python(|py| {
            let path = eval(py, "import pathlib", "pathlib.Path('/tmp/x')");
            assert_eq!(path.extract::<PathBuf>().unwrap(), PathBuf::from("/tmp/x"));
            let bytes = Bytes(b"\x00ab".to_vec()).into_py(py).unwrap();
            assert_eq!(bytes.type_name(), "bytes");
            assert_eq!(bytes.extract::<Bytes>().unwrap().0, b"\x00ab");
        });
    }
}
