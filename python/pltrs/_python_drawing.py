"""
Drawing defined in Python, inside pltrs (Rust)'s draws.

matplotlib draws a figure by calling each artist's ``draw(renderer)``, so
a subclass of the program's draws whatever its ``draw`` draws, and path
effects of the program's classes draw through ``draw_path(renderer, gc,
tpath, affine, rgbFace)``. pltrs (Rust) draws figures itself, and calls
the Python code back where matplotlib would:

* **Artists.** An artist whose class takes ``draw`` from outside this
  package, or computes what matplotlib's ``draw`` reads, gets a painter
  (`attach`, ``_native.drawing.set_painter``).
  Before a figure's artists are drawn, once it is laid out, pltrs (Rust)
  calls `record_drawing` for each such artist the draw reaches, in drawing
  order: the artist's ``draw`` runs with the renderer of the draw (the
  canvas's `RendererAgg`, or a `MixedModeRenderer` around the vector
  renderer of the format being saved, see `drawing_with`) recording into
  a drawing, which pltrs (Rust) draws at the artist's place in the
  z-order. The ``draw`` may change the figure, as matplotlib's ``draw``
  methods do; ``super().draw(renderer)`` draws the artist as pltrs (Rust)
  draws it, there. Before the figure is laid out or its tight box
  computed, an artist whose class measures it (``get_window_extent``,
  ``get_tightbbox``) is measured (`measure_artist`).
* **Path effects** of the program's classes are called while pltrs (Rust)
  draws a path (`effect_draw_path`), with the renderer recording.
* **Renderers of the program's** (``RendererBase`` subclasses this
  package does not define): pltrs (Rust) draws an artist through their
  methods (`draw_through`), calling them through `forward_drawing`.

Figures without such artists, effects and renderers never call Python.
"""

import contextlib
import weakref

import numpy as np

from . import _live_transforms, transforms as mtransforms
from ._native import drawing as _drawing, register as _register
from ._native import transforms as _transforms
from ._program_classes import takes_from_program
from .path import Path

#: The renderers pltrs (Rust) draws with, innermost last: what the program's
#: drawing receives, as ``[make, renderer]`` (the renderer ``make()`` gives
#: is made when first needed).
_renderers = []

#: How many of the calls running draw while pltrs (Rust) is drawing the
#: figure (an artist the draw reached before it was prepared, a path
#: effect, a renderer of the program's), which cannot change it then.
_drawing_figures = 0


def figure_changeable():
    """Whether the program's drawing running now may change the figure it
    draws: not while pltrs (Rust) draws it."""
    return _drawing_figures == 0


@contextlib.contextmanager
def _figure_drawn(drawn=True):
    """While pltrs (Rust) draws the figure, with *drawn*."""
    global _drawing_figures
    _drawing_figures += drawn
    try:
        yield
    finally:
        _drawing_figures -= drawn


def _own_draw(cls):
    """The ``draw`` of *cls* without the rasterization wrapper the class
    got (``allow_rasterization``, or ``Artist``'s own), which pltrs (Rust)
    applies around the drawing; whether that wrapper lets the artist be
    rasterized."""
    draw = cls.draw
    rasterizes = getattr(draw, "_supports_rasterization", None)
    if rasterizes is None:
        return draw, False
    return getattr(draw, "__wrapped__", draw), rasterizes


def attach(artist):
    """Have pltrs (Rust) draw *artist* through its ``draw`` when its class
    takes ``draw`` from the program, or computes what matplotlib's ``draw``
    reads (the ``_computed`` attributes of the package's class, such as an
    arc's ``theta1``)."""
    cls = type(artist)
    if any(takes_from_program(cls, name)
           for name in ("draw", *getattr(cls, "_computed", ()))):
        measures = any(takes_from_program(cls, name)
                       for name in ("get_window_extent", "get_tightbbox"))
        _drawing.set_painter(artist._native, weakref.ref(artist),
                             _own_draw(cls)[1], measures)


@contextlib.contextmanager
def drawing_with(make):
    """Hand the program's drawing the renderer *make()* makes (when first
    needed) while pltrs (Rust) draws."""
    _renderers.append([make, None])
    try:
        yield
    finally:
        _renderers.pop()


def _renderer_for(target, figure):
    """The renderer the program's drawing receives in a draw of *figure*
    with *target* (see ``_native.drawing``): the one installed
    (`drawing_with`), else one standing for pltrs (Rust)'s renderer."""
    if _renderers:
        installed = _renderers[-1]
        if installed[1] is None:
            installed[1] = installed[0]()
        return installed[1]
    width, height, dpi, vector, magnification = target[:5]
    if vector:
        from .backends.backend_mixed import _stand_in
        return _stand_in(figure, width, height, magnification)
    from .backends.backend_agg import RendererAgg
    return RendererAgg(width, height, dpi)


@contextlib.contextmanager
def _recording(renderer, raster_depth=0):
    """Record what is drawn with *renderer* into a new drawing (yielded),
    the renderer standing *raster_depth* rasterized artists deep."""
    drawing = _drawing.new()
    saved = (renderer._recording, renderer._raster_depth, renderer._rasterizing)
    renderer._recording = drawing
    renderer._raster_depth = raster_depth
    renderer._rasterizing = raster_depth > 0
    try:
        yield drawing
    finally:
        (renderer._recording, renderer._raster_depth,
         renderer._rasterizing) = saved


def record_drawing(artist, target):
    """Draw *artist* with the renderer of a draw with *target* (see
    ``_native.drawing``), recording: the drawing, which pltrs (Rust) draws
    at the artist's place."""
    renderer = _renderer_for(target, artist.get_figure(root=True))
    draw, _ = _own_draw(type(artist))
    with _figure_drawn(target[6]), _recording(renderer, target[5]) as drawing:
        if figure_changeable():
            _evaluate_transforms(artist)
        draw(artist, renderer)
    return drawing


def _evaluate_transforms(artist):
    """Bring the transforms *artist*'s ``draw`` reads up to date, as
    matplotlib evaluates them when it draws: those built on live
    transforms compute again, and the transform the artist was given is
    handed to pltrs (Rust) again as it is now (a `.BboxTransformTo` of a
    box in data coordinates follows the limits)."""
    _live_transforms.invalidate_all()
    given = artist.__dict__.get("_given_transform")
    if given is not None:
        _transforms.set_transform(artist._native, _live_transforms.to_native(given))


def effect_draw_path(effect, target, gc_spec, geometry):
    """Draw a path with *effect*, a path effect of the program's class, with
    the renderer of a draw with *target*, recording: the drawing, which
    pltrs (Rust) draws in the path's place, and the graphics context as the
    effect left it, which the effects after it receive (matplotlib hands
    them all the same one). *geometry* is ``((vertices, codes), matrix,
    rgbFace)``: the path, the affine transform into display pixels and the
    fill."""
    from .backends._native_drawing import gc_from_native, native_gc
    (vertices, codes), matrix, rgbFace = geometry
    renderer = _renderer_for(target, None)
    with _figure_drawn(), _recording(renderer, target[5]) as drawing:
        gc = gc_from_native(gc_spec, renderer.new_gc())
        effect.draw_path(renderer, gc, Path(vertices, codes),
                         mtransforms.Affine2D(matrix), rgbFace)
    return drawing, native_gc(gc)


def measure_artist(artist):
    """The extents of the box *artist* occupies in display coordinates as
    its class measures it (its ``get_tightbbox``, as matplotlib's layouts
    and tight boxes ask it), None for no space: what pltrs (Rust) takes
    for it, before it lays the figure out or computes its tight box."""
    figure = artist.get_figure(root=True)
    bbox = artist.get_tightbbox(None if figure is None else figure._get_renderer())
    if bbox is None or (bbox.width == 0 and bbox.height == 0):
        return None
    return tuple(map(float, bbox.extents))


def draw_through(renderer, artist):
    """Draw *artist*, as pltrs (Rust) draws it, through the drawing methods
    of *renderer*, a renderer of the program's (matplotlib's
    ``artist.draw(renderer)``); a figure is laid out and drawn whole."""
    with _figure_drawn():
        _drawing.draw_through(renderer, artist._native)


@contextlib.contextmanager
def _gc_of(renderer, spec):
    """A graphics context of *renderer* set as pltrs (Rust) describes one
    (*spec*), restored once the drawing call is done."""
    from .backends._native_drawing import gc_from_native
    gc = gc_from_native(spec, renderer.new_gc())
    try:
        yield gc
    finally:
        gc.restore()


class forward_drawing:
    """
    pltrs (Rust)'s drawing calls through a renderer of the program's: each
    function turns its arguments into matplotlib's (a graphics context from
    ``renderer.new_gc()``, `.Path`, `.Affine2D`) and calls the renderer's
    method of the same name.
    """

    @staticmethod
    def draw_path(renderer, gc, path, matrix, rgbFace):
        """``renderer.draw_path``."""
        with _gc_of(renderer, gc) as gc:
            renderer.draw_path(gc, Path(*path), mtransforms.Affine2D(matrix),
                               rgbFace)

    @staticmethod
    def draw_markers(renderer, gc, marker, marker_matrix, path, matrix, rgbFace):
        """``renderer.draw_markers``."""
        with _gc_of(renderer, gc) as gc:
            renderer.draw_markers(gc, Path(*marker),
                                  mtransforms.Affine2D(marker_matrix),
                                  Path(*path), mtransforms.Affine2D(matrix),
                                  rgbFace)

    @staticmethod
    def draw_path_collection(renderer, gc, master, paths, transforms, offsets,
                             colors, strokes):
        """``renderer.draw_path_collection``, the *offsets* in display
        pixels."""
        facecolors, edgecolors, hatchcolors = colors
        linewidths, linestyles, antialiaseds, urls = strokes
        with _gc_of(renderer, gc) as gc:
            renderer.draw_path_collection(
                gc, mtransforms.Affine2D(master), [Path(*path) for path in paths],
                transforms, offsets, mtransforms.IdentityTransform(), facecolors,
                edgecolors, np.asarray(linewidths, dtype=float), linestyles,
                antialiaseds, urls, "screen", hatchcolors=hatchcolors)

    @staticmethod
    def draw_quad_mesh(renderer, gc, master, coordinates, facecolors,
                       antialiased, edgecolors):
        """``renderer.draw_quad_mesh`` of the ``(rows + 1, columns + 1, 2)``
        vertices *coordinates*."""
        rows, columns = coordinates.shape[0] - 1, coordinates.shape[1] - 1
        with _gc_of(renderer, gc) as gc:
            renderer.draw_quad_mesh(
                gc, mtransforms.Affine2D(master), columns, rows, coordinates,
                np.zeros((1, 2)), mtransforms.IdentityTransform(), facecolors,
                antialiased, edgecolors if len(edgecolors) else None)

    @staticmethod
    def draw_gouraud_triangles(renderer, gc, triangles, colors, matrix):
        """``renderer.draw_gouraud_triangles``."""
        with _gc_of(renderer, gc) as gc:
            renderer.draw_gouraud_triangles(gc, triangles, colors,
                                            mtransforms.Affine2D(matrix))

    @staticmethod
    def draw_image(renderer, gc, origin, im, matrix):
        """``renderer.draw_image``, with a transform for renderers that
        scale images."""
        x, y = origin
        with _gc_of(renderer, gc) as gc:
            if matrix is None:
                renderer.draw_image(gc, x, y, im)
            else:
                renderer.draw_image(gc, x, y, im, mtransforms.Affine2D(matrix))


_register("record_drawing", record_drawing)
_register("effect_draw_path", effect_draw_path)
_register("measure_artist", measure_artist)
_register("forward_drawing", forward_drawing)
_register("artist_reference", weakref.ref)
