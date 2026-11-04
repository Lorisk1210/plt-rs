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
the Axes out and draws it:

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
asked about themselves: every hook hands the adapter the view limits
pltrs (Rust) works with, which the Axes' ``_viewLim`` is set to, so that
the class' transforms compute with them (and polar Axes answer
``get_rorigin`` and ``get_rsign`` from them, as matplotlib's do).
"""

import contextlib
import functools
import weakref

import numpy as np

from .. import _transform_trees
from .._native import artist as _artist
from .._native import polar as _polar
from .._native import projections as _projections, spines as _spines
from .._native import transforms as _transforms
from ..axes import Axes
from ..axis import Axis
from .._axis_artists import Tick
from ..path import Path
from ..spines import _LINEAR_PATHS, Spine
from ..transforms import (CompositeAffine2D, CompositeGenericTransform,
                          IdentityTransform, TransformWrapper)

#: The projections pltrs (Rust) has: its own Axes are made with them.
_NATIVE_PROJECTIONS = frozenset(
    ["rectilinear", "polar", "aitoff", "hammer", "lambert", "mollweide", "3d"])

#: The axis and the label of a tick (1 or 2) on each side.
_LABEL_TRANSFORMS = {"bottom": ("x", 1), "top": ("x", 2),
                     "left": ("y", 1), "right": ("y", 2)}


def native_name(cls):
    """
    The name of the projection of pltrs (Rust) the Axes class *cls* is
    drawn with: that of the nearest of its classes this package defines
    (a subclass of `.PolarAxes` registered under a name of its own is
    polar).
    """
    package = __name__.partition(".")[0]
    for base in cls.__mro__:
        if (base.__module__.partition(".")[0] == package
                and getattr(base, "name", None) in _NATIVE_PROJECTIONS):
            return base.name
    return "rectilinear"


def is_program_class(cls):
    """Whether *cls* is an Axes class of a projection the program defines
    (one whose ``name`` pltrs (Rust) does not know, deriving from the
    rectilinear or the polar Axes, and which does not make its native Axes
    itself)."""
    return (issubclass(cls, Axes)
            and cls.name not in _NATIVE_PROJECTIONS
            and native_name(cls) in ("rectilinear", "polar")
            and getattr(cls, "_make_native", None) is None)


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


def _to_axes(ax, transform):
    """*transform* of *ax* up to its final ``transAxes``; a transform that
    does not end with it, followed by the inverse of ``transAxes`` (the
    trees are those of the laid-out figure while pltrs (Rust) asks)."""
    found = _split(transform, ax.transAxes)
    if found is None or not isinstance(found[1], IdentityTransform):
        return transform + ax.transAxes.inverted()
    return found[0]


def _tick_label_placement(ax, transform, point):
    """Where a tick label of *ax* at *point* through the label transform
    *transform* (``get_xaxis_text1_transform`` and friends) goes, as pltrs
    (Rust) takes it: the anchor in axes fractions and the shift in display
    pixels after ``transAxes`` (none for a transform not made with it, or
    moving the label after it otherwise than by a translation: the anchor
    is then where the whole transform puts the label)."""
    point = np.array([point], dtype=float)
    found = _split(transform, ax.transAxes)
    matrix = None
    if found is not None and found[1].is_affine:
        matrix = found[1].get_matrix()
    if matrix is None or not np.allclose(matrix[:2, :2], np.eye(2)):
        anchor = ax.transAxes.inverted().transform(transform.transform(point))[0]
        return tuple(map(float, anchor)), (0.0, 0.0)
    anchor = found[0].transform(point)[0]
    return (tuple(map(float, anchor)), (float(matrix[0, 2]), float(matrix[1, 2])))


class _NonAffinePart:
    """
    The non-affine part of a transform, as pltrs (Rust) maps points and
    paths through it (with its inverse). *viewing*, when given, is the
    context its every computation runs in (the view it was evaluated at,
    which polar transforms read from their Axes).
    """

    def __init__(self, transform, viewing=None):
        self._transform = transform
        self._viewing = viewing

    def _computing(self):
        """The context of a computation."""
        return contextlib.nullcontext() if self._viewing is None else self._viewing()

    def transform(self, values):
        """*values*, a point or points, through the non-affine part (which
        a class may define for arrays of points only, as matplotlib's
        `.Transform.transform` hands them)."""
        values = np.asarray(values, dtype=float)
        with self._computing():
            points = self._transform.transform_non_affine(values.reshape(-1, 2))
        return np.reshape(points, values.shape)

    def transform_path(self, vertices, codes, steps=1):
        """The `.Path` of *vertices* and *codes* (with *steps*
        interpolation steps) through the non-affine part, as the class'
        ``transform_path_non_affine`` maps it (curving straight segments,
        as the Hammer projection's subdivides them)."""
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
    alone shows the parts its class' ``draw`` draws)."""

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
        #: The sides of the native spines the class' spines are drawn as.
        self.spine_sides = [_spines.side(spine._native) for spine in spines.values()]
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
        #: A tick of each axis whose class overrides ``Tick.draw``.
        self.probes = {}
        #: A tick of each axis made alone, whose padding the labels take.
        self.ticks = {}
        #: The spines whose class overrides ``_adjust_location``, by side.
        self.spines = {}
        #: The ratio the class' ``get_data_ratio`` gives, which pltrs
        #: (Rust) keeps (matplotlib's geographic axes give a constant).
        self.data_ratio = None
        #: The hooks the class overrides (see `_Adapter`).
        self.overrides = dict.fromkeys(
            ["axis_transform", "tick_interval", "tick_sides", "spine_interval",
             "tick_label"], [])
        if not self.polar:
            self._ask_rectilinear(ax, spines)

    def _ask_rectilinear(self, ax, spines):
        """Find what the class of the rectilinear Axes *ax* overrides."""
        given = ax.__dict__.get("_rectilinear_axis_transforms", (None, None))
        for name, rectilinear in zip("xy", given):
            transform = getattr(ax, f"get_{name}axis_transform")(which="grid")
            if transform is not rectilinear:
                self.axis_transforms[name] = _to_axes(ax, transform)
            axis = getattr(ax, f"{name}axis")
            probe = axis._get_tick(major=True)
            # The ticks of the axis are of the class the program gives.
            axis._tick_class = type(probe)
            self.ticks[name] = probe
            if type(probe).draw is not Tick.draw:
                self.probes[name] = probe
        self.spines = {side: spine for side, spine in spines.items()
                       if type(spine)._adjust_location is not Spine._adjust_location}
        self.data_ratio = (
            None if type(ax).get_data_ratio is Axes.get_data_ratio
            else float(ax.get_data_ratio()))
        self.overrides = {
            "axis_transform": list(self.axis_transforms),
            "tick_interval": [
                name for name in "xy"
                if type(getattr(ax, f"{name}axis")).get_view_interval
                is not Axis.get_view_interval],
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
    alive. Each hook gets the view limits pltrs (Rust) works with."""

    def __init__(self, ax):
        self._axes = weakref.ref(ax)
        projection = ax._program_projection
        self.name = type(ax).name
        self.spine_order = projection.spine_order
        self.spine_sides = projection.spine_sides
        self.data_ratio = projection.data_ratio
        self.overrides = projection.overrides

    def __getstate__(self):
        # A pickle of the figure holds the Axes anyway.
        return {**self.__dict__, "_axes": self._axes()}

    def __setstate__(self, state):
        self.__dict__.update(state, _axes=weakref.ref(state["_axes"]))

    @contextlib.contextmanager
    def _viewing(self, limits):
        """The Axes and its projection, its ``_viewLim`` at *limits*
        (``(x0, y0, x1, y1)``), those pltrs (Rust) works with."""
        ax = self._axes()
        if ax is None:
            raise ReferenceError("the Axes of the projection is gone")
        answering = ax.__dict__.get("_answering_from_trees", False)
        ax.__dict__["_answering_from_trees"] = True
        try:
            with _transform_trees.setting_leaves():
                _transform_trees.set_extents(ax._viewLim, limits)
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
            viewing = (functools.partial(self._viewing, limits)
                       if projection.polar else None)
            return _evaluated(projection.to_axes, viewing)

    def axis_transform(self, limits, name):
        """The transform of the gridlines and ticks of axis *name* up to
        ``transAxes``, evaluated."""
        with self._viewing(limits) as (ax, projection):
            return _evaluated(projection.axis_transforms[name])

    def tick_interval(self, limits, name):
        """The interval axis *name* is ticked over."""
        with self._viewing(limits) as (ax, projection):
            v0, v1 = getattr(ax, f"{name}axis").get_view_interval()
            return float(v0), float(v1)

    def tick_sides(self, limits, name, loc):
        """The sides the tick at *loc* of axis *name* draws."""
        with self._viewing(limits) as (ax, projection):
            return _drawn_sides(projection.probes[name], loc)

    def tick_label(self, limits, side, loc):
        """Where the label of the tick at *loc* on *side* goes:
        ``(anchor, shift, halign, valign)``, the anchor in axes fractions
        and the shift in display pixels, as the class' label transform of
        the side puts it."""
        with self._viewing(limits) as (ax, projection):
            name, which = _LABEL_TRANSFORMS[side]
            pad = projection.ticks[name]._pad
            transform, valign, halign = getattr(
                ax, f"get_{name}axis_text{which}_transform")(pad)
            point = (loc, 0.0) if name == "x" else (0.0, loc)
            anchor, shift = _tick_label_placement(ax, transform, point)
            return anchor, shift, halign, valign

    def spine_interval(self, limits, side):
        """The interval the spine on *side* spans along its axis."""
        with self._viewing(limits) as (ax, projection):
            spine = projection.spines[side]
            spine._path = Path(np.array(_LINEAR_PATHS[side]))
            spine._adjust_location()
            along = 1 if side in ("left", "right") else 0
            v0, v1 = spine._path.vertices[:, along]
            return float(v0), float(v1)


def hide_unmade_spines(ax, sides):
    """Hide the native spines of *ax* on the sides no spine of its class is
    drawn on (*sides*: those it made)."""
    for _, native in _spines.axes_spines(ax._native):
        if _spines.side(native) not in sides:
            _artist.set_visible(native, False)


def cleared(ax):
    """
    Give the Axes *ax* the background its class' ``_gen_axes_patch`` makes,
    as matplotlib's ``clear`` makes one, if the class is a program's that
    overrides it; for polar Axes, its outline clips what the Axes draws
    unless it is a `.Wedge`, which pltrs (Rust) fits to the view.
    """
    from ..axes._decorations import DecorationsMixin
    from .polar import PolarAxes
    cls = type(ax)
    if (not is_program_class(cls)
            or cls._gen_axes_patch in (DecorationsMixin._gen_axes_patch,
                                       PolarAxes._gen_axes_patch)):
        return
    patch = ax.patch
    if native_name(cls) != "polar":
        return
    from ..patches import Wedge
    outline = (None if isinstance(patch, Wedge)
               else patch.get_patch_transform().transform_path(patch.get_path()))
    _polar.set_frame_path(ax._native, outline)


def _build_trees(ax):
    """Build the trees of the Axes *ax* of a program's projection class with
    the class' ``_set_lim_and_transforms``; their transforms reach pltrs
    (Rust) as the Axes' own."""
    if "_transform_trees" not in ax.__dict__:
        ax._build_transform_trees()
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
    ax._program_projection = _Projection(ax, ax._gen_axes_spines())


def set_up(ax):
    """
    Build the Axes *ax* of a program's projection class as matplotlib's
    ``__init__`` does (``_set_lim_and_transforms``, ``_gen_axes_spines``,
    ``_init_axis``) and hand it over to pltrs (Rust).
    """
    _build_trees(ax)
    spines = ax._gen_axes_spines()
    ax._init_axis()
    ax._update_transScale()
    ax._program_projection = _Projection(ax, spines)
    # `ax.spines` gives the class' spines from now on.
    ax.__dict__.pop("_spines_kept", None)
    _projections.install(ax._native, _Adapter(ax))


def maker(projection_class):
    """How ``add_subplot`` and ``add_axes`` make an Axes of the program's
    *projection_class*: a rectilinear pltrs (Rust) Axes, set up as an Axes
    of the class (see `.figure._native_maker`)."""
    def make(figure, kwargs, *, spec=None, rect=None):
        from .._native import figure as _figure, gridspec as _gridspec
        from .. import _handles
        native = (_figure.add_axes(figure._native, rect) if rect is not None
                  else _gridspec.add_subplot(figure._native, spec))
        ax = _handles.wrap(native, projection_class, figure)
        ax._init_projection()
        return native, kwargs
    return make
