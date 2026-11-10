"""
What matplotlib's renderers fail on as they draw a collection whose line
widths or hatch line width are no numbers, or whose sizes are none or no
numbers, which its setters keep as given (see `._unusable`): Agg's
extension casts them to C++ values, the other renderers read each line
width as a float, the PostScript renderer formats the hatch's width; the
PDF and SVG renderers draw regular polygons of no sizes but at offsets.
A collection of offsets, none of them, draws nothing, so fails on none.
"""

import numpy as np

from . import _lazy_imports, _pybind, _python_drawing

_path_collection_call = _lazy_imports.module(globals(), "._path_collection_call")


def _drawn_with(collection, kind):
    """Whether the renderer drawing *collection* now is a *kind* renderer
    (``"agg"``, ``"pdf"``, ``"ps"`` or ``"svg"``)."""
    from .backends import backend_agg, backend_pdf, backend_ps, backend_svg
    renderer_class = {"agg": backend_agg.RendererAgg, "pdf": backend_pdf.RendererPdf,
                      "ps": backend_ps.RendererPS, "svg": backend_svg.RendererSVG}[kind]
    return isinstance(_drawing_renderer(collection), renderer_class)


def _drawing_renderer(collection):
    """The renderer drawing *collection* now: a mixed-mode renderer draws
    with the vector or the raster renderer it holds."""
    from .backends.backend_mixed import MixedModeRenderer
    renderer = _python_drawing.drawing_renderer(collection)
    if isinstance(renderer, MixedModeRenderer):
        renderer = renderer._renderer
    return renderer


def _draws_nothing(collection):
    """Whether matplotlib's draw of *collection* returns before it draws
    an item: it has offsets, none of them."""
    return not len(collection.get_offsets())


def linewidths(collection, widths):
    """Fail as matplotlib's renderer drawing *collection* now fails on its
    line *widths*, given as text: Agg's casts the dash patterns scaled by
    them (``0 * '2'`` is ``''``) to no C++ value; widths no float reads it
    refuses with pybind11's argument listing (see `._path_collection_call`),
    or, drawn as markers, its graphics context with float's error. The
    other renderers set each item's width on their graphics context
    (``float(w)``), which fails on those."""
    if _draws_nothing(collection):
        return
    if _drawn_with(collection, "agg"):
        try:
            np.asarray(widths, dtype=float)
        except (TypeError, ValueError):
            refused = _path_collection_call.refused(collection,
                                                    _drawing_renderer(collection))
            if refused is None:
                raise
            raise refused from None
        raise _pybind.cast_error(())
    for width in widths:
        float(width)


def hatch_linewidth(collection, width):
    """Fail as matplotlib's renderer drawing *collection* now fails on its
    hatch line *width*, no number: Agg's casts it to no double, the
    PostScript renderer formats it (``f"{lw:g}"``) when it hatches."""
    if _draws_nothing(collection):
        return
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
    if len(sizes) or _draws_nothing(collection):
        return
    if _drawn_with(collection, "agg"):
        raise ValueError("Expected 3-dimensional array, got 1")
    if collection._has_offsets() and (_drawn_with(collection, "pdf")
                                      or _drawn_with(collection, "svg")):
        raise AttributeError("'IdentityTransform' object has no attribute 'translate'")
