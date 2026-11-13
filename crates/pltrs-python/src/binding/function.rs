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
//! capsule holding the Rust closure ([`NativeFunction`]).
//!
//! A native call Python code makes while another native call of its
//! thread runs (a callback pltrs (Rust) calls while it lays a figure out or
//! draws it) may reach that figure, which is in use. It then stops
//! there ([`pltrs::figure::reenter`]): a function returning nothing (a
//! change) is applied once the figure is free ([`reentrant`]), any other
//! raises `RuntimeError`.

use std::ffi::{CStr, CString, c_void};

use super::convert::{FromPy, IntoPy};
use super::error::{PyErr, PyResult};
use super::gil::{Python, entry, gil_held};
use super::object::Bound;
use super::{deferred, home, reentrant, stack, threads};
use crate::ffi;

/// The body of a native function: arguments in, a result out.
type Body = Box<dyn for<'py> Fn(Python<'py>, &[Bound<'py>]) -> PyResult<Bound<'py>>>;

/// The Rust side of a native function.
pub(crate) struct NativeFunction {
    /// Its body.
    body: Body,
    /// Whether it returns nothing (`None`): a change, which a call
    /// reaching a figure in use makes once the figure is free.
    returns_nothing: bool,
}

/// The capsule name marking a native function's body.
const BODY_CAPSULE: &CStr = c"pltrs._native.function";

/// A Rust function that can become a native function; `Args` is the
/// tuple of its argument types, which tells the impls for each arity
/// apart.
pub(crate) trait IntoNative<Args> {
    /// The native function, checking arity and converting arguments;
    /// `name` is for messages.
    fn into_native(self, name: &'static str) -> NativeFunction;
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
            fn into_native(self, name: &'static str) -> NativeFunction {
                NativeFunction {
                    body: Box::new(move |py, args| {
                        check_arity(name, $count, args.len())?;
                        let mut args = args.iter();
                        $(let $arg = $ty::from_py(args.next().expect("the arity was checked"))?;)*
                        self($($arg),*)?.into_py(py)
                    }),
                    returns_nothing: Output::IS_NONE,
                }
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

/// A builtin function object running `native`, named `name`, of the module
/// named `module`.
pub(crate) fn function<'py>(
    py: Python<'py>,
    module: &Bound<'py>,
    name: &'static str,
    native: NativeFunction,
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
    let native = Box::into_raw(Box::new(native));
    // SAFETY: the GIL is held; the capsule takes the boxed function and
    // frees it in `drop_native`.
    let capsule = unsafe {
        Bound::from_owned_ptr(
            py,
            ffi::PyCapsule_New(
                native.cast::<c_void>(),
                BODY_CAPSULE.as_ptr(),
                Some(drop_native),
            ),
        )
    };
    let capsule = match capsule {
        Ok(capsule) => capsule,
        Err(err) => {
            // SAFETY: the capsule was not made, so the function is still
            // ours.
            drop(unsafe { Box::from_raw(native) });
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

/// Frees a native function's Rust side when its capsule goes.
unsafe extern "C" fn drop_native(capsule: *mut ffi::PyObject) {
    // SAFETY: the capsule was made by `function` around a boxed function,
    // which is dropped exactly once, here.
    unsafe {
        let native =
            ffi::PyCapsule_GetPointer(capsule, BODY_CAPSULE.as_ptr()).cast::<NativeFunction>();
        if !native.is_null() {
            drop(Box::from_raw(native));
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

/// Runs the function in `capsule` with `args` on this thread: as it is
/// when it is the thread's outermost native call, else as a call that may
/// reach a figure in use ([`call_nested`]).
///
/// # Safety
///
/// The GIL is held, `capsule` is a native function's capsule and `args`
/// are live.
unsafe fn call_here(
    capsule: *mut ffi::PyObject,
    args: &[*mut ffi::PyObject],
) -> *mut ffi::PyObject {
    let nested = gil_held();
    // SAFETY: (caller) the GIL is held and the objects are live.
    unsafe {
        entry(|py| {
            let capsule = Bound::from_borrowed_ptr(py, capsule);
            let args: Vec<Bound<'_>> = args
                .iter()
                .map(|&arg| Bound::from_borrowed_ptr(py, arg))
                .collect();
            if nested {
                call_nested(&capsule, &args)
            } else {
                (native_of(&capsule)?.body)(py, &args)
            }
        })
    }
}

/// The Rust side of the native function whose capsule is `capsule`.
fn native_of<'a>(capsule: &'a Bound<'_>) -> PyResult<&'a NativeFunction> {
    // SAFETY: the GIL is held (`capsule` is bound); the pointer is the
    // boxed function `function` gave the capsule, which lives as long as
    // the capsule, or NULL with an exception set for another object.
    unsafe {
        ffi::PyCapsule_GetPointer(capsule.as_ptr(), BODY_CAPSULE.as_ptr())
            .cast::<NativeFunction>()
            .as_ref()
            .ok_or_else(|| PyErr::fetch(capsule.py()))
    }
}

/// Calls the function in `capsule` with `args` from inside another native
/// call of the thread, whose figures may be in use: a call reaching one
/// stops there; a change (a function returning nothing) is then made once
/// the figure is free, as the outermost native call ends ([`reentrant`]),
/// and any other call raises `RuntimeError`. A call with little of the
/// thread's stack left raises `RecursionError` ([`stack`]).
pub(super) fn call_nested<'py>(capsule: &Bound<'py>, args: &[Bound<'py>]) -> PyResult<Bound<'py>> {
    let py = capsule.py();
    stack::check()?;
    match try_nested(capsule, args)? {
        Ok(result) => result,
        Err(_) if native_of(capsule)?.returns_nothing => {
            reentrant::push(capsule, args);
            Ok(Bound::none(py))
        }
        Err(in_use) => Err(reentrant::in_use_error(in_use)),
    }
}

/// Calls the function in `capsule` with `args`, which stops where it
/// reaches a figure in use (`Err` inside).
pub(super) fn try_nested<'py>(
    capsule: &Bound<'py>,
    args: &[Bound<'py>],
) -> PyResult<Result<PyResult<Bound<'py>>, pltrs::figure::FigureInUse>> {
    let native = native_of(capsule)?;
    Ok(pltrs::figure::reenter(|| (native.body)(capsule.py(), args)))
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
        function(py, &module, name, rust.into_native(name)).unwrap()
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
