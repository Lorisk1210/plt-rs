"""
The wxPython backend drawing with Cairo (matplotlib's ``backend_wxcairo``).

pltrs has no Cairo: its canvas draws with pltrs (Rust)'s raster, as the
WXAgg canvas does (`.backend_wxagg`), and shows the same pixels; it needs
no pycairo.
"""

from .backend_wx import _BackendWx
from .backend_wxagg import FigureCanvasWxAgg
from .backend_wx import (  # noqa: F401 # pylint: disable=W0611
    NavigationToolbar2Wx as NavigationToolbar2WxCairo)


class FigureCanvasWxCairo(FigureCanvasWxAgg):
    """The canvas of ``wxcairo``: `.FigureCanvasWxAgg`'s drawing."""


@_BackendWx.export
class _BackendWxCairo(_BackendWx):
    FigureCanvas = FigureCanvasWxCairo
