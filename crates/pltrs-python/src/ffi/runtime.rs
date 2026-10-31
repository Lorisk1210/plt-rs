//! The interpreter: the GIL, imports and, for the unit tests, embedding.

#[cfg(test)]
use std::ffi::c_int;
use std::ffi::{c_char, c_void};

use super::PyObject;

/// `PyGILState_STATE`: what `PyGILState_Ensure` found, for
/// `PyGILState_Release`.
pub(crate) type PyGILState_STATE = std::ffi::c_int;

unsafe extern "C" {
    /// `PyGILState_Ensure`: takes the GIL for the current thread
    /// (re-entrantly), creating its thread state if needed.
    pub(crate) fn PyGILState_Ensure() -> PyGILState_STATE;
    /// `PyGILState_Release`: undoes the matching `PyGILState_Ensure`.
    pub(crate) fn PyGILState_Release(state: PyGILState_STATE);
    /// `PyImport_ImportModule`: `import name`, a new reference.
    pub(crate) fn PyImport_ImportModule(name: *const c_char) -> *mut PyObject;
    /// `PyEval_SaveThread`: releases the GIL the current thread holds,
    /// returning its thread state.
    pub(crate) fn PyEval_SaveThread() -> *mut c_void;
    /// `PyEval_RestoreThread`: takes the GIL back for the thread state
    /// `PyEval_SaveThread` returned.
    pub(crate) fn PyEval_RestoreThread(state: *mut c_void);
}

/// `PyInitConfig` (PEP 741, 3.14), opaque.
#[cfg(test)]
#[repr(C)]
#[derive(Debug)]
pub(crate) struct PyInitConfig {
    _opaque: [u8; 0],
}

#[cfg(test)]
unsafe extern "C" {
    /// `Py_IsInitialized`.
    pub(crate) fn Py_IsInitialized() -> c_int;
    /// `PyInitConfig_Create`: an initialisation configuration.
    pub(crate) fn PyInitConfig_Create() -> *mut PyInitConfig;
    /// `PyInitConfig_SetStr`: sets an option to a UTF-8 string.
    pub(crate) fn PyInitConfig_SetStr(
        config: *mut PyInitConfig,
        name: *const c_char,
        value: *const c_char,
    ) -> c_int;
    /// `PyInitConfig_SetInt`: sets an option to an integer.
    pub(crate) fn PyInitConfig_SetInt(
        config: *mut PyInitConfig,
        name: *const c_char,
        value: i64,
    ) -> c_int;
    /// `Py_InitializeFromInitConfig`: starts the interpreter.
    pub(crate) fn Py_InitializeFromInitConfig(config: *mut PyInitConfig) -> c_int;
    /// `PyInitConfig_Free`.
    pub(crate) fn PyInitConfig_Free(config: *mut PyInitConfig);
}
