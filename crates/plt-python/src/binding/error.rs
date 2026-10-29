//! Python exceptions in Rust: [`PyErr`], [`PyResult`] and the exception
//! classes Rust code raises ([`ExceptionKind`]).
//!
//! A native function returns `PyResult<T>`; its `Err` is raised in
//! Python when the function returns. Errors of plt-rs convert into
//! `PyErr` (each matplotlib module's file implements `From` for the error
//! types of its plt module), so that `?` raises the exception class and
//! message matplotlib raises for the same mistake.

use std::any::Any;
use std::ffi::CString;
use std::fmt;

use super::gil::{Python, panic_message};
use super::object::{Bound, Owned};
use crate::ffi;

/// The result of anything that can raise.
pub(crate) type PyResult<T> = Result<T, PyErr>;

/// The built-in exception classes Rust code raises.
#[expect(
    clippy::enum_variant_names,
    reason = "the variants are Python's class names"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExceptionKind {
    /// `ValueError`: a value of the right type that is not acceptable.
    ValueError,
    /// `TypeError`: a value of the wrong type.
    TypeError,
    /// `KeyError`: an unknown key (rcParams, style names, ...).
    KeyError,
    /// `AttributeError`: an unknown attribute or property.
    AttributeError,
    /// `RuntimeError`: an operation that cannot run now.
    RuntimeError,
    /// `OSError`: a file that cannot be read or written.
    OSError,
    /// `OverflowError`: a number too large for its Rust type.
    OverflowError,
    /// `SystemError`: a broken promise of the C API.
    SystemError,
}

impl ExceptionKind {
    /// The exception class.
    fn class(self) -> *mut ffi::PyObject {
        // SAFETY: the `PyExc_*` statics are set when the interpreter
        // starts and never change; they are only read.
        unsafe {
            match self {
                Self::ValueError => ffi::PyExc_ValueError,
                Self::TypeError => ffi::PyExc_TypeError,
                Self::KeyError => ffi::PyExc_KeyError,
                Self::AttributeError => ffi::PyExc_AttributeError,
                Self::RuntimeError => ffi::PyExc_RuntimeError,
                Self::OSError => ffi::PyExc_OSError,
                Self::OverflowError => ffi::PyExc_OverflowError,
                Self::SystemError => ffi::PyExc_SystemError,
            }
        }
    }
}

/// A Python exception: one Rust code is about to raise, or one Python
/// raised that Rust caught.
pub(crate) struct PyErr {
    state: State,
}

/// What a [`PyErr`] holds.
enum State {
    /// A built-in exception made from Rust, instantiated when raised.
    New {
        kind: ExceptionKind,
        message: String,
    },
    /// An exception instance Python raised.
    Raised(Owned),
}

impl PyErr {
    /// An exception of `kind` with `message`.
    pub(crate) fn new(kind: ExceptionKind, message: impl Into<String>) -> Self {
        Self {
            state: State::New {
                kind,
                message: message.into(),
            },
        }
    }

    /// `ValueError(message)`.
    pub(crate) fn value_error(message: impl Into<String>) -> Self {
        Self::new(ExceptionKind::ValueError, message)
    }

    /// `TypeError(message)`.
    pub(crate) fn type_error(message: impl Into<String>) -> Self {
        Self::new(ExceptionKind::TypeError, message)
    }

    /// `KeyError(message)`.
    pub(crate) fn key_error(message: impl Into<String>) -> Self {
        Self::new(ExceptionKind::KeyError, message)
    }

    /// `RuntimeError(message)`.
    pub(crate) fn runtime_error(message: impl Into<String>) -> Self {
        Self::new(ExceptionKind::RuntimeError, message)
    }

    /// An exception instance (any Python object deriving from
    /// `BaseException`), to be raised as it is.
    pub(crate) fn from_instance(instance: Bound<'_>) -> Self {
        Self {
            state: State::Raised(instance.unbind()),
        }
    }

    /// A Rust panic, which pltrs reports as a `RuntimeError` rather than
    /// unwinding into the interpreter.
    pub(crate) fn from_panic(payload: &(dyn Any + Send)) -> Self {
        Self::runtime_error(format!(
            "pltrs internal error (Rust panic): {}",
            panic_message(payload)
        ))
    }

    /// Takes the exception that is set, if any.
    pub(crate) fn take(py: Python<'_>) -> Option<Self> {
        // SAFETY: the GIL is held (`py`); the call returns a new reference
        // or NULL when nothing is raised.
        let raised = unsafe { ffi::PyErr_GetRaisedException() };
        if raised.is_null() {
            return None;
        }
        // SAFETY: a new reference from the call, not NULL.
        let instance = unsafe { Bound::from_owned_ptr(py, raised) }.ok()?;
        Some(Self::from_instance(instance))
    }

    /// Takes the exception a failed C API call set (a `SystemError` if
    /// the call broke its promise and set none).
    pub(crate) fn fetch(py: Python<'_>) -> Self {
        Self::take(py).unwrap_or_else(|| {
            Self::new(
                ExceptionKind::SystemError,
                "a C API call failed without setting an exception",
            )
        })
    }

    /// Raises the exception in the interpreter.
    pub(crate) fn restore(self, py: Python<'_>) {
        match self.state {
            State::New { kind, message } => {
                let message =
                    CString::new(message.replace('\0', "\\0")).expect("NULs were escaped");
                // SAFETY: the GIL is held; a valid class and C string.
                unsafe { ffi::PyErr_SetString(kind.class(), message.as_ptr()) };
            }
            // SAFETY: the GIL is held; the call steals the reference.
            State::Raised(instance) => unsafe {
                ffi::PyErr_SetRaisedException(instance.bind(py).into_ptr());
            },
        }
    }

    /// `str(exception)`.
    pub(crate) fn message(&self, py: Python<'_>) -> String {
        match &self.state {
            State::New { message, .. } => message.clone(),
            State::Raised(instance) => instance.bind(py).str().unwrap_or_default(),
        }
    }
}

#[cfg(test)]
impl PyErr {
    /// The exception's class name, for tests (`"ValueError"`).
    pub(crate) fn class_name(&self, py: Python<'_>) -> String {
        match &self.state {
            State::New { kind, .. } => format!("{kind:?}"),
            State::Raised(instance) => instance.bind(py).type_name(),
        }
    }
}

impl fmt::Debug for PyErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.state {
            State::New { kind, message } => write!(f, "{kind:?}({message:?})"),
            State::Raised(instance) => write!(f, "raised {instance:?}"),
        }
    }
}

/// Issues a `RuntimeWarning`, as `warnings.warn` does; an `Err` when the
/// warning filters turned it into an exception.
pub(crate) fn runtime_warning(py: Python<'_>, message: &str) -> PyResult<()> {
    let message = super::object::c_string(message)?;
    // SAFETY: the GIL is held (`py`); `PyExc_RuntimeWarning` is set at
    // start-up and never changes; a valid C string.
    let status = unsafe { ffi::PyErr_WarnEx(ffi::PyExc_RuntimeWarning, message.as_ptr(), 1) };
    super::object::check_status(py, status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{eval, with_python};

    #[test]
    fn raised_exceptions_come_back_as_they_were() {
        with_python(|py| {
            PyErr::key_error("missing").restore(py);
            let err = PyErr::take(py).expect("an exception is set");
            assert_eq!(err.class_name(py), "KeyError");
            assert_eq!(err.message(py), "'missing'", "KeyError's str is a repr");
            assert!(PyErr::take(py).is_none(), "taking clears it");
            assert_eq!(PyErr::fetch(py).class_name(py), "SystemError");
        });
    }

    #[test]
    fn python_exceptions_keep_their_class() {
        with_python(|py| {
            let instance = eval(py, "", "ZeroDivisionError('by zero')");
            let err = PyErr::from_instance(instance);
            assert_eq!(err.class_name(py), "ZeroDivisionError");
            assert_eq!(err.message(py), "by zero");
        });
    }

    #[test]
    fn panics_become_runtime_errors() {
        with_python(|py| {
            let err = PyErr::from_panic(&"boom");
            assert_eq!(err.class_name(py), "RuntimeError");
            assert!(err.message(py).contains("boom"));
        });
    }

    #[test]
    fn nul_bytes_in_messages_do_not_break_raising() {
        with_python(|py| {
            PyErr::value_error("a\0b").restore(py);
            assert_eq!(PyErr::fetch(py).message(py), "a\\0b");
        });
    }
}
