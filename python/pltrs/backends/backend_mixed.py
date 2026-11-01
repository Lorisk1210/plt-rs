"""
A renderer switching between vector and raster drawing, from matplotlib
3.11.2 under the Matplotlib License.

pltrs (Rust) rasterizes the artists of its own vector outputs itself; this
is matplotlib's helper for the vector renderers of programs. pltrs's print
methods hand the area to save to pltrs (Rust) instead of adjusting the
figure (matplotlib's ``bbox_inches_restore``), so a renderer here cannot
take part in a tight bounding box.
"""

import numpy as np

from .. import cbook
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
        if bbox_inches_restore is not None:
            # matplotlib's value is the state `_tight_bbox.adjust_bbox`
            # returns when it moves the figure's artists to crop it; pltrs
            # (Rust) crops a saved figure itself and keeps no such state.
            raise NotImplementedError(
                "pltrs: MixedModeRenderer(bbox_inches_restore=...): pltrs "
                "crops saved figures natively, without matplotlib's "
                "_tight_bbox state to restore")
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

        self._renderer = vector_renderer

    def __getattr__(self, attr):
        # Proxy everything that hasn't been overridden to the base
        # renderer. Things that *are* overridden can call methods
        # on self._renderer directly, but must not cache/store
        # methods (because things like RendererAgg change their
        # methods on the fly in order to optimise proxying down
        # to the underlying C implementation).
        return getattr(self._renderer, attr)

    def start_rasterizing(self):
        """
        Enter "raster" mode.  All subsequent drawing commands (until
        `stop_rasterizing` is called) will be drawn with the raster backend.
        """
        # change the dpi of the figure temporarily.
        self.figure.dpi = self.dpi

        self._raster_renderer = self._raster_renderer_class(
            self._width*self.dpi, self._height*self.dpi, self.dpi)
        self._renderer = self._raster_renderer

    def stop_rasterizing(self):
        """
        Exit "raster" mode.  All of the drawing that was done since
        the last `start_rasterizing` call will be copied to the
        vector backend by calling draw_image.
        """

        self._renderer = self._vector_renderer

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
        self._raster_renderer = None

        # restore the figure dpi.
        self.figure.dpi = self._figdpi
