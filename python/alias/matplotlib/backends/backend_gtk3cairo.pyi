# The type stub of matplotlib.backends.backend_gtk3cairo: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

from contextlib import nullcontext
from .backend_cairo import FigureCanvasCairo
from .backend_gtk3 import GLib, Gtk, FigureCanvasGTK3, _BackendGTK3

class FigureCanvasGTK3Cairo(FigureCanvasCairo, FigureCanvasGTK3):

    def on_draw_event(self, widget, ctx):
        self._idle_draw_id = 0
        self._renderer.width = allocation.width * scale
        self._renderer.height = allocation.height * scale
        self._renderer.dpi = self.figure.dpi

@_BackendGTK3.export
class _BackendGTK3Cairo(_BackendGTK3):
    FigureCanvas = FigureCanvasGTK3Cairo
__all__ = ['nullcontext', 'FigureCanvasCairo', 'GLib', 'Gtk', 'FigureCanvasGTK3', 'FigureCanvasGTK3Cairo']
