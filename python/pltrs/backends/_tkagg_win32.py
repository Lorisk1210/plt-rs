"""
The Windows part of the Tk helper (`._tkagg`): per-monitor DPI awareness
of Tk windows, which matplotlib's C helper provides, through ``ctypes``.

A Tk window of a per-monitor DPI aware process gets ``WM_DPICHANGED``
when it moves to a screen of another DPI. `enable_dpi_awareness` hooks
(subclasses) the window's procedure to catch it and set the Tcl variable
``window_dpi<frame handle>`` of the window's interpreter to the new DPI,
which the Tk backend traces. As in the C helper, the hook calls Tcl itself
(``Tcl_SetVar2``) and no tkinter code: tkinter holds its own lock while a
window procedure runs.
"""

import ctypes
from ctypes import wintypes

_user32 = ctypes.WinDLL("user32")
_comctl32 = ctypes.WinDLL("comctl32")
_kernel32 = ctypes.WinDLL("kernel32")

#: The message a window gets when its DPI changes.
WM_DPICHANGED = 0x02E0
#: The last message a window gets.
WM_NCDESTROY = 0x0082
#: Windows' ``DPI_AWARENESS_CONTEXT`` handles of per-monitor awareness.
_PER_MONITOR_CONTEXTS = (-4, -3)  # ..._PER_MONITOR_AWARE_V2, ..._PER_MONITOR_AWARE

_LRESULT = ctypes.c_ssize_t
_SUBCLASSPROC = ctypes.WINFUNCTYPE(
    _LRESULT, wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM,
    ctypes.c_size_t, ctypes.c_size_t)
_TCL_SETVAR2 = ctypes.CFUNCTYPE(
    ctypes.c_char_p, ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p,
    ctypes.c_char_p, ctypes.c_int)

_comctl32.SetWindowSubclass.argtypes = [
    wintypes.HWND, _SUBCLASSPROC, ctypes.c_size_t, ctypes.c_size_t]
_comctl32.SetWindowSubclass.restype = wintypes.BOOL
_comctl32.RemoveWindowSubclass.argtypes = [wintypes.HWND, _SUBCLASSPROC, ctypes.c_size_t]
_comctl32.RemoveWindowSubclass.restype = wintypes.BOOL
_comctl32.DefSubclassProc.argtypes = [
    wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM]
_comctl32.DefSubclassProc.restype = _LRESULT
_kernel32.GetCurrentProcess.restype = wintypes.HANDLE
_kernel32.K32EnumProcessModules.argtypes = [
    wintypes.HANDLE, ctypes.POINTER(wintypes.HMODULE), wintypes.DWORD,
    ctypes.POINTER(wintypes.DWORD)]
_kernel32.K32EnumProcessModules.restype = wintypes.BOOL
_kernel32.GetProcAddress.argtypes = [wintypes.HMODULE, ctypes.c_char_p]
_kernel32.GetProcAddress.restype = ctypes.c_void_p


def _loaded_modules():
    """The handles of the modules (DLLs) the process has loaded."""
    process = _kernel32.GetCurrentProcess()
    size = ctypes.sizeof(wintypes.HMODULE)
    count = 256
    while True:
        modules = (wintypes.HMODULE * count)()
        needed = wintypes.DWORD()
        if not _kernel32.K32EnumProcessModules(
                process, modules, ctypes.sizeof(modules), ctypes.byref(needed)):
            raise ctypes.WinError()
        if needed.value <= ctypes.sizeof(modules):
            return modules[:needed.value // size]
        count = needed.value // size


def _find_tcl_setvar2():
    """``Tcl_SetVar2`` of the Tcl library tkinter loaded, found among the
    process' modules as the C helper finds it."""
    for module in _loaded_modules():
        address = _kernel32.GetProcAddress(module, b"Tcl_SetVar2")
        if address:
            return _TCL_SETVAR2(address)
    raise ImportError("Failed to load Tcl_SetVar or Tcl_SetVar2")


_tcl_setvar2 = _find_tcl_setvar2()


@_SUBCLASSPROC
def _dpi_subclass_proc(hwnd, message, wparam, lparam, subclass_id, interp):
    """The hook of a Tk window's procedure: a new DPI goes to the window's
    ``window_dpi<hwnd>`` Tcl variable (its low word; both words are
    equal); the hook removes itself when the window goes."""
    if message == WM_DPICHANGED:
        _tcl_setvar2(interp, f"window_dpi{hwnd}".encode(), None,
                     str(wparam & 0xFFFF).encode(), 0)
        return 0
    if message == WM_NCDESTROY:
        _comctl32.RemoveWindowSubclass(hwnd, _dpi_subclass_proc, subclass_id)
    return _comctl32.DefSubclassProc(hwnd, message, wparam, lparam)


def enable_dpi_awareness(frame_handle, interp):
    """
    Hook the Tk window *frame_handle* (its frame's handle) of the Tcl
    interpreter at address *interp* so that its DPI changes reach Tcl, when
    the window is per-monitor DPI aware: whether it is (False also where
    Windows is too old to tell).
    """
    get_context = getattr(_user32, "GetWindowDpiAwarenessContext", None)
    are_equal = getattr(_user32, "AreDpiAwarenessContextsEqual", None)
    if get_context is None or are_equal is None:
        return False
    get_context.argtypes = [wintypes.HWND]
    get_context.restype = ctypes.c_void_p
    are_equal.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
    are_equal.restype = wintypes.BOOL
    context = get_context(frame_handle)
    per_monitor = any(are_equal(context, aware) for aware in _PER_MONITOR_CONTEXTS)
    if per_monitor:
        _comctl32.SetWindowSubclass(frame_handle, _dpi_subclass_proc, 0, interp)
    return per_monitor
