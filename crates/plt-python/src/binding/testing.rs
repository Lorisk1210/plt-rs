//! An embedded interpreter for the unit tests (`build.rs` links the
//! libpython it found and names its home in `PLT_PYTHON_HOME`).

use std::sync::Once;

use super::gil::Python;
use super::object::Bound;
use crate::ffi;

/// Starts the interpreter once per test process, then releases the GIL
/// so that every test thread can take it.
fn start() {
    static START: Once = Once::new();
    START.call_once(|| {
        // SAFETY: called once, before any other use of the C API; the
        // configuration is made, filled with NUL-terminated strings, used
        // and freed here, and the GIL the initialising thread then holds
        // is released for `with_gil`.
        unsafe {
            assert_eq!(ffi::Py_IsInitialized(), 0, "nothing else starts Python");
            let config = ffi::PyInitConfig_Create();
            assert!(!config.is_null(), "a configuration");
            let home = std::ffi::CString::new(env!("PLT_PYTHON_HOME")).expect("a path");
            assert_eq!(
                ffi::PyInitConfig_SetStr(config, c"home".as_ptr(), home.as_ptr()),
                0
            );
            assert_eq!(
                ffi::PyInitConfig_SetInt(config, c"site_import".as_ptr(), 0),
                0
            );
            assert_eq!(
                ffi::PyInitConfig_SetInt(config, c"install_signal_handlers".as_ptr(), 0),
                0
            );
            assert_eq!(ffi::Py_InitializeFromInitConfig(config), 0, "Python starts");
            ffi::PyInitConfig_Free(config);
            ffi::PyEval_SaveThread();
        }
    });
}

/// Runs `f` with the GIL of the embedded interpreter.
pub(crate) fn with_python<R>(f: impl for<'py> FnOnce(Python<'py>) -> R) -> R {
    start();
    Python::with_gil(f)
}

/// Evaluates a Python expression, with `import`s in `setup` run first.
pub(crate) fn eval<'py>(py: Python<'py>, setup: &str, expression: &str) -> Bound<'py> {
    let builtins = import(py, "builtins");
    let namespace = super::containers::new_dict(py).expect("a dict");
    let exec = builtins.getattr("exec").expect("exec");
    exec.call(&[str(py, setup), namespace.clone()], &[])
        .expect("the setup runs");
    builtins
        .getattr("eval")
        .expect("eval")
        .call(&[str(py, expression), namespace], &[])
        .expect("the expression evaluates")
}

/// `import name`.
pub(crate) fn import<'py>(py: Python<'py>, name: &str) -> Bound<'py> {
    let name = std::ffi::CString::new(name).expect("a module name");
    // SAFETY: the GIL is held; a new reference or NULL.
    unsafe { Bound::from_owned_ptr(py, ffi::PyImport_ImportModule(name.as_ptr())) }
        .expect("the module imports")
}

/// A Python `str`.
pub(crate) fn str<'py>(py: Python<'py>, text: &str) -> Bound<'py> {
    use super::convert::IntoPy as _;
    text.into_py(py).expect("a str")
}

/// The extension module, initialised as Python would on import (the
/// native type, the registry and every matplotlib module).
pub(crate) fn extension(py: Python<'_>) -> Bound<'_> {
    let module = crate::PyInit__native().cast::<ffi::PyObject>();
    // SAFETY: the initialisation function returns a new reference, or NULL
    // with an exception set.
    unsafe { Bound::from_owned_ptr(py, module) }.expect("the extension initialises")
}
