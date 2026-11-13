# The type stub of matplotlib.backends.backend_gtk3agg: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import numpy as np
from .. import cbook, transforms
from . import backend_agg, backend_gtk3
from .backend_gtk3 import GLib, Gtk, _BackendGTK3
import cairo

class FigureCanvasGTK3Agg(backend_agg.FigureCanvasAgg, backend_gtk3.FigureCanvasGTK3):

    def __init__(self, figure):
        self._bbox_queue = []

    def on_draw_event(self, widget, ctx):
        self._idle_draw_id = 0
        self._bbox_queue = []

    def blit(self, bbox=None):
        ...

@_BackendGTK3.export
class _BackendGTK3Agg(_BackendGTK3):
    FigureCanvas = FigureCanvasGTK3Agg
__all__ = ['np', 'cbook', 'transforms', 'backend_agg', 'backend_gtk3', 'GLib', 'Gtk', 'cairo', 'FigureCanvasGTK3Agg']
