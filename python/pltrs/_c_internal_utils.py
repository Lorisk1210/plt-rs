"""
Whether a display server is reachable, and what the GUI backends set up on
Windows (matplotlib's ``_c_internal_utils``).

pltrs (Rust) connects to the X server itself (and to a Wayland compositor's
socket), where matplotlib loads libX11 and libwayland-client. The Windows
helpers call the Windows API through ``ctypes`` there and, as matplotlib's,
do nothing elsewhere.
"""

import sys

from ._native import c_internal_utils as _utils


def display_is_valid():
    """
    Check whether the current X11 or Wayland display is valid.

    On Linux, returns True if either $DISPLAY is set and XOpenDisplay(NULL)
    succeeds, or $WAYLAND_DISPLAY is set and wl_display_connect(NULL)
    succeeds.

    On other platforms, always returns True.
    """
    return _utils.display_is_valid()


def xdisplay_is_valid():
    """
    Check whether the current X11 display is valid.

    On Linux, returns True if either $DISPLAY is set and XOpenDisplay(NULL)
    succeeds. Use this function if you need to specifically check for X11
    only (e.g., for Tkinter).

    On other platforms, always returns True.
    """
    return _utils.xdisplay_is_valid()


if sys.platform == "win32":
    import ctypes
    from ctypes import wintypes

    _shell32 = ctypes.WinDLL("shell32")
    _user32 = ctypes.WinDLL("user32")
    _ole32 = ctypes.WinDLL("ole32")

    def Win32_GetCurrentProcessExplicitAppUserModelID():
        """
        Wrapper for Windows's GetCurrentProcessExplicitAppUserModelID.

        Returns the explicit app user model id of the process; `OSError`
        when none was set.
        """
        appid = wintypes.LPWSTR()
        result = _shell32.GetCurrentProcessExplicitAppUserModelID(
            ctypes.byref(appid))
        if result != 0:
            raise ctypes.WinError(result)
        try:
            return appid.value
        finally:
            _ole32.CoTaskMemFree(appid)

    def Win32_SetCurrentProcessExplicitAppUserModelID(appid):
        """Wrapper for Windows's SetCurrentProcessExplicitAppUserModelID."""
        result = _shell32.SetCurrentProcessExplicitAppUserModelID(
            wintypes.LPCWSTR(appid))
        if result != 0:
            raise ctypes.WinError(result)

    def Win32_GetForegroundWindow():
        """Wrapper for Windows' GetForegroundWindow: the window's handle, or
        None without one."""
        return _user32.GetForegroundWindow() or None

    def Win32_SetForegroundWindow(hwnd):
        """Wrapper for Windows' SetForegroundWindow."""
        if not _user32.SetForegroundWindow(wintypes.HWND(hwnd)):
            raise RuntimeError("Error setting window")

    def Win32_SetProcessDpiAwareness_max():
        """Set Windows' process DPI awareness to the best option available:
        per monitor (version 2), else per monitor, else system wide."""
        per_monitor_v2 = ctypes.c_void_p(-4)
        context = getattr(_user32, "SetProcessDpiAwarenessContext", None)
        if context is not None and context(per_monitor_v2):
            return
        try:
            ctypes.WinDLL("shcore").SetProcessDpiAwareness(2)
        except (OSError, AttributeError):
            _user32.SetProcessDPIAware()
else:
    def Win32_GetCurrentProcessExplicitAppUserModelID():
        """
        Wrapper for Windows's GetCurrentProcessExplicitAppUserModelID.

        On non-Windows platforms, always returns None.
        """
        return None

    def Win32_SetCurrentProcessExplicitAppUserModelID(appid):
        """
        Wrapper for Windows's SetCurrentProcessExplicitAppUserModelID.

        On non-Windows platforms, does nothing.
        """

    def Win32_GetForegroundWindow():
        """
        Wrapper for Windows' GetForegroundWindow.

        On non-Windows platforms, always returns None.
        """
        return None

    def Win32_SetForegroundWindow(hwnd):
        """
        Wrapper for Windows' SetForegroundWindow.

        On non-Windows platforms, does nothing.
        """

    def Win32_SetProcessDpiAwareness_max():
        """
        Set Windows' process DPI awareness to best option available.

        On non-Windows platforms, does nothing.
        """
