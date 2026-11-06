"""
What the canvases matplotlib draws with Cairo add to the Agg canvases they
are in pltrs: the ``cairo`` backend's (`.backend_cairo`) and the ``*cairo``
GUI backends'.

pltrs (Rust) draws their figures, onto the raster canvas or as a document,
as matplotlib's Cairo renderer does: each artist's graphics context starts
from a Cairo context's state, not from `.GraphicsContextBase`'s, so lines an
artist sets no join for (a collection's edges) are mitred rather than
rounded. They print PDF, PostScript and SVG themselves, at 72 dpi as
matplotlib's Cairo surfaces do.
"""

import functools

from .._native import backends as _backends


class _CairoCanvas:
    """
    Mixin putting a canvas's drawing and printing in Cairo's graphics state;
    it comes before the Agg canvas class it is mixed into.
    """

    def draw(self, *args, **kwargs):
        # docstring inherited (wx's takes the device context to draw on)
        _backends.drawing_like_cairo(
            functools.partial(super().draw, *args, **kwargs))

    def print_png(self, *args, **kwargs):
        # docstring inherited
        _backends.drawing_like_cairo(
            functools.partial(super().print_png, *args, **kwargs))

    def print_raw(self, *args, **kwargs):
        # docstring inherited
        _backends.drawing_like_cairo(
            functools.partial(super().print_raw, *args, **kwargs))

    print_rgba = print_raw

    def _save(self, fmt, fobj, *, orientation='portrait',
              bbox_inches_restore=None, facecolor=None, edgecolor=None):
        """Write the figure as a *fmt* document: ``pdf``, ``ps``, ``svg`` or
        ``svgz``; a PostScript page in *orientation*."""
        self.figure.dpi = 72
        _backends.drawing_like_cairo(functools.partial(
            self._print_native, fmt, fobj, orientation=orientation,
            bbox_inches_restore=bbox_inches_restore, facecolor=facecolor,
            edgecolor=edgecolor))

    print_pdf = functools.partialmethod(_save, "pdf")
    print_ps = functools.partialmethod(_save, "ps")
    print_svg = functools.partialmethod(_save, "svg")
    print_svgz = functools.partialmethod(_save, "svgz")
