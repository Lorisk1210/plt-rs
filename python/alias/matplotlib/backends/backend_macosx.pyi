# The type stub of matplotlib.backends.backend_macosx: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import os
import matplotlib as mpl
from matplotlib import _api, cbook
from matplotlib._pylab_helpers import Gcf
from . import _macosx
from .backend_agg import FigureCanvasAgg
from matplotlib.backend_bases import _Backend, FigureCanvasBase, FigureManagerBase, NavigationToolbar2, ResizeEvent, TimerBase, _allow_interrupt

class TimerMac(_macosx.Timer, TimerBase):
    ...

def _allow_interrupt_macos():
    ...

class FigureCanvasMac(FigureCanvasAgg, _macosx.FigureCanvas, FigureCanvasBase):
    required_interactive_framework = 'macosx'
    _timer_cls = TimerMac
    manager_class = _api.classproperty(lambda cls: FigureManagerMac)

    def __init__(self, figure):
        self._draw_pending = False
        self._is_drawing = False
        self._timers = set()

    def draw(self):
        ...

    def draw_idle(self):
        self._draw_pending = True

    def _single_shot_timer(self, callback):
        ...

    def _draw_idle(self):
        self._draw_pending = False

    def blit(self, bbox=None):
        ...

    def resize(self, width, height):
        ...

    def start_event_loop(self, timeout=0):
        ...

class NavigationToolbar2Mac(_macosx.NavigationToolbar2, NavigationToolbar2):

    def __init__(self, canvas):
        ...

    def draw_rubberband(self, event, x0, y0, x1, y1):
        ...

    def remove_rubberband(self):
        ...

    def save_figure(self, *args):
        ...

class FigureManagerMac(_macosx.FigureManager, FigureManagerBase):
    _toolbar2_class = NavigationToolbar2Mac

    def __init__(self, canvas, num):
        self._shown = False

    def _close_button_pressed(self):
        ...

    def destroy(self):
        ...

    @classmethod
    def start_main_loop(cls):
        ...

    def show(self):
        self._shown = True

@_Backend.export
class _BackendMac(_Backend):
    FigureCanvas = FigureCanvasMac
    FigureManager = FigureManagerMac
    mainloop = FigureManagerMac.start_main_loop
__all__ = ['os', 'mpl', 'cbook', 'Gcf', 'FigureCanvasAgg', 'FigureCanvasBase', 'FigureManagerBase', 'NavigationToolbar2', 'ResizeEvent', 'TimerBase', 'TimerMac', 'FigureCanvasMac', 'NavigationToolbar2Mac', 'FigureManagerMac']
