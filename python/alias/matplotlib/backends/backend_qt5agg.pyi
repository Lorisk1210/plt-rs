# The type stub of matplotlib.backends.backend_qt5agg: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

from .. import backends
backends._QT_FORCE_QT5_BINDING = True
from .backend_qtagg import _BackendQTAgg, FigureCanvasQTAgg, FigureManagerQT, NavigationToolbar2QT, FigureCanvasAgg, FigureCanvasQT

@_BackendQTAgg.export
class _BackendQT5Agg(_BackendQTAgg):
    pass
__all__ = ['backends', 'FigureCanvasQTAgg', 'FigureManagerQT', 'NavigationToolbar2QT', 'FigureCanvasAgg', 'FigureCanvasQT']
