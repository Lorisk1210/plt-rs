"""
The drawing methods of the renderers pltrs hands Python (`RendererAgg`,
`RendererPdf`, `RendererSVG`, `RendererPS`, `RendererPgf`), done by pltrs
(Rust).

A renderer of these classes draws into a *drawing* (``_native.drawing``):
each of matplotlib's drawing methods (``draw_path``, ``draw_markers``,
``draw_path_collection``, ``draw_quad_mesh``, ``draw_gouraud_triangles``,
``draw_image``, ``draw_text``, ``draw_tex``) adds its step, as do groups
and filters, and an artist drawn with the renderer (``artist.draw``)
adds itself as pltrs (Rust) draws it. While pltrs (Rust) draws a figure
with the renderer, the drawing is the one it records the program's
drawing into (see `.._python_drawing`), which it then draws where it is
drawing; otherwise each call's drawing is drawn at once where the
renderer draws (`NativeDrawingMixin._draw_now`).

Graphics contexts cross as `native_gc` describes them, and come back
from pltrs (Rust) through `gc_from_native`.
"""

from contextlib import contextmanager

import numpy as np

from .. import _python_drawing, colors as mcolors
from .._native import backends as _drawing_pages, drawing as _drawing
from ..font_manager import FontManager
from ..path import Path
from ..transforms import Affine2D, Bbox, TransformedPath


def native_gc(gc):
    """The graphics context *gc* as pltrs (Rust) takes it (``(stroke,
    dashes, clipping, hatch, extras)``)."""
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


def gc_from_native(spec, gc):
    """Set the graphics context *gc* as pltrs (Rust) describes one
    (*spec*, the form `native_gc` gives, its clip path as the
    ``(vertices, codes)`` of a path); return *gc*."""
    stroke, (offset, dashes), (clip, clip_path), hatch, extras = spec
    rgba, alpha, antialiased, capstyle, joinstyle, linewidth = stroke
    gc.set_alpha(alpha)
    gc.set_foreground(tuple(rgba), isRGBA=True)
    gc.set_antialiased(antialiased)
    gc.set_capstyle(capstyle)
    gc.set_joinstyle(joinstyle)
    gc.set_linewidth(linewidth)
    gc.set_dashes(offset, dashes)
    gc.set_clip_rectangle(None if clip is None else Bbox.from_extents(*clip))
    if clip_path is not None:
        (vertices, codes), matrix = clip_path
        gc.set_clip_path(TransformedPath(Path(vertices, codes), Affine2D(matrix)))
    pattern, hatch_color, hatch_linewidth = hatch
    gc.set_hatch(pattern)
    if hatch_color is not None:
        gc.set_hatch_color(hatch_color)
    if hatch_linewidth is not None:
        gc.set_hatch_linewidth(hatch_linewidth)
    snap, sketch, url, gid = extras
    gc.set_snap(snap)
    gc.set_sketch_params(*(sketch or ()))
    gc.set_url(url)
    gc.set_gid(gid)
    return gc


def _matrix(transform):
    """The 3×3 matrix of the affine *transform*."""
    return np.asarray(transform.get_matrix(), dtype=float)


def _face(rgbFace):
    """The fill colour *rgbFace* as an RGBA tuple, or None."""
    return None if rgbFace is None else tuple(mcolors.to_rgba(rgbFace))


def _rgba_array(colors):
    """*colors* as an ``(N, 4)`` RGBA array."""
    if colors is None or len(colors) == 0:
        return np.zeros((0, 4))
    return mcolors.to_rgba_array(colors)


class NativeDrawingMixin:
    """
    matplotlib's drawing methods of a renderer, done by pltrs (Rust): each
    adds its step to the drawing the renderer draws into (see the module
    documentation).
    """

    #: The drawing pltrs (Rust) records into while it draws a figure with
    #: the renderer (see `.._python_drawing`), else None.
    _recording = None

    @contextmanager
    def _target(self):
        """The drawing the renderer draws into now: the one it records into
        while pltrs (Rust) draws a figure with it, else one of the call's
        own, drawn once the call is done (`_draw_now`)."""
        if self._recording is not None:
            yield self._recording
            return
        drawing = _drawing.new()
        yield drawing
        self._draw_now(drawing)

    def _display_y(self, y):
        """The height of *y* above the bottom of the canvas, *y* given as a
        text's baseline is (from the top for a renderer that flips y)."""
        if self.flipy():
            return self.get_canvas_width_height()[1] - y
        return y

    def draw_path(self, gc, path, transform, rgbFace=None):
        # docstring inherited
        with self._target() as drawing:
            _drawing.draw_path(drawing, native_gc(gc), path, _matrix(transform),
                               _face(rgbFace))

    def draw_markers(self, gc, marker_path, marker_trans, path, trans,
                     rgbFace=None):
        # docstring inherited
        with self._target() as drawing:
            _drawing.draw_markers(drawing, native_gc(gc), marker_path,
                                  _matrix(marker_trans), path, _matrix(trans),
                                  _face(rgbFace))

    def draw_path_collection(self, gc, master_transform, paths, all_transforms,
                             offsets, offset_trans, facecolors, edgecolors,
                             linewidths, linestyles, antialiaseds, urls,
                             offset_position, *, hatchcolors=None):
        # docstring inherited
        offsets = np.asarray(offsets, dtype=float).reshape((-1, 2))
        if len(offsets):
            offsets = offset_trans.transform(offsets)
        transforms = np.asarray(all_transforms, dtype=float).reshape((-1, 3, 3))
        linestyles = [(float(offset or 0),
                       None if dashes is None else [float(d) for d in dashes])
                      for offset, dashes in linestyles]
        with self._target() as drawing:
            _drawing.draw_path_collection(
                drawing, native_gc(gc),
                (_matrix(master_transform), list(paths), transforms, offsets),
                (_rgba_array(facecolors), _rgba_array(edgecolors),
                 _rgba_array(hatchcolors)),
                ([float(w) for w in np.atleast_1d(linewidths)], linestyles,
                 [bool(a) for a in np.atleast_1d(antialiaseds)],
                 [None if url is None else str(url) for url in urls]))

    def draw_quad_mesh(self, gc, master_transform, meshWidth, meshHeight,
                       coordinates, offsets, offsetTrans, facecolors,
                       antialiased, edgecolors):
        # docstring inherited
        coordinates = np.asarray(coordinates, dtype=float).reshape(
            (meshHeight + 1, meshWidth + 1, 2))
        with self._target() as drawing:
            _drawing.draw_quad_mesh(
                drawing, native_gc(gc), _matrix(master_transform), coordinates,
                _rgba_array(facecolors), bool(antialiased),
                _rgba_array(edgecolors))

    def draw_gouraud_triangles(self, gc, triangles_array, colors_array,
                               transform):
        # docstring inherited
        with self._target() as drawing:
            _drawing.draw_gouraud_triangles(
                drawing, native_gc(gc),
                np.asarray(triangles_array, dtype=float).reshape((-1, 3, 2)),
                np.asarray(colors_array, dtype=float).reshape((-1, 3, 4)),
                _matrix(transform))

    def draw_image(self, gc, x, y, im, transform=None):
        # docstring inherited
        with self._target() as drawing:
            _drawing.draw_image(
                drawing, native_gc(gc), (float(x), float(y)),
                np.asarray(im, dtype=np.uint8),
                None if transform is None else _matrix(transform))

    def _draw_line(self, gc, x, y, s, prop, angle, ismath):
        """Draw the line *s* typeset as *ismath* says (``draw_text``, and
        ``draw_tex`` with ``"TeX"``, which the classes define themselves,
        as matplotlib's do), the left end of its baseline at *x*, *y* (from
        the top for a renderer that flips y, as `.Text.draw` gives it),
        rotated by *angle* degrees. pltrs (Rust) lays the line out at the
        renderer's resolution, without the features and language of the
        text it may belong to."""
        with self._target() as drawing:
            _drawing.draw_text(
                drawing, native_gc(gc), (float(x), float(self._display_y(y))),
                (s, FontManager._props(prop), ismath), float(angle),
                float(self.points_to_pixels(72)))

    def open_group(self, s, gid=None):
        # docstring inherited
        with self._target() as drawing:
            _drawing.open_group(drawing, str(s), None if gid is None else str(gid))

    def close_group(self, s):
        # docstring inherited
        with self._target() as drawing:
            _drawing.close_group(drawing, str(s))

    def start_filter(self):
        # docstring inherited
        with self._target() as drawing:
            _drawing.start_filter(drawing)

    def stop_filter(self, filter_func):
        # docstring inherited
        with self._target() as drawing:
            _drawing.stop_filter(drawing, filter_func)

    def _draw_artist(self, artist):
        """Draw *artist* as pltrs (Rust) draws it (`.Artist.draw` with this
        renderer): through the renderer's methods when the program's
        subclass overrides some."""
        if self._recording is None and _python_drawing.overrides_drawing(self):
            _python_drawing.draw_through(self, artist)
            return
        with self._target() as drawing:
            _drawing.draw_artist(drawing, artist._native)

    def _draw_cell(self, cell):
        """Draw the table `.Cell` *cell* alone, as pltrs (Rust) draws it
        where its table places it (`.Cell.draw` with this renderer)."""
        from .._native import table as _table
        with self._target() as drawing:
            _table.draw_cell_into(drawing, cell._ref())


class MathtextDrawingMixin:
    """``draw_mathtext``, which the Agg, PDF and PostScript renderers have:
    a line typeset as mathtext, drawn as `NativeDrawingMixin` draws text."""

    def draw_mathtext(self, gc, x, y, s, prop, angle):
        """Draw mathtext using :mod:`matplotlib.mathtext`."""
        self._draw_line(gc, x, y, s, prop, angle, True)


class PdfPsDrawingMixin(MathtextDrawingMixin):
    """The drawing methods `RendererPdf` and `RendererPS` name as
    matplotlib's do."""

    def draw_gouraud_triangles(self, gc, points, colors, trans):
        # docstring inherited
        super().draw_gouraud_triangles(gc, points, colors, trans)


class DocumentDrawingMixin(NativeDrawingMixin):
    """
    The drawing methods of the renderers of the vector backends
    (`RendererPdf`, `RendererSVG`, `RendererPS`, `RendererPgf`): they draw
    into the document pltrs (Rust) writes while it saves a figure through
    the format's canvas, which hands the program's drawing one of them
    (see `.._python_drawing`). A renderer the program makes around a file
    of its own draws into a document of its own (`_page`, see
    ``_native.backends``), whose drawing it writes to the file as
    matplotlib's renderers do.
    """

    #: The document a renderer made around a file of the program's draws
    #: into, None for the renderers of pltrs's saves.
    _page = None
    #: The file what the renderer draws is written to.
    _file = None

    def _document_page(self):
        """The renderer's own document; refused for the renderers of
        pltrs's saves, whose document pltrs (Rust) writes."""
        if self._page is None:
            raise RuntimeError(
                f"{type(self).__name__} draws into the document pltrs writes "
                "while it saves a figure: save the figure through its canvas "
                "(savefig), or make the renderer with a file to write to")
        return self._page

    def _write(self, written):
        """Write what drawing added to the renderer's document, ``(text,
        images)``, to its file and the image files beside it."""
        text, images = written
        for path, png in images:
            with open(path, "wb") as image:
                image.write(png)
        if text:
            self._file.write(text)

    def draw_text(self, gc, x, y, s, prop, angle, ismath=False, mtext=None):
        # docstring inherited
        self._draw_line(gc, x, y, s, prop, angle, ismath)

    def draw_tex(self, gc, x, y, s, prop, angle, *, mtext=None):
        # docstring inherited
        self._draw_line(gc, x, y, s, prop, angle, "TeX")

    def _draw_now(self, drawing):
        """Draw *drawing* into the renderer's own document, writing what
        it adds to the renderer's file."""
        self._write(_drawing_pages.page_draw(self._document_page(), drawing))
