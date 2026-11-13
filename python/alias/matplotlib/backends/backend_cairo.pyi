# The type stub of matplotlib.backends.backend_cairo: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

import functools
import gzip
import math
import numpy as np
try:
    import cairo
    if cairo.version_info < (1, 14, 0):
        raise ImportError(f'Cairo backend requires cairo>=1.14.0, but only {cairo.version_info} is available')
except ImportError:
    try:
        import cairocffi as cairo
    except ImportError as err:
        raise ImportError('cairo backend requires that pycairo>=1.14.0 or cairocffi is installed') from err
from .. import _api, cbook, font_manager
from matplotlib.backend_bases import _Backend, FigureCanvasBase, FigureManagerBase, GraphicsContextBase, RendererBase
from matplotlib.font_manager import ttfFontProperty
from matplotlib.path import Path
from matplotlib.transforms import Affine2D

def _set_rgba(ctx, color, alpha, forced_alpha):
    ...

def _append_path(ctx, path, transform, clip=None):
    ...

def _cairo_font_args_from_font_prop(prop):
    ...

class RendererCairo(RendererBase):

    def __init__(self, dpi):
        self.dpi = dpi
        self.gc = GraphicsContextCairo(renderer=self)
        self.width = None
        self.height = None
        self.text_ctx = cairo.Context(cairo.ImageSurface(cairo.FORMAT_ARGB32, 1, 1))

    def set_context(self, ctx):
        self.gc.ctx = ctx
        self.width, self.height = size

    @staticmethod
    def _fill_and_stroke(ctx, fill_c, alpha, alpha_overrides):
        ...

    def draw_path(self, gc, path, transform, rgbFace=None):
        ...

    def draw_markers(self, gc, marker_path, marker_trans, path, transform, rgbFace=None):
        ...

    def draw_image(self, gc, x, y, im):
        ...

    def draw_text(self, gc, x, y, s, prop, angle, ismath=False, mtext=None):
        ...

    def _draw_mathtext(self, gc, x, y, s, prop, angle):
        ...

    def get_canvas_width_height(self):
        ...

    def get_text_width_height_descent(self, s, prop, ismath):
        ...

    def new_gc(self):
        self.gc._alpha = 1
        self.gc._forced_alpha = False
        self.gc._hatch = None

    def points_to_pixels(self, points):
        ...

class GraphicsContextCairo(GraphicsContextBase):
    _joind = {'bevel': cairo.LINE_JOIN_BEVEL, 'miter': cairo.LINE_JOIN_MITER, 'round': cairo.LINE_JOIN_ROUND}
    _capd = {'butt': cairo.LINE_CAP_BUTT, 'projecting': cairo.LINE_CAP_SQUARE, 'round': cairo.LINE_CAP_ROUND}

    def __init__(self, renderer):
        self.renderer = renderer

    def restore(self):
        ...

    def set_alpha(self, alpha):
        ...

    def set_antialiased(self, b):
        ...

    def get_antialiased(self):
        ...

    def set_capstyle(self, cs):
        self._capstyle = cs

    def set_clip_rectangle(self, rectangle):
        ...

    def set_clip_path(self, path):
        ...

    def set_dashes(self, offset, dashes):
        self._dashes = (offset, dashes)

    def set_foreground(self, fg, isRGBA=None):
        ...

    def get_rgb(self):
        ...

    def set_joinstyle(self, js):
        self._joinstyle = js

    def set_linewidth(self, w):
        self._linewidth = float(w)

class _CairoRegion:

    def __init__(self, slices, data):
        self._slices = slices
        self._data = data

class FigureCanvasCairo(FigureCanvasBase):

    @property
    def _renderer(self):
        self._cached_renderer = RendererCairo(self.figure.dpi)

    def get_renderer(self):
        ...

    def copy_from_bbox(self, bbox):
        ...

    def restore_region(self, region):
        ...

    def print_png(self, fobj):
        ...

    def print_rgba(self, fobj):
        ...
    print_raw = print_rgba

    def _get_printed_image_surface(self):
        self._renderer.dpi = self.figure.dpi

    def _save(self, fmt, fobj, *, orientation='portrait'):
        self.figure.dpi = dpi
        self._renderer.dpi = self.figure.dpi
    print_pdf = functools.partialmethod(_save, 'pdf')
    print_ps = functools.partialmethod(_save, 'ps')
    print_svg = functools.partialmethod(_save, 'svg')
    print_svgz = functools.partialmethod(_save, 'svgz')

@_Backend.export
class _BackendCairo(_Backend):
    backend_version = cairo.version
    FigureCanvas = FigureCanvasCairo
    FigureManager = FigureManagerBase
__all__ = ['functools', 'gzip', 'math', 'np', 'cbook', 'font_manager', 'FigureCanvasBase', 'FigureManagerBase', 'GraphicsContextBase', 'RendererBase', 'ttfFontProperty', 'Path', 'Affine2D', 'RendererCairo', 'GraphicsContextCairo', 'FigureCanvasCairo']
