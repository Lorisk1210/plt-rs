//! Native functions: Rust functions Python calls.
//!
//! A native function is a plain Rust function or closure of up to eight
//! arguments that convert from Python ([`FromPy`]), returning
//! `PyResult<R>` for a result that converts to Python ([`IntoPy`]):
//!
//! ```ignore
//! module.add("set_linewidth", |line: Line2D, width: f64| {
//!     line.set_linewidth(width);
//!     Ok(())
//! })?;
//! ```
//!
//! Native functions take positional arguments only, exactly as many as
//! the Rust function has: the Python class calling one has already dealt
//! with matplotlib's signature (defaults, keywords, aliases) and passes
//! `None` for what is absent. A function that needs the GIL token takes
//! it with `Python::with_gil`, which is free inside a native call.
//!
//! Every native function is the same C function, [`dispatch`], bound to a
//! capsule holding the Rust closure ([`Body`]).

use std::ffi::{CStr, CString, c_void};

use super::convert::{FromPy, IntoPy};
use super::error::{PyErr, PyResult};
use super::gil::{Python, entry, gil_held};
use super::object::Bound;
use super::{deferred, home, threads};
use crate::ffi;

/// The Rust side of a native function: arguments in, a result out.
pub(crate) type Body = Box<dyn for<'py> Fn(Python<'py>, &[Bound<'py>]) -> PyResult<Bound<'py>>>;

/// The capsule name marking a native function's body.
const BODY_CAPSULE: &CStr = c"pltrs._native.function";

/// A Rust function that can become a native function; `Args` is the
/// tuple of its argument types, which tells the impls for each arity
/// apart.
pub(crate) trait IntoNative<Args> {
    /// The function's body, checking arity and converting arguments;
    /// `name` is for messages.
    fn into_body(self, name: &'static str) -> Body;
}

/// Raises `TypeError` unless `given` arguments match the arity.
fn check_arity(name: &str, expected: usize, given: usize) -> PyResult<()> {
    if given == expected {
        Ok(())
    } else {
        Err(PyErr::type_error(format!(
            "{name}() takes exactly {expected} argument{} ({given} given)",
            if expected == 1 { "" } else { "s" }
        )))
    }
}

/// Implements [`IntoNative`] for functions of each arity.
macro_rules! into_native {
    ($($count:literal: ($($arg:ident: $ty:ident),*);)*) => {$(
        impl<Function, Output, $($ty),*> IntoNative<($($ty,)*)> for Function
        where
            Function: Fn($($ty),*) -> PyResult<Output> + 'static,
            Output: IntoPy,
            $($ty: for<'py> FromPy<'py>,)*
        {
            #[allow(unused_variables, unused_mut, reason = "the arity-0 expansion uses neither")]
            fn into_body(self, name: &'static str) -> Body {
                Box::new(move |py, args| {
                    check_arity(name, $count, args.len())?;
                    let mut args = args.iter();
                    $(let $arg = $ty::from_py(args.next().expect("the arity was checked"))?;)*
                    self($($arg),*)?.into_py(py)
                })
            }
        }
    )*};
}

into_native! {
    0: ();
    1: (a: A);
    2: (a: A, b: B);
    3: (a: A, b: B, c: C);
    4: (a: A, b: B, c: C, d: D);
    5: (a: A, b: B, c: C, d: D, e: E);
    6: (a: A, b: B, c: C, d: D, e: E, f: F);
    7: (a: A, b: B, c: C, d: D, e: E, f: F, g: G);
    8: (a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H);
}

/// A builtin function object running `body`, named `name`, of the module
/// named `module`.
pub(crate) fn function<'py>(
    py: Python<'py>,
    module: &Bound<'py>,
    name: &'static str,
    body: Body,
) -> PyResult<Bound<'py>> {
    // A function's `PyMethodDef` must outlive it, and module functions
    // live as long as the interpreter: the definition and its name are
    // leaked, once per function made at import.
    let name = CString::new(name).map_err(|_| PyErr::value_error("a NUL in a function name"))?;
    let definition = Box::leak(Box::new(ffi::PyMethodDef {
        ml_name: Box::leak(name.into_boxed_c_str()).as_ptr(),
        ml_meth: Some(dispatch),
        ml_flags: ffi::METH_FASTCALL,
        ml_doc: std::ptr::null(),
    }));
    let body = Box::into_raw(Box::new(body));
    // SAFETY: the GIL is held; the capsule takes the boxed body and frees
    // it in `drop_body`.
    let capsule = unsafe {
        Bound::from_owned_ptr(
            py,
            ffi::PyCapsule_New(
                body.cast::<c_void>(),
                BODY_CAPSULE.as_ptr(),
                Some(drop_body),
            ),
        )
    };
    let capsule = match capsule {
        Ok(capsule) => capsule,
        Err(err) => {
            // SAFETY: the capsule was not made, so the body is still ours.
            drop(unsafe { Box::from_raw(body) });
            return Err(err);
        }
    };
    let module_name = module.getattr("__name__")?;
    // SAFETY: a leaked (so static) definition, the capsule as `self`, the
    // module's name; a new reference or NULL.
    unsafe {
        Bound::from_owned_ptr(
            py,
            ffi::PyCFunction_NewEx(definition, capsule.as_ptr(), module_name.as_ptr()),
        )
    }
}

/// Frees a native function's body when its capsule goes.
unsafe extern "C" fn drop_body(capsule: *mut ffi::PyObject) {
    // SAFETY: the capsule was made by `function` around a boxed body,
    // which is dropped exactly once, here.
    unsafe {
        let body = ffi::PyCapsule_GetPointer(capsule, BODY_CAPSULE.as_ptr()).cast::<Body>();
        if !body.is_null() {
            drop(Box::from_raw(body));
        }
    }
}

/// The C function behind every native function: runs the body in the
/// capsule bound as `self`, once the work Python deferred to the next
/// native call is done ([`deferred`]) and the figures of its artist
/// arguments are no longer in use by another thread ([`threads`]), and on
/// the main thread when an argument is tied to it, this is another thread
/// and the main thread serves others ([`home`]).
unsafe extern "C" fn dispatch(
    capsule: *mut ffi::PyObject,
    args: *const *mut ffi::PyObject,
    nargs: ffi::Py_ssize_t,
) -> *mut ffi::PyObject {
    // SAFETY: CPython calls a `METH_FASTCALL` function with the GIL held,
    // `self` (the capsule `function` bound), and `nargs` live borrowed
    // arguments at `args`; without a GIL scope open, this is the thread's
    // outermost native call.
    unsafe {
        let count = usize::try_from(nargs).unwrap_or_default();
        let args = if count == 0 {
            &[][..]
        } else {
            std::slice::from_raw_parts(args, count)
        };
        if !deferred::run() {
            return std::ptr::null_mut();
        }
        if !gil_held() {
            threads::wait_for_figures(args);
        }
        let on_main = if home::belongs_to_main(args) {
            home::call_on_main(call_here, capsule, args)
        } else {
            None
        };
        on_main.unwrap_or_else(|| call_here(capsule, args))
    }
}

/// Runs the body in `capsule` with `args` on this thread.
///
/// # Safety
///
/// The GIL is held, `capsule` is a native function's capsule and `args`
/// are live.
unsafe fn call_here(
    capsule: *mut ffi::PyObject,
    args: &[*mut ffi::PyObject],
) -> *mut ffi::PyObject {
    // SAFETY: (caller) the GIL is held and the objects are live.
    unsafe {
        entry(|py| {
            let body = ffi::PyCapsule_GetPointer(capsule, BODY_CAPSULE.as_ptr()).cast::<Body>();
            let body = body.as_ref().ok_or_else(|| PyErr::fetch(py))?;
            let args: Vec<Bound<'_>> = args
                .iter()
                .map(|&arg| Bound::from_borrowed_ptr(py, arg))
                .collect();
            body(py, &args)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{eval, with_python};

    /// A native function named `name` running `rust`, in a scratch module.
    fn native<'py, Args>(
        py: Python<'py>,
        name: &'static str,
        rust: impl IntoNative<Args>,
    ) -> Bound<'py> {
        let module = eval(py, "import types", "types.ModuleType('scratch')");
        function(py, &module, name, rust.into_body(name)).unwrap()
    }

    /// Calls `function` with float arguments.
    fn call<'py>(function: &Bound<'py>, args: &[f64]) -> PyResult<Bound<'py>> {
        let py = function.py();
        let args: Vec<Bound<'py>> = args.iter().map(|&v| v.into_py(py).unwrap()).collect();
        function.call(&args, &[])
    }

    #[test]
    fn arguments_convert_and_results_return() {
        with_python(|py| {
            let add = native(py, "add", |a: f64, b: f64| Ok(a + b));
            assert_eq!(
                call(&add, &[1.0, 2.0]).unwrap().extract::<f64>().unwrap(),
                3.0
            );
            assert_eq!(
                add.getattr("__name__")
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                "add"
            );
            let nothing = native(py, "nothing", || Ok(()));
            assert!(call(&nothing, &[]).unwrap().is_none());
        });
    }

    #[test]
    fn wrong_calls_raise_type_errors() {
        with_python(|py| {
            let add = native(py, "add", |a: f64, b: f64| Ok(a + b));
            let err = call(&add, &[1.0]).unwrap_err();
            assert_eq!(err.class_name(py), "TypeError");
            assert_eq!(err.message(py), "add() takes exactly 2 arguments (1 given)");
            let text = "x".into_py(py).unwrap();
            let err = add.call(&[text.clone(), text], &[]).unwrap_err();
            assert_eq!(err.class_name(py), "TypeError");
            let keyword = 1.0.into_py(py).unwrap();
            assert!(
                add.call(&[], &[("a", keyword)]).is_err(),
                "no keyword arguments"
            );
        });
    }

    #[test]
    fn errors_and_panics_raise() {
        with_python(|py| {
            let refuse = native(py, "refuse", || -> PyResult<()> {
                Err(PyErr::value_error("refused"))
            });
            let err = call(&refuse, &[]).unwrap_err();
            assert_eq!(
                (err.class_name(py), err.message(py)),
                ("ValueError".into(), "refused".into())
            );
            let panic = native(py, "panic", || -> PyResult<()> { panic!("boom") });
            let err = call(&panic, &[]).unwrap_err();
            assert_eq!(err.class_name(py), "RuntimeError");
            assert!(err.message(py).contains("boom"));
        });
    }
}
