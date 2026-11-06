"""
The GTK 3 backend drawing with Cairo (matplotlib's ``backend_gtk3cairo``).

pltrs has no Cairo: pltrs (Rust) draws the canvas's figure onto its raster,
as the GTK3Agg canvas does (`.backend_gtk3agg`), but in Cairo's graphics
state, as matplotlib's Cairo renderer draws (`._cairo_canvas`), and paints
the pixels it draws into the widget (through GTK's cairo, as GTK draws).
"""

from .backend_gtk3 import _BackendGTK3
from ._cairo_canvas import _CairoCanvas
from .backend_gtk3agg import FigureCanvasGTK3Agg


class FigureCanvasGTK3Cairo(_CairoCanvas, FigureCanvasGTK3Agg):
    """The canvas of ``gtk3cairo``: `.FigureCanvasGTK3Agg` drawing as Cairo."""


@_BackendGTK3.export
class _BackendGTK3Cairo(_BackendGTK3):
    FigureCanvas = FigureCanvasGTK3Cairo
