"""
The Tk backend drawing with Cairo (matplotlib's ``backend_tkcairo``).

pltrs has no Cairo: pltrs (Rust) draws the canvas's figure onto its raster,
as the TkAgg canvas does (`.backend_tkagg`), but in Cairo's graphics state,
as matplotlib's Cairo renderer draws (`._cairo_canvas`), and shows the
pixels it draws; it needs no pycairo or cairocffi.
"""

from ._backend_tk import _BackendTk
from ._cairo_canvas import _CairoCanvas
from .backend_tkagg import FigureCanvasTkAgg


class FigureCanvasTkCairo(_CairoCanvas, FigureCanvasTkAgg):
    """The canvas of ``tkcairo``: `.FigureCanvasTkAgg` drawing as Cairo."""


@_BackendTk.export
class _BackendTkCairo(_BackendTk):
    FigureCanvas = FigureCanvasTkCairo
