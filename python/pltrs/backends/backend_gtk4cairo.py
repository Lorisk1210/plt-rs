"""
The GTK 4 backend drawing with Cairo (matplotlib's ``backend_gtk4cairo``).

pltrs has no Cairo renderer: its canvas draws with pltrs (Rust)'s raster,
as the GTK4Agg canvas does (`.backend_gtk4agg`), and paints the same
pixels into the widget (through GTK's cairo, as GTK draws).
"""

from .backend_gtk4 import _BackendGTK4
from .backend_gtk4agg import FigureCanvasGTK4Agg


class FigureCanvasGTK4Cairo(FigureCanvasGTK4Agg):
    """The canvas of ``gtk4cairo``: `.FigureCanvasGTK4Agg`'s drawing."""


@_BackendGTK4.export
class _BackendGTK4Cairo(_BackendGTK4):
    FigureCanvas = FigureCanvasGTK4Cairo
