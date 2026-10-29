//! Python objects the package registers for native code: classes and
//! exceptions defined in Python that Rust needs, such as a warning class
//! to warn with or a class to instantiate.
//!
//! The package calls `_native.register(name, object)` once it has
//! defined them (`pltrs.testing.exceptions` registers
//! `ImageComparisonFailure`); native code looks them up with [`lookup`]. The native module never
//! imports the package by name, so it works whatever name the package is
//! imported under.

use std::collections::BTreeMap;
use std::sync::Mutex;

use super::convert::IntoPy;
use super::error::{PyErr, PyResult};
use super::gil::Python;
use super::module::Module;
use super::object::{Bound, Owned};

/// The registered objects, by name.
static REGISTRY: Mutex<BTreeMap<String, Owned>> = Mutex::new(BTreeMap::new());

/// Registers `object` under `name`, replacing an earlier one.
#[expect(
    clippy::unnecessary_wraps,
    reason = "a native function, in the form `Module::add` registers"
)]
fn register(name: String, object: Owned) -> PyResult<()> {
    REGISTRY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(name, object);
    Ok(())
}

/// The object registered as `name`.
pub(crate) fn lookup<'py>(py: Python<'py>, name: &str) -> PyResult<Bound<'py>> {
    REGISTRY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(name)
        .map(|object| object.bind(py))
        .ok_or_else(|| PyErr::runtime_error(format!("pltrs registered no object named {name:?}")))
}

/// An instance of the exception class registered as `name`, made with
/// `message` (`ImageComparisonFailure`, numpy's `LinAlgError`), to be
/// raised; the error making it, should that fail.
pub(crate) fn registered_error(name: &str, message: &str) -> PyErr {
    Python::with_gil(|py| {
        lookup(py, name)
            .and_then(|class| class.call(&[message.into_py(py)?], &[]))
            .map_or_else(|err| err, PyErr::from_instance)
    })
}

/// Adds `register` to the extension module.
pub(crate) fn add_to(module: &Module<'_>) -> PyResult<()> {
    module.add("register", register)
}
