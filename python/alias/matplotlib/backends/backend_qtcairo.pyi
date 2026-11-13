# The type stub of matplotlib.backends.backend_qtcairo: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import ctypes
from .backend_cairo import cairo, FigureCanvasCairo
from .backend_qt import _BackendQT, FigureCanvasQT
from .qt_compat import QT_API, QtCore, QtGui

class FigureCanvasQTCairo(FigureCanvasCairo, FigureCanvasQT):

    def draw(self):
        self._renderer.dpi = self.figure.dpi

    def paintEvent(self, event):
        self._renderer.dpi = self.figure.dpi

@_BackendQT.export
class _BackendQTCairo(_BackendQT):
    FigureCanvas = FigureCanvasQTCairo
__all__ = ['ctypes', 'cairo', 'FigureCanvasCairo', 'FigureCanvasQT', 'QT_API', 'QtCore', 'QtGui', 'FigureCanvasQTCairo']
