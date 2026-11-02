"""
The Tk backend drawing with Cairo (matplotlib's ``backend_tkcairo``).

pltrs has no Cairo: its canvas draws with pltrs (Rust)'s raster, as the
TkAgg canvas does (`.backend_tkagg`), and shows the same pixels; it needs
no pycairo or cairocffi.
"""

from ._backend_tk import _BackendTk
from .backend_tkagg import FigureCanvasTkAgg


class FigureCanvasTkCairo(FigureCanvasTkAgg):
    """The canvas of ``tkcairo``: `.FigureCanvasTkAgg`'s drawing."""


@_BackendTk.export
class _BackendTkCairo(_BackendTk):
    FigureCanvas = FigureCanvasTkCairo
