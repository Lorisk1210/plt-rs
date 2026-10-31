"""
The TkAgg backend: figures drawn by pltrs (Rust)'s Agg canvas, shown in
Tk (matplotlib's ``backend_tkagg``, from matplotlib 3.11.2, under the
Matplotlib License).

`FigureCanvasTkAgg` is the canvas programs embed in their Tk windows: each
draw blits the canvas' pixels into its Tk ``PhotoImage``.
"""

from . import _backend_tk
from .backend_agg import FigureCanvasAgg
from ._backend_tk import _BackendTk, FigureCanvasTk
from ._backend_tk import (  # noqa: F401 # pylint: disable=W0611
    FigureManagerTk, NavigationToolbar2Tk)


class FigureCanvasTkAgg(FigureCanvasAgg, FigureCanvasTk):
    def draw(self):
        super().draw()
        self.blit()

    def blit(self, bbox=None):
        _backend_tk.blit(self._tkphoto, self.renderer.buffer_rgba(),
                         (0, 1, 2, 3), bbox=bbox)


@_BackendTk.export
class _BackendTkAgg(_BackendTk):
    FigureCanvas = FigureCanvasTkAgg
