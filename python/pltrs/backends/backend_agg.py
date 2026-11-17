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
import inspect

import numpy as np

from .. import _api, _python_drawing, rc_context
from .._native import (
    artist as _artist, backends as _backends, drawing as _drawing, ft2font as _ft2font,
    table as _table, text_overrides as _text_overrides)
from .._program_classes import from_program
from .._python_drawing import drawing_with, printed_window
from ..backend_bases import (
    _Backend, DrawEvent, FigureCanvasBase, FigureManagerBase, RendererBase)
from ..font_manager import FontManager, FontProperties
from ..ft2font import LoadFlags
from ..image import imsave
from ..text import _measuring_with
from ..transforms import Bbox, BboxBase
from . import _agg_arguments, _backend_agg
from ._agg_arguments import AggArgumentsMixin
from ._backend_agg import BufferRegion
from ._native_drawing import NativeDrawingMixin, native_gc

#: The raster formats written by Pillow, as matplotlib writes them: GIF,
#: whose palette and transparency are Pillow's choice, and AVIF, whose
#: files come from libavif in matplotlib (pltrs (Rust)'s own AV1 encoder
#: writes other bytes).
_PILLOW_FORMATS = frozenset({"gif", "avif"})


def _check_pil_arguments(fmt, metadata, pil_kwargs):
    """Compute with *metadata* and *pil_kwargs* as matplotlib's
    `.image.imsave` does before Pillow writes the format *fmt*, for its
    errors: the keywords copied, the text chunks of a PNG added (Pillow's
    ``PngInfo.add_text`` encodes each key and value), metadata refused
    for the other formats, the format and resolution given as keywords."""
    pil_kwargs = {} if pil_kwargs is None else pil_kwargs.copy()
    if fmt == "png":
        if "pnginfo" not in pil_kwargs:
            metadata = {"Software": "", **(metadata if metadata is not None else {})}
            pil_kwargs["pnginfo"] = None
            for key, value in metadata.items():
                if value is not None:
                    for text in (value, key):
                        if not isinstance(text, bytes):
                            text.encode
    elif metadata is not None:
        raise ValueError(f"metadata not supported for format {fmt!r}")
    pil_kwargs.setdefault("format", fmt)


def get_hinting_flag():
    return LoadFlags(_ft2font.hinting_flag())


class RendererAgg(AggArgumentsMixin, NativeDrawingMixin, RendererBase):
    """
    The renderer handles all the drawing primitives using a graphics
    context instance that controls the colors/styles
    """
    # The renderer of the Agg canvas: the pixels pltrs (Rust) drew for a
    # figure of *width* × *height* pixels at *dpi*.
    #
    # Its drawing methods draw onto the canvas over what it holds, or, while
    # pltrs (Rust) draws the figure, where pltrs is drawing (see
    # `._native_drawing`). A renderer the program makes, behind no figure's
    # canvas, draws onto pixels of its own.

    #: How many filters `start_filter` opened that `stop_filter` has not
    #: closed.
    _filters_open = 0

    def __init__(self, width, height, dpi):
        super().__init__()
        self.dpi = dpi
        self.width = width
        self.height = height
        _backend_agg.check_renderer_arguments(width, height, dpi)
        self.bbox = Bbox.from_bounds(0, 0, self.width, self.height)
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
        measured = _text_overrides.measure(s, FontManager._props(prop), ismath,
                                           float(self.dpi))
        if ismath is True:
            # The numpy floats of the boxes matplotlib's mathtext parser
            # sets.
            return tuple(map(np.float64, measured))
        return measured

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
        # Clear the canvas (pltrs (Rust) clears it at every draw).
        pass

    def draw_path(self, gc, path, transform, rgbFace=None):
        # docstring inherited
        _agg_arguments.path_arguments(gc, path, transform, rgbFace)
        if self._queued is not None:
            self._queued.append(
                (native_gc(gc), path,
                 np.asarray(transform.get_matrix(), dtype=float),
                 None if rgbFace is None else tuple(rgbFace)))
        else:
            super().draw_path(gc, path, transform, rgbFace)

    def _draw_now(self, drawing):
        """Draw *drawing* onto the canvas, or onto the pixels of a renderer
        behind no figure's canvas."""
        if self._figure is None:
            _drawing.draw_onto_raster(self._own_raster(), drawing)
        else:
            _drawing.draw_onto_canvas(self._canvas_figure(), drawing)

    def draw_text(self, gc, x, y, s, prop, angle, ismath=False, mtext=None):
        # docstring inherited
        self._text_arguments(gc, x, y, s, prop, angle, ismath, mtext)
        # Any true *ismath* draws mathtext, as in matplotlib's renderer.
        self._draw_line(gc, x, y, s, prop, angle, bool(ismath))

    def draw_tex(self, gc, x, y, s, prop, angle, *, mtext=None):
        # docstring inherited
        self._tex_arguments(gc, x, y, s, prop, angle)
        self._draw_line(gc, x, y, s, prop, angle, "TeX")

    def draw_mathtext(self, gc, x, y, s, prop, angle):
        """Draw mathtext using :mod:`matplotlib.mathtext`."""
        self._mathtext_arguments(gc, x, y, s, prop, angle)
        self._draw_line(gc, x, y, s, prop, angle, True)

    def start_filter(self):
        """
        Start filtering. It simply creates a new canvas (the old one is saved).
        """
        super().start_filter()
        self._filters_open += 1

    def stop_filter(self, post_processing):
        """
        Save the current canvas as an image and apply post processing.

        The *post_processing* function::

           def post_processing(image, dpi):
             # ny, nx, depth = image.shape
             # image (numpy array) has RGBA channels and has a depth of 4.
             ...
             # create a new_image (numpy array of 4 channels, size can be
             # different). The resulting image may have offsets from
             # lower-left corner of the original image
             return new_image, offset_x, offset_y

        The saved renderer is restored and the returned image from
        post_processing is plotted (using draw_image) on it.
        """
        if not self._filters_open:
            # matplotlib takes back the canvas `start_filter` saved.
            raise IndexError("pop from empty list")
        self._filters_open -= 1
        super().stop_filter(post_processing)

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

    def _replaced_drawing(self):
        """The drawing methods the program replaced on this renderer or on
        `RendererAgg` (a renderer of a class of its own draws through
        Python whole, see `._python_drawing.overrides_drawing`), which the
        canvas' draw hands it as matplotlib's artists call them."""
        own = vars(self)
        held = RendererAgg.__dict__
        return ([name for name, method in _OWN_DRAWING.items()
                 if name in own or held.get(name) is not method]
                + [name for name in _BOUND_DRAWING if name in own])

    @contextmanager
    def _drawing(self):
        """Draw with this renderer: its text metrics cache measures the
        texts, a replaced ``draw_tex`` draws the TeX lines, a ``new_gc``
        of the program's makes the graphics contexts the artists start
        from, and the program's drawing receives it."""
        replaced = getattr(self.draw_tex, "__func__", None) is not _OWN_DRAW_TEX
        program_gc = from_program(inspect.getattr_static(type(self), "new_gc"))
        figure = getattr(self, "_figure", None)
        draws = None if figure is None else _artist.draw_count(figure._native)
        with _measuring_with(self), drawing_with(self._for_window, figure):
            if replaced:
                _backends.set_tex_painter(self._paint_tex)
            if program_gc:
                _backends.set_program_gc(lambda: native_gc(self.new_gc()))
            try:
                yield
            finally:
                if replaced:
                    _backends.set_tex_painter(None)
                if program_gc:
                    _backends.set_program_gc(None)
        if figure is not None and _artist.draw_count(figure._native) != draws:
            # The renderer of the figure's last draw (see
            # `.text._drawing_with`, here inline, as the canvas draws often).
            figure.__dict__["_drawn_with"] = (self, _artist.draw_count(figure._native))

    def _for_window(self):
        """What the program's drawing receives while this renderer draws:
        itself, but while a print of its figure is cut to a window, its
        canvas' renderer of the window's size, which matplotlib's print
        draws the window with (having measured it with this one)."""
        figure = getattr(self, "_figure", None)
        if figure is None or printed_window(figure) is None:
            return self
        canvas_renderer = getattr(figure.canvas, "get_renderer", None)
        if canvas_renderer is not None:
            return canvas_renderer()
        width, height = figure.bbox.size
        return RendererAgg(int(width), int(height), figure.dpi)

    def _draw_artist(self, artist):
        """Draw *artist*, as pltrs (Rust) draws it, onto the canvas over
        what it holds, placed and clipped as the figure was last laid out
        (`.Artist.draw` with this renderer, as ``draw_artist`` calls it),
        or where pltrs (Rust) is drawing while it draws the figure."""
        if self._recording is not None:
            super()._draw_artist(artist)
        elif _python_drawing.overrides_drawing(self):
            _python_drawing.draw_through(self, artist)
        elif self._figure is None:
            _backends.draw_onto_raster(self._own_raster(), artist._native)
        else:
            _backends.draw_artist(self._canvas_figure(), artist._native)

    def _draw_cell(self, cell):
        """Draw the table `.Cell` *cell* alone, as `_draw_artist` draws an
        artist (`.Cell.draw` with this renderer)."""
        if self._recording is not None:
            super()._draw_cell(cell)
        elif self._figure is None:
            _table.draw_cell_onto_raster(self._own_raster(), cell._ref())
        else:
            _table.draw_cell(self._canvas_figure(), cell._ref())

    def _own_raster(self):
        """The pixels of a renderer behind no figure's canvas."""
        if self._raster is None:
            self._raster = _backends.new_raster(
                float(self.width), float(self.height), float(self.dpi))
        return self._raster

    def buffer_rgba(self):
        drawn = (None if self._figure is None
                 else _backends.buffer_rgba(self._canvas_figure()))
        if drawn is None:
            return memoryview(_backends.raster_rgba(self._own_raster()))
        return memoryview(drawn)

    @property
    def _renderer(self):
        """
        The pixels the renderer draws into, as matplotlib's C++ renderer
        gives them (see `._backend_agg.RendererAgg`).
        """
        return _backend_agg.RendererAgg(self)

    def tostring_argb(self):
        return np.asarray(self.buffer_rgba()).take([3, 0, 1, 2], axis=2).tobytes()

    def copy_from_bbox(self, bbox):
        """Save the pixels inside *bbox* (display coordinates)."""
        return BufferRegion(_backends.copy_from_bbox(
            self._canvas_figure(), _backend_agg.rect(bbox)))

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
            corners = tuple(int(v) for v in (x1, y1, x2, y2))
            _backend_agg.check_restore_region(
                self._renderer, region, *corners, int(ox), int(oy))
            _backends.restore_region(
                self._canvas_figure(), region._native,
                tuple(map(float, corners)), (float(int(ox)), float(int(oy))))
        else:
            _backend_agg.check_restore_region(self._renderer, region)
            _backends.restore_region(
                self._canvas_figure(), region._native, None, None)


#: The ``draw_tex`` of `RendererAgg`, which pltrs (Rust) draws TeX with.
_OWN_DRAW_TEX = RendererAgg.draw_tex

#: The drawing methods of `RendererAgg` a program may replace on the class
#: to see what matplotlib's artists draw, as the class holds them (None:
#: inherited).
_OWN_DRAWING = {name: RendererAgg.__dict__.get(name) for name in (
    "draw_path", "draw_text", "open_group", "close_group")}

#: Those matplotlib's renderer binds on each renderer to its C++ one
#: (``RendererAgg._update_methods``), which only a replacement on the
#: renderer itself replaces.
_BOUND_DRAWING = ("draw_markers", "draw_path_collection", "draw_quad_mesh",
                  "draw_gouraud_triangles", "draw_image")


# Raster canvases draw text as paths while the program has taken
# `RendererAgg.draw_text` away, as `RendererBase.draw_text` draws it.
_backends.set_texts_as_paths((RendererAgg, "draw_text", RendererBase.draw_text))


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
        with self._wait_cursor():
            with self.renderer._drawing():
                # Through the figure's class' own draw, if it has one, and
                # the renderer's drawing methods the program replaced.
                from .. import _figure_draws
                replaced = self.renderer._replaced_drawing()
                native = self.figure._native
                _figure_draws.draw(
                    self.figure,
                    (lambda: _backends.draw_replacing(native, self.renderer, replaced))
                    if replaced else (lambda: _backends.draw(native)),
                    lambda: self.renderer)
            DrawEvent("draw_event", self, self.renderer)._process()
            # A GUI class may be need to update a window using this draw, so
            # don't forget to call the superclass.
            super().draw()

    def _wait_cursor(self):
        """The toolbar's wait cursor for a draw, as matplotlib's
        `FigureCanvasAgg.draw` sets it."""
        return (self.toolbar._wait_cursor_for_draw_cm() if self.toolbar
                else nullcontext())

    @contextmanager
    def _printing(self, bbox_inches_restore=None):
        """Draw for a print as matplotlib's raster prints draw, through
        `FigureCanvasAgg.draw`: with the renderer drawing, under the
        toolbar's wait cursor (which a print does not show, its canvas
        having no manager meanwhile, but which times the cursor of the next
        draw). A print of a part of the figure (*bbox_inches_restore*)
        draws with a renderer of the part's size, which matplotlib's
        canvas keeps in place of its renderer: the next draw of the whole
        figure makes a new one."""
        with self._wait_cursor(), self.get_renderer()._drawing():
            yield
        if bbox_inches_restore is not None:
            self._lastKey = None

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

    def _print_raster(self, filename_or_obj, fmt, metadata, pil_kwargs):
        """
        Write the figure as the raster format *fmt*. pltrs (Rust) encodes
        it, refusing *metadata* for the formats other than PNG as
        matplotlib does, but for what only matplotlib's own writer, Pillow,
        does: the formats of `_PILLOW_FORMATS`, and *pil_kwargs* other
        than a JPEG quality as a number (Pillow's presets, and its errors
        for any other value, are Pillow's).
        """
        _check_pil_arguments(fmt, metadata, pil_kwargs)
        settings = {"metadata": metadata}
        if (fmt == "jpeg" and pil_kwargs is not None and set(pil_kwargs) == {"quality"}
                and isinstance(pil_kwargs["quality"], int)):
            settings["jpeg_quality"] = pil_kwargs["quality"]
        elif fmt in _PILLOW_FORMATS or pil_kwargs:
            self._print_pil(filename_or_obj, fmt, pil_kwargs, metadata)
            return
        with self._printing(self._print_area(None)):
            self._print_native(fmt, filename_or_obj, **settings)

    def _print_pil(self, filename_or_obj, fmt, pil_kwargs, metadata):
        """
        Draw the figure with the print settings of the moment, then save its
        pixels using `.image.imsave` (to which *pil_kwargs* and *metadata*
        are forwarded), as matplotlib's ``_print_pil`` does.
        """
        with self._printing(self._print_area(None)):
            rgba = _backends.print_rgba(self.figure._native,
                                        self._print_settings(),
                                        self._draw_event_sender())
        # savefig() has already applied savefig.facecolor; imsave() blends
        # semi-transparent JPEG figures against an assumed white background.
        with (rc_context({"savefig.facecolor": "white"}) if fmt == "jpeg"
              else nullcontext()):
            imsave(filename_or_obj, rgba, format=fmt, origin="upper",
                   dpi=self.figure.dpi, metadata=metadata,
                   pil_kwargs=pil_kwargs)

    def print_raw(self, filename_or_obj, *, metadata=None):
        if metadata is not None:
            raise ValueError("metadata not supported for raw/rgba")
        with self._printing(self._print_area(None)):
            self._print_native("raw", filename_or_obj)

    print_rgba = print_raw

    def print_png(self, filename_or_obj, *, metadata=None, pil_kwargs=None):
        """
        Write the figure to a PNG file.

        Parameters
        ----------
        filename_or_obj : str or path-like or file-like
            The file to write to.

        metadata : dict, optional
            Metadata in the PNG file as key-value pairs of bytes or latin-1
            encodable strings.
            According to the PNG specification, keys must be shorter than 79
            chars.

            The `PNG specification`_ defines some common keywords that may be
            used as appropriate:

            - Title: Short (one line) title or caption for image.
            - Author: Name of image's creator.
            - Description: Description of image (possibly long).
            - Copyright: Copyright notice.
            - Creation Time: Time of original image creation
              (usually RFC 1123 format).
            - Software: Software used to create the image.
            - Disclaimer: Legal disclaimer.
            - Warning: Warning of nature of content.
            - Source: Device used to create the image.
            - Comment: Miscellaneous comment;
              conversion from other image format.

            Other keywords may be invented for other purposes.

            If 'Software' is not given, an autogenerated value for Matplotlib
            will be used.  This can be removed by setting it to *None*.

            For more details see the `PNG specification`_.

            .. _PNG specification: \
                https://www.w3.org/TR/2003/REC-PNG-20031110/#11keywords

        pil_kwargs : dict, optional
            Keyword arguments passed to `PIL.Image.Image.save`.

            If the 'pnginfo' key is present, it completely overrides
            *metadata*, including the default 'Software' key.
        """
        self._print_raster(filename_or_obj, "png", metadata, pil_kwargs)

    def print_to_buffer(self):
        FigureCanvasAgg.draw(self)
        renderer = self.get_renderer()
        return (bytes(renderer.buffer_rgba()),
                (int(renderer.width), int(renderer.height)))

    # Note that these methods should typically be called via savefig() and
    # print_figure(), and the latter ensures that `self.figure.dpi` already
    # matches the dpi kwarg (if any).

    def print_gif(self, filename_or_obj, *, metadata=None, pil_kwargs=None):
        self._print_raster(filename_or_obj, "gif", metadata, pil_kwargs)

    def print_jpg(self, filename_or_obj, *, metadata=None, pil_kwargs=None):
        self._print_raster(filename_or_obj, "jpeg", metadata, pil_kwargs)

    print_jpeg = print_jpg

    def print_tif(self, filename_or_obj, *, metadata=None, pil_kwargs=None):
        self._print_raster(filename_or_obj, "tiff", metadata, pil_kwargs)

    print_tiff = print_tif

    def print_webp(self, filename_or_obj, *, metadata=None, pil_kwargs=None):
        self._print_raster(filename_or_obj, "webp", metadata, pil_kwargs)

    def print_avif(self, filename_or_obj, *, metadata=None, pil_kwargs=None):
        self._print_raster(filename_or_obj, "avif", metadata, pil_kwargs)

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


# What matplotlib's module also has from its imports, which programs
# import from it.
import math  # noqa: E402, F401
from PIL import features  # noqa: E402, F401
from .. import cbook  # noqa: E402, F401
from ..dviread import Dvi  # noqa: E402, F401
from ..font_manager import get_font  # noqa: E402, F401
from ..ft2font import RenderMode  # noqa: E402, F401
from ..mathtext import MathTextParser  # noqa: E402, F401
from ..path import Path  # noqa: E402, F401
mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
