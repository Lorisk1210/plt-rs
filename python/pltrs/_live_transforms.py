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

from ._native import transforms as _transforms
from .transforms import (
    Affine2D, Affine2DBase, AffineDeltaTransform, BlendedAffine2D,
    BlendedGenericTransform, CompositeAffine2D, CompositeGenericTransform,
    IdentityTransform, ScaledTranslation, Transform, TransformedPath,
    TransformWrapper, blended_transform_factory, composite_transform_factory)
from .path import Path


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


def to_native(transform):
    """The native transform of the matplotlib transform *transform* (None:
    display coordinates)."""
    return _transforms.native(_description(transform))


def given_to_native(transform):
    """`to_native` of *transform*, None when none is given (for the
    ``transform`` keywords whose default the native function picks)."""
    return None if transform is None else to_native(transform)


def for_axes(transform, axes):
    """
    *transform*, or the transform an object of the program that is no
    `.Transform` gives for *axes* through its ``_as_mpl_transform`` (as
    matplotlib's ``Artist.get_transform`` asks it).
    """
    if (not isinstance(transform, Transform)
            and hasattr(transform, '_as_mpl_transform')):
        return transform._as_mpl_transform(axes)
    return transform


def _description(t):
    """The nested tuples describing *t* to pltrs (Rust)."""
    live = getattr(t, "_live", None)
    if live is not None:  # A transform pltrs (Rust) has.
        return ("native", live)
    if t is None or isinstance(t, IdentityTransform):
        return ("display",)
    if isinstance(t, TransformWrapper):
        return _description(t._child)
    if isinstance(t, (CompositeGenericTransform, CompositeAffine2D)):
        return ("composite", _description(t._a), _description(t._b))
    if isinstance(t, (BlendedGenericTransform, BlendedAffine2D)):
        return ("blended", _description(t._x), _description(t._y))
    if isinstance(t, ScaledTranslation):
        dx, dy = t._t
        return ("translation", float(dx), float(dy),
                _description(t._scale_trans))
    if isinstance(t, AffineDeltaTransform):
        return ("affine_delta", _description(t._base_transform))
    if isinstance(t, Affine2DBase) or t.is_affine:
        return ("affine", np.asarray(t.get_matrix(), dtype=float))
    return ("python", _ProgramMap(t))


class _ProgramMap:
    """
    A non-affine transform of the program's as pltrs (Rust) maps through
    it: points through its ``transform``, called with the ``(N, 2)`` array
    of all the points it needs at once, as matplotlib calls it, and paths as
    matplotlib's artists map theirs, through a `.TransformedPath`, which
    keeps the path the transform last gave until the transform is
    invalidated.
    """

    def __init__(self, transform):
        self._transform = transform
        self._cached = None

    def __getstate__(self):
        return {"_transform": self._transform, "_cached": None}

    def transform(self, values):
        """*values* through the transform."""
        return self._transform.transform(values)

    def inverted(self):
        """The inverse of the transform."""
        return self._transform.inverted()

    def transform_path(self, vertices, codes, steps=1):
        """The `.Path` of *vertices* and *codes* (with *steps*
        interpolation steps) through the transform: the one computed last
        while the path and the transform stay the same."""
        key = (vertices.tobytes(), None if codes is None else codes.tobytes(), steps)
        if self._cached is None or self._cached[0] != key:
            path = Path(vertices, codes, _interpolation_steps=steps)
            self._cached = (key, TransformedPath(path, self._transform))
        return self._cached[1].get_fully_transformed_path()


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


#: What `kept` keeps, by artist (not pickled with it: the objects it holds
#: are made again from the native state).
_kept = weakref.WeakKeyDictionary()


def kept(artist, role, description, make):
    """
    The object *make()* made for *artist* as its *role* while the native
    *description* (of `_native.transforms.describe`, or any value
    comparable by content) stays the same: the transform or clip box an
    artist keeps in matplotlib, made again when pltrs (Rust)'s changes.
    """
    made = _kept.setdefault(artist, {})
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
