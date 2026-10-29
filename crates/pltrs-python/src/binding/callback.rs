//! Calling Python from code pltrs (Rust) runs: [`Callback`], and the frames
//! that carry a callback's exception back to Python.
//!
//! pltrs (Rust) calls user code through Rust traits and closures that cannot
//! fail the Python way: a formatter returns a `String`, an event callback
//! returns nothing. When the Python callable behind one raises, the
//! exception is kept in the frame of the native call that is running and
//! the callback returns a fallback; once pltrs (Rust) returns, the native
//! function raises the kept exception instead of returning ([`entry`]).
//! Later callbacks of the same native call are skipped, so one mistake
//! raises once. Each native call has a frame of its own, so a native
//! function a callback calls cannot pick up its caller's exception.
//!
//! [`entry`]: super::gil::entry

use std::cell::RefCell;

use super::convert::{FromPy, IntoPy};
use super::error::{PyErr, PyResult};
use super::gil::Python;
use super::object::{Bound, Owned};

thread_local! {
    /// One slot per native call running on the thread, innermost last:
    /// the first exception a callback raised during it.
    static FRAMES: RefCell<Vec<Option<PyErr>>> = const { RefCell::new(Vec::new()) };
}

/// The exception frame of a running native call.
#[derive(Debug)]
pub(crate) struct ErrorFrame {
    depth: usize,
}

impl ErrorFrame {
    /// Opens the frame of a native call.
    pub(crate) fn push() -> Self {
        FRAMES.with(|frames| {
            let mut frames = frames.borrow_mut();
            frames.push(None);
            Self {
                depth: frames.len(),
            }
        })
    }

    /// Closes the frame, returning the exception a callback raised in it.
    pub(crate) fn pop(self) -> Option<PyErr> {
        let depth = self.depth;
        std::mem::forget(self);
        close(depth)
    }
}

impl Drop for ErrorFrame {
    /// Closes the frame if the native call unwound past it.
    fn drop(&mut self) {
        drop(close(self.depth));
    }
}

/// Closes the frames down to `depth`, returning the exception of the
/// frame at `depth`.
fn close(depth: usize) -> Option<PyErr> {
    FRAMES.with(|frames| {
        let mut frames = frames.borrow_mut();
        frames.truncate(depth);
        frames.pop().flatten()
    })
}

/// Whether the innermost frame already holds an exception.
fn failed() -> bool {
    FRAMES.with(|frames| frames.borrow().last().is_some_and(Option::is_some))
}

/// Keeps `err` in the innermost frame (the first one wins); outside any
/// native call there is no Python caller to raise it to, so it becomes a
/// `RuntimeWarning`.
fn keep(py: Python<'_>, err: PyErr) {
    let kept = FRAMES.with(|frames| match frames.borrow_mut().last_mut() {
        Some(slot) => {
            slot.get_or_insert(err);
            None
        }
        None => Some(err),
    });
    if let Some(err) = kept {
        let message = err.message(py);
        let _ = super::error::runtime_warning(
            py,
            &format!("exception in a pltrs callback outside a native call: {message}"),
        );
    }
}

/// A Python callable that pltrs (Rust) code calls.
#[derive(Clone, Debug)]
pub(crate) struct Callback {
    callable: Owned,
}

impl Callback {
    /// Calls it with `args` and converts the result; `None` when it
    /// raised (the exception goes to the running native call) or an
    /// earlier callback of the same call did.
    pub(crate) fn call<R>(&self, args: impl CallArgs) -> Option<R>
    where
        R: for<'py> FromPy<'py>,
    {
        Python::with_gil(|py| {
            if failed() {
                return None;
            }
            let outcome = args
                .into_args(py)
                .and_then(|args| self.callable.bind(py).call(&args, &[]))
                .and_then(|result| result.extract::<R>());
            match outcome {
                Ok(value) => Some(value),
                Err(err) => {
                    keep(py, err);
                    None
                }
            }
        })
    }
}

impl FromPy<'_> for Callback {
    /// Any callable.
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let callable = object.getattr("__call__").is_ok();
        if callable {
            Ok(Self {
                callable: object.clone().unbind(),
            })
        } else {
            Err(PyErr::type_error(format!(
                "'{}' object is not callable",
                object.type_name()
            )))
        }
    }
}

impl IntoPy for Callback {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        Ok(self.callable.bind(py))
    }
}

/// The positional arguments of a callback: a tuple of values.
pub(crate) trait CallArgs {
    /// The arguments as Python objects.
    fn into_args(self, py: Python<'_>) -> PyResult<Vec<Bound<'_>>>;
}

impl CallArgs for () {
    fn into_args(self, _py: Python<'_>) -> PyResult<Vec<Bound<'_>>> {
        Ok(Vec::new())
    }
}

/// Tuples of values are callback arguments.
macro_rules! call_args {
    ($(($($name:ident: $ty:ident),+);)*) => {$(
        impl<$($ty: IntoPy),+> CallArgs for ($($ty,)+) {
            fn into_args(self, py: Python<'_>) -> PyResult<Vec<Bound<'_>>> {
                let ($($name,)+) = self;
                Ok(vec![$($name.into_py(py)?),+])
            }
        }
    )*};
}

call_args! {
    (a: A);
    (a: A, b: B);
    (a: A, b: B, c: C);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::function::{IntoNative, function};
    use crate::binding::testing::{eval, with_python};

    /// Calls the native function `rust`, made in a scratch module, with the
    /// Python callable `callable`.
    fn run_with<'py, Args>(
        py: Python<'py>,
        rust: impl IntoNative<Args>,
        callable: &Bound<'py>,
    ) -> PyResult<Bound<'py>> {
        let module = eval(py, "import types", "types.ModuleType('scratch')");
        let native = function(py, &module, "run", rust.into_body("run")).unwrap();
        native.call(std::slice::from_ref(callable), &[])
    }

    #[test]
    fn callbacks_call_python_and_convert_the_result() {
        with_python(|py| {
            let label = eval(py, "", "lambda x, pos: f'{x}:{pos}'");
            let callback: Callback = label.extract().unwrap();
            assert_eq!(
                callback.call::<String>((1.5, Some(2_usize))).unwrap(),
                "1.5:2"
            );
            assert!(
                eval(py, "", "3").extract::<Callback>().is_err(),
                "not callable"
            );
        });
    }

    #[test]
    fn an_exception_in_a_callback_raises_from_the_native_call() {
        with_python(|py| {
            let raising = eval(py, "", "lambda: 1 / 0");
            let calls_twice = |callback: Callback| {
                let first = callback.call::<f64>(());
                let second = callback.call::<f64>(());
                Ok((first, second))
            };
            let err = run_with(py, calls_twice, &raising).unwrap_err();
            assert_eq!(err.class_name(py), "ZeroDivisionError");
        });
    }

    #[test]
    fn later_callbacks_of_a_failed_call_are_skipped() {
        with_python(|py| {
            let counting = eval(
                py,
                "calls = []",
                "lambda: calls.append(1) or (1 / 0 if len(calls) == 1 else 2.0)",
            );
            let calls_twice = |callback: Callback| {
                assert!(callback.call::<f64>(()).is_none());
                assert!(callback.call::<f64>(()).is_none(), "skipped");
                Ok(())
            };
            assert!(run_with(py, calls_twice, &counting).is_err());
            let globals = counting.getattr("__globals__").unwrap();
            let name = "calls".into_py(py).unwrap();
            let calls = globals.call_method("__getitem__", &[name]).unwrap();
            assert_eq!(calls.extract::<Vec<i64>>().unwrap(), [1], "called once");
        });
    }

    #[test]
    fn a_nested_native_call_keeps_its_own_frame() {
        with_python(|py| {
            let succeed = eval(py, "", "lambda: 2.0");
            let outer = |callback: Callback| {
                let inner = ErrorFrame::push();
                let value = callback.call::<f64>(());
                assert!(inner.pop().is_none(), "the inner frame saw no exception");
                Ok(value)
            };
            let result = run_with(py, outer, &succeed).unwrap();
            assert_eq!(result.extract::<Option<f64>>().unwrap(), Some(2.0));
        });
    }
}
