"""
The Qt backend drawing with Cairo (matplotlib's ``backend_qtcairo``).

pltrs has no Cairo: pltrs (Rust) draws the canvas's figure onto its raster,
as the QtAgg canvas does (`.backend_qtagg`), but in Cairo's graphics state,
as matplotlib's Cairo renderer draws (`._cairo_canvas`), and shows the
pixels it draws; it needs no pycairo or cairocffi.
"""

from .backend_qt import _BackendQT
from ._cairo_canvas import _CairoCanvas
from .backend_qtagg import FigureCanvasQTAgg


class FigureCanvasQTCairo(_CairoCanvas, FigureCanvasQTAgg):
    """The canvas of ``qtcairo``: `.FigureCanvasQTAgg` drawing as Cairo."""


@_BackendQT.export
class _BackendQTCairo(_BackendQT):
    FigureCanvas = FigureCanvasQTCairo
