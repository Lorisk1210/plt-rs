# The type stub of matplotlib.backends.backend_agg: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

from contextlib import nullcontext
import math
import numpy as np
from PIL import features
import matplotlib as mpl
from matplotlib import _api, cbook
from matplotlib.backend_bases import _Backend, FigureCanvasBase, FigureManagerBase, RendererBase
from matplotlib.dviread import Dvi
from matplotlib.font_manager import fontManager as _fontManager, get_font
from matplotlib.ft2font import LoadFlags, RenderMode
from matplotlib.mathtext import MathTextParser
from matplotlib.path import Path
from matplotlib.transforms import Bbox, BboxBase
from matplotlib.backends._backend_agg import RendererAgg as _RendererAgg

def get_hinting_flag():
    ...

class RendererAgg(RendererBase):

    def __init__(self, width, height, dpi):
        self.dpi = dpi
        self.width = width
        self.height = height
        self._renderer = _RendererAgg(int(width), int(height), dpi)
        self._filter_renderers = []
        self.mathtext_parser = MathTextParser('path')
        self.bbox = Bbox.from_bounds(0, 0, self.width, self.height)

    def __getstate__(self):
        ...

    def __setstate__(self, state):
        ...

    def _update_methods(self):
        self.draw_gouraud_triangles = self._renderer.draw_gouraud_triangles
        self.draw_image = self._renderer.draw_image
        self.draw_markers = self._renderer.draw_markers
        self.draw_path_collection = self._renderer.draw_path_collection
        self.draw_quad_mesh = self._renderer.draw_quad_mesh
        self.copy_from_bbox = self._renderer.copy_from_bbox

    def draw_path(self, gc, path, transform, rgbFace=None):
        ...

    def _draw_text_glyphs_and_boxes(self, gc, x, y, angle, glyphs, boxes):
        ...

    def draw_mathtext(self, gc, x, y, s, prop, angle):
        ...

    def draw_text(self, gc, x, y, s, prop, angle, ismath=False, mtext=None):
        ...

    def get_text_width_height_descent(self, s, prop, ismath):
        ...

    def draw_tex(self, gc, x, y, s, prop, angle, *, mtext=None):
        ...

    def get_canvas_width_height(self):
        ...

    def _prepare_font(self, font_prop):
        ...

    def points_to_pixels(self, points):
        ...

    def buffer_rgba(self):
        ...

    def tostring_argb(self):
        ...

    def clear(self):
        ...

    def option_image_nocomposite(self):
        ...

    def option_scale_image(self):
        ...

    def restore_region(self, region, bbox=None, xy=None):
        ...

    def start_filter(self):
        self._renderer = _RendererAgg(int(self.width), int(self.height), self.dpi)

    def stop_filter(self, post_processing):
        self._renderer = self._filter_renderers.pop()

class FigureCanvasAgg(FigureCanvasBase):
    _lastKey = None

    def copy_from_bbox(self, bbox):
        ...

    def restore_region(self, region, bbox=None, xy=None):
        ...

    def draw(self):
        self.renderer = self.get_renderer()

    def get_renderer(self):
        self.renderer = RendererAgg(w, h, self.figure.dpi)
        self._lastKey = key

    def tostring_argb(self):
        ...

    def buffer_rgba(self):
        ...

    def print_raw(self, filename_or_obj, *, metadata=None):
        ...
    print_rgba = print_raw

    def _print_pil(self, filename_or_obj, fmt, pil_kwargs, metadata=None):
        ...

    def print_png(self, filename_or_obj, *, metadata=None, pil_kwargs=None):
        ...

    def print_to_buffer(self):
        ...

    def print_gif(self, filename_or_obj, *, metadata=None, pil_kwargs=None):
        ...

    def print_jpg(self, filename_or_obj, *, metadata=None, pil_kwargs=None):
        ...
    print_jpeg = print_jpg

    def print_tif(self, filename_or_obj, *, metadata=None, pil_kwargs=None):
        ...
    print_tiff = print_tif

    def print_webp(self, filename_or_obj, *, metadata=None, pil_kwargs=None):
        ...

    def print_avif(self, filename_or_obj, *, metadata=None, pil_kwargs=None):
        ...
    print_gif.__doc__, print_jpg.__doc__, print_tif.__doc__, print_webp.__doc__, print_avif.__doc__ = map('\n        Write the figure to a {} file.\n\n        Parameters\n        ----------\n        filename_or_obj : str or path-like or file-like\n            The file to write to.\n        metadata : None\n            Unused for pillow-based writers. All supported options\n            can be passed via *pil_kwargs*.\n        pil_kwargs : dict, optional\n            Additional keyword arguments that are passed to\n            `PIL.Image.Image.save` when saving the figure.\n        '.format, ['GIF', 'JPEG', 'TIFF', 'WebP', 'AVIF'])

@_Backend.export
class _BackendAgg(_Backend):
    backend_version = 'v2.2'
    FigureCanvas = FigureCanvasAgg
    FigureManager = FigureManagerBase
__all__ = ['nullcontext', 'math', 'np', 'features', 'mpl', 'cbook', 'FigureCanvasBase', 'FigureManagerBase', 'RendererBase', 'Dvi', 'get_font', 'LoadFlags', 'RenderMode', 'MathTextParser', 'Path', 'Bbox', 'BboxBase', 'get_hinting_flag', 'RendererAgg', 'FigureCanvasAgg']
