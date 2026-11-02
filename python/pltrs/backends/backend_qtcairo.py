"""
The Qt backend drawing with Cairo (matplotlib's ``backend_qtcairo``).

pltrs has no Cairo: its canvas draws with pltrs (Rust)'s raster, as the
QtAgg canvas does (`.backend_qtagg`), and shows the same pixels; it needs
no pycairo or cairocffi.
"""

from .backend_qt import _BackendQT
from .backend_qtagg import FigureCanvasQTAgg


class FigureCanvasQTCairo(FigureCanvasQTAgg):
    """The canvas of ``qtcairo``: `.FigureCanvasQTAgg`'s drawing."""


@_BackendQT.export
class _BackendQTCairo(_BackendQT):
    FigureCanvas = FigureCanvasQTCairo
