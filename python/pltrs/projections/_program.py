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
* the interval each spine spans (an overridden ``Spine._adjust_location``)
  and the order of the spines (``_gen_axes_spines``).

While pltrs (Rust) lays the figure out or draws it, its Axes cannot be
asked about themselves: every hook hands the adapter the view limits
pltrs (Rust) works with, which the Axes' ``_viewLim`` is set to, so that
the class' transforms compute with them.
"""

import contextlib
import weakref

import numpy as np

from .. import _transform_trees
from .._native import projections as _projections
from .._native import transforms as _transforms
from ..axes import Axes
from ..axis import Axis
from .._axis_artists import Tick
from ..path import Path
from ..spines import Spine
from ..transforms import (CompositeAffine2D, CompositeGenericTransform,
                          IdentityTransform, TransformWrapper)

#: The projections pltrs (Rust) has: its own Axes are made with them.
_NATIVE_PROJECTIONS = frozenset(
    ["rectilinear", "polar", "aitoff", "hammer", "lambert", "mollweide", "3d"])

#: matplotlib's paths of the linear spines (``Spine.linear_spine``), in
#: data along their axis and axes fractions across it.
_LINEAR_PATHS = {
    "left": [(0.0, 0.999), (0.0, 0.999)],
    "right": [(1.0, 0.999), (1.0, 0.999)],
    "bottom": [(0.999, 0.0), (0.999, 0.0)],
    "top": [(0.999, 1.0), (0.999, 1.0)],
}


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
    rectilinear Axes, and which does not make its native Axes itself)."""
    return (issubclass(cls, Axes)
            and cls.name not in _NATIVE_PROJECTIONS
            and native_name(cls) == "rectilinear"
            and getattr(cls, "_make_native", None) is None)


def _before(transform, last):
    """The part of *transform* applied before *last*, when *transform* ends
    with it (``transData - transAxes``, found by identity so that nothing
    is evaluated); None otherwise."""
    if transform is last:
        return IdentityTransform()
    if isinstance(transform, TransformWrapper):
        return _before(transform._child, last)
    if isinstance(transform, (CompositeGenericTransform, CompositeAffine2D)):
        rest = _before(transform._b, last)
        if rest is not None:
            return transform._a + rest
    return None


def _to_axes(ax, transform, what):
    """*transform* of *ax* up to its final ``transAxes``."""
    before = _before(transform, ax.transAxes)
    if before is None:
        raise NotImplementedError(
            f"pltrs: a projection whose {what} does not end with transAxes")
    return before


class _NonAffinePart:
    """The non-affine part of a transform, as pltrs (Rust) maps points
    through it (with its inverse)."""

    def __init__(self, transform):
        self._transform = transform

    def transform(self, values):
        """*values* through the non-affine part."""
        return self._transform.transform_non_affine(np.asarray(values, dtype=float))

    def inverted(self):
        """The inverse of the non-affine part."""
        return _InverseNonAffinePart(self._transform)


class _InverseNonAffinePart:
    """The inverse of a `_NonAffinePart`: the inverse of the whole
    transform after its affine part."""

    def __init__(self, transform):
        self._transform = transform

    def transform(self, values):
        """*values* through the inverse of the non-affine part."""
        affine = self._transform.get_affine().transform(np.asarray(values, dtype=float))
        return self._transform.inverted().transform(affine)


def _evaluated(transform):
    """*transform* as pltrs (Rust) takes it: the matrix of its affine part
    and, for a non-affine transform, its non-affine part."""
    if transform.is_affine:
        return np.asarray(transform.get_matrix(), dtype=float), None
    return (np.asarray(transform.get_affine().get_matrix(), dtype=float),
            _NonAffinePart(transform))


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
        if sorted(spines) != sorted(_LINEAR_PATHS):
            raise NotImplementedError(
                "pltrs: a projection whose spines are not the four sides")
        self.spine_order = list(spines)
        self.to_axes = _to_axes(ax, ax.transData, "transData")
        given = ax.__dict__.get("_rectilinear_axis_transforms", (None, None))
        self.axis_transforms = {}
        self.probes = {}
        for name, rectilinear in zip("xy", given):
            transform = getattr(ax, f"get_{name}axis_transform")(which="grid")
            if transform is not rectilinear:
                self.axis_transforms[name] = _to_axes(
                    ax, transform, f"get_{name}axis_transform()")
            axis = getattr(ax, f"{name}axis")
            probe = axis._get_tick(major=True)
            # The ticks of the axis are of the class the program gives.
            axis._tick_class = type(probe)
            if type(probe).draw is not Tick.draw:
                self.probes[name] = probe
        self.spines = {side: spine for side, spine in spines.items()
                       if type(spine)._adjust_location is not Spine._adjust_location}
        self.overrides = {
            "axis_transform": list(self.axis_transforms),
            "tick_interval": [
                name for name in "xy"
                if type(getattr(ax, f"{name}axis")).get_view_interval
                is not Axis.get_view_interval],
            "tick_sides": list(self.probes),
            "spine_interval": list(self.spines),
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
        with _transform_trees.setting_leaves():
            _transform_trees.set_extents(ax._viewLim, limits)
            yield ax, ax._program_projection

    def set_up(self):
        """Have the trees follow the Axes again (it is new, or was
        cleared, which set its scales anew)."""
        ax = self._axes()
        if ax is not None:
            ax._update_transScale()

    def data_to_axes(self, limits):
        """``transData`` up to ``transAxes``, evaluated."""
        with self._viewing(limits) as (ax, projection):
            return _evaluated(projection.to_axes)

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

    def spine_interval(self, limits, side):
        """The interval the spine on *side* spans along its axis."""
        with self._viewing(limits) as (ax, projection):
            spine = projection.spines[side]
            spine._path = Path(np.array(_LINEAR_PATHS[side]))
            spine._adjust_location()
            along = 1 if side in ("left", "right") else 0
            v0, v1 = spine._path.vertices[:, along]
            return float(v0), float(v1)


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
