"""
A renderer switching between vector and raster drawing, from matplotlib
3.11.2 under the Matplotlib License.

pltrs (Rust) rasterizes the artists of its own vector outputs itself. A
`MixedModeRenderer` is what the program's drawing receives while pltrs
(Rust) saves a figure as PDF, SVG, PostScript or PGF, around the
renderer of the format, as matplotlib's print methods hand it to the
figure: its rasterizing is recorded with the rest of the drawing (see
`.._python_drawing`), and pltrs (Rust) rasterizes there. A
`MixedModeRenderer` the program makes around a renderer of its own
rasterizes into a `RendererAgg` of its own, as matplotlib's does.
pltrs's print methods crop a saved figure natively, so
*bbox_inches_restore* (the area they save) needs no processing here.
"""

import numpy as np

from .. import _python_drawing, cbook
from .._native import drawing as _drawing
from ..backend_bases import RendererBase
from ._native_drawing import DocumentDrawingMixin
from .backend_agg import RendererAgg


class MixedModeRenderer:
    """
    A helper class to implement a renderer that switches between
    vector and raster drawing.  An example may be a PDF writer, where
    most things are drawn with PDF vector commands, but some very
    complex objects, such as quad meshes, are rasterised and then
    output as images.
    """
    def __init__(self, figure, width, height, dpi, vector_renderer,
                 raster_renderer_class=None,
                 bbox_inches_restore=None):
        """
        Parameters
        ----------
        figure : `~matplotlib.figure.Figure`
            The figure instance.
        width : float
            The width of the canvas in logical units
        height : float
            The height of the canvas in logical units
        dpi : float
            The dpi of the canvas
        vector_renderer : `~matplotlib.backend_bases.RendererBase`
            An instance of a subclass of
            `~matplotlib.backend_bases.RendererBase` that will be used for the
            vector drawing.
        raster_renderer_class : `~matplotlib.backend_bases.RendererBase`
            The renderer class to use for the raster drawing.  If not provided,
            this will use the Agg backend (which is currently the only viable
            option anyway.)

        """
        if raster_renderer_class is None:
            raster_renderer_class = RendererAgg

        self._raster_renderer_class = raster_renderer_class
        self._width = width
        self._height = height
        self.dpi = dpi

        self._vector_renderer = vector_renderer

        self._raster_renderer = None

        # A reference to the figure is needed as we need to change
        # the figure dpi before and after the rasterization. Although
        # this looks ugly, I couldn't find a better solution. -JJL
        self.figure = figure
        self._figdpi = figure.dpi

        self._bbox_inches_restore = bbox_inches_restore

        self._renderer = vector_renderer

    def __getattr__(self, attr):
        # Proxy everything that hasn't been overridden to the base
        # renderer. Things that *are* overridden can call methods
        # on self._renderer directly, but must not cache/store
        # methods (because things like RendererAgg change their
        # methods on the fly in order to optimise proxying down
        # to the underlying C implementation).
        return getattr(self._renderer, attr)

    @property
    def _recording(self):
        """The drawing pltrs (Rust) records the program's drawing into, if
        it draws with this renderer now (see `.._python_drawing`); never
        around a renderer of the program's."""
        return getattr(self._vector_renderer, "_recording", None)

    @_recording.setter
    def _recording(self, drawing):
        self._vector_renderer._recording = drawing
        if self._raster_renderer is not None:
            self._raster_renderer._recording = drawing

    def _raster_dpi(self):
        """The resolution drawing goes to while rasterizing, else None."""
        return None if self._raster_renderer is None else self.dpi

    def _draw_artist(self, artist):
        """Draw *artist* (`.Artist.draw` with this renderer): where pltrs
        (Rust) draws while it records with this renderer, else through this
        renderer's methods, so that it rasterizes what is rasterized."""
        if self._recording is not None:
            self._renderer._draw_artist(artist)
        else:
            _python_drawing.draw_through(self, artist)

    def _draw_cell(self, cell):
        """Draw the table `.Cell` *cell* alone (`.Cell.draw` with this
        renderer), as `_draw_artist` draws an artist."""
        if self._recording is not None:
            self._renderer._draw_cell(cell)
        else:
            _python_drawing.draw_cell_through(self, cell)

    def _set_figure_dpi(self, dpi):
        """Draw the figure at *dpi* from now on, unless pltrs (Rust) is
        drawing it, which then changes it itself."""
        if _python_drawing.figure_changeable():
            self.figure.dpi = dpi

    def start_rasterizing(self):
        """
        Enter "raster" mode.  All subsequent drawing commands (until
        `stop_rasterizing` is called) will be drawn with the raster backend.
        """
        drawing = self._recording
        if drawing is not None:
            _drawing.start_rasterizing(drawing)
        # change the dpi of the figure temporarily.
        self._set_figure_dpi(self.dpi)

        self._raster_renderer = self._raster_renderer_class(
            self._width*self.dpi, self._height*self.dpi, self.dpi)
        self._raster_renderer._recording = drawing
        self._renderer = self._raster_renderer

    def stop_rasterizing(self):
        """
        Exit "raster" mode.  All of the drawing that was done since
        the last `start_rasterizing` call will be copied to the
        vector backend by calling draw_image.
        """

        self._renderer = self._vector_renderer
        drawing = self._recording
        if drawing is not None:
            _drawing.stop_rasterizing(drawing)
        else:
            self._place_raster()
        self._raster_renderer = None

        # restore the figure dpi.
        self._set_figure_dpi(self._figdpi)

    def _place_raster(self):
        """Draw what was rasterized onto the vector renderer, as an image
        cropped to its drawn pixels."""
        height = self._height * self.dpi
        img = np.asarray(self._raster_renderer.buffer_rgba())
        slice_y, slice_x = cbook._get_nonzero_slices(img[..., 3])
        cropped_img = img[slice_y, slice_x]
        if cropped_img.size:
            gc = self._renderer.new_gc()
            # If the mixedmode resolution differs from the figure's dpi, the
            # image must be scaled (dpi->_figdpi), which not all backends
            # support.
            self._renderer.draw_image(
                gc,
                slice_x.start * self._figdpi / self.dpi,
                (height - slice_y.stop) * self._figdpi / self.dpi,
                cropped_img[::-1])


class _VectorRecorder(DocumentDrawingMixin, RendererBase):
    """
    The vector renderer the program's drawing receives in a vector draw no
    canvas of this package started: a page of *width* × *height* points
    whose images are magnified *magnification* times, recording only.
    """

    def __init__(self, width, height, magnification):
        super().__init__()
        self.width = width
        self.height = height
        self._magnification = magnification

    def flipy(self):
        # docstring inherited
        return False

    def get_canvas_width_height(self):
        # docstring inherited
        return self.width, self.height

    def option_scale_image(self):
        # docstring inherited
        return True

    def get_image_magnification(self):
        # docstring inherited
        return self._magnification


def _stand_in(figure, width, height, magnification):
    """What the program's drawing receives in a vector draw no canvas of
    this package started, of a page of *width* × *height* points whose
    images are magnified *magnification* times: a `MixedModeRenderer` of
    *figure*, rasterizing at *magnification* × 72 dpi, or for a drawing
    of no figure (a path effect's) the vector renderer alone."""
    recorder = _VectorRecorder(width, height, magnification)
    if figure is None:
        return recorder
    return MixedModeRenderer(figure, width / 72, height / 72,
                             72 * magnification, recorder)


# What matplotlib's module also has from its imports, which programs
# import from it.
from .._tight_bbox import process_figure_for_rasterizing  # noqa: E402, F401
