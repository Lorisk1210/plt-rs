"""
The Agg backend: the raster canvas (matplotlib's ``backend_agg``).

pltrs (Rust) keeps a raster canvas per figure, drawn by its port of Agg:
`FigureCanvasAgg.draw` draws the figure onto it, `buffer_rgba` gives its
pixels, `copy_from_bbox` and `restore_region` blit, and the ``print_*``
methods write the figure as PNG, JPEG, TIFF, GIF, WebP or raw RGBA.
"""

from contextlib import nullcontext

import numpy as np

from .._native import backends as _backends
from ..backend_bases import (
    _Backend, DrawEvent, FigureCanvasBase, FigureManagerBase, RendererBase)
from ..transforms import BboxBase


class BufferRegion:
    """
    The pixels of a rectangle of the canvas, saved by
    `RendererAgg.copy_from_bbox` to be put back by `~RendererAgg.restore_region`.
    """

    def __init__(self, native):
        self._native = native

    def get_extents(self):
        """The saved rectangle ``(x0, y0, x1, y1)``, in canvas pixels from its
        top-left corner."""
        return _backends.region_extents(self._native)


class RendererAgg(RendererBase):
    """
    The renderer of the Agg canvas: the pixels pltrs (Rust) drew for a
    figure of *width* × *height* pixels at *dpi*.
    """

    def __init__(self, width, height, dpi):
        super().__init__()
        self.dpi = dpi
        self.width = width
        self.height = height
        self._figure = None

    def _canvas_figure(self):
        """The figure whose canvas this renderer is."""
        if self._figure is None:
            raise RuntimeError(
                "pltrs draws figures through their canvas; this renderer "
                "belongs to none")
        return self._figure._native

    def get_canvas_width_height(self):
        # docstring inherited
        return self.width, self.height

    def points_to_pixels(self, points):
        # docstring inherited
        return points * self.dpi / 72

    def option_image_nocomposite(self):
        # docstring inherited

        # It is generally faster to composite each image directly to
        # the Figure, and there's no file size benefit to compositing
        # with the Agg backend
        return True

    def option_scale_image(self):
        # docstring inherited
        return False

    def clear(self):
        """Clear the canvas (pltrs (Rust) clears it at every draw)."""

    def buffer_rgba(self):
        """The pixels, as a ``memoryview`` of RGBA rows from the top."""
        return memoryview(_backends.buffer_rgba(self._canvas_figure()))

    def tostring_argb(self):
        """The pixels as ARGB `bytes`."""
        return np.asarray(self.buffer_rgba()).take([3, 0, 1, 2], axis=2).tobytes()

    def copy_from_bbox(self, bbox):
        """Save the pixels inside *bbox* (display coordinates)."""
        return BufferRegion(_backends.copy_from_bbox(
            self._canvas_figure(), tuple(map(float, bbox.extents))))

    def restore_region(self, region, bbox=None, xy=None):
        """
        Restore the saved region. If bbox (instance of BboxBase, or
        its extents) is given, only the region specified by the bbox
        will be restored. *xy* (a pair of floats) optionally
        specifies the new position (the LLC of the original region,
        not the LLC of the bbox) where the region will be restored.

        >>> region = renderer.copy_from_bbox()
        >>> x1, y1, x2, y2 = region.get_extents()
        >>> renderer.restore_region(region, bbox=(x1+dx, y1, x2, y2),
        ...                         xy=(x1-dx, y1))

        """
        if bbox is not None or xy is not None:
            if bbox is None:
                x1, y1, x2, y2 = region.get_extents()
            elif isinstance(bbox, BboxBase):
                x1, y1, x2, y2 = bbox.extents
            else:
                x1, y1, x2, y2 = bbox
            if xy is None:
                ox, oy = x1, y1
            else:
                ox, oy = xy
            _backends.restore_region(
                self._canvas_figure(), region._native,
                tuple(float(int(v)) for v in (x1, y1, x2, y2)),
                (float(int(ox)), float(int(oy))))
        else:
            _backends.restore_region(
                self._canvas_figure(), region._native, None, None)


class FigureCanvasAgg(FigureCanvasBase):
    # docstring inherited

    _lastKey = None  # Overwritten per-instance on the first draw.

    def copy_from_bbox(self, bbox):
        renderer = self.get_renderer()
        return renderer.copy_from_bbox(bbox)

    def restore_region(self, region, bbox=None, xy=None):
        renderer = self.get_renderer()
        return renderer.restore_region(region, bbox, xy)

    def draw(self):
        # docstring inherited
        self.renderer = self.get_renderer()
        self.renderer.clear()
        with (self.toolbar._wait_cursor_for_draw_cm() if self.toolbar
              else nullcontext()):
            _backends.draw(self.figure._native)
            DrawEvent("draw_event", self, self.renderer)._process()
            # A GUI class may be need to update a window using this draw, so
            # don't forget to call the superclass.
            super().draw()

    def blit(self, bbox=None):
        # docstring inherited
        _backends.blit(self.figure._native,
                       None if bbox is None else tuple(map(float, bbox.extents)))

    def get_renderer(self):
        w, h = self.get_width_height(physical=True)
        key = w, h, self.figure.dpi
        reuse_renderer = (self._lastKey == key)
        if not reuse_renderer:
            self.renderer = RendererAgg(w, h, self.figure.dpi)
            self.renderer._figure = self.figure
            self._lastKey = key
        return self.renderer

    def tostring_argb(self):
        """
        Get the image as ARGB `bytes`.

        `draw` must be called at least once before this function will work and
        to update the renderer for any subsequent changes to the Figure.
        """
        return self.renderer.tostring_argb()

    def buffer_rgba(self):
        """
        Get the image as a `memoryview` to the renderer's buffer.

        `draw` must be called at least once before this function will work and
        to update the renderer for any subsequent changes to the Figure.
        """
        return self.renderer.buffer_rgba()

    def _print_raster(self, filename_or_obj, fmt, metadata, pil_kwargs,
                      **kwargs):
        """Write the figure as the raster format *fmt*, after the checks of
        matplotlib's ``_print_pil``; pltrs (Rust) refuses *metadata* for
        the formats other than PNG, as matplotlib does."""
        settings = {"metadata": metadata}
        for key, value in (pil_kwargs or {}).items():
            if fmt == "jpeg" and key == "quality":
                settings["jpeg_quality"] = int(value)
            else:
                raise NotImplementedError(
                    f"pltrs: print_{fmt}(pil_kwargs={{{key!r}: ...}})")
        self._print_native(fmt, filename_or_obj, **kwargs, **settings)

    def print_raw(self, filename_or_obj, *, metadata=None,
                  bbox_inches_restore=None, facecolor=None, edgecolor=None):
        if metadata is not None:
            raise ValueError("metadata not supported for raw/rgba")
        self._print_native("raw", filename_or_obj,
                           bbox_inches_restore=bbox_inches_restore,
                           facecolor=facecolor, edgecolor=edgecolor)

    print_rgba = print_raw

    def print_png(self, filename_or_obj, *, metadata=None, pil_kwargs=None,
                  bbox_inches_restore=None, facecolor=None, edgecolor=None):
        """
        Write the figure to a PNG file.

        Parameters
        ----------
        filename_or_obj : str or path-like or file-like
            The file to write to.

        metadata : dict, optional
            Metadata in the PNG file as key-value pairs of bytes or latin-1
            encodable strings.

        pil_kwargs : dict, optional
            Keyword arguments passed to `PIL.Image.Image.save`.
        """
        self._print_raster(filename_or_obj, "png", metadata, pil_kwargs,
                           bbox_inches_restore=bbox_inches_restore,
                           facecolor=facecolor, edgecolor=edgecolor)

    def print_to_buffer(self):
        FigureCanvasAgg.draw(self)
        renderer = self.get_renderer()
        return (bytes(renderer.buffer_rgba()),
                (int(renderer.width), int(renderer.height)))

    # Note that these methods should typically be called via savefig() and
    # print_figure(), and the latter ensures that `self.figure.dpi` already
    # matches the dpi kwarg (if any).

    def print_gif(self, filename_or_obj, *, metadata=None, pil_kwargs=None,
                  bbox_inches_restore=None, facecolor=None, edgecolor=None):
        self._print_raster(filename_or_obj, "gif", metadata, pil_kwargs,
                           bbox_inches_restore=bbox_inches_restore,
                           facecolor=facecolor, edgecolor=edgecolor)

    def print_jpg(self, filename_or_obj, *, metadata=None, pil_kwargs=None,
                  bbox_inches_restore=None, facecolor=None, edgecolor=None):
        self._print_raster(filename_or_obj, "jpeg", metadata, pil_kwargs,
                           bbox_inches_restore=bbox_inches_restore,
                           facecolor=facecolor, edgecolor=edgecolor)

    print_jpeg = print_jpg

    def print_tif(self, filename_or_obj, *, metadata=None, pil_kwargs=None,
                  bbox_inches_restore=None, facecolor=None, edgecolor=None):
        self._print_raster(filename_or_obj, "tiff", metadata, pil_kwargs,
                           bbox_inches_restore=bbox_inches_restore,
                           facecolor=facecolor, edgecolor=edgecolor)

    print_tiff = print_tif

    def print_webp(self, filename_or_obj, *, metadata=None, pil_kwargs=None,
                   bbox_inches_restore=None, facecolor=None, edgecolor=None):
        self._print_raster(filename_or_obj, "webp", metadata, pil_kwargs,
                           bbox_inches_restore=bbox_inches_restore,
                           facecolor=facecolor, edgecolor=edgecolor)

    (print_gif.__doc__,
     print_jpg.__doc__,
     print_tif.__doc__,
     print_webp.__doc__) = map(
        """
        Write the figure to a {} file.

        Parameters
        ----------
        filename_or_obj : str or path-like or file-like
            The file to write to.
        metadata : None
            Unused for pillow-based writers. All supported options
            can be passed via *pil_kwargs*.
        pil_kwargs : dict, optional
            Additional keyword arguments that are passed to
            `PIL.Image.Image.save` when saving the figure.
        """.format, ["GIF", "JPEG", "TIFF", "WebP"])


@_Backend.export
class _BackendAgg(_Backend):
    backend_version = 'v2.2'
    FigureCanvas = FigureCanvasAgg
    FigureManager = FigureManagerBase
