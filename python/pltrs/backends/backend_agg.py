"""
The Agg backend: the raster canvas (matplotlib's ``backend_agg``).

pltrs (Rust) keeps a raster canvas per figure, drawn by its port of Agg:
`FigureCanvasAgg.draw` draws the figure onto it, `buffer_rgba` gives its
pixels, `copy_from_bbox` and `restore_region` blit, and the ``print_*``
methods write the figure as PNG, JPEG, TIFF, GIF, WebP or raw RGBA.

While the canvas draws, its renderer's text metrics cache measures the
texts (see `.text._get_text_metrics_function`), and the program's
replacements of the renderer's text drawing take effect: a replaced
``draw_tex`` draws the TeX lines, and without its own ``draw_text``
(``RendererBase.draw_text``) the canvas draws text as paths, as
matplotlib's ``RendererBase._draw_text_as_path`` does.
"""

from contextlib import contextmanager, nullcontext

import numpy as np

from .. import _api, rc_context
from .._native import backends as _backends, text_overrides as _text_overrides
from ..backend_bases import (
    _Backend, DrawEvent, FigureCanvasBase, FigureManagerBase, RendererBase)
from ..font_manager import FontManager, FontProperties
from ..image import imsave
from ..text import _measuring_with
from ..transforms import BboxBase

#: The raster formats written by Pillow, as matplotlib writes them: GIF,
#: whose palette and transparency are Pillow's choice, and AVIF, which
#: pltrs (Rust) does not encode.
_PILLOW_FORMATS = frozenset({"gif", "avif"})


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


def _native_gc(gc):
    """The graphics context *gc* as pltrs (Rust) takes it (see
    ``_native.backends.draw_path``)."""
    offset, dashes = gc.get_dashes()
    clip = gc.get_clip_rectangle()
    clip_path, clip_transform = gc.get_clip_path()
    hatch_color = gc.get_hatch_color()
    return (
        (tuple(gc.get_rgb()), gc.get_alpha() if gc.get_forced_alpha() else None,
         bool(gc.get_antialiased()), gc.get_capstyle(), gc.get_joinstyle(),
         float(gc.get_linewidth())),
        (float(offset or 0), None if dashes is None else [float(d) for d in dashes]),
        (None if clip is None else tuple(map(float, clip.extents)),
         None if clip_path is None
         else (clip_path, np.asarray(clip_transform.get_matrix(), dtype=float))),
        (gc.get_hatch(), None if hatch_color is None else tuple(hatch_color),
         float(gc.get_hatch_linewidth())),
        (gc.get_snap(), gc.get_sketch_params(), gc.get_url(), gc.get_gid()))


class _WatchesTextDrawing(type):
    """The metaclass of `RendererAgg`: it tells pltrs (Rust) when
    ``RendererAgg.draw_text`` is taken away or given back, so that raster
    canvases draw text as paths while it is `RendererBase.draw_text`."""

    def __setattr__(cls, name, value):
        super().__setattr__(name, value)
        if name == "draw_text":
            _sync_texts_as_paths()

    def __delattr__(cls, name):
        super().__delattr__(name)
        if name == "draw_text":
            _sync_texts_as_paths()


def _sync_texts_as_paths():
    """Hand pltrs (Rust) whether Agg draws text through
    `RendererBase.draw_text`, as paths."""
    _backends.set_texts_as_paths(RendererAgg.draw_text is RendererBase.draw_text)


class RendererAgg(RendererBase, metaclass=_WatchesTextDrawing):
    """
    The renderer of the Agg canvas: the pixels pltrs (Rust) drew for a
    figure of *width* × *height* pixels at *dpi*.

    A renderer the program makes, behind no figure's canvas, draws the
    artists given to it (``artist.draw(renderer)``) onto pixels of its own.
    """

    def __init__(self, width, height, dpi):
        super().__init__()
        self.dpi = dpi
        self.width = width
        self.height = height
        self._figure = None
        # The pixels of a renderer behind no figure's canvas, made when it
        # first draws.
        self._raster = None
        # The paths `draw_path` keeps while the program's `draw_tex` draws
        # a TeX line pltrs (Rust) is drawing (see `_paint_tex`).
        self._queued = None

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

    def get_text_width_height_descent(self, s, prop, ismath):
        # docstring inherited
        _api.check_in_list(["TeX", True, False], ismath=ismath)
        return _text_overrides.measure(s, FontManager._props(prop), ismath,
                                       float(self.dpi))

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

    def draw_path(self, gc, path, transform, rgbFace=None):
        # docstring inherited
        drawn = (_native_gc(gc), path,
                 np.asarray(transform.get_matrix(), dtype=float),
                 None if rgbFace is None else tuple(rgbFace))
        if self._queued is not None:
            self._queued.append(drawn)
        else:
            _backends.draw_path(self._canvas_figure(), *drawn)

    def draw_text(self, gc, x, y, s, prop, angle, ismath=False, mtext=None):
        # docstring inherited
        self._draw_line(gc, x, y, s, prop, angle, ismath)

    def draw_tex(self, gc, x, y, s, prop, angle, *, mtext=None):
        # docstring inherited
        self._draw_line(gc, x, y, s, prop, angle, "TeX")

    def _draw_line(self, gc, x, y, s, prop, angle, ismath):
        """Draw the line *s* typeset as *ismath* says onto the canvas, the
        left end of its baseline at *x*, *y* (from the top, as `.Text.draw`
        gives it to a renderer that flips y), rotated by *angle* degrees.
        pltrs (Rust) lays the line out without the features and language
        of the text it may belong to."""
        _backends.draw_text(
            self._canvas_figure(), _native_gc(gc),
            (float(x), float(self.height - y)), s, FontManager._props(prop),
            float(angle), ismath)

    def _paint_tex(self, gc_spec, line, props, placement):
        """
        Draw a TeX line pltrs (Rust) is drawing with the program's
        ``draw_tex``, called as `.Text.draw` calls it (*gc_spec*: the
        colour, antialiasing and url of the text; *placement*: the left
        end of the baseline in display coordinates and the rotation), and
        return the paths it drew (`draw_path`), which pltrs then draws.
        The text object is not passed (``mtext=None``).
        """
        rgba, antialiased, url = gc_spec
        x, y, angle = placement
        gc = self.new_gc()
        gc.set_foreground(rgba, isRGBA=True)
        gc.set_antialiased(antialiased)
        gc.set_url(url)
        if self.flipy():
            y = self.height - y
        self._queued = []
        try:
            self.draw_tex(gc, x, y, line, FontProperties._from_native(props),
                          angle, mtext=None)
            return self._queued
        finally:
            self._queued = None

    @contextmanager
    def _drawing(self):
        """Draw with this renderer: its text metrics cache measures the
        texts, and a replaced ``draw_tex`` draws the TeX lines."""
        replaced = getattr(self.draw_tex, "__func__", None) is not _OWN_DRAW_TEX
        with _measuring_with(self):
            if replaced:
                _backends.set_tex_painter(self._paint_tex)
            try:
                yield
            finally:
                if replaced:
                    _backends.set_tex_painter(None)

    def _draw_artist(self, artist):
        """Draw *artist* onto the canvas, over what it holds, placed and
        clipped as the figure was last laid out (`.Artist.draw` with this
        renderer, as ``draw_artist`` calls it)."""
        if self._figure is None:
            _backends.draw_onto_raster(self._own_raster(), artist._native)
        else:
            _backends.draw_artist(self._canvas_figure(), artist._native)

    def _own_raster(self):
        """The pixels of a renderer behind no figure's canvas."""
        if self._raster is None:
            self._raster = _backends.new_raster(
                float(self.width), float(self.height), float(self.dpi))
        return self._raster

    def buffer_rgba(self):
        """The pixels, as a ``memoryview`` of RGBA rows from the top."""
        if self._figure is None:
            return memoryview(_backends.raster_rgba(self._own_raster()))
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


#: The ``draw_tex`` of `RendererAgg`, which pltrs (Rust) draws TeX with.
_OWN_DRAW_TEX = RendererAgg.draw_tex


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
            with self.renderer._drawing():
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
        """
        Write the figure as the raster format *fmt*. pltrs (Rust) encodes
        it, refusing *metadata* for the formats other than PNG as
        matplotlib does, but for what only matplotlib's own writer, Pillow,
        does: the formats of `_PILLOW_FORMATS`, and *pil_kwargs* other
        than a JPEG quality.
        """
        settings = {"metadata": metadata}
        if fmt == "jpeg" and pil_kwargs is not None and set(pil_kwargs) == {"quality"}:
            settings["jpeg_quality"] = int(pil_kwargs["quality"])
        elif fmt in _PILLOW_FORMATS or pil_kwargs:
            self._print_pil(filename_or_obj, fmt, pil_kwargs, metadata,
                            **kwargs)
            return
        with self.get_renderer()._drawing():
            self._print_native(fmt, filename_or_obj, **kwargs, **settings)

    def _print_pil(self, filename_or_obj, fmt, pil_kwargs, metadata,
                   **kwargs):
        """
        Draw the figure with the print settings *kwargs*, then save its
        pixels using `.image.imsave` (to which *pil_kwargs* and *metadata*
        are forwarded), as matplotlib's ``_print_pil`` does.
        """
        with self.get_renderer()._drawing():
            rgba = _backends.print_rgba(self.figure._native,
                                        self._print_settings(**kwargs))
        DrawEvent("draw_event", self, None)._process()
        # savefig() has already applied savefig.facecolor; imsave() blends
        # semi-transparent JPEG figures against an assumed white background.
        with (rc_context({"savefig.facecolor": "white"}) if fmt == "jpeg"
              else nullcontext()):
            imsave(filename_or_obj, rgba, format=fmt, origin="upper",
                   dpi=self.figure.dpi, metadata=metadata,
                   pil_kwargs=pil_kwargs)

    def print_raw(self, filename_or_obj, *, metadata=None,
                  bbox_inches_restore=None, facecolor=None, edgecolor=None):
        if metadata is not None:
            raise ValueError("metadata not supported for raw/rgba")
        with self.get_renderer()._drawing():
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

    def print_avif(self, filename_or_obj, *, metadata=None, pil_kwargs=None,
                   bbox_inches_restore=None, facecolor=None, edgecolor=None):
        self._print_raster(filename_or_obj, "avif", metadata, pil_kwargs,
                           bbox_inches_restore=bbox_inches_restore,
                           facecolor=facecolor, edgecolor=edgecolor)

    (print_gif.__doc__,
     print_jpg.__doc__,
     print_tif.__doc__,
     print_webp.__doc__,
     print_avif.__doc__) = map(
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
        """.format, ["GIF", "JPEG", "TIFF", "WebP", "AVIF"])


@_Backend.export
class _BackendAgg(_Backend):
    backend_version = 'v2.2'
    FigureCanvas = FigureCanvasAgg
    FigureManager = FigureManagerBase
