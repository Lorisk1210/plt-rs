# The type stub of matplotlib.backends.backend_wxcairo: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import wx.lib.wxcairo as wxcairo
from .backend_cairo import cairo, FigureCanvasCairo
from .backend_wx import _BackendWx, _FigureCanvasWxBase
from .backend_wx import NavigationToolbar2Wx as NavigationToolbar2WxCairo

class FigureCanvasWxCairo(FigureCanvasCairo, _FigureCanvasWxBase):

    def draw(self, drawDC=None):
        self._renderer.dpi = self.figure.dpi
        self.bitmap = wxcairo.BitmapFromImageSurface(surface)
        self._isDrawn = True

@_BackendWx.export
class _BackendWxCairo(_BackendWx):
    FigureCanvas = FigureCanvasWxCairo
__all__ = ['wxcairo', 'cairo', 'FigureCanvasCairo', 'NavigationToolbar2WxCairo', 'FigureCanvasWxCairo']
