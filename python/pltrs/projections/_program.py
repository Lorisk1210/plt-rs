"""
The Axes of projection classes the program defines (matplotlib's
``register_projection`` of an `~.axes.Axes` subclass with transforms,
axes, ticks and spines of its own, such as the SkewT-logP example's
``SkewXAxes``).

Such an Axes is a rectilinear pltrs (Rust) Axes that matplotlib's
construction then turns into the class' own: `set_up` runs the class'
``_set_lim_and_transforms``, ``_gen_axes_spines`` and ``_init_axis`` as
matplotlib's ``__init__`` does, and hands the Axes over to a pltrs (Rust)
projection (``_native.projections``) asking an `_Adapter` while it lays
the Axes out and draws it, and makes the patch its class'
``_gen_axes_patch`` makes after each clear the background and what the
Axes clips to (`cleared`):

* the data transform: ``transData`` up to its final ``transAxes``, and
  the transforms of the gridlines and ticks (``get_xaxis_transform``)
  where the class gives its own;
* the interval each axis is ticked over (an overridden
  ``Axis.get_view_interval``);
* the sides each tick draws (an overridden ``Tick.draw``, run on a tick
  made alone with a renderer recording what it draws);
* where the tick labels go (an overridden ``get_xaxis_text1_transform``
  and friends);
* the interval each spine spans (an overridden ``Spine._adjust_location``)
  and the order of the spines (``_gen_axes_spines``; the sides none of
  them is drawn on are hidden, and a spine of another shape, such as a
  circle, is the native spine of a side drawn in that shape);
* the ratio of the data for the aspect (an overridden ``get_data_ratio``,
  asked when the Axes is set up).

A subclass of `.PolarAxes` (the radar chart example's ``RadarAxes``) is a
polar pltrs (Rust) Axes handed over the same way: pltrs (Rust) keeps its
polar settings, ticks and labels, and asks the adapter for the data
transform only (made with the class' ``PolarTransform``, whose
``transform_path_non_affine`` draws every path), the spines the class
made, and, after each clear, the background its ``_gen_axes_patch``
makes (`cleared`), which clips what the Axes draws unless it is a
`.Wedge` (which pltrs (Rust) fits to the view, as matplotlib's ``draw``
fits one).

While pltrs (Rust) lays the figure out or draws it, its Axes cannot be
asked about themselves: every hook hands the adapter the view limits and
the position
pltrs (Rust) works with, which the Axes' ``_viewLim`` is set to, so that
the class' transforms compute with them (and polar Axes answer
``get_rorigin`` and ``get_rsign`` from them, as matplotlib's do).
"""

import collections
import contextlib
import functools
import weakref

import numpy as np

from .. import _lazy_imports, _transform_trees
from .._program_classes import class_cache, takes_from_program
from .._native import artist as _artist
from .._native import polar as _polar
from .._native import projections as _projections, spines as _spines
from .._native import transforms as _transforms
from ..axes import Axes, _program_tick_params
from ..axis import _AxisOfAxes
from .._axis_artists import Tick
from ..path import Path
from ..spines import _LINEAR_PATHS, Spine
from ..transforms import (Affine2DBase, CompositeAffine2D,
                          CompositeGenericTransform, IdentityTransform,
                          TransformWrapper, TransformedPath)

_geo = _lazy_imports.module(globals(), ".geo", "_geo")
_own_state = _lazy_imports.module(globals(), ".._own_state")

#: The projections pltrs (Rust) has: its own Axes are made with them.
_NATIVE_PROJECTIONS = frozenset(
    ["rectilinear", "polar", "aitoff", "hammer", "lambert", "mollweide", "3d"])

#: The axis and the label of a tick (1 or 2) on each side.
_LABEL_TRANSFORMS = {"bottom": ("x", 1), "top": ("x", 2),
                     "left": ("y", 1), "right": ("y", 2)}


def _nearest_own(cls):
    """The nearest of the classes of *cls* this package defines that names
    a projection of pltrs (Rust); None without one."""
    package = __name__.partition(".")[0]
    for base in cls.__mro__:
        if (base.__module__.partition(".")[0] == package
                and getattr(base, "name", None) in _NATIVE_PROJECTIONS):
            return base
    return None


def is_program_geo(cls):
    """Whether *cls* is a class of the program's deriving from one of the
    geographic Axes classes (mplstereonet's ``StereonetAxes``, a
    `.LambertAxes` with a transform, grids and a ``cla`` of its own) or
    from `.GeoAxes` itself."""
    return issubclass(cls, _geo.GeoAxes) and cls not in _geo._OWN_CLASSES


def native_name(cls):
    """
    The name of the projection of pltrs (Rust) the Axes class *cls* is
    drawn with: that of the nearest of its classes this package defines
    (a subclass of `.PolarAxes` registered under a name of its own is
    polar), and rectilinear for a program's subclass of a geographic
    class (`is_program_geo`).
    """
    own = _nearest_own(cls)
    if own is None or is_program_geo(cls):
        return "rectilinear"
    return own.name


#: The methods matplotlib's ``Axes.__init__`` builds an Axes with, and the
#: transforms its axes are drawn with: a class of the program's overriding
#: one is a projection of the program's, whatever its ``name``.
_PROJECTION_HOOKS = ("_set_lim_and_transforms", "_gen_axes_spines", "_init_axis", "_gen_axes_patch",
                     "get_xaxis_transform", "get_yaxis_transform")


def is_program_class(cls):
    """Whether *cls* is an Axes class of a projection the program defines:
    one whose ``name`` pltrs (Rust) does not know, or which overrides how
    matplotlib's ``__init__`` builds an Axes, deriving from the rectilinear
    or the polar Axes, and which does not make its native Axes itself; or
    one deriving from a geographic class (`is_program_geo`)."""
    if not issubclass(cls, Axes):
        return False
    if is_program_geo(cls):
        return True
    return ((cls.name not in _NATIVE_PROJECTIONS or _overrides_projection_hooks(cls))
            and native_name(cls) in ("rectilinear", "polar")
            and getattr(cls, "_make_native", None) is None)


@class_cache
def _overrides_projection_hooks(cls):
    """Whether the class *cls* takes one of `_PROJECTION_HOOKS` from the
    program (kept per class, as `.takes_from_program` keeps its answers)."""
    return any(takes_from_program(cls, hook) for hook in _PROJECTION_HOOKS)


def _split(transform, middle):
    """``(before, after)`` such that *transform* is ``before + middle +
    after``, *middle* found by identity so that nothing is evaluated; None
    when *transform* is not made with *middle*."""
    if transform is middle:
        return IdentityTransform(), IdentityTransform()
    if isinstance(transform, TransformWrapper):
        return _split(transform._child, middle)
    if isinstance(transform, (CompositeGenericTransform, CompositeAffine2D)):
        found = _split(transform._b, middle)
        if found is not None:
            return transform._a + found[0], found[1]
        found = _split(transform._a, middle)
        if found is not None:
            return found[0], found[1] + transform._b
    return None


class _AxesToUnit(Affine2DBase):
    """
    The inverse of an Axes' ``transAxes`` as it is whenever it is asked
    for. `.Transform.inverted` gives the inverse of the matrix the
    transform has then, which the Axes leaves behind when it moves (its
    aspect applied, a layout): what follows a transform of the program's
    that does not end with ``transAxes`` (MetPy's skew, applied in pixels
    after ``transData``) has to follow the Axes.
    """

    def __init__(self, trans_axes):
        super().__init__()
        self._trans_axes = trans_axes
        self.set_children(trans_axes)
        self._mtx = None

    def get_matrix(self):
        # docstring inherited
        if self._invalid or self._mtx is None:
            self._mtx = np.linalg.inv(self._trans_axes.get_matrix())
            self._inverted = None
            self._invalid = 0
        return self._mtx


def _to_axes(ax, transform):
    """*transform* of *ax* up to its final ``transAxes``; a transform that
    does not end with it, followed by the inverse of ``transAxes`` as it
    is when pltrs (Rust) asks (`_AxesToUnit`; the trees are those of the
    laid-out figure then)."""
    found = _split(transform, ax.transAxes)
    if found is None or not isinstance(found[1], IdentityTransform):
        return transform + _AxesToUnit(ax.transAxes)
    return found[0]


def _tick_label_parts(ax, transform):
    """How the tick labels of *ax* placed by the label transform
    *transform* (``get_xaxis_text1_transform`` and friends) go, as pltrs
    (Rust) takes them: the transform taking a tick's point to axes
    fractions, and the translation after ``transAxes`` whose offset is the
    shift in display pixels (None for a transform not made with it, or
    moving the label after it otherwise than by a translation: the anchor
    is then where the whole transform puts the label)."""
    found = _split(transform, ax.transAxes)
    matrix = None
    if found is not None and found[1].is_affine:
        matrix = found[1].get_matrix()
    if matrix is None or not np.allclose(matrix[:2, :2], np.eye(2)):
        return transform + _AxesToUnit(ax.transAxes), None
    return found[0], found[1]


#: How many mapped paths the data transform of an Axes keeps (see
#: `_NonAffinePart`): those of its lines, the least recently drawn dropped
#: first.
_KEPT_PATHS = 128


class _NonAffinePart:
    """
    The non-affine part of a transform, as pltrs (Rust) maps points and
    paths through it (with its inverse). *viewing*, when given, is the
    context its every computation runs in (the view it was evaluated at,
    which polar transforms read from their Axes).

    With *keeps_paths* (the data transform of an Axes), a path is mapped
    as matplotlib's artists map theirs: a line's through a
    `.TransformedPath`, which calls the transform for the path and then
    for its vertices as points, and keeps both until the transform is
    invalidated, so that the next draw calls nothing; the path an artist
    owns (a collection's, a patch's outline) anew at every draw. The
    vertices of a kept path asked for as points (a line's markers) are
    those the `.TransformedPath` computed. The parts of an axis (its
    gridlines and marks) are mapped call by call by pltrs (Rust), which
    keeps nothing.
    """

    def __init__(self, transform, viewing=None, keeps_paths=False):
        self._transform = transform
        self._viewing = viewing
        #: Whether the part gives the same answer for the same point for as
        #: long as it lives (computing in no context of a view), so that
        #: pltrs (Rust) may answer from what it answered before.
        self._answers_alike = viewing is None
        #: The paths mapped and kept, by content, the last drawn last
        #: (None: none is kept), and the key of the path kept for each
        #: array of vertices.
        self._paths = collections.OrderedDict() if keeps_paths else None
        self._of_vertices = {}

    def _computing(self):
        """The context of a computation."""
        return contextlib.nullcontext() if self._viewing is None else self._viewing()

    def transform(self, values):
        """*values*, a point or points, through the non-affine part (which
        a class may define for arrays of points only, as matplotlib's
        `.Transform.transform` hands them)."""
        values = np.asarray(values, dtype=float)
        with self._computing():
            kept = (self._paths.get(self._of_vertices.get(values.tobytes()))
                    if self._of_vertices and values.ndim == 2 else None)
            if kept is not None:
                points = kept.get_transformed_points_and_affine()[0].vertices
            else:
                points = self._transform.transform_non_affine(values.reshape(-1, 2))
        return np.reshape(points, values.shape)

    def transform_path(self, vertices, codes, steps=1):
        """The `.Path` of *vertices* and *codes* (with *steps*
        interpolation steps) through the non-affine part, as the class'
        ``transform_path_non_affine`` maps it (curving straight segments,
        as the Hammer projection's subdivides them)."""
        with self._computing():
            if self._paths is None:
                path = Path(vertices, codes, _interpolation_steps=steps)
                return self._transform.transform_path_non_affine(path)
            held = _own_state.held_path(vertices, codes, steps)
            if held is not None:
                return self._transform.transform_path_non_affine(held)
            key = (vertices.tobytes(), None if codes is None else codes.tobytes(), steps)
            kept = self._paths.get(key)
            if kept is None:
                path = Path(vertices, codes, _interpolation_steps=steps)
                kept = self._paths[key] = TransformedPath(path, self._transform)
                self._of_vertices[key[0]] = key
                if len(self._paths) > _KEPT_PATHS:
                    dropped, _ = self._paths.popitem(last=False)
                    if self._of_vertices.get(dropped[0]) == dropped:
                        del self._of_vertices[dropped[0]]
            else:
                self._paths.move_to_end(key)
            return kept.get_transformed_path_and_affine()[0]

    def transform_outline(self, vertices, codes, steps=1):
        """The outline of a patch through the non-affine part, mapped anew
        as matplotlib's ``Patch.draw`` maps it at every draw."""
        path = Path(vertices, codes, _interpolation_steps=steps)
        with self._computing():
            return self._transform.transform_path_non_affine(path)

    def inverted(self):
        """The inverse of the non-affine part."""
        return _InverseNonAffinePart(self)


class _InverseNonAffinePart:
    """The inverse of a `_NonAffinePart`: the inverse of the whole
    transform after its affine part."""

    def __init__(self, part):
        self._part = part

    def transform(self, values):
        """*values* through the inverse of the non-affine part."""
        transform = self._part._transform
        with self._part._computing():
            affine = transform.get_affine().transform(np.asarray(values, dtype=float))
            return transform.inverted().transform(affine)


def _evaluated(transform, viewing=None):
    """*transform* as pltrs (Rust) takes it: the matrix of its affine part
    and, for a non-affine transform, its non-affine part (computing in the
    context *viewing* makes, see `_NonAffinePart`)."""
    if transform.is_affine:
        return np.asarray(transform.get_matrix(), dtype=float), None
    return (np.asarray(transform.get_affine().get_matrix(), dtype=float),
            _NonAffinePart(transform, viewing))


class _PartsProbe:
    """A renderer recording the artists drawn with it (how a tick made
    alone shows the parts its class' ``draw`` draws), rasterizing nothing
    (``allow_rasterization`` reads its state)."""

    #: Not rasterizing, as `.RendererBase` starts (the draws of artists
    #: read it).
    _raster_depth = 0
    _rasterizing = False

    #: The rasterization state a renderer's draws read, never rasterizing.
    _raster_depth = 0
    _rasterizing = False

    def __init__(self):
        self.drawn = set()

    def _draw_artist(self, artist):
        self.drawn.add(id(artist))

    def open_group(self, s, gid=None):
        """Groups record nothing."""

    def close_group(self, s):
        """Groups record nothing."""


def _drawn_sides(tick, loc):
    """Whether the tick at *loc* draws its first and its second side (its
    mark or its label), all its parts shown before its class draws it."""
    tick.update_position(loc)
    for part in (tick.tick1line, tick.label1, tick.tick2line, tick.label2):
        part.set_visible(True)
    probe = _PartsProbe()
    tick.draw(probe)
    return (id(tick.tick1line) in probe.drawn or id(tick.label1) in probe.drawn,
            id(tick.tick2line) in probe.drawn or id(tick.label2) in probe.drawn)


class _Projection:
    """
    What the hooks of an Axes of a program's projection class work with,
    kept on the Axes (``_program_projection``) so that the adapter pltrs
    (Rust) holds reaches it only through the Axes, weakly.
    """

    def __init__(self, ax, spines):
        #: The spines of the class by name, as ``ax.spines`` gives them.
        self.named_spines = dict(spines)
        draw_class_spines(ax, spines)
        #: The sides of the native spines the class' spines are drawn as
        #: (not those of no side, which pltrs (Rust) draws beside them).
        self.spine_sides = [_spines.side(spine._native) for spine in spines.values()
                            if _spines.frames_side(spine._native)]
        hide_unmade_spines(ax, self.spine_sides)
        self.spine_order = self.spine_sides + [
            side for side in _LINEAR_PATHS if side not in self.spine_sides]
        self.to_axes = _to_axes(ax, ax.transData)
        #: Whether the Axes is polar, which only asks for its transform and
        #: its spines (pltrs (Rust) places its ticks and labels).
        self.polar = native_name(type(ax)) == "polar"
        #: The transforms of the gridlines and ticks of the axes whose
        #: class gives its own, up to ``transAxes``.
        self.axis_transforms = {}
        #: How many times the Axes was cleared since: the transforms its
        #: class gives are taken anew at each clear (`take_anew`).
        self.clears = 0
        #: A tick of each axis whose class overrides ``Tick.draw``.
        self.probes = {}
        #: A tick of each axis made alone, whose padding the labels take.
        self.ticks = {}
        #: The spines whose class overrides ``_adjust_location``, by side.
        self.spines = {}
        #: Whether the class overrides ``get_data_ratio``, which pltrs
        #: (Rust) then asks wherever it applies the aspect.
        self.fixes_data_ratio = False
        #: The hooks the class overrides (see `_Adapter`).
        self.overrides = dict.fromkeys(
            ["axis_transform", "tick_interval", "tick_sides", "spine_interval",
             "tick_label"], [])
        if not self.polar:
            self._ask_rectilinear(ax, spines)

    def take_anew(self, ax):
        """Take the transforms of the gridlines and ticks the class of the
        Axes *ax*, just cleared, gives anew, as the ticks matplotlib's
        clear makes take them: a class whose transform depends on state
        the program changed since draws with the new one."""
        for name in self.axis_transforms:
            transform = getattr(ax, f"get_{name}axis_transform")(which="grid")
            self.axis_transforms[name] = _to_axes(ax, transform)
        self.clears += 1

    def _ask_rectilinear(self, ax, spines):
        """Find what the class of the rectilinear Axes *ax* overrides."""
        given = ax.__dict__.get("_rectilinear_axis_transforms", (None, None))
        for name, rectilinear in zip("xy", given):
            transform = getattr(ax, f"get_{name}axis_transform")(which="grid")
            # The rectilinear one blends the ``transData`` it was made
            # with: a class that replaced ``transData`` since (a skew
            # added after it) left its gridlines and ticks with the old.
            follows_data = getattr(transform, f"_{name}", None) is ax.transData
            if transform is not rectilinear or not follows_data:
                self.axis_transforms[name] = _to_axes(ax, transform)
            axis = getattr(ax, f"{name}axis")
            probe = axis._get_tick(major=True)
            # The ticks of the axis are of the class the program gives.
            axis._tick_class = type(probe)
            self.ticks[name] = probe
            if type(probe).draw is not Tick.draw:
                self.probes[name] = probe
        self.spines = {side: spine for side, spine in spines.items()
                       if type(spine)._adjust_location is not Spine._adjust_location
                       and _spines.frames_side(spine._native)}
        self.fixes_data_ratio = type(ax).get_data_ratio is not Axes.get_data_ratio
        self.overrides = {
            "axis_transform": list(self.axis_transforms),
            "tick_interval": [
                name for name in "xy"
                if type(getattr(ax, f"{name}axis")).get_view_interval
                is not _AxisOfAxes.get_view_interval],
            "tick_sides": list(self.probes),
            "spine_interval": list(self.spines),
            "tick_label": [
                side for side, (name, which) in _LABEL_TRANSFORMS.items()
                if getattr(type(ax), f"get_{name}axis_text{which}_transform")
                is not getattr(Axes, f"get_{name}axis_text{which}_transform")],
        }


class _Adapter:
    """What pltrs (Rust) asks of the Axes of a program's projection class,
    holding the Axes weakly so that pltrs (Rust) keeps no Python object
    alive. Each hook gets the view limits and the position pltrs (Rust)
    works with (`_viewing`)."""

    def __init__(self, ax):
        self._axes = weakref.ref(ax)
        projection = ax._program_projection
        self.name = type(ax).name
        self.spine_order = projection.spine_order
        self.spine_sides = projection.spine_sides
        self.fixes_data_ratio = projection.fixes_data_ratio
        self.overrides = projection.overrides
        self._forget_parts(None)

    def __getstate__(self):
        # A pickle of the figure holds the Axes anyway; the evaluated parts
        # are made anew.
        state = {**self.__dict__, "_axes": self._axes()}
        del state["_parts"], state["_parts_at"], state["_parts_clears"]
        return state

    def __setstate__(self, state):
        self.__dict__.update(state, _axes=weakref.ref(state["_axes"]))
        self._forget_parts(None)

    def _forget_parts(self, limits):
        """Forget the transforms evaluated so far, as the view limits
        become *limits*."""
        #: The transforms evaluated at the limits ``_parts_at``, by key:
        #: what each was evaluated from and what it gave.
        self._parts = {}
        self._parts_at = limits
        #: The clears of the Axes the parts were evaluated after.
        self._parts_clears = None

    def _kept(self, key, limits, make):
        """What *make* gives, kept under *key* while the view limits (of
        *limits*, see `_viewing`) stay (*make* gives what it is made from,
        kept alive, and the object)."""
        view, _ = limits
        ax = self._axes()
        clears = None if ax is None else ax._program_projection.clears
        if self._parts_at != view or self._parts_clears != clears:
            self._forget_parts(view)
            self._parts_clears = clears
        kept = self._parts.get(key)
        if kept is None:
            kept = self._parts[key] = make()
        return kept[1]

    def _evaluated_kept(self, key, transform, limits, keeps_paths=False):
        """*transform* evaluated (see `_evaluated`), its affine matrix anew
        and its non-affine part the same object while *key* and the view
        limits *limits* stay: the part names the transform's code to pltrs
        (Rust), which answers from what it answered before while it lays
        the figure out. *keeps_paths*: see `_NonAffinePart`."""
        if transform.is_affine:
            return np.asarray(transform.get_matrix(), dtype=float), None
        # By the transform's identity too: the memo keeps it alive.
        part = self._kept(
            (key, id(transform)), limits,
            lambda: (transform, _NonAffinePart(transform, keeps_paths=keeps_paths)))
        return np.asarray(transform.get_affine().get_matrix(), dtype=float), part

    @contextlib.contextmanager
    def _viewing(self, limits):
        """The Axes and its projection as pltrs (Rust) works with them:
        *limits* are its view limits and its position, each ``(x0, y0, x1,
        y1)``. Its ``_viewLim`` is set to the first. The ``_position`` of
        an Axes placed in its (sub)figure is set to the second (its
        aspect applied, which the trees otherwise learn of at the end of
        the native call running), so that a transform made of the Axes'
        box computes with it, once per move of the Axes; an Axes placed
        in another stays where its host put it."""
        ax = self._axes()
        if ax is None:
            raise ReferenceError("the Axes of the projection is gone")
        view, position = limits
        answering = ax.__dict__.get("_answering_from_trees", False)
        ax.__dict__["_answering_from_trees"] = True
        try:
            with _transform_trees.setting_leaves():
                _transform_trees.set_extents(ax._viewLim, view)
                if "_child_of" not in ax.__dict__:
                    _transform_trees.set_extents(ax._position, position)
                yield ax, ax._program_projection
        finally:
            ax.__dict__["_answering_from_trees"] = answering

    def set_up(self):
        """Have the trees follow the Axes again (it is new, or was
        cleared, which set its scales anew and showed every spine)."""
        ax = self._axes()
        if ax is not None:
            ax._update_transScale()
            hide_unmade_spines(ax, self.spine_sides)

    def data_to_axes(self, limits):
        """``transData`` up to ``transAxes``, evaluated; the non-affine part
        of a polar one computes at *limits* whenever pltrs (Rust) maps
        through it, since its transform asks its Axes (``get_rorigin``)."""
        with self._viewing(limits) as (ax, projection):
            if projection.polar:
                return _evaluated(projection.to_axes,
                                  functools.partial(self._viewing, limits))
            return self._evaluated_kept("data", projection.to_axes, limits,
                                        keeps_paths=True)

    def axes_to_data(self, limits):
        """The matrix of the inverse of ``transData`` up to ``transAxes``
        when it is affine, as matplotlib's tree inverts it: each affine
        part its own matrix, the inverses multiplied (what pans and zooms
        map the pointer back through, whose last bit differs from the
        inverse of the product); None for a transform that is not affine,
        which pltrs (Rust) inverts whole."""
        with self._viewing(limits) as (ax, projection):
            if projection.polar or not projection.to_axes.is_affine:
                return None
            inverse = projection.to_axes.inverted()
            return np.asarray(inverse.get_matrix(), dtype=float)

    def axis_transform(self, limits, name):
        """The transform of the gridlines and ticks of axis *name* up to
        ``transAxes``, evaluated."""
        with self._viewing(limits) as (ax, projection):
            return self._evaluated_kept(("axis", name), projection.axis_transforms[name],
                                        limits)

    def tick_interval(self, limits, name):
        """The interval axis *name* is ticked over."""
        with self._viewing(limits) as (ax, projection):
            v0, v1 = getattr(ax, f"{name}axis").get_view_interval()
            return float(v0), float(v1)

    def tick_sides(self, limits, name, loc):
        """The sides the tick at *loc* of axis *name* draws."""
        with self._viewing(limits) as (ax, projection):
            return _drawn_sides(projection.probes[name], loc)

    def tick_label(self, limits, side):
        """How the labels of the ticks on *side* go: ``(transform, shift,
        halign, valign)``, the transform taking a tick's point (its
        location along the axis, 0 across) to axes fractions, evaluated,
        and the shift in display pixels after it, as the class' label
        transform of the side places them."""
        with self._viewing(limits) as (ax, projection):
            name, which = _LABEL_TRANSFORMS[side]
            pad = projection.ticks[name]._pad

            def made():
                # As matplotlib's tick asks for its label transforms once.
                transform, valign, halign = getattr(
                    ax, f"get_{name}axis_text{which}_transform")(pad)
                return transform, (_tick_label_parts(ax, transform), halign, valign)

            (to_axes, after), halign, valign = self._kept(("label", side, pad), limits, made)
            shift = ((0.0, 0.0) if after is None
                     else tuple(float(v) for v in after.get_matrix()[:2, 2]))
            evaluated = self._evaluated_kept(("label part", side, pad), to_axes, limits)
            return evaluated, shift, halign, valign

    def data_ratio(self, limits):
        """The ratio of the data the class' ``get_data_ratio`` gives."""
        with self._viewing(limits) as (ax, _):
            return float(ax.get_data_ratio())

    def spine_interval(self, limits, side):
        """The interval the spine on *side* spans along its axis."""
        with self._viewing(limits) as (ax, projection):
            spine = projection.spines[side]
            spine._path = Path(np.array(_LINEAR_PATHS[side]))
            spine._adjust_location()
            along = 1 if side in ("left", "right") else 0
            v0, v1 = spine._path.vertices[:, along]
            return float(v0), float(v1)


def draw_class_spines(ax, spines):
    """
    Have *ax* draw the spines its class made, *spines* by name: each on its
    side, and those of a type naming no side beside them when the class
    made spines of the sides too (cartopy's ``GeoSpine``) or several of
    them (mpltern's six, each with a path and a transform of its own), else
    in the place of the spine framing its outline (a single ``'geo'``
    spine).
    """
    natives = dict(_spines.axes_spines(ax._native))
    drawn = {_spines.side(native): native for native in natives.values()}
    named = [spine.__dict__.get("_spine_type") in natives for spine in spines.values()]
    beside = any(named) or len(named) - sum(named) > 1
    for spine, names_side in zip(spines.values(), named):
        if beside and not names_side:
            _spines.add_spine(ax._native, spine._native)
            if type(spine)._adjust_location is not Spine._adjust_location:
                spine._adjust_before_draws()
            continue
        own = drawn.get(_spines.side(spine._native))
        if own is not None:
            _spines.replace_spine(ax._native, own, spine._native)


def hide_unmade_spines(ax, sides):
    """Hide the native spines of *ax* on the sides no spine of its class is
    drawn on (*sides*: those it made)."""
    for _, native in _spines.axes_spines(ax._native):
        if _spines.side(native) not in sides:
            _artist.set_visible(native, False)


def makes_background(cls):
    """Whether the Axes class *cls* is a program's projection class that
    makes the patch of its background itself (``_gen_axes_patch``)."""
    from ..axes._decorations import DecorationsMixin
    from .polar import PolarAxes
    return (is_program_class(cls)
            and cls._gen_axes_patch not in (DecorationsMixin._gen_axes_patch,
                                            PolarAxes._gen_axes_patch))


def shapes_background(cls):
    """Whether the outline of the patch the Axes class *cls* makes is the
    shape of the background of its rectilinear Axes (see `cleared`)."""
    return makes_background(cls) and native_name(cls) != "polar"


def cleared(ax):
    """
    Take the axis transforms of the class of the Axes *ax*, just cleared,
    anew (`_Projection.take_anew`), and give it the background its class'
    ``_gen_axes_patch`` makes,
    as matplotlib's ``clear`` makes one, if the class is a program's that
    overrides it: its outline is the background's shape and clips what the
    Axes draws (matplotlib's ``set_clip_path(self.patch)``: its box for a
    `.Rectangle`), whenever the program changes it, unless it is the unit
    square or, for polar Axes, a `.Wedge`, which pltrs (Rust) fits to the
    view.
    """
    projection = ax.__dict__.get("_program_projection")
    if projection is not None and not projection.polar:
        projection.take_anew(ax)
    cls = type(ax)
    if not makes_background(cls):
        return
    from .._background_patch import push_outline
    from ..patches import Wedge
    patch = ax.patch
    if native_name(cls) == "polar":
        outline = patch.get_patch_transform().transform_path(patch.get_path())
        _polar.set_frame_path(ax._native, None if isinstance(patch, Wedge) else outline)
    else:
        push_outline(ax, patch)


def _build_trees(ax, as_made=False):
    """Build the trees of the Axes *ax* of a program's projection class with
    the class' ``_set_lim_and_transforms`` (from the boxes matplotlib's
    ``__init__`` builds them from when *as_made*); their transforms reach
    pltrs (Rust) as the Axes' own."""
    if "_transform_trees" not in ax.__dict__:
        ax._build_transform_trees(as_made)
    for key, native in (("_xaxis_transform", _transforms.trans_xaxis),
                        ("_yaxis_transform", _transforms.trans_yaxis)):
        transform = ax.__dict__.get(key)
        if transform is not None and not hasattr(transform, "_live"):
            transform._live = native(ax._native)


def connect(ax):
    """Give the Axes *ax* of a program's projection class, read back from a
    pickle, trees of its own again (trees are not pickled), and its
    projection the transforms made of them."""
    _build_trees(ax)
    ax._update_transScale()
    # The spines the class made, which the pickle kept (matplotlib's
    # unpickled Axes makes none).
    spines = ax.__dict__.get("_spines_kept")
    if spines is None:
        spines = ax.__dict__["_program_projection"].named_spines
    ax._program_projection = _Projection(ax, dict(spines))


def _figure_set(ax):
    """
    Run the ``set_figure`` of the class of *ax*, a program's projection class
    that overrides it, where matplotlib's ``__init__`` runs it: the trees are
    built within it (``_AxesBase.set_figure``), after what the class does
    first (mpltern's ``TernaryAxes`` makes there the boxes its
    ``_set_lim_and_transforms`` reads).
    """
    if type(ax).set_figure is Axes.set_figure:
        return
    ax.__dict__["_figure_set_as_made"] = True
    try:
        ax.set_figure(ax.get_figure(root=False))
    finally:
        del ax.__dict__["_figure_set_as_made"]


def set_up(ax):
    """
    Build the Axes *ax* of a program's projection class as matplotlib's
    ``__init__`` does (``set_figure``, which builds the trees with
    ``_set_lim_and_transforms``, ``_gen_axes_spines``, ``_init_axis``) and
    hand it over to pltrs (Rust).
    """
    _figure_set(ax)
    _build_trees(ax, as_made=True)
    spines = ax._spines_made = ax._gen_axes_spines()
    try:
        ax._init_axis()
    finally:
        del ax._spines_made
    ax._update_transScale()
    ax._program_projection = _Projection(ax, spines)
    # `ax.spines` gives the class' spines from now on.
    ax.__dict__.pop("_spines_kept", None)
    _projections.install(ax._native, _Adapter(ax))
    if ax._axes_measure_themselves():
        # Measured in Python, asking the axis as matplotlib's tight box does.
        from .. import _python_drawing
        _python_drawing.draw_through_python(ax, measures=True)


def maker(projection_class):
    """How ``add_subplot`` and ``add_axes`` make an Axes of the program's
    *projection_class*: a rectilinear pltrs (Rust) Axes, set up as an Axes
    of the class, with the background its clear gives it (see
    `.figure._native_maker`)."""
    def make(figure, kwargs, *, spec=None, rect=None):
        from .._native import figure as _figure, gridspec as _gridspec
        from .. import _handles
        native = (_figure.add_axes(figure._native, rect) if rect is not None
                  else _gridspec.add_subplot(figure._native, spec))
        ax = _handles.wrap(native, projection_class, figure)
        ax._init_projection()
        if projection_class.clear is not Axes.clear:
            # matplotlib's `__init__` clears the Axes it made, running the
            # class' own `clear`, then gives the ticks their sides.
            ax.clear()
        else:
            cleared(ax)
        _program_tick_params.apply(ax)
        return native, kwargs
    return make


