"""
The GTK 3 backend drawing with Cairo (matplotlib's ``backend_gtk3cairo``).

pltrs has no Cairo renderer: its canvas draws with pltrs (Rust)'s raster,
as the GTK3Agg canvas does (`.backend_gtk3agg`), and paints the same
pixels into the widget (through GTK's cairo, as GTK draws).
"""

from .backend_gtk3 import _BackendGTK3
from .backend_gtk3agg import FigureCanvasGTK3Agg


class FigureCanvasGTK3Cairo(FigureCanvasGTK3Agg):
    """The canvas of ``gtk3cairo``: `.FigureCanvasGTK3Agg`'s drawing."""


@_BackendGTK3.export
class _BackendGTK3Cairo(_BackendGTK3):
    FigureCanvas = FigureCanvasGTK3Cairo
