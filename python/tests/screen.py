"""
Whether the smoke tests may open windows on this machine's screen.

On Linux they reach a display only where one is given to them
(``$DISPLAY``, ``$WAYLAND_DISPLAY``: an Xvfb, a nested weston, the
desktop session they run in). A Mac always has one, the user's, even for
a process started over ssh, so there the tests showing windows (Tk's,
pltrs' own) run only when asked to with ``PLTRS_MACOS_LIVE=1``, as
pltrs-platform's AppKit tests do; Qt's draw on its offscreen platform
instead. On Windows a process shows windows only from the interactive
window station, which a session started over ssh (or a service) lacks:
Tk's windows there never map, so their sizes and DPI events do not come.
"""

import ctypes
import os
import sys

#: The variable allowing tests to open windows on a Mac's screen.
LIVE_VARIABLE = "PLTRS_MACOS_LIVE"

#: ``UOI_FLAGS``: what ``GetUserObjectInformationW`` tells of a window
#: station's flags.
UOI_FLAGS = 1
#: ``WSF_VISIBLE``: the window station is the interactive one.
WSF_VISIBLE = 1


class _Flags(ctypes.Structure):
    """``USEROBJECTFLAGS``."""

    _fields_ = [("fInherit", ctypes.c_int), ("fReserved", ctypes.c_int),
                ("dwFlags", ctypes.c_uint32)]


def _interactive_window_station():
    """Whether this Windows process's window station is the interactive
    one (assumed when Windows does not tell)."""
    user32 = ctypes.WinDLL("user32")
    user32.GetProcessWindowStation.restype = ctypes.c_void_p
    user32.GetUserObjectInformationW.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_void_p,
                                                ctypes.c_uint32, ctypes.POINTER(ctypes.c_uint32)]
    flags, needed = _Flags(), ctypes.c_uint32()
    if not user32.GetUserObjectInformationW(user32.GetProcessWindowStation(), UOI_FLAGS,
                                            ctypes.byref(flags), ctypes.sizeof(flags),
                                            ctypes.byref(needed)):
        return True
    return bool(flags.dwFlags & WSF_VISIBLE)


def _refused():
    """Why the tests must not open windows on this machine's screen, or
    None."""
    if sys.platform == "darwin" and os.environ.get(LIVE_VARIABLE) != "1":
        return f"windows would open on the Mac's screen (set {LIVE_VARIABLE}=1 to allow it)"
    if sys.platform == "win32" and not _interactive_window_station():
        return "no interactive window station (a session over ssh)"
    return None


#: Why the tests must not open windows on this machine's screen, or None.
REFUSED = _refused()
