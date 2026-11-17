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
import sys
import threading
import types
import warnings
import weakref

import numpy as np

from . import _live_transforms, _transform_trees, transforms as mtransforms
from ._native import drawing as _drawing, figure as _figure, register as _register
from ._native import transforms as _transforms
from ._program_classes import class_cache, takes_from_program
from .path import Path


class _Drawing(threading.local):
    """What the draws running on each thread draw with: a thread's draws
    are its own."""

    def __init__(self):
        #: The renderers pltrs (Rust) draws with, innermost last: what the
        #: program's drawing receives, as ``(make, figure, made)``: the
        #: renderer ``make()`` gives, made when first needed for each
        #: window of *figure* printed (``made`` by window, see
        #: `drawing_with`).
        self.renderers = []
        #: How many of the calls running draw while pltrs (Rust) is
        #: drawing the figure (an artist the draw reached before it was
        #: prepared, a path effect, a renderer of the program's), which
        #: cannot change it then.
        self.figures = 0
        #: The root figures drawn through renderers of the program's,
        #: innermost last (`draw_through`), whose texts their drawing
        #: methods receive.
        self.through = []


_drawing_now = _Drawing()

#: What runs before every artist drawn through Python is drawn (see
#: `before_drawing`).
_preparations = []


def before_drawing(prepare):
    """Have *prepare* called with every artist drawn through Python
    before its ``draw``, once its figure is laid out and free to change
    (how a 3-D axes projects the program's artists, as matplotlib's
    ``Axes3D.draw`` does before drawing them)."""
    _preparations.append(prepare)


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
    such as an arc's ``theta1``), or measures it (`_measures`: a painter
    is what pltrs (Rust) measures it through); *again* for an artist whose
    painter may no longer be needed (its own ``draw`` deleted)."""
    cls = type(artist)
    if "draw" in vars(artist) or _measures(cls) or _draws_from_program(cls):
        _drawing.set_painter(artist._native, weakref.ref(artist),
                             _draw_of(artist)[1],
                             _measures(cls) or _readies_itself(artist))
    elif again:
        _drawing.set_painter(artist._native, None, False, False)


@class_cache
def _draws_from_program(cls):
    """Whether the class *cls* takes ``draw``, or one of the attributes its
    package class computes for matplotlib's ``draw`` (``_computed``), from
    the program (kept per class, as `.takes_from_program` keeps its
    answers)."""
    return any(takes_from_program(cls, name)
               for name in ("draw", *getattr(cls, "_computed", ())))


@class_cache
def _measures(cls):
    """Whether the class *cls* measures its artists itself (its
    ``get_window_extent`` or ``get_tightbbox`` is the program's, for an
    Axes the artists its tight box takes in,
    ``get_default_bbox_extra_artists``, or a getter matplotlib's measure
    reads, ``_measured_reads``; kept per class, as `.takes_from_program`
    keeps its answers)."""
    return any(takes_from_program(cls, name)
               for name in ("get_window_extent", "get_tightbbox",
                            "get_default_bbox_extra_artists",
                            *getattr(cls, "_measured_reads", ())))


def _readies_itself(artist):
    """Whether *artist* makes itself ready before it is measured
    (``_before_measured``, which answers whether pltrs (Rust) is to
    measure it then: an Axes whose class places its titles its own way,
    where matplotlib's ``get_tightbbox`` calls it; a held axis, whose Axes
    applies its aspect first)."""
    readies = getattr(artist, "_readies_before_measured", None)
    return readies is not None and readies()


def draw_through_python(artist, measures):
    """Have pltrs (Rust) draw *artist* through its ``draw`` whatever its
    class, and measure it through its ``get_tightbbox`` when *measures*:
    for an artist whose ``draw`` must run where matplotlib's does (see
    `._unusable`); `attach` with *again* undoes it."""
    _drawing.set_painter(artist._native, weakref.ref(artist), _draw_of(artist)[1],
                         measures)


@contextlib.contextmanager
def drawing_with(make, figure=None):
    """Hand the program's drawing the renderer *make()* makes (when first
    needed) while pltrs (Rust) draws on this thread; one for each window
    of *figure* printed meanwhile: matplotlib draws a print cut to a window
    once whole before, measuring the window with the renderer of that
    draw, and then the window with a renderer of the window's size, which
    *make()* gives while the figure stands for the window."""
    renderers = _drawing_now.renderers
    renderers.append((make, figure, {}))
    try:
        yield
    finally:
        renderers.pop()


def _installed():
    """The renderers installed innermost (`drawing_with`), and the key of
    the one for the window printed now; None outside prints."""
    renderers = _drawing_now.renderers
    if not renderers:
        return None
    make, figure, made = renderers[-1]
    window = None if figure is None else printed_window(figure)
    return make, made, window


def printed_window(figure):
    """The window of *figure* printed now (`.figure.Figure` and its
    canvas' print), None for the whole figure."""
    native = getattr(figure, "_native", None)
    return None if native is None else _figure.print_window(native)


def installed_renderer():
    """The renderer a print installed for the program's drawing
    (`drawing_with`), made when first needed; None outside prints."""
    installed = _installed()
    if installed is None:
        return None
    make, made, window = installed
    if window not in made:
        made[window] = make()
    return made[window]


def drawing_renderer(artist):
    """The renderer matplotlib's draw running now hands *artist*: that of
    the format being saved (`installed_renderer`), else its figure's
    canvas'."""
    installed = installed_renderer()
    return installed or artist.get_figure(root=True)._get_renderer()


def _renderer_for(target, figure):
    """The renderer the program's drawing receives in a draw of *figure*
    with *target* (see ``_native.drawing``): the one installed
    (`drawing_with`), else one standing for pltrs (Rust)'s renderer."""
    installed = installed_renderer()
    if installed is not None:
        return installed
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
    with (_figure_drawn(target[6]), _recording(renderer, target[5]) as drawing,
          _warnings_held(artist, target[6])):
        if figure_changeable():
            for prepare in _preparations:
                prepare(artist)
            _evaluate_transforms(artist)
            place = getattr(artist, "_place_as_laid_out", None)
            if place is not None:
                # An offset box of the program's, placed by its parent.
                place()
        try:
            draw(renderer)
        finally:
            if target[6]:
                # Drawn where the figure's draw reached it.
                _reached(artist)
    return drawing


def when_reached(artist, name, value):
    """Set *artist*'s attribute *name* to *value* once the figure's draw
    reaches it (`_reached`): what its ``draw``, recorded before the figure
    is drawn, changes in matplotlib only where the figure's draw reaches
    it (no artist drawn before failing)."""
    artist.__dict__.setdefault("_set_when_reached", {})[name] = value


def _reached(artist):
    """The figure's draw reached *artist*: set what `when_reached` held."""
    for name, value in artist.__dict__.pop("_set_when_reached", {}).items():
        setattr(artist, name, value)


@contextlib.contextmanager
def _warnings_held(artist, live):
    """Hold the warnings *artist*'s ``draw`` gives while it is recorded
    before the figure is drawn (not *live*), for pltrs (Rust) to give them
    where it draws the artist (`emit_held_warnings`): matplotlib's draw
    gives them there, and none when an artist drawn before fails."""
    artist.__dict__.pop("_held_warnings", None)
    if live:
        yield
        return
    held = []
    try:
        with warnings.catch_warnings(record=True) as held:
            # Each as it comes: the filters in force decide when it is given.
            warnings.simplefilter("always")
            yield
    finally:
        artist.__dict__["_held_warnings"] = held
        if not _drawing.has_painter(artist._native):
            # Its draw handed pltrs (Rust) what it draws the artist with
            # itself: given now.
            emit_held_warnings(artist)


def emit_held_warnings(artist):
    """Give the warnings held while *artist* was recorded
    (`_warnings_held`), from where they came, under the filters in force:
    the figure's draw reached it (see `when_reached`)."""
    _reached(artist)
    for held in artist.__dict__.pop("_held_warnings", ()):
        warnings.warn_explicit(held.message, held.category, held.filename, held.lineno,
                               registry=_warning_registry(held.filename),
                               source=held.source)


@functools.cache
def _module_named_by(filename):
    """The name of the module loaded from *filename*, or None."""
    for name, module in list(sys.modules.items()):
        if getattr(module, "__file__", None) == filename:
            return name
    return None


def _warning_registry(filename):
    """The registry of the warnings given from the module loaded from
    *filename* (as `warnings.warn` keeps them, for the "default" and
    "module" filters), or None."""
    module = sys.modules.get(_module_named_by(filename))
    return None if module is None else module.__dict__.setdefault("__warningregistry__", {})


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
    it draws its children). Not for an artist whose ``draw`` reads no
    transform of its own (``_draw_reads_transform``: an offset box)."""
    given = artist.__dict__.get("_given_transform")
    if given is not None and getattr(type(artist), "_draw_reads_transform", True):
        if not _live_transforms.holds_natively(given):
            # Kept as given, no transform: the artist's ``get_transform``
            # fails on it as matplotlib's draw does (a patch composes it
            # with its own); pltrs (Rust) keeps the one it had. What the
            # arithmetic warns of is said where the draw reaches the
            # artist (`._transform_replay`).
            with np.errstate(all="ignore"):
                artist.get_transform()
            return
        _transforms.set_transform(artist._native, _live_transforms.to_native(given, held_by=artist._figure or artist))


def effect_draw_path(effect, target, gc_spec, geometry):
    """Draw a path with *effect*, a path effect of the program's class, with
    the renderer of a draw with *target*, recording: the drawing, which
    pltrs (Rust) draws in the path's place, and the graphics context as the
    effect left it, which the effects after it receive (matplotlib hands
    them all the same one). *geometry* is ``((vertices, codes), matrix,
    rgbFace)``: the path, the affine transform into display pixels and the
    fill."""
    from .backends._native_drawing import gc_from_native, native_gc
    from ._program_classes import unheld
    effect = unheld(effect)
    (vertices, codes), matrix, rgbFace = geometry
    renderer = _renderer_for(target, None)
    with _figure_drawn(), _recording(renderer, target[5]) as drawing:
        gc = gc_from_native(gc_spec, renderer.new_gc())
        effect.draw_path(renderer, gc, Path(vertices, codes),
                         mtransforms.Affine2D(matrix), rgbFace)
    return drawing, native_gc(gc)


def _tight_box(artist, renderer, for_layout_only):
    """The ``get_tightbbox`` of *artist* as matplotlib asks for it: of an
    Axes or of one of its axes with ``for_layout_only=True`` when a
    layout engine asks (*for_layout_only*, as matplotlib's
    ``_get_tightbbox_for_layout_only``), of an Axes with
    ``bbox_extra_artists=None`` otherwise (as ``Figure.get_tightbbox``
    asks its Axes), each without the keyword for a class whose method
    does not take it, and plainly of any other artist."""
    from .axes._base import _AxesBase
    from .axis import Axis
    if for_layout_only and isinstance(artist, (_AxesBase, Axis)):
        keywords = {"for_layout_only": True}
    elif isinstance(artist, _AxesBase):
        keywords = {"bbox_extra_artists": None}
    else:
        keywords = {}
    if keywords:
        try:
            return artist.get_tightbbox(renderer, **keywords)
        except TypeError:
            pass
    return artist.get_tightbbox(renderer)


def measure_artist(artist, for_layout_only=False):
    """The extents of the box *artist* occupies in display coordinates as
    its class measures it (its ``get_tightbbox``, as matplotlib's layouts
    and tight boxes ask it: see `_tight_box` for *for_layout_only*), None
    for no space: what pltrs (Rust) takes for it, before it lays the
    figure out or computes its tight box. No extents (an empty tuple) for
    an artist that only makes itself ready there, which pltrs (Rust) then
    measures itself."""
    figure = artist.get_figure(root=True)
    if figure is not None:
        # Where the figure is now (a layout engine between its passes).
        _transform_trees.sync(figure)
    if _readies_itself(artist) and artist._before_measured():
        # pltrs (Rust) measures it, now that it is ready.
        return ()
    if _left_out_of_axes_box(artist):
        return None
    axes = getattr(artist, "axes", None)
    if axes is not None:
        # matplotlib's transData reads the view limits autoscaled first.
        axes._unstale_viewLim()
    if figure is None:
        bbox = _tight_box(artist, None, for_layout_only)
    else:
        # That of the print running (a vector format's mixed-mode one), as
        # matplotlib's tight box of a print and its draw hand it, else the
        # canvas'.
        renderer = drawing_renderer(artist)
        # What measuring draws with a renderer of this package (astropy's
        # WCSAxes draws its ticks to measure them) is dropped, as what
        # matplotlib's layouts draw is drawn over.
        with (_recording(renderer) if hasattr(renderer, "_recording")
              else contextlib.nullcontext()):
            bbox = _tight_box(artist, renderer, for_layout_only)
    if bbox is None or (bbox.width == 0 and bbox.height == 0):
        return None
    return tuple(map(float, bbox.extents))


def _left_out_of_axes_box(artist):
    """Whether *artist* is an artist of an Axes that the Axes' tight box
    leaves out (matplotlib's ``get_default_bbox_extra_artists``): one fully
    clipped to the Axes, but for Axes, axes and offset boxes."""
    from .axes._base import _AxesBase
    from .axis import Axis
    from .offsetbox import AnnotationBbox, OffsetBox
    return (getattr(artist, "axes", None) is not None
            and not isinstance(artist, (_AxesBase, Axis, AnnotationBbox, OffsetBox))
            and artist._fully_clipped_to_axes())


def measure_box(box):
    """The bounds ``(x0, y0, width, height)`` of *box*, an offset box of
    the program's in the tree of a box pltrs (Rust) lays out, relative to
    its origin: its ``get_bbox`` with the renderer matplotlib's draw hands
    it (that of the print running, else its figure's), which pltrs (Rust)
    asks once the figure is laid out (matplotlib's parent asks it when it
    draws)."""
    bbox = box.get_bbox(drawing_renderer(box))
    return tuple(map(float, bbox.bounds))


def draw_through(renderer, artist):
    """Draw *artist*, as pltrs (Rust) draws it, through the drawing methods
    of *renderer*, a renderer of the program's (matplotlib's
    ``artist.draw(renderer)``); a figure is laid out and drawn whole."""
    from .text import _measuring_with
    through = _drawing_now.through
    through.append(artist.get_figure(root=True))
    try:
        # Its texts laid out with the renderer's metrics, as matplotlib's
        # `Text.draw` lays them out.
        with _figure_drawn(), _measuring_with(renderer):
            _drawing.draw_through(renderer, artist._native)
    finally:
        through.pop()


def _text_of(owner):
    """The `.Text` a text pltrs (Rust) draws through a renderer of the
    program's is (``draw_text``'s *mtext*), from its *owner*:
    ``("artist", handle)``, ``("cell", table handle, (row, column))``,
    ``("axis", axes handle,
    axis name, (part, minor, index, second))``, or None for a text pltrs
    does not name."""
    figure = _drawing_now.through[-1] if _drawing_now.through else None
    if owner is None or figure is None:
        return None
    from . import _handles
    kind, native, *rest = owner
    artist = _handles.wrap_any(native, figure)
    if kind == "artist":
        return artist
    if kind == "cell":
        return artist[rest[0]].get_text()
    (name, (part, minor, index, second)) = rest
    axis = getattr(artist, f"{name}axis")
    if part == "label":
        return axis.label
    if part == "offset":
        return axis.offsetText
    ticks = axis._ticks[minor]
    if index < len(ticks):
        tick = ticks[index]
    else:
        from .axis import Tick
        tick = (axis._tick_class or Tick)._of_axis(axis, not minor, index)
    return tick.label2 if second else tick.label1


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
def _gc_of(renderer, spec, with_gid=False):
    """A graphics context of *renderer* set as pltrs (Rust) describes one
    (*spec*), its gid only *with_gid* (matplotlib's images set it, no other
    artist), restored once the drawing call is done."""
    from .backends._native_drawing import gc_from_native
    gc = gc_from_native(spec, renderer.new_gc(), with_gid)
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
            renderer.draw_path(gc, Path(*path), _frozen(matrix), rgbFace)

    @staticmethod
    def draw_markers(renderer, gc, marker, marker_matrix, path, matrix, rgbFace):
        """``renderer.draw_markers``."""
        with _gc_of(renderer, gc) as gc:
            renderer.draw_markers(gc, Path(*marker),
                                  mtransforms.Affine2D(marker_matrix),
                                  Path(*path), _frozen(matrix), rgbFace)

    @staticmethod
    def draw_path_collection(renderer, gc, master, paths, transforms, offsets,
                             colors, strokes):
        """``renderer.draw_path_collection``, the *offsets* in display
        pixels, as matplotlib's `.Collection.draw` calls it: first with no
        paths and the hatch colours, to learn whether the renderer takes
        them (a renderer written before them raises `TypeError`), then with
        the paths, with the hatch colours where it takes them, else without
        them, or, for a hatched collection, item by item through
        ``draw_path`` (`.RendererBase._iter_collection`)."""
        facecolors, edgecolors, hatchcolors = colors
        linewidths, linestyles, antialiaseds, urls = strokes
        master = _frozen(master)
        offset_trf = mtransforms.IdentityTransform()
        linewidths = np.asarray(linewidths, dtype=float)
        antialiaseds = np.asarray(antialiaseds, dtype=bool)
        paths = [Path(*path) for path in paths]
        with _gc_of(renderer, gc) as gc:
            def draw(paths, **hatch):
                renderer.draw_path_collection(
                    gc, master, paths, transforms, offsets, offset_trf, facecolors,
                    edgecolors, linewidths, linestyles, antialiaseds, urls, "screen",
                    **hatch)
            try:
                draw([], hatchcolors=hatchcolors)
            except TypeError:
                if gc.get_hatch() is None:
                    draw(paths)
                    return
                path_ids = renderer._iter_collection_raw_paths(master, paths, transforms)
                for xo, yo, (path, transform), gc0, rgbFace in renderer._iter_collection(
                        gc, list(path_ids), offsets, offset_trf, facecolors, edgecolors,
                        linewidths, linestyles, antialiaseds, urls, "screen",
                        hatchcolors=hatchcolors):
                    if xo != 0 or yo != 0:
                        transform = transform.frozen()
                        transform.translate(xo, yo)
                    renderer.draw_path(gc0, path, transform, rgbFace)
            else:
                draw(paths, hatchcolors=hatchcolors)

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
    def draw_text_lines(renderer, gc, lines, angle, props, owner):
        """A text as matplotlib's ``Text.draw`` draws it: the canvas'
        height asked, a graphics context made, then each line ``(s,
        ismath, x, y)`` (the left end of its baseline, in display
        coordinates from the bottom) handed to ``draw_text``, or to
        ``draw_tex`` for TeX, with *y* from the top where the renderer's
        ``flipy()`` says so, the text's font and, for a single line, the
        text itself (*owner*, see `_text_of`)."""
        from .font_manager import FontProperties
        text = _text_of(owner)
        prop = (text.get_fontproperties() if text is not None
                else FontProperties._from_native(props))
        _, canvash = renderer.get_canvas_width_height()
        mtext = text if len(lines) == 1 else None
        with _gc_of(renderer, gc) as gc:
            for s, ismath, x, y in lines:
                if renderer.flipy():
                    y = canvash - y
                if ismath == "TeX":
                    renderer.draw_tex(gc, x, y, s, prop, angle, mtext=mtext)
                else:
                    renderer.draw_text(gc, x, y, s, prop, angle, ismath=ismath,
                                       mtext=mtext)

    @staticmethod
    def recorded(renderer, method, *args):
        """``forward_drawing.<method>(renderer, *args)`` with *renderer*
        (the canvas' `RendererAgg`, some of whose methods the program
        replaced) recording what its own methods draw: the drawing, which
        pltrs (Rust) draws onto the canvas."""
        drawing = _drawing.new()
        recording, renderer._recording = renderer._recording, drawing
        try:
            getattr(forward_drawing, method)(renderer, *args)
        finally:
            renderer._recording = recording
        return drawing

    @staticmethod
    def draw_image(renderer, gc, origin, im, matrix):
        """``renderer.draw_image``, with a transform for renderers that
        scale images."""
        x, y = origin
        with _gc_of(renderer, gc, with_gid=True) as gc:
            if matrix is None:
                renderer.draw_image(gc, x, y, im)
            else:
                renderer.draw_image(gc, x, y, im, mtransforms.Affine2D(matrix))


def _frozen(matrix):
    """The transform matplotlib's drawing hands a renderer for the affine
    *matrix*: `.IdentityTransform` where it is the identity (a frozen
    identity), else an `.Affine2D`."""
    if np.array_equal(matrix, np.identity(3)):
        return mtransforms.IdentityTransform()
    return mtransforms.Affine2D(matrix)


def filter_function(shared):
    """The image filter *shared* pltrs (Rust) hands a renderer of the
    program's, as the function matplotlib's renderers receive in
    ``stop_filter``: of an ``(H, W, 4)`` image of fractions and the
    resolution, returning the filtered image and the offset of its
    corner."""
    return functools.partial(_drawing.apply_filter, shared)


_register("filter_function", filter_function)
_register("record_drawing", record_drawing)
_register("emit_held_warnings", emit_held_warnings)
_register("effect_draw_path", effect_draw_path)
_register("measure_artist", measure_artist)
_register("measure_box", measure_box)
_register("forward_drawing", forward_drawing)
_register("artist_reference", weakref.ref)
