# The type stub of matplotlib.backends.backend_template: matplotlib 3.11.2's, under the
# Matplotlib License, not typed or typed inline there: its code without docstrings
# and function bodies but for what they assign on the instance
# (python/tools/matplotlib_stubs.py).

from matplotlib import _api
from matplotlib._pylab_helpers import Gcf
from matplotlib.backend_bases import FigureCanvasBase, FigureManagerBase, GraphicsContextBase, RendererBase
from matplotlib.figure import Figure

class RendererTemplate(RendererBase):

    def __init__(self, dpi):
        self.dpi = dpi

    def draw_path(self, gc, path, transform, rgbFace=None):
        ...

    def draw_image(self, gc, x, y, im):
        ...

    def draw_text(self, gc, x, y, s, prop, angle, ismath=False, mtext=None):
        ...

    def flipy(self):
        ...

    def get_canvas_width_height(self):
        ...

    def get_text_width_height_descent(self, s, prop, ismath):
        ...

    def new_gc(self):
        ...

    def points_to_pixels(self, points):
        ...

class GraphicsContextTemplate(GraphicsContextBase):
    ...

class FigureManagerTemplate(FigureManagerBase):
    ...

class FigureCanvasTemplate(FigureCanvasBase):
    manager_class = FigureManagerTemplate

    def draw(self):
        ...
    filetypes = {**FigureCanvasBase.filetypes, 'foo': 'My magic Foo format'}

    def print_foo(self, filename, **kwargs):
        ...

    def get_default_filetype(self):
        ...
FigureCanvas = FigureCanvasTemplate
FigureManager = FigureManagerTemplate
__all__ = ['Gcf', 'FigureCanvasBase', 'FigureManagerBase', 'GraphicsContextBase', 'RendererBase', 'Figure', 'RendererTemplate', 'GraphicsContextTemplate', 'FigureManagerTemplate', 'FigureCanvasTemplate', 'FigureCanvas', 'FigureManager']
