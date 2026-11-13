# The type stub of matplotlib.backends.backend_qt5: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

from .. import backends
backends._QT_FORCE_QT5_BINDING = True
from .backend_qt import SPECIAL_KEYS, cursord, _create_qApp, _BackendQT, TimerQT, MainWindow, FigureCanvasQT, FigureManagerQT, ToolbarQt, NavigationToolbar2QT, SubplotToolQt, SaveFigureQt, ConfigureSubplotsQt, RubberbandQt, HelpQt, ToolCopyToClipboardQT, FigureCanvasBase, FigureManagerBase, MouseButton, NavigationToolbar2, TimerBase, ToolContainerBase, figureoptions, Gcf
from . import backend_qt as _backend_qt

@_BackendQT.export
class _BackendQT5(_BackendQT):
    pass

def __getattr__(name):
    ...
__all__ = ['backends', 'SPECIAL_KEYS', 'cursord', 'TimerQT', 'MainWindow', 'FigureCanvasQT', 'FigureManagerQT', 'ToolbarQt', 'NavigationToolbar2QT', 'SubplotToolQt', 'SaveFigureQt', 'ConfigureSubplotsQt', 'RubberbandQt', 'HelpQt', 'ToolCopyToClipboardQT', 'FigureCanvasBase', 'FigureManagerBase', 'MouseButton', 'NavigationToolbar2', 'TimerBase', 'ToolContainerBase', 'figureoptions', 'Gcf']
