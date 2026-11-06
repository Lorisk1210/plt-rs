"""
Drawing defined in Python, inside pltrs (Rust)'s draws.

matplotlib draws a figure by calling each artist's ``draw(renderer)``, so
a subclass of the program's draws whatever its ``draw`` draws, and path
effects of the program's classes draw through ``draw_path(renderer, gc,
tpath, affine, rgbFace)``. pltrs (Rust) draws figures itself, and calls
the Python code back where matplotlib would:

* **Artists.** An artist whose class takes ``draw`` from outside this
  package, or that the program gave a ``draw`` of its own
  (``artist.draw = ...``), or whose class computes what matplotlib's
  ``draw`` reads, gets a painter (`attach`, ``_native.drawing.set_painter``).
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
import functools
import threading
import types
import weakref

import numpy as np

from . import _live_transforms, _transform_trees, transforms as mtransforms
from ._native import drawing as _drawing, register as _register
from ._native import transforms as _transforms
from ._program_classes import takes_from_program
from .path import Path


class _Drawing(threading.local):
    """What the draws running on each thread draw with: a thread's draws
    are its own."""

    def __init__(self):
        #: The renderers pltrs (Rust) draws with, innermost last: what the
        #: program's drawing receives, as ``[make, renderer]`` (the
        #: renderer ``make()`` gives is made when first needed).
        self.renderers = []
        #: How many of the calls running draw while pltrs (Rust) is
        #: drawing the figure (an artist the draw reached before it was
        #: prepared, a path effect, a renderer of the program's), which
        #: cannot change it then.
        self.figures = 0


_drawing_now = _Drawing()


def figure_changeable():
    """Whether the program's drawing running now may change the figure it
    draws: not while pltrs (Rust) draws it."""
    return _drawing_now.figures == 0


@contextlib.contextmanager
def _figure_drawn(drawn=True):
    """While pltrs (Rust) draws the figure, with *drawn*."""
    _drawing_now.figures += drawn
    try:
        yield
    finally:
        _drawing_now.figures -= drawn


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


def _draw_of(artist):
    """The ``draw`` *artist* is drawn with, as a function of the renderer,
    without the rasterization wrapper it got (which pltrs (Rust) applies
    around the drawing), and whether that wrapper lets the artist be
    rasterized: the artist's own (``artist.draw = ...``), else its
    class'."""
    own = vars(artist).get("draw")
    if own is None:
        draw, rasterizes = _own_draw(type(artist))
        return functools.partial(draw, artist), rasterizes
    rasterizes = getattr(own, "_supports_rasterization", None)
    if rasterizes is None:
        return own, False
    if isinstance(own, types.MethodType):
        unwrapped = getattr(own.__func__, "__wrapped__", own.__func__)
        return types.MethodType(unwrapped, own.__self__), rasterizes
    return getattr(own, "__wrapped__", own), rasterizes


def attach(artist, again=False):
    """Have pltrs (Rust) draw *artist* through its ``draw`` when the program
    gives it one (its class takes ``draw`` from the program, or the program
    set one on the artist), or its class computes what matplotlib's
    ``draw`` reads (the ``_computed`` attributes of the package's class,
    such as an arc's ``theta1``); *again* for an artist whose painter may
    no longer be needed (its own ``draw`` deleted)."""
    cls = type(artist)
    if "draw" in vars(artist) or any(
            takes_from_program(cls, name)
            for name in ("draw", *getattr(cls, "_computed", ()))):
        measures = any(takes_from_program(cls, name)
                       for name in ("get_window_extent", "get_tightbbox"))
        _drawing.set_painter(artist._native, weakref.ref(artist),
                             _draw_of(artist)[1], measures)
    elif again:
        _drawing.set_painter(artist._native, None, False, False)


def draw_through_python(artist, measures):
    """Have pltrs (Rust) draw *artist* through its ``draw`` whatever its
    class, and measure it through its ``get_tightbbox`` when *measures*:
    for an artist whose ``draw`` must run where matplotlib's does (see
    `._unusable`); `attach` with *again* undoes it."""
    _drawing.set_painter(artist._native, weakref.ref(artist), _draw_of(artist)[1],
                         measures)


@contextlib.contextmanager
def drawing_with(make):
    """Hand the program's drawing the renderer *make()* makes (when first
    needed) while pltrs (Rust) draws on this thread."""
    renderers = _drawing_now.renderers
    renderers.append([make, None])
    try:
        yield
    finally:
        renderers.pop()


def _renderer_for(target, figure):
    """The renderer the program's drawing receives in a draw of *figure*
    with *target* (see ``_native.drawing``): the one installed
    (`drawing_with`), else one standing for pltrs (Rust)'s renderer."""
    renderers = _drawing_now.renderers
    if renderers:
        installed = renderers[-1]
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
    draw, _ = _draw_of(artist)
    with _figure_drawn(target[6]), _recording(renderer, target[5]) as drawing:
        if figure_changeable():
            _evaluate_transforms(artist)
            place = getattr(artist, "_place_as_laid_out", None)
            if place is not None:
                # An offset box of the program's, placed by its parent.
                place()
        draw(renderer)
    return drawing


def _evaluate_transforms(artist):
    """Bring the transforms *artist*'s ``draw`` reads up to date, as
    matplotlib evaluates them when it draws: the trees of its figure
    follow the laid-out figure, and the transform the artist was given is
    handed to pltrs (Rust) again as it is now (a `.BboxTransformTo` of a
    box in data coordinates follows the limits)."""
    figure = artist.get_figure(root=True)
    if figure is not None:
        _transform_trees.sync(figure)
    evaluate_given_transform(artist)


def evaluate_given_transform(artist):
    """Hand pltrs (Rust) the transform *artist* was given as it is now, as
    matplotlib's ``draw`` reads ``get_transform()``: a transform of the
    program's may have changed in place since (the ``offset_transform`` of
    an offset box of the program's, which its ``set_offset`` moves before
    it draws its children)."""
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


def measure_box(box):
    """The bounds ``(x0, y0, width, height)`` of *box*, an offset box of
    the program's in the tree of a box pltrs (Rust) lays out, relative to
    its origin: its ``get_bbox`` with the renderer of its figure, which
    pltrs (Rust) asks once the figure is laid out (matplotlib's parent asks
    it when it draws)."""
    bbox = box.get_bbox(box.get_figure(root=True)._get_renderer())
    return tuple(map(float, bbox.bounds))


def draw_through(renderer, artist):
    """Draw *artist*, as pltrs (Rust) draws it, through the drawing methods
    of *renderer*, a renderer of the program's (matplotlib's
    ``artist.draw(renderer)``); a figure is laid out and drawn whole."""
    with _figure_drawn():
        _drawing.draw_through(renderer, artist._native)


#: The methods matplotlib's drawing calls on a renderer.
_DRAWING_METHODS = (
    "draw_path", "draw_markers", "draw_path_collection", "draw_quad_mesh",
    "draw_gouraud_triangles", "draw_image", "draw_text", "draw_tex",
    "start_filter", "stop_filter", "start_rasterizing", "stop_rasterizing",
    "open_group", "close_group", "new_gc")


def overrides_drawing(renderer):
    """Whether the class of *renderer*, a renderer of this package's
    classes, takes one of the methods drawing calls from the program (a
    subclass of ``RendererSVG`` with its own ``start_filter``, say), so
    that drawing with it goes through its methods, as matplotlib's does
    (`draw_through`)."""
    cls = type(renderer)
    return any(takes_from_program(cls, name) for name in _DRAWING_METHODS)


def draw_cell_through(renderer, cell):
    """Draw the table *cell* alone, as pltrs (Rust) draws it, through the
    drawing methods of *renderer*, a renderer of the program's
    (matplotlib's ``cell.draw(renderer)``)."""
    from ._native import table as _table
    with _figure_drawn():
        _table.draw_cell_through(renderer, cell._ref())


def paint(artist):
    """Have pltrs (Rust) draw *artist* through its class' ``draw`` from now
    on, whichever class defines it (a table holding cells of the program's
    draws each cell by its ``draw``)."""
    _drawing.set_painter(artist._native, weakref.ref(artist),
                         _draw_of(artist)[1], False)


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


def filter_function(shared):
    """The image filter *shared* pltrs (Rust) hands a renderer of the
    program's, as the function matplotlib's renderers receive in
    ``stop_filter``: of an ``(H, W, 4)`` image of fractions and the
    resolution, returning the filtered image and the offset of its
    corner."""
    return functools.partial(_drawing.apply_filter, shared)


_register("filter_function", filter_function)
_register("record_drawing", record_drawing)
_register("effect_draw_path", effect_draw_path)
_register("measure_artist", measure_artist)
_register("measure_box", measure_box)
_register("forward_drawing", forward_drawing)
_register("artist_reference", weakref.ref)
