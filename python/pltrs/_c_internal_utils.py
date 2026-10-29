"""
Whether a display server is reachable (matplotlib's ``_c_internal_utils``).

plt-rs connects to the X server itself (and to a Wayland compositor's
socket), where matplotlib loads libX11 and libwayland-client.
"""

from ._native import c_internal_utils as _utils


def display_is_valid():
    """
    Check whether the current X11 or Wayland display is valid.

    On Linux, returns True if either $DISPLAY is set and XOpenDisplay(NULL)
    succeeds, or $WAYLAND_DISPLAY is set and wl_display_connect(NULL)
    succeeds.
    """
    return _utils.display_is_valid()


def xdisplay_is_valid():
    """
    Check whether the current X11 display is valid.

    On Linux, returns True if either $DISPLAY is set and XOpenDisplay(NULL)
    succeeds. Use this function if you need to specifically check for X11
    only (e.g., for Tkinter).
    """
    return _utils.xdisplay_is_valid()
