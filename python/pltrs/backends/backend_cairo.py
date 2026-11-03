"""
A Cairo backend for Matplotlib
==============================
:Author: Steve Chaplin and others

This backend depends on cairocffi or pycairo (matplotlib's ``backend_cairo``).

`RendererCairo` and `GraphicsContextCairo` draw onto a Cairo context, as
matplotlib's do: a figure drawn with one (``figure.draw(renderer)`` after
``renderer.set_context(ctx)``) is drawn through their methods. The canvas
of the ``cairo`` backend, `FigureCanvasCairo`, draws with pltrs (Rust)
instead, as the ``*cairo`` GUI canvases do: onto its raster canvas for PNG
and raw RGBA, as `.FigureCanvasAgg` does, and with its PDF, PostScript and
SVG writers for those formats.

From matplotlib 3.11.2, under the Matplotlib License, kept as it is but for
its imports of the package and the canvas.
"""

import functools

import numpy as np

try:
    import cairo
    if cairo.version_info < (1, 14, 0):  # Introduced set_device_scale.
        raise ImportError(f"Cairo backend requires cairo>=1.14.0, "
                          f"but only {cairo.version_info} is available")
except ImportError:
    try:
        import cairocffi as cairo
    except ImportError as err:
        raise ImportError(
            "cairo backend requires that pycairo>=1.14.0 or cairocffi "
            "is installed") from err

from .. import _api, cbook, font_manager
from ..backend_bases import (
    _Backend, FigureManagerBase, GraphicsContextBase, RendererBase)
from ..font_manager import ttfFontProperty
from ..path import Path
from ..transforms import Affine2D
from .backend_agg import FigureCanvasAgg


def _set_rgba(ctx, color, alpha, forced_alpha):
    if len(color) == 3 or forced_alpha:
        ctx.set_source_rgba(*color[:3], alpha)
    else:
        ctx.set_source_rgba(*color)


def _append_path(ctx, path, transform, clip=None):
    for points, code in path.iter_segments(
            transform, remove_nans=True, clip=clip):
        if code == Path.MOVETO:
            ctx.move_to(*points)
        elif code == Path.CLOSEPOLY:
            ctx.close_path()
        elif code == Path.LINETO:
            ctx.line_to(*points)
        elif code == Path.CURVE3:
            cur = np.asarray(ctx.get_current_point())
            a = points[:2]
            b = points[-2:]
            ctx.curve_to(*(cur / 3 + a * 2 / 3), *(a * 2 / 3 + b / 3), *b)
        elif code == Path.CURVE4:
            ctx.curve_to(*points)


def _cairo_font_args_from_font_prop(prop):
    """
    Convert a `.FontProperties` or a `.FontEntry` to arguments that can be
    passed to `.Context.select_font_face`.
    """
    def attr(field):
        try:
            return getattr(prop, f"get_{field}")()
        except AttributeError:
            return getattr(prop, field)

    name = attr("name")
    slant = getattr(cairo, f"FONT_SLANT_{attr('style').upper()}")
    weight = attr("weight")
    weight = (cairo.FONT_WEIGHT_NORMAL
              if font_manager.weight_dict.get(weight, weight) < 550
              else cairo.FONT_WEIGHT_BOLD)
    return name, slant, weight


class RendererCairo(RendererBase):
    """A renderer drawing onto the Cairo context given to `set_context`."""

    def __init__(self, dpi):
        self.dpi = dpi
        self.gc = GraphicsContextCairo(renderer=self)
        self.width = None
        self.height = None
        self.text_ctx = cairo.Context(
           cairo.ImageSurface(cairo.FORMAT_ARGB32, 1, 1))
        super().__init__()

    def set_context(self, ctx):
        """Draw onto the Cairo context *ctx*, sized as its surface."""
        surface = ctx.get_target()
        if hasattr(surface, "get_width") and hasattr(surface, "get_height"):
            size = surface.get_width(), surface.get_height()
        elif hasattr(surface, "get_extents"):  # GTK4 RecordingSurface.
            ext = surface.get_extents()
            size = ext.width, ext.height
        else:  # vector surfaces.
            ctx.save()
            ctx.reset_clip()
            rect, *rest = ctx.copy_clip_rectangle_list()
            if rest:
                raise TypeError("Cannot infer surface size")
            _, _, *size = rect
            ctx.restore()
        self.gc.ctx = ctx
        self.width, self.height = size

    @staticmethod
    def _fill_and_stroke(ctx, fill_c, alpha, alpha_overrides):
        if fill_c is not None:
            ctx.save()
            _set_rgba(ctx, fill_c, alpha, alpha_overrides)
            ctx.fill_preserve()
            ctx.restore()
        ctx.stroke()

    def draw_path(self, gc, path, transform, rgbFace=None):
        # docstring inherited
        ctx = gc.ctx
        # Clip the path to the actual rendering extents if it isn't filled.
        clip = (ctx.clip_extents()
                if rgbFace is None and gc.get_hatch() is None
                else None)
        transform = (transform
                     + Affine2D().scale(1, -1).translate(0, self.height))
        ctx.new_path()
        _append_path(ctx, path, transform, clip)
        if rgbFace is not None:
            ctx.save()
            _set_rgba(ctx, rgbFace, gc.get_alpha(), gc.get_forced_alpha())
            ctx.fill_preserve()
            ctx.restore()
        hatch_path = gc.get_hatch_path()
        if hatch_path:
            dpi = int(self.dpi)
            hatch_surface = ctx.get_target().create_similar(
                cairo.Content.COLOR_ALPHA, dpi, dpi)
            hatch_ctx = cairo.Context(hatch_surface)
            _append_path(hatch_ctx, hatch_path,
                         Affine2D().scale(dpi, -dpi).translate(0, dpi),
                         None)
            hatch_ctx.set_line_width(self.points_to_pixels(gc.get_hatch_linewidth()))
            hatch_ctx.set_source_rgba(*gc.get_hatch_color())
            hatch_ctx.fill_preserve()
            hatch_ctx.stroke()
            hatch_pattern = cairo.SurfacePattern(hatch_surface)
            hatch_pattern.set_extend(cairo.Extend.REPEAT)
            ctx.save()
            ctx.set_source(hatch_pattern)
            ctx.fill_preserve()
            ctx.restore()
        ctx.stroke()

    def draw_markers(self, gc, marker_path, marker_trans, path, transform,
                     rgbFace=None):
        # docstring inherited

        ctx = gc.ctx
        ctx.new_path()
        # Create the path for the marker; it needs to be flipped here already!
        _append_path(ctx, marker_path, marker_trans + Affine2D().scale(1, -1))
        marker_path = ctx.copy_path_flat()

        # Figure out whether the path has a fill
        x1, y1, x2, y2 = ctx.fill_extents()
        if x1 == 0 and y1 == 0 and x2 == 0 and y2 == 0:
            filled = False
            # No fill, just unset this (so we don't try to fill it later on)
            rgbFace = None
        else:
            filled = True

        transform = (transform
                     + Affine2D().scale(1, -1).translate(0, self.height))

        ctx.new_path()
        for i, (vertices, codes) in enumerate(
                path.iter_segments(transform, simplify=False)):
            if len(vertices):
                x, y = vertices[-2:]
                ctx.save()

                # Translate and apply path
                ctx.translate(x, y)
                ctx.append_path(marker_path)

                ctx.restore()

                # Slower code path if there is a fill; we need to draw
                # the fill and stroke for each marker at the same time.
                # Also flush out the drawing every once in a while to
                # prevent the paths from getting way too long.
                if filled or i % 1000 == 0:
                    self._fill_and_stroke(
                        ctx, rgbFace, gc.get_alpha(), gc.get_forced_alpha())

        # Fast path, if there is no fill, draw everything in one step
        if not filled:
            self._fill_and_stroke(
                ctx, rgbFace, gc.get_alpha(), gc.get_forced_alpha())

    def draw_image(self, gc, x, y, im):
        # docstring inherited
        im = cbook._unmultiplied_rgba8888_to_premultiplied_argb32(im[::-1])
        surface = cairo.ImageSurface.create_for_data(
            im.ravel().data, cairo.FORMAT_ARGB32,
            im.shape[1], im.shape[0], im.shape[1] * 4)
        ctx = gc.ctx
        y = self.height - y - im.shape[0]

        ctx.save()
        ctx.set_source_surface(surface, float(x), float(y))
        ctx.paint()
        ctx.restore()

    def draw_text(self, gc, x, y, s, prop, angle, ismath=False, mtext=None):
        # docstring inherited

        # Note: (x, y) are device/display coords, not user-coords, unlike other
        # draw_* methods
        if ismath:
            self._draw_mathtext(gc, x, y, s, prop, angle)

        else:
            ctx = gc.ctx
            ctx.new_path()
            ctx.move_to(x, y)

            ctx.save()
            ctx.select_font_face(*_cairo_font_args_from_font_prop(prop))
            ctx.set_font_size(self.points_to_pixels(prop.get_size_in_points()))
            opts = cairo.FontOptions()
            opts.set_antialias(gc.get_antialiased())
            ctx.set_font_options(opts)
            if angle:
                ctx.rotate(np.deg2rad(-angle))
            ctx.show_text(s)
            ctx.restore()

    def _draw_mathtext(self, gc, x, y, s, prop, angle):
        ctx = gc.ctx
        width, height, descent, glyphs, rects = \
            self._text2path.mathtext_parser.parse(s, self.dpi, prop)

        ctx.save()
        ctx.translate(x, y)
        if angle:
            ctx.rotate(np.deg2rad(-angle))

        for font, fontsize, ccode, _glyph_index, ox, oy in glyphs:
            ctx.new_path()
            ctx.move_to(ox, -oy)
            ctx.select_font_face(*_cairo_font_args_from_font_prop(ttfFontProperty(font)))
            ctx.set_font_size(self.points_to_pixels(fontsize))
            ctx.show_text(chr(ccode))

        for ox, oy, w, h in rects:
            ctx.new_path()
            ctx.rectangle(ox, -oy, w, -h)
            ctx.set_source_rgb(0, 0, 0)
            ctx.fill_preserve()

        ctx.restore()

    def get_canvas_width_height(self):
        # docstring inherited
        return self.width, self.height

    def get_text_width_height_descent(self, s, prop, ismath):
        # docstring inherited

        if ismath == 'TeX':
            return super().get_text_width_height_descent(s, prop, ismath)

        if ismath:
            width, height, descent, *_ = \
                self._text2path.mathtext_parser.parse(s, self.dpi, prop)
            return width, height, descent

        ctx = self.text_ctx
        # problem - scale remembers last setting and font can become
        # enormous causing program to crash
        # save/restore prevents the problem
        ctx.save()
        ctx.select_font_face(*_cairo_font_args_from_font_prop(prop))
        ctx.set_font_size(self.points_to_pixels(prop.get_size_in_points()))

        y_bearing, w, h = ctx.text_extents(s)[1:4]
        ctx.restore()

        return w, h, h + y_bearing

    def new_gc(self):
        # docstring inherited
        self.gc.ctx.save()
        # The graphics context is a single-level stack: artists never rely
        # on nesting its states, so resetting the attributes is enough.
        self.gc._alpha = 1
        self.gc._forced_alpha = False  # if True, _alpha overrides A from RGBA
        self.gc._hatch = None
        return self.gc

    def points_to_pixels(self, points):
        # docstring inherited
        return points / 72 * self.dpi


class GraphicsContextCairo(GraphicsContextBase):
    """The graphics context of `RendererCairo`: the state of its Cairo
    context."""

    _joind = {
        'bevel':  cairo.LINE_JOIN_BEVEL,
        'miter':  cairo.LINE_JOIN_MITER,
        'round':  cairo.LINE_JOIN_ROUND,
    }

    _capd = {
        'butt':        cairo.LINE_CAP_BUTT,
        'projecting':  cairo.LINE_CAP_SQUARE,
        'round':       cairo.LINE_CAP_ROUND,
    }

    def __init__(self, renderer):
        super().__init__()
        self.renderer = renderer

    def restore(self):
        self.ctx.restore()

    def set_alpha(self, alpha):
        super().set_alpha(alpha)
        _set_rgba(
            self.ctx, self._rgb, self.get_alpha(), self.get_forced_alpha())

    def set_antialiased(self, b):
        self.ctx.set_antialias(
            cairo.ANTIALIAS_DEFAULT if b else cairo.ANTIALIAS_NONE)

    def get_antialiased(self):
        return self.ctx.get_antialias()

    def set_capstyle(self, cs):
        self.ctx.set_line_cap(_api.getitem_checked(self._capd, capstyle=cs))
        self._capstyle = cs

    def set_clip_rectangle(self, rectangle):
        if not rectangle:
            return
        x, y, w, h = np.round(rectangle.bounds)
        ctx = self.ctx
        ctx.new_path()
        ctx.rectangle(x, self.renderer.height - h - y, w, h)
        ctx.clip()

    def set_clip_path(self, path):
        if not path:
            return
        tpath, affine = path.get_transformed_path_and_affine()
        ctx = self.ctx
        ctx.new_path()
        affine = (affine
                  + Affine2D().scale(1, -1).translate(0, self.renderer.height))
        _append_path(ctx, tpath, affine)
        ctx.clip()

    def set_dashes(self, offset, dashes):
        self._dashes = offset, dashes
        if dashes is None:
            self.ctx.set_dash([], 0)  # switch dashes off
        else:
            self.ctx.set_dash(
                list(self.renderer.points_to_pixels(np.asarray(dashes))),
                offset)

    def set_foreground(self, fg, isRGBA=None):
        super().set_foreground(fg, isRGBA)
        if len(self._rgb) == 3:
            self.ctx.set_source_rgb(*self._rgb)
        else:
            self.ctx.set_source_rgba(*self._rgb)

    def get_rgb(self):
        return self.ctx.get_source().get_rgba()[:3]

    def set_joinstyle(self, js):
        self.ctx.set_line_join(_api.getitem_checked(self._joind, joinstyle=js))
        self._joinstyle = js

    def set_linewidth(self, w):
        self._linewidth = float(w)
        self.ctx.set_line_width(self.renderer.points_to_pixels(w))


class FigureCanvasCairo(FigureCanvasAgg):
    """
    The canvas of the ``cairo`` backend. pltrs (Rust) draws the figure: onto
    its raster canvas, as `.FigureCanvasAgg` does, for PNG and raw RGBA
    (``copy_from_bbox`` and ``restore_region`` work on that canvas), and as
    a PDF, PostScript or SVG document, at 72 dpi as matplotlib's Cairo
    surfaces are, for those formats.
    """

    def _save(self, fmt, fobj, *, orientation='portrait',
              bbox_inches_restore=None, facecolor=None, edgecolor=None):
        """Write the figure as a *fmt* document: ``pdf``, ``ps``, ``svg`` or
        ``svgz``; a PostScript page in *orientation*."""
        self.figure.dpi = 72
        self._print_native(fmt, fobj, orientation=orientation,
                           bbox_inches_restore=bbox_inches_restore,
                           facecolor=facecolor, edgecolor=edgecolor)

    print_pdf = functools.partialmethod(_save, "pdf")
    print_ps = functools.partialmethod(_save, "ps")
    print_svg = functools.partialmethod(_save, "svg")
    print_svgz = functools.partialmethod(_save, "svgz")


@_Backend.export
class _BackendCairo(_Backend):
    backend_version = cairo.version
    FigureCanvas = FigureCanvasCairo
    FigureManager = FigureManagerBase
