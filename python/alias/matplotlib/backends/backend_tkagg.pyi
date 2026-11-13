# The type stub of matplotlib.backends.backend_tkagg: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

from . import _backend_tk
from .backend_agg import FigureCanvasAgg
from ._backend_tk import _BackendTk, FigureCanvasTk
from ._backend_tk import FigureManagerTk, NavigationToolbar2Tk

class FigureCanvasTkAgg(FigureCanvasAgg, FigureCanvasTk):

    def draw(self):
        ...

    def blit(self, bbox=None):
        ...

@_BackendTk.export
class _BackendTkAgg(_BackendTk):
    FigureCanvas = FigureCanvasTkAgg
__all__ = ['FigureCanvasAgg', 'FigureCanvasTk', 'FigureManagerTk', 'NavigationToolbar2Tk', 'FigureCanvasTkAgg']
