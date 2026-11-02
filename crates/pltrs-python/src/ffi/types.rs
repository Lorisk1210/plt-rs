//! Making types, functions and modules: `PyType_FromSpec`, method
//! definitions and module definitions.

use std::ffi::{c_char, c_int, c_uint, c_void};

use super::{Py_ssize_t, PyObject, PyTypeObject};

/// `Py_TPFLAGS_DEFAULT` in the limited API.
pub(crate) const Py_TPFLAGS_DEFAULT: c_uint = 0;
/// `Py_TPFLAGS_DISALLOW_INSTANTIATION`: Python code cannot call the type.
pub(crate) const Py_TPFLAGS_DISALLOW_INSTANTIATION: c_uint = 1 << 7;
/// `Py_TPFLAGS_IMMUTABLETYPE`: the type's attributes cannot be set.
pub(crate) const Py_TPFLAGS_IMMUTABLETYPE: c_uint = 1 << 8;

/// `Py_tp_dealloc`.
pub(crate) const Py_tp_dealloc: c_int = 52;
/// `Py_tp_doc`.
pub(crate) const Py_tp_doc: c_int = 56;
/// `Py_tp_free`.
pub(crate) const Py_tp_free: c_int = 74;
/// `Py_tp_repr`.
pub(crate) const Py_tp_repr: c_int = 66;

/// `METH_FASTCALL`: a C function taking `(self, args, nargs)`.
pub(crate) const METH_FASTCALL: c_int = 0x0080;

/// `PYTHON_ABI_VERSION`, what `PyModule_Create2` is told under the
/// limited API.
pub(crate) const PYTHON_ABI_VERSION: c_int = 3;

/// A `METH_FASTCALL` C function (`_PyCFunctionFast`).
pub(crate) type PyCFunctionFast = unsafe extern "C" fn(
    slf: *mut PyObject,
    args: *const *mut PyObject,
    nargs: Py_ssize_t,
) -> *mut PyObject;

/// `destructor`: a `tp_dealloc` slot.
pub(crate) type destructor = unsafe extern "C" fn(o: *mut PyObject);

/// `reprfunc`: a `tp_repr` slot.
pub(crate) type reprfunc = unsafe extern "C" fn(o: *mut PyObject) -> *mut PyObject;

/// `freefunc`: a `tp_free` slot.
pub(crate) type freefunc = unsafe extern "C" fn(p: *mut c_void);

/// `PyType_Slot`: one slot of a type spec.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct PyType_Slot {
    /// The slot id (`Py_tp_*`), 0 for the terminator.
    pub(crate) slot: c_int,
    /// The slot's function or value.
    pub(crate) pfunc: *mut c_void,
}

/// `PyType_Spec`: what `PyType_FromSpec` makes a type from.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct PyType_Spec {
    /// The qualified name, `module.Type`.
    pub(crate) name: *const c_char,
    /// The instance size; negative: the size of the type's own data,
    /// reached through `PyObject_GetTypeData` (3.12).
    pub(crate) basicsize: c_int,
    /// The size of variable items (0).
    pub(crate) itemsize: c_int,
    /// `Py_TPFLAGS_*`.
    pub(crate) flags: c_uint,
    /// The slots, ending with a zero slot.
    pub(crate) slots: *mut PyType_Slot,
}

/// `PyMethodDef`: a C function's name, implementation and convention.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct PyMethodDef {
    /// `__name__`.
    pub(crate) ml_name: *const c_char,
    /// The implementation, cast to `PyCFunction`.
    pub(crate) ml_meth: Option<PyCFunctionFast>,
    /// `METH_*`.
    pub(crate) ml_flags: c_int,
    /// `__doc__`, or NULL.
    pub(crate) ml_doc: *const c_char,
}

/// `PyModuleDef_Base`: the object header of a module definition.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct PyModuleDef_Base {
    /// `PyObject_HEAD`.
    pub(crate) ob_base: PyObject,
    /// `m_init`.
    pub(crate) m_init: Option<unsafe extern "C" fn() -> *mut PyObject>,
    /// `m_index`.
    pub(crate) m_index: Py_ssize_t,
    /// `m_copy`.
    pub(crate) m_copy: *mut PyObject,
}

/// `PyModuleDef`: a module definition for `PyModule_Create2`.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct PyModuleDef {
    /// `PyModuleDef_HEAD_INIT`.
    pub(crate) m_base: PyModuleDef_Base,
    /// The module's name.
    pub(crate) m_name: *const c_char,
    /// Its docstring.
    pub(crate) m_doc: *const c_char,
    /// Its per-module state size (-1: no state, no sub-interpreters).
    pub(crate) m_size: Py_ssize_t,
    /// Its functions (NULL: added later).
    pub(crate) m_methods: *mut PyMethodDef,
    /// Multi-phase slots (NULL: single-phase initialisation).
    pub(crate) m_slots: *mut c_void,
    /// `m_traverse`.
    pub(crate) m_traverse: *mut c_void,
    /// `m_clear`.
    pub(crate) m_clear: *mut c_void,
    /// `m_free`.
    pub(crate) m_free: *mut c_void,
}

#[cfg_attr(windows, link(name = "python3", kind = "raw-dylib"))]
unsafe extern "C" {
    /// `PyType_FromSpec`: a new heap type.
    pub(crate) fn PyType_FromSpec(spec: *mut PyType_Spec) -> *mut PyObject;
    /// `PyType_GetSlot`: a slot's function of a type.
    pub(crate) fn PyType_GetSlot(t: *mut PyTypeObject, slot: c_int) -> *mut c_void;
    /// `PyType_GenericAlloc`: a zeroed instance of the type.
    pub(crate) fn PyType_GenericAlloc(t: *mut PyTypeObject, nitems: Py_ssize_t) -> *mut PyObject;
    /// `PyObject_GetTypeData`: the data a type with a negative
    /// `basicsize` reserved in an instance of it (3.12).
    pub(crate) fn PyObject_GetTypeData(o: *mut PyObject, cls: *mut PyTypeObject) -> *mut c_void;

    /// `PyCFunction_NewEx`: a builtin function bound to `slf`.
    pub(crate) fn PyCFunction_NewEx(
        ml: *mut PyMethodDef,
        slf: *mut PyObject,
        module: *mut PyObject,
    ) -> *mut PyObject;

    /// `PyModule_Create2`: a module from its definition.
    pub(crate) fn PyModule_Create2(def: *mut PyModuleDef, apiver: c_int) -> *mut PyObject;
    /// `PyModule_New`: an empty module.
    pub(crate) fn PyModule_New(name: *const c_char) -> *mut PyObject;
    /// `PyModule_AddObjectRef`: sets a module attribute, without stealing.
    pub(crate) fn PyModule_AddObjectRef(
        module: *mut PyObject,
        name: *const c_char,
        value: *mut PyObject,
    ) -> c_int;
}
