# The type stub of matplotlib.backends.backend_wxagg: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import wx
from .backend_agg import FigureCanvasAgg
from .backend_wx import _BackendWx, _FigureCanvasWxBase
from .backend_wx import NavigationToolbar2Wx as NavigationToolbar2WxAgg

class FigureCanvasWxAgg(FigureCanvasAgg, _FigureCanvasWxBase):

    def draw(self, drawDC=None):
        self.bitmap = self._create_bitmap()
        self._isDrawn = True

    def blit(self, bbox=None):
        self.bitmap = bitmap

    def _create_bitmap(self):
        ...

@_BackendWx.export
class _BackendWxAgg(_BackendWx):
    FigureCanvas = FigureCanvasWxAgg
__all__ = ['wx', 'FigureCanvasAgg', 'NavigationToolbar2WxAgg', 'FigureCanvasWxAgg']
