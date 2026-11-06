"""
The Tk helper of the Tk backends (matplotlib's C extension ``_tkagg``).

matplotlib's helper finds the ``PhotoImage`` by its name in the Tcl
interpreter at an address and copies pixels into it with Tk's C API
(``Tk_PhotoPutBlock``). This one keeps the interpreters it is told about
(`register_interpreter`, which the Tk backend calls for every canvas and
blit), finds the photo there by name and hands Tk the pixels with Tk's own
photo commands, as PNG data (the format Tk reads since 8.6, which keeps
the alpha channel): ``TK_PHOTO_COMPOSITE_SET`` replaces the photo's
pixels, ``TK_PHOTO_COMPOSITE_OVERLAY`` composites the block over them by
``image copy -compositingrule overlay``. Its checks and messages are the
C helper's.

On Windows, `enable_dpi_awareness` hooks a per-monitor DPI aware Tk
window's procedure, as the C helper does (through ``ctypes``), so that a
move to a screen of another DPI sets the window's ``window_dpi<frame>``
Tcl variable, which the Tk backend traces; elsewhere it returns None.
"""

import io
import sys
import tkinter as tk

import numpy as np
from PIL import Image

#: Composite the block over the photo's pixels by their alpha.
TK_PHOTO_COMPOSITE_OVERLAY = 0
#: Replace the photo's pixels by the block's.
TK_PHOTO_COMPOSITE_SET = 1

#: The Tcl interpreters seen, by address: ``{interpaddr: tkapp}``.
_interpreters = {}


def register_interpreter(tkapp):
    """Make the photos of the Tcl interpreter *tkapp* reachable by `blit`
    through its address (``tkapp.interpaddr()``)."""
    _interpreters[tkapp.interpaddr()] = tkapp


def blit(interp, photo_name, data, comp_rule, offset, bbox):
    """
    Put the pixels of *data* inside *bbox* into the photo *photo_name* of
    the Tcl interpreter at address *interp*.

    *data* is a ``(height, width, 4)`` array of bytes, rows from the top;
    *offset* gives its channels of red, green, blue and alpha; *bbox* is
    ``(x1, x2, y1, y2)`` in pixels from the bottom left, and the block
    goes to the same place in the photo, replacing its pixels or
    composited over them by *comp_rule*.
    """
    tkapp = _interpreters.get(interp)
    if tkapp is None or not tkapp.call("info", "commands", photo_name):
        raise ValueError("Failed to extract Tk_PhotoHandle")
    data = np.asarray(data)
    if data.ndim != 3:
        raise ValueError(f"array has incorrect number of dimensions: "
                         f"{data.ndim}; expected 3")
    if data.shape[2] != 4:
        raise ValueError(f"Data pointer must be RGBA; last dimension is "
                         f"{data.shape[2]}, not 4")
    height, width = data.shape[:2]
    x1, x2, y1, y2 = bbox
    if not (0 <= y1 <= y2 <= height and 0 <= x1 <= x2 <= width):
        raise ValueError("Attempting to draw out of bounds")
    if comp_rule not in (TK_PHOTO_COMPOSITE_OVERLAY, TK_PHOTO_COMPOSITE_SET):
        raise ValueError("Invalid comp_rule argument")
    block = np.ascontiguousarray(
        data[height - y2:height - y1, x1:x2][..., list(offset)],
        dtype=np.uint8)
    if not block.size:
        return
    png = io.BytesIO()
    Image.fromarray(block, "RGBA").save(png, format="png", compress_level=0)
    x, y = x1, height - y2
    if comp_rule == TK_PHOTO_COMPOSITE_SET:
        tkapp.call(photo_name, "put", png.getvalue(), "-format", "png",
                   "-to", x, y)
        return
    # The scratch photo is deleted with its Python object, on return.
    region = tk.PhotoImage(master=tkapp, data=png.getvalue(), format="png")
    tkapp.call(photo_name, "copy", region, "-to", x, y,
               "-compositingrule", "overlay")


if sys.platform == "win32":
    from ._tkagg_win32 import enable_dpi_awareness
else:
    def enable_dpi_awareness(frame_handle, interp):
        """Per-monitor DPI awareness of a Tk window, which only Windows has:
        None (no awareness to report) elsewhere, as matplotlib's helper."""
        return None
