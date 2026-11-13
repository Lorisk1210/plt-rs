# The type stub of matplotlib.backends.backend_qtagg: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import ctypes
from matplotlib.transforms import Bbox
from .qt_compat import QT_API, QtCore, QtGui
from .backend_agg import FigureCanvasAgg
from .backend_qt import _BackendQT, FigureCanvasQT
from .backend_qt import FigureManagerQT, NavigationToolbar2QT

class FigureCanvasQTAgg(FigureCanvasAgg, FigureCanvasQT):

    def paintEvent(self, event):
        ...

    def print_figure(self, *args, **kwargs):
        self._draw_pending = True

@_BackendQT.export
class _BackendQTAgg(_BackendQT):
    FigureCanvas = FigureCanvasQTAgg
__all__ = ['ctypes', 'Bbox', 'QT_API', 'QtCore', 'QtGui', 'FigureCanvasAgg', 'FigureCanvasQT', 'FigureManagerQT', 'NavigationToolbar2QT', 'FigureCanvasQTAgg']
