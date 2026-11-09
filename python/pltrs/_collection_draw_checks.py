"""
What matplotlib's renderers fail on as they draw a collection whose line
widths or hatch line width are no numbers, or whose sizes are none or no
numbers, which its setters keep as given (see `._unusable`): Agg's
extension casts them to C++ values, the PostScript renderer formats the
hatch's width; the PDF and SVG renderers draw, but for regular polygons
of no sizes.
"""

import numpy as np

from . import _pybind, _python_drawing


def _drawn_with(collection, kind):
    """Whether the renderer drawing *collection* now is a *kind* renderer
    (``"agg"``, ``"pdf"``, ``"ps"`` or ``"svg"``)."""
    from .backends import backend_agg, backend_pdf, backend_ps, backend_svg
    from .backends.backend_mixed import MixedModeRenderer
    renderer_class = {"agg": backend_agg.RendererAgg, "pdf": backend_pdf.RendererPdf,
                      "ps": backend_ps.RendererPS, "svg": backend_svg.RendererSVG}[kind]
    renderer = _python_drawing.drawing_renderer(collection)
    if isinstance(renderer, MixedModeRenderer):
        # It draws with the vector or the raster renderer it holds.
        renderer = renderer._renderer
    return isinstance(renderer, renderer_class)


def linewidths(collection, widths):
    """Fail as matplotlib's renderer drawing *collection* now fails on its
    line *widths*, given as text: Agg's casts the dash patterns scaled by
    them (``0 * '2'`` is ``''``) to no C++ value; widths no float reads it
    refuses with pybind11's argument listing, for which numpy's error of
    reading them stands."""
    if _drawn_with(collection, "agg"):
        np.asarray(widths, dtype=float)
        raise _pybind.cast_error(())


def hatch_linewidth(collection, width):
    """Fail as matplotlib's renderer drawing *collection* now fails on its
    hatch line *width*, no number: Agg's casts it to no double, the
    PostScript renderer formats it (``f"{lw:g}"``) when it hatches."""
    if _drawn_with(collection, "agg"):
        raise _pybind.cast_error(width)
    if _drawn_with(collection, "ps") and collection.get_hatch():
        format(width, "g")


def size_scales(sizes, dpi=72.0):
    """Fail as matplotlib's collection fails to scale its items by the
    *sizes* it keeps (``set_sizes``, which its draw calls again): one
    transform per size, scaled by its square root."""
    transforms = np.zeros((len(sizes), 3, 3))
    transforms[:, 0, 0] = np.sqrt(sizes) * dpi / 72.0


def no_sizes(collection, sizes):
    """Fail as matplotlib's renderer drawing *collection*, a regular polygon
    collection of no *sizes*, fails on the list of rotated transforms its
    draw makes of none: Agg's casts it to no 3-D array; the PDF and SVG
    renderers draw items of no transforms at the offsets, translating a
    transform that is none."""
    if len(sizes):
        return
    if _drawn_with(collection, "agg"):
        raise ValueError("Expected 3-dimensional array, got 1")
    if collection._has_offsets() and (_drawn_with(collection, "pdf")
                                      or _drawn_with(collection, "svg")):
        raise AttributeError("'IdentityTransform' object has no attribute 'translate'")
