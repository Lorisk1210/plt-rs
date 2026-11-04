//! Modules: the extension module `_native` and its submodules, one per
//! matplotlib module, which the files of [`crate::modules`] fill with
//! native functions.

use super::convert::IntoPy;
use super::error::PyResult;
use super::function::{IntoNative, function};
use super::gil::Python;
use super::object::{Bound, c_string};
use crate::ffi;

/// A module being filled with native functions and objects.
#[derive(Debug)]
pub(crate) struct Module<'py> {
    module: Bound<'py>,
    name: String,
}

impl<'py> Module<'py> {
    /// The extension module itself, from its static definition.
    ///
    /// # Safety
    ///
    /// `definition` is a complete `PyModuleDef` that lives as long as the
    /// interpreter, and the GIL is held (`py`).
    pub(crate) unsafe fn create(
        py: Python<'py>,
        definition: *mut ffi::PyModuleDef,
    ) -> PyResult<Self> {
        // SAFETY: (caller) a static definition and the GIL; a new
        // reference or NULL.
        let module = unsafe {
            Bound::from_owned_ptr(
                py,
                ffi::PyModule_Create2(definition, ffi::PYTHON_ABI_VERSION),
            )
        }?;
        // The import system names the module after the package it is
        // imported from: `pltrs._native`, or `matplotlib._native` when the
        // suite loads pltrs under matplotlib's name.
        let name = module.getattr("__name__")?.extract()?;
        Ok(Self { module, name })
    }

    /// The token of the GIL held while the module is filled.
    pub(crate) fn py(&self) -> Python<'py> {
        self.module.py()
    }

    /// The module's qualified name (`pltrs._native.lines`).
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// The module object.
    pub(crate) fn into_object(self) -> Bound<'py> {
        self.module
    }

    /// Adds a native function made from a Rust function of up to eight
    /// converted arguments.
    pub(crate) fn add<Args>(
        &self,
        name: &'static str,
        rust: impl IntoNative<Args>,
    ) -> PyResult<()> {
        let native = rust.into_native(name);
        let object = function(self.py(), &self.module, name, native)?;
        self.add_object(name, object)
    }

    /// Sets the attribute `name` of the module.
    pub(crate) fn add_object(&self, name: &str, value: impl IntoPy) -> PyResult<()> {
        let value = value.into_py(self.py())?;
        let name = c_string(name)?;
        // SAFETY: a live module and value, the GIL held; the value is not
        // stolen.
        let status = unsafe {
            ffi::PyModule_AddObjectRef(self.module.as_ptr(), name.as_ptr(), value.as_ptr())
        };
        super::object::check_status(self.py(), status)
    }

    /// Adds the submodule `name` (`_native.lines`, reached from Python as
    /// an attribute: `from ._native import lines`) and returns it.
    pub(crate) fn submodule(&self, name: &str) -> PyResult<Module<'py>> {
        let qualified = format!("{}.{name}", self.name);
        let c_name = c_string(&qualified)?;
        // SAFETY: the GIL is held; a new reference or NULL.
        let module =
            unsafe { Bound::from_owned_ptr(self.py(), ffi::PyModule_New(c_name.as_ptr())) }?;
        self.add_object(name, &module)?;
        Ok(Module {
            module,
            name: qualified,
        })
    }
}
