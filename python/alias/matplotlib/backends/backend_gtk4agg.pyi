# The type stub of matplotlib.backends.backend_gtk4agg: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import numpy as np
from .. import cbook
from . import backend_agg, backend_gtk4
from .backend_gtk4 import GLib, Gtk, _BackendGTK4
import cairo

class FigureCanvasGTK4Agg(backend_agg.FigureCanvasAgg, backend_gtk4.FigureCanvasGTK4):

    def on_draw_event(self, widget, ctx):
        self._idle_draw_id = 0

@_BackendGTK4.export
class _BackendGTK4Agg(_BackendGTK4):
    FigureCanvas = FigureCanvasGTK4Agg
__all__ = ['np', 'cbook', 'backend_agg', 'backend_gtk4', 'GLib', 'Gtk', 'cairo', 'FigureCanvasGTK4Agg']
