"""
The transforms pltrs (Rust) resolves in a figure's current layout:
``ax.transData``, ``ax.transAxes``, ``fig.transFigure``,
``fig.dpi_scale_trans`` and the boxes they span (``ax.bbox``,
``fig.bbox``), and the transforms artists are drawn with.

A live transform holds a native transform (``_native.transforms``) and
the root figure resolving it; its points come from pltrs (Rust) whenever
they are asked for, so it follows layout, limits and scales without
invalidation. matplotlib's own transform classes combine with it as with
any `.Transform`. Between Python and pltrs (Rust) a transform crosses as
the nested tuples ``_native.transforms.native`` reads: `to_native` writes
them for matplotlib's classes (and for transforms of the program's: an
affine one as its matrix, any other as a map pltrs (Rust) calls back),
`from_native` reads the description of a native transform back into the
objects matplotlib code compares (``line.get_transform() is
ax.transData``).
"""

import numpy as np

from ._native import transforms as _transforms
from .transforms import (
    Affine2D, Affine2DBase, AffineDeltaTransform, BboxBase,
    BlendedAffine2D, BlendedGenericTransform, CompositeAffine2D,
    CompositeGenericTransform, IdentityTransform, ScaledTranslation,
    Transform, TransformWrapper, blended_transform_factory,
    composite_transform_factory)


class LiveTransform(Transform):
    """A transform pltrs (Rust) resolves in its figure's current layout."""

    input_dims = 2
    output_dims = 2
    has_inverse = True

    def __init__(self, figure, native, shorthand_name=None):
        super().__init__(shorthand_name=shorthand_name)
        self._root = figure
        self._native = native

    def _apply(self, values, part):
        values = np.asanyarray(values, dtype=float)
        if values.size == 0:
            return values.reshape((-1, 2))
        return _transforms.apply(self._root._native, self._native,
                                 values, part)

    @property
    def is_affine(self):
        return _transforms.is_affine(self._root._native, self._native)

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


def cached(owner, name, native):
    """The live transform *name* of the axes or figure *owner*
    (``ax.transData``, ``fig.transFigure``...), made the first time from
    the native transform ``native(owner._native)``, so that every read
    gives the same object."""
    key = f"_live_{name}"
    transform = owner.__dict__.get(key)
    if transform is None:
        transform = LiveTransform(owner.figure, native(owner._native),
                                  shorthand_name=name)
        owner.__dict__[key] = transform
    return transform


def inches(figure):
    """The native transform of physical inches (``fig.dpi_scale_trans``)."""
    return _transforms.native(("inches",))


class LiveBbox(BboxBase):
    """
    A box pltrs (Rust) gives the points of whenever they are read: the
    display box a live transform maps the unit square to (``ax.bbox``,
    ``fig.bbox``), or the limits of an axes (``ax.dataLim``,
    ``ax.viewLim``). It is read-only, as matplotlib's `.BboxBase`.
    """

    def __init__(self, points, minpos=None):
        super().__init__()
        self._points_of = points
        self._minpos_of = minpos

    @classmethod
    def spanned(cls, transform):
        """The box *transform* maps the unit square to (kept as
        ``_spanning``, by which an artist clipped to the box follows it)."""
        box = cls(lambda: transform.transform([[0.0, 0.0], [1.0, 1.0]]))
        box._spanning = transform
        return box

    def get_points(self):
        return np.asarray(self._points_of(), dtype=float).reshape(2, 2)

    def frozen(self):
        """A frozen copy of the box, with its smallest positive coordinates
        when it has them (``dataLim``)."""
        bbox = super().frozen()
        if self._minpos_of is not None:
            bbox._minpos = self.minpos.copy()
        return bbox

    @property
    def minpos(self):
        """The smallest positive coordinates of the data (``dataLim``)."""
        if self._minpos_of is None:
            raise AttributeError("minpos")
        return np.asarray(self._minpos_of(), dtype=float)

    @property
    def minposx(self):
        """The smallest positive x of the data."""
        return self.minpos[0]

    @property
    def minposy(self):
        """The smallest positive y of the data."""
        return self.minpos[1]


def to_native(transform):
    """The native transform of the matplotlib transform *transform* (None:
    display coordinates)."""
    return _transforms.native(_description(transform))


def given_to_native(transform):
    """`to_native` of *transform*, None when none is given (for the
    ``transform`` keywords whose default the native function picks)."""
    return None if transform is None else to_native(transform)


def _description(t):
    """The nested tuples describing *t* to pltrs (Rust)."""
    if t is None or isinstance(t, IdentityTransform):
        return ("display",)
    if isinstance(t, LiveTransform):
        return ("native", t._native)
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
    return ("python", t)


class _Unnamed(Exception):
    """A description names an artist that has no Python object yet."""


def of_artist(artist, native):
    """The matplotlib transform of the native transform *native* set on
    *artist* (`from_native`): `.IdentityTransform` for none, the display
    coordinates an artist without a transform is drawn in, and while the
    artist is outside any figure."""
    if native is None or artist._figure is None:
        return IdentityTransform()
    return from_native(artist._figure, native)


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
        artist = figure._artists.get(args[0])
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
    ax = figure._artists.get(x[1])
    if ax is None:
        return None
    if kinds == ("data", "axes"):
        return ax.get_xaxis_transform()
    return ax.get_yaxis_transform()
