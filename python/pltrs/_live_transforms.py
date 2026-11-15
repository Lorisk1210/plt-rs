"""
The bridge between matplotlib's transforms and pltrs (Rust)'s.

Between Python and pltrs (Rust) a transform crosses as the nested tuples
``_native.transforms.native`` reads: `to_native` writes them for
matplotlib's classes (and for transforms of the program's: an affine one
as its matrix, any other as a map pltrs (Rust) calls back), the nodes of
the trees of Axes and figures that stand for a transform pltrs (Rust) has
(``ax.transData``, ``fig.transFigure``; see `._transform_trees`) as that
transform, which pltrs (Rust) resolves in the figure's layout whenever it
draws. `from_native` reads the description of a native transform back
into the objects matplotlib code compares (``line.get_transform() is
ax.transData``). A transform pltrs (Rust) has that no Python object stands
for (a map of a projection's) comes back as a `LiveTransform`.
"""

import weakref

import numpy as np

from . import _lazy_imports
from ._native import artist as _artist, drawing as _drawing, register as _register, transforms as _transforms
from . import _program_classes
from ._program_classes import class_cache
from .transforms import (
    Affine2D, Affine2DBase, AffineDeltaTransform, BboxTransformTo, BlendedAffine2D,
    BlendedGenericTransform, CompositeAffine2D, CompositeGenericTransform,
    IdentityTransform, ScaledTranslation, Transform, TransformedPath,
    TransformNode, TransformWrapper, blended_transform_factory,
    composite_transform_factory)
from .path import Path

_owned_paths = _lazy_imports.module(globals(), "._owned_paths")
_transform_trees = _lazy_imports.module(globals(), "._transform_trees")


class LiveTransform(Transform):
    """A transform pltrs (Rust) resolves in its figure's current layout."""

    input_dims = 2
    output_dims = 2
    has_inverse = True

    def __init__(self, figure, native, shorthand_name=None):
        super().__init__(shorthand_name=shorthand_name)
        self._root = figure
        self._native = native
        self._live = native

    def _apply(self, values, part):
        values = np.asanyarray(values, dtype=float)
        if values.size == 0:
            return values.reshape((-1, 2))
        return _transforms.apply(self._root._native, self._native,
                                 values, part)

    @property
    def is_affine(self):
        return _transforms.is_affine(self._root._native, self._native)

    @property
    def is_separable(self):
        return _transforms.is_separable(self._root._native, self._native)

    def transform(self, values):
        return self._apply(values, "all")

    def transform_affine(self, values):
        return self._apply(values, "affine")

    def transform_non_affine(self, values):
        return self._apply(values, "non_affine")

    def get_matrix(self):
        """The matrix of the affine part, in the current layout."""
        return _transforms.affine_matrix(self._root._native, self._native)

    def get_affine(self):
        return Affine2D(self.get_matrix())

    def inverted(self):
        return LiveTransform(self._root, _transforms.inverted(self._native))

    def frozen(self):
        if self.is_affine:
            return Affine2D(self.get_matrix())
        return self


def to_native(transform, held_by=None):
    """The native transform of the matplotlib transform *transform* (None:
    display coordinates). With *held_by*, the root figure of the artist
    given *transform* (or the artist, in no figure yet), pltrs (Rust) holds
    the transforms of the program's in it weakly, *held_by* strongly (see
    `._program_classes.held`)."""
    return _transforms.native(_description(transform, held=held_by))


def root_figure(artist):
    """The root figure of *artist* (a figure's own), read without calling
    its ``get_figure``, which a class of the program's may override (and
    whose calls the program sees); None outside any figure."""
    return getattr(getattr(artist, "_figure", None), "_figure", None)


def given_to_native(transform, axes=None, holder=None):
    """`to_native` of *transform*, None when none is given (for the
    ``transform`` keywords whose default the native function picks); an
    object of the program's giving its transform for an Axes (see
    `for_axes`) gives that of *axes*, which plots with it. The program's
    transforms in it are held for the root figure of *axes* (or *holder*,
    the root figure the artist is made in)."""
    if transform is None:
        return None
    if holder is None and axes is not None:
        holder = root_figure(axes)
    return to_native(for_axes(transform, axes), held_by=holder)


def holds_natively(transform):
    """Whether pltrs (Rust) can hold *transform* as an artist's: a
    `.Transform`, or None (display coordinates). matplotlib keeps any other
    object an artist is given, which its draw then fails on."""
    return transform is None or isinstance(transform, Transform)


def gives_transform(transform):
    """Whether *transform* is no `.Transform` but an object of the
    program's giving its transform for an Axes (``_as_mpl_transform``, as
    a cartopy CRS does), which matplotlib's ``Artist.get_transform``
    asks once the artist is in an Axes."""
    return (not isinstance(transform, Transform)
            and hasattr(transform, '_as_mpl_transform'))


def for_axes(transform, axes):
    """
    *transform*, or the transform an object of the program that is no
    `.Transform` gives for *axes* through its ``_as_mpl_transform`` (as
    matplotlib's ``Artist.get_transform`` asks it).
    """
    if gives_transform(transform):
        return transform._as_mpl_transform(axes)
    return transform


def _computes_matrix(t):
    """Whether *t* holds, anywhere in its tree, an affine transform of a
    class of the program's that computes its matrix itself (``get_matrix``,
    as mpltern's shift of a tick label away from its corner does, from
    where the Axes is when asked)."""
    if getattr(t, "_live", None) is not None or not isinstance(t, Transform):
        return False
    if isinstance(t, TransformWrapper):
        return _computes_matrix(t._child)
    if isinstance(t, (CompositeGenericTransform, CompositeAffine2D)):
        return _computes_matrix(t._a) or _computes_matrix(t._b)
    if isinstance(t, (BlendedGenericTransform, BlendedAffine2D)):
        return _computes_matrix(t._x) or _computes_matrix(t._y)
    from ._program_classes import takes_from_program
    return t.is_affine and takes_from_program(type(t), "get_matrix")


def follow_given(artist):
    """
    Hand pltrs (Rust) the transform *artist* was given again where the
    artist is measured, when it holds a matrix the program's class
    computes: pltrs (Rust) took the matrix as it was then, and matplotlib
    computes it wherever it maps through the transform.
    """
    given = artist.__dict__.get("_given_transform")
    if not isinstance(given, Transform):
        return
    computes = given.__dict__.get("_computes_matrix")
    if computes is None:
        # (The tree of a transform is made once.)
        computes = given.__dict__["_computes_matrix"] = _computes_matrix(given)
    if computes:
        _transforms.set_transform(artist._native, to_native(given, held_by=artist._figure or artist))


def ask_when_drawn(artist, transform):
    """
    Have pltrs (Rust) ask *artist* for *transform*, the transform it was
    given, before every draw and measure reaching it when *transform*
    computes a matrix in the program's code (`_computes_matrix`), as
    matplotlib asks it for the matrix wherever it draws or measures the
    artist (pltrs (Rust) holds a matrix as it was when handed over); no
    more once the artist is given another. Called only for transforms
    pltrs (Rust) has no node for, or for an artist asked so before.
    """
    computes = isinstance(transform, Transform) and _computes_matrix(transform)
    if computes:
        artist.__dict__["_asked_transform"] = True
        _drawing.ask_for_transform(artist._native, weakref.ref(artist))
    elif artist.__dict__.pop("_asked_transform", False):
        _drawing.ask_for_transform(artist._native, None)


class _GivenWatcher(TransformNode):
    """
    A parent of the transform an artist was given, holding parts pltrs
    (Rust) takes by their matrix as it is when handed (an affine transform
    of Python's, on a box the program changes): a change of one of them
    hands pltrs (Rust) the transform again, as matplotlib maps through the
    transform's parts as they are wherever it draws or measures.
    """

    def __init__(self, artist, transform):
        super().__init__()
        self._artist = weakref.ref(artist)
        self._given = transform
        self.set_children(transform)

    def _invalidate_internal(self, level, invalidating_node):
        artist = self._artist()
        # Changes made while pltrs (Rust) lays the figure out or draws it
        # are its own syncs and the code of the program's it calls (an
        # axis placing its spine), which it follows itself.
        if (artist is not None and not _transform_trees._syncing()
                and artist.__dict__.get("_given_transform") is self._given
                and "_native" in artist.__dict__
                and not _artist.figure_in_use(artist._native)):
            _transforms.set_transform(artist._native, to_native(self._given))

    def __reduce__(self):
        # Pickled with the artist, it unpickles as a plain node (the artist
        # is handed its transform again when it is read back).
        return (TransformNode, ())


def watch_given(artist, transform):
    """Have *artist*, made with *transform* by a plotting method (``plot``'s
    ``transform``), follow it where pltrs (Rust) took parts of it by their
    matrix (see `_GivenWatcher`); nothing to follow otherwise."""
    artist.__dict__.pop("_given_watcher", None)
    if isinstance(transform, Transform) and _takes_matrices(_description(transform)):
        artist.__dict__["_given_watcher"] = _GivenWatcher(artist, transform)
    if getattr(transform, "_live", None) is None or "_asked_transform" in artist.__dict__:
        ask_when_drawn(artist, transform)


def _takes_matrices(description):
    """Whether the transform *description* holds a part by its matrix."""
    tag, *parts = description
    if tag == "affine":
        return True
    return any(isinstance(part, tuple) and part and isinstance(part[0], str)
               and _takes_matrices(part) for part in parts)


def _description(t, by_matrix=False, held=None):
    """The nested tuples describing *t* to pltrs (Rust); *by_matrix* for a
    part of a composition matplotlib maps through its matrix
    (``get_matrix``), whatever methods its class gives for points; *held*
    the figure holding the program's transforms (`to_native`'s
    *held_by*)."""
    live = getattr(t, "_live", None)
    if live is not None:  # A transform pltrs (Rust) has.
        return ("native", live)
    if t is None or isinstance(t, IdentityTransform):
        return ("display",)
    spanning = getattr(getattr(t, "_boxout", None), "_spanning", None)
    if isinstance(t, BboxTransformTo) and spanning is not None:
        # Onto the box of an Axes or figure, which follows it.
        return _description(spanning, by_matrix)
    if isinstance(t, TransformWrapper):
        return _description(t._child, by_matrix, held)
    if isinstance(t, (CompositeGenericTransform, CompositeAffine2D)):
        # matplotlib maps points through the first part's methods only
        # where a non-affine second part follows it; an affine part
        # otherwise goes into the composition's matrix.
        first_by_matrix = by_matrix or isinstance(t, CompositeAffine2D) or (
            _maps_points_itself(type(t._a)) and t._b.is_affine)
        return ("composite", _description(t._a, first_by_matrix, held),
                _description(t._b, True, held))
    if isinstance(t, (BlendedGenericTransform, BlendedAffine2D)):
        return ("blended", _description(t._x, True, held), _description(t._y, True, held))
    if isinstance(t, ScaledTranslation):
        dx, dy = t._t
        return ("translation", float(dx), float(dy),
                _description(t._scale_trans, True, held))
    if isinstance(t, AffineDeltaTransform):
        return ("affine_delta", _description(t._base_transform, by_matrix, held))
    if (isinstance(t, Affine2DBase) or t.is_affine) and (
            by_matrix or not _maps_points_itself(type(t))):
        return ("affine", np.asarray(t.get_matrix(), dtype=float))
    return ("python", _ProgramMap(_program_classes.held(t, held)))


@class_cache
def _maps_points_itself(cls):
    """Whether the affine class *cls* maps points through a method of the
    program's (``transform_affine``, ``transform``, ``transform_point``),
    which matplotlib's code calls wherever it maps points through the
    transform: pltrs (Rust) then maps through it (`_ProgramMap`), not
    through its matrix."""
    from ._program_classes import takes_from_program
    return any(takes_from_program(cls, name)
               for name in ("transform_affine", "transform", "transform_point"))


class _ProgramMap:
    """
    A non-affine transform of the program's as pltrs (Rust) maps through
    it: points through its ``transform``, called with the ``(N, 2)`` array
    of all the points it needs at once, as matplotlib calls it, and paths as
    matplotlib's artists map theirs, through a `.TransformedPath`, which
    keeps the path the transform last gave until the transform is
    invalidated.
    """

    #: The map gives the same answer for the same point for as long as it
    #: lives, so that pltrs (Rust) may answer from what it answered before
    #: while it lays the figure out (the program's transform is called
    #: where matplotlib calls it, see ``transforms::program_calls``).
    _answers_alike = True

    def __init__(self, transform):
        #: The transform, or what pltrs holds for it
        #: (`._program_classes.held`).
        self._held = transform
        self._cached = None

    @property
    def _transform(self):
        return _program_classes.unheld(self._held)

    def __getstate__(self):
        return {"_held": self._transform, "_cached": None}

    def transform(self, values):
        """*values*, a point or an ``(N, 2)`` array of points, through the
        transform, which gets an ``(N, 2)`` array whatever *values* is, as
        matplotlib's code hands it its points (astropy's world transforms
        take their columns): through its ``transform_non_affine``, then its
        affine part, as matplotlib's composite transforms map points
        through a non-affine transform they start with."""
        points = np.asarray(values, dtype=float)
        if self._transform.is_affine:
            # An affine class mapping points itself (`_maps_points_itself`):
            # through its ``transform``, a point as the pair it is, as
            # matplotlib's text places itself.
            mapped = np.asarray(self._transform.transform(values if points.ndim == 1 else points),
                                dtype=float)
            return tuple(mapped) if points.ndim == 1 else mapped
        mapped = self._transform.transform_non_affine(points.reshape(-1, 2))
        affine = self._transform.get_affine()
        if not isinstance(affine, IdentityTransform):
            mapped = affine.transform(mapped)
        mapped = np.asarray(mapped, dtype=float)
        return tuple(mapped[0]) if points.ndim == 1 else mapped

    def inverted(self):
        """The way back through the transform (`_InverseMap`)."""
        return _InverseMap(self._held)

    def transform_path(self, vertices, codes, steps=1):
        """The `.Path` of *vertices* and *codes* (with *steps*
        interpolation steps) through the transform: the one computed last
        while the path and the transform stay the same. The path an
        artist was given or handed out with that content, if one lives, is
        mapped itself, anew, as matplotlib's collections and patches map
        their own at every draw (programs keep what they computed by path:
        cartopy's projections), and not kept. An affine transform maps
        paths through its matrix, as matplotlib draws them (its renderers
        take ``get_affine()``), whatever methods its class gives for
        points."""
        if self._transform.is_affine:
            matrix = Affine2D(self._transform.get_affine().get_matrix())
            return matrix.transform_path(Path(vertices, codes, _interpolation_steps=steps))
        held = _owned_paths.held(vertices, codes, steps)
        if held is not None:
            return self._transform.transform_path(held)
        key = (vertices.tobytes(), None if codes is None else codes.tobytes(), steps)
        if self._cached is None or self._cached[0] != key:
            path = Path(vertices, codes, _interpolation_steps=steps)
            transform = self._transform
            if self._held is not transform:
                # (Kept as weakly as the map keeps the transform.)
                transform = weakref.proxy(transform)
            self._cached = (key, TransformedPath(path, transform))
        return self._cached[1].get_fully_transformed_path()

    def transform_outline(self, vertices, codes, steps=1):
        """The outline of a patch, of *vertices* and *codes* (with *steps*
        interpolation steps), through the transform, mapped anew as
        matplotlib's ``Patch.draw`` maps it at every draw (an affine
        one through its matrix, see `transform_path`)."""
        if self._transform.is_affine:
            return self.transform_path(vertices, codes, steps)
        path = Path(vertices, codes, _interpolation_steps=steps)
        mapped = self._transform.transform_path_non_affine(path)
        affine = self._transform.get_affine()
        if isinstance(affine, IdentityTransform):
            return mapped
        return affine.transform_path(mapped)

    def transform_members(self, paths):
        """The paths of a collection, *paths* as ``(vertices, codes,
        steps)``, through the transform, as matplotlib's
        ``Collection._prepare_points`` maps them at every draw: each through
        ``transform_path_non_affine``, then all through ``get_affine()``,
        asked once (an affine one through its matrix, see
        `transform_path`)."""
        if self._transform.is_affine:
            return [self.transform_path(*path) for path in paths]
        transform = self._transform
        mapped = [transform.transform_path_non_affine(_given_path(*path)) for path in paths]
        affine = transform.get_affine()
        if isinstance(affine, IdentityTransform):
            return mapped
        return [affine.transform_path(path) for path in mapped]


def _given_path(vertices, codes, steps):
    """The `.Path` of *vertices* and *codes* with *steps* interpolation
    steps: the one an artist was given or handed out with that content, if
    one lives (programs keep what they computed by path, see
    `_ProgramMap.transform_path`), else a new one."""
    held = _owned_paths.held(vertices, codes, steps)
    if held is not None:
        return held
    return Path(vertices, codes, _interpolation_steps=steps)


class _InverseMap:
    """
    The way back through a non-affine transform of the program's, as pltrs
    (Rust) maps points back through it. The transform is asked for its
    inverse when the first point goes back, and once: matplotlib asks a
    transform for its inverse where it needs one, and handing an artist a
    transform inverts nothing (a program counts the calls of its
    ``inverted``). A transform that gives no inverse maps every point
    back to NaN.
    """

    def __init__(self, transform):
        #: The transform, or what pltrs holds for it (`._program_classes.held`).
        self._held = transform
        #: The map through the inverse once asked for; False without one.
        self._back = None

    @property
    def _forward(self):
        return _program_classes.unheld(self._held)

    def transform(self, values):
        """*values*, a point or an ``(N, 2)`` array of points, back through
        the transform (`_ProgramMap.transform`)."""
        if self._back is None:
            try:
                self._back = _ProgramMap(self._forward.inverted())
            except Exception:
                # The program's transform has no inverse (its ``inverted``
                # fails): every point maps back to NaN.
                self._back = False
        if self._back is False:
            points = np.asarray(values, dtype=float)
            return (np.nan, np.nan) if points.ndim == 1 else np.full(points.shape, np.nan)
        return self._back.transform(values)


class _Unnamed(Exception):
    """A description names an artist that has no Python object yet."""


def of_artist(artist, native, role="transform"):
    """
    The matplotlib transform of the native transform *native* set on
    *artist* as its *role* (``"transform"``, ``"offset"``; `from_native`):
    `.IdentityTransform` for none, the display coordinates an artist
    without a transform is drawn in, and while the artist is outside any
    figure. The object made is kept for the artist and given again while
    the native transform stays the same, as matplotlib keeps an artist's
    ``_transform``.
    """
    in_figure = native is not None and artist._figure is not None
    if not in_figure:
        return kept(artist, role, None, IdentityTransform)
    return kept(artist, role, _transforms.describe(native),
                lambda: from_native(artist._figure, native))


#: The attribute of an artist holding what `kept` made for it (not
#: pickled with it: the objects are made again from the native state).
#: The artist holds them, as what they refer to may reach the artist
#: again (a clip path reaching its Axes and figure, the figure the artist):
#: kept apart, by an artist held weakly, they would keep it alive.
KEPT = "_kept_made"


def kept(artist, role, description, make):
    """
    The object *make()* made for *artist* as its *role* while the native
    *description* (of `_native.transforms.describe`, or any value
    comparable by content) stays the same: the transform or clip box an
    artist keeps in matplotlib, made again when pltrs (Rust)'s changes.
    """
    made = vars(artist).setdefault(KEPT, {})
    key = _key(description)
    entry = made.get(role)
    if entry is None or entry[0] != key:
        entry = made[role] = (key, make())
    return entry[1]


def _key(description):
    """*description* (of `_native.transforms.describe`) as a value that
    compares by content (its matrices as tuples)."""
    if isinstance(description, tuple):
        return tuple(_key(item) for item in description)
    if isinstance(description, np.ndarray):
        return ("array", description.tobytes())
    return description


def from_native(figure, native):
    """The matplotlib transform of the native transform *native* of the
    root *figure*: the live transforms of its axes and figures where they
    are, matplotlib's classes around them (one live transform for the whole
    when it names an artist that has no Python object)."""
    try:
        return _transform_of(figure, _transforms.describe(native))
    except _Unnamed:
        return LiveTransform(figure, native)


def _transform_of(figure, description):
    kind, *args = description
    if kind in ("data", "axes", "figure"):
        artist = _named(figure, args[0])
        if artist is None:
            raise _Unnamed
        return {"data": lambda: artist.transData,
                "axes": lambda: artist.transAxes,
                "figure": lambda: artist.transSubfigure}[kind]()
    if kind == "blended":
        axis = _axis_transform(figure, *args)
        if axis is not None:
            return axis
    children = [_transform_of(figure, a) for a in args
                if isinstance(a, tuple)]
    if kind == "display":
        return IdentityTransform()
    if kind == "inches":
        return figure.dpi_scale_trans
    if kind == "affine":
        return Affine2D(args[0])
    if kind == "blended":
        return blended_transform_factory(*children)
    if kind == "composite":
        return composite_transform_factory(*children)
    if kind == "translation":
        return ScaledTranslation(args[0], args[1], children[0])
    if kind == "inverted":
        return children[0].inverted()
    if kind == "affine_delta":
        return AffineDeltaTransform(children[0])
    return LiveTransform(figure, args[0])  # ("native", a custom map)


def _axis_transform(figure, x, y):
    """``ax.get_xaxis_transform()`` or ``ax.get_yaxis_transform()`` when the
    blend of the descriptions *x* and *y* is one of them, else None."""
    kinds = (x[0], y[0])
    if kinds not in (("data", "axes"), ("axes", "data")) or x[1] != y[1]:
        return None
    ax = _named(figure, x[1])
    if ax is None:
        return None
    if kinds == ("data", "axes"):
        return ax.get_xaxis_transform()
    return ax.get_yaxis_transform()


def _named(figure, artist_id):
    """The object of the artist *artist_id* of the root *figure*: of an
    Axes, a figure, or an Axes standing for the projection plane it is
    (a 3-D Axes); None for an artist without one yet."""
    artist = figure._artists.get(artist_id)
    if artist is None:
        artist = figure.__dict__.get("_tree_owners", {}).get(artist_id)
    return artist


# The masked arrays pltrs (Rust) hands the program's scales where
# matplotlib hands masked ones (a mesh's coordinates).
_register("masked_invalid", np.ma.masked_invalid)
