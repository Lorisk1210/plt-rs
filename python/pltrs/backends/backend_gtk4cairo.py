"""
The GTK 4 backend drawing with Cairo (matplotlib's ``backend_gtk4cairo``).

pltrs has no Cairo: pltrs (Rust) draws the canvas's figure onto its raster,
as the GTK4Agg canvas does (`.backend_gtk4agg`), but in Cairo's graphics
state, as matplotlib's Cairo renderer draws (`._cairo_canvas`), and paints
the pixels it draws into the widget (through GTK's cairo, as GTK draws).
"""

from .backend_gtk4 import _BackendGTK4
from ._cairo_canvas import _CairoCanvas
from .backend_gtk4agg import FigureCanvasGTK4Agg


class FigureCanvasGTK4Cairo(_CairoCanvas, FigureCanvasGTK4Agg):
    """The canvas of ``gtk4cairo``: `.FigureCanvasGTK4Agg` drawing as Cairo."""


@_BackendGTK4.export
class _BackendGTK4Cairo(_BackendGTK4):
    FigureCanvas = FigureCanvasGTK4Cairo
