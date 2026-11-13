# The type stub of matplotlib.backends.backend_gtk4cairo: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

from contextlib import nullcontext
from .backend_cairo import FigureCanvasCairo
from .backend_gtk4 import GLib, Gtk, FigureCanvasGTK4, _BackendGTK4

class FigureCanvasGTK4Cairo(FigureCanvasCairo, FigureCanvasGTK4):

    def _set_device_pixel_ratio(self, ratio):
        ...

    def on_draw_event(self, widget, ctx):
        self._idle_draw_id = 0

@_BackendGTK4.export
class _BackendGTK4Cairo(_BackendGTK4):
    FigureCanvas = FigureCanvasGTK4Cairo
__all__ = ['nullcontext', 'FigureCanvasCairo', 'GLib', 'Gtk', 'FigureCanvasGTK4', 'FigureCanvasGTK4Cairo']
