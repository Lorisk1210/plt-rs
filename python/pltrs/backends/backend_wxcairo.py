"""
The wxPython backend drawing with Cairo (matplotlib's ``backend_wxcairo``).

pltrs has no Cairo: pltrs (Rust) draws the canvas's figure onto its raster,
as the WXAgg canvas does (`.backend_wxagg`), but in Cairo's graphics state,
as matplotlib's Cairo renderer draws (`._cairo_canvas`), and shows the
pixels it draws; it needs no pycairo.
"""

from .backend_wx import _BackendWx
from ._cairo_canvas import _CairoCanvas
from .backend_wxagg import FigureCanvasWxAgg
from .backend_wx import (  # noqa: F401 # pylint: disable=W0611
    NavigationToolbar2Wx as NavigationToolbar2WxCairo)


class FigureCanvasWxCairo(_CairoCanvas, FigureCanvasWxAgg):
    """The canvas of ``wxcairo``: `.FigureCanvasWxAgg` drawing as Cairo."""


@_BackendWx.export
class _BackendWxCairo(_BackendWx):
    FigureCanvas = FigureCanvasWxCairo
