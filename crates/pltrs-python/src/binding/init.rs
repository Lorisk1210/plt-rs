//! The extension module's definition and initialisation.

use std::cell::UnsafeCell;

use super::error::PyResult;
use super::gil::entry;
use super::module::Module;
use super::{native, registry};
use crate::ffi;

/// The module definition CPython creates `_native` from.
struct Definition(UnsafeCell<ffi::PyModuleDef>);

// SAFETY: CPython reads and writes the definition (it fills in the
// header) only while creating the module, with the GIL held; Rust never
// touches it otherwise.
unsafe impl Sync for Definition {}

static DEFINITION: Definition = Definition(UnsafeCell::new(ffi::PyModuleDef {
    m_base: ffi::PyModuleDef_Base {
        ob_base: ffi::PyObject {
            ob_refcnt: 1,
            ob_type: std::ptr::null_mut(),
        },
        m_init: None,
        m_index: 0,
        m_copy: std::ptr::null_mut(),
    },
    m_name: c"_native".as_ptr(),
    m_doc: c"pltrs (Rust) for pltrs (Python): the native side of its modules.".as_ptr(),
    m_size: -1,
    m_methods: std::ptr::null_mut(),
    m_slots: std::ptr::null_mut(),
    m_traverse: std::ptr::null_mut(),
    m_clear: std::ptr::null_mut(),
    m_free: std::ptr::null_mut(),
}));

/// Creates the extension module with the native type and `register`,
/// then lets `fill` add the matplotlib modules: what `PyInit__native`
/// returns.
pub(crate) fn init_module(fill: fn(&Module<'_>) -> PyResult<()>) -> *mut ffi::PyObject {
    // SAFETY: CPython calls `PyInit__native` with the GIL held; the
    // definition is static.
    unsafe {
        entry(|py| {
            let module = Module::create(py, DEFINITION.0.get())?;
            native::add_type(&module)?;
            registry::add_to(&module)?;
            fill(&module)?;
            Ok(module.into_object())
        })
    }
}
