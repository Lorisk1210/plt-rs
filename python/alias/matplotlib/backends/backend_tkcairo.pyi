# The type stub of matplotlib.backends.backend_tkcairo: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import numpy as np
from .. import cbook
from . import _backend_tk
from .backend_cairo import cairo, FigureCanvasCairo
from ._backend_tk import _BackendTk, FigureCanvasTk

class FigureCanvasTkCairo(FigureCanvasCairo, FigureCanvasTk):

    def draw(self):
        self._renderer.dpi = self.figure.dpi

@_BackendTk.export
class _BackendTkCairo(_BackendTk):
    FigureCanvas = FigureCanvasTkCairo
__all__ = ['np', 'cbook', 'cairo', 'FigureCanvasCairo', 'FigureCanvasTk', 'FigureCanvasTkCairo']
