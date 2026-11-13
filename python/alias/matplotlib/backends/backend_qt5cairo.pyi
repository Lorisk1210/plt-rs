# The type stub of matplotlib.backends.backend_qt5cairo: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

from .. import backends
backends._QT_FORCE_QT5_BINDING = True
from .backend_qtcairo import _BackendQTCairo, FigureCanvasQTCairo, FigureCanvasCairo, FigureCanvasQT

@_BackendQTCairo.export
class _BackendQT5Cairo(_BackendQTCairo):
    pass
__all__ = ['backends', 'FigureCanvasQTCairo', 'FigureCanvasCairo', 'FigureCanvasQT']
