"""
matplotlib's ``_tri`` extension module: the triangulation, contour
generator and trifinder its `matplotlib.tri` classes are built on, here
backed by pltrs (Rust)'s triangulation (``_native.tri``).

The classes take the arguments of matplotlib's C++ classes, check them as
those do and raise their messages. pltrs (Rust) keeps every triangle
anticlockwise, so a triangulation with a clockwise triangle can only be
made with ``correct_triangle_orientations=True``.
"""

import inspect

import numpy as np

from ._native import tri as _native_tri


def _bind(cls, signature, args, kwargs, types=None):
    """
    The arguments of a constructor call bound to *signature*, the objects
    named in *types* of their types, or the `TypeError` matplotlib's
    extension raises for a call matching none of its constructors.
    """
    try:
        bound = signature.bind(*args, **kwargs).arguments
    except TypeError:
        bound = None
    if bound is None or not all(isinstance(bound[name], kind)
                                for name, kind in (types or {}).items()):
        invoked = ", ".join([*map(repr, args),
                             *(f"{key}={value!r}" for key, value in kwargs.items())])
        raise TypeError(
            "__init__(): incompatible constructor arguments. The following "
            "argument types are supported:\n"
            f"    1. matplotlib._tri.{cls.__name__}{signature}\n\n"
            f"Invoked with: {invoked}")
    return bound


def _array(value, dtype):
    """*value* as an array of *dtype*, as the extension converts it."""
    return np.asarray(value, dtype=dtype)


class Triangulation:
    """
    Create a new C++ Triangulation object.
    This should not be called directly, use the python class
    matplotlib.tri.Triangulation instead.
    """

    _signature = inspect.Signature([
        inspect.Parameter(name, inspect.Parameter.POSITIONAL_OR_KEYWORD)
        for name in ("x", "y", "triangles", "mask", "edges", "neighbors",
                     "correct_triangle_orientations")])

    def __init__(self, *args, **kwargs):
        bound = _bind(type(self), self._signature, args, kwargs)
        x = _array(bound["x"], np.float64)
        y = _array(bound["y"], np.float64)
        if x.ndim != 1 or y.ndim != 1 or x.shape[0] != y.shape[0]:
            raise ValueError("x and y must be 1D arrays of the same length")
        triangles = _array(bound["triangles"], np.int64)
        if triangles.ndim != 2 or triangles.shape[1] != 3:
            raise ValueError("triangles must be a 2D array of shape (?,3)")
        mask = self._checked_mask(bound["mask"], len(triangles))
        edges = _array(bound["edges"], np.int32)
        if edges.size > 0 and (edges.ndim != 2 or edges.shape[1] != 2):
            raise ValueError("edges must be a 2D array with shape (?,2)")
        neighbors = _array(bound["neighbors"], np.int32)
        if neighbors.size > 0 and (neighbors.ndim != 2
                                   or neighbors.shape != triangles.shape):
            raise ValueError("neighbors must be a 2D array with the same shape "
                             "as the triangles array")
        self._native = _native_tri.new(x, y, triangles, None)
        if (not bound["correct_triangle_orientations"]
                and not np.array_equal(_native_tri.triangles(self._native),
                                       _native_tri.oriented_triangles(self._native))):
            raise NotImplementedError(
                "pltrs: _tri.Triangulation(correct_triangle_orientations=False) "
                "with clockwise triangles")
        self._ntri = len(triangles)
        # Given edges and neighbors stand until the mask changes, as the
        # extension keeps them.
        self._edges = edges if edges.size > 0 else None
        self._neighbors = neighbors if neighbors.size > 0 else None
        self._set_native_mask(mask)

    @staticmethod
    def _checked_mask(mask, ntri):
        """*mask* as a bool array, empty for none, checked against *ntri*."""
        mask = _array(mask, bool)
        if mask.size > 0 and (mask.ndim != 1 or mask.shape[0] != ntri):
            raise ValueError("mask must be a 1D array with the same length as "
                             "the triangles array")
        return mask

    def _set_native_mask(self, mask):
        _native_tri.set_mask(self._native, mask.tolist() if mask.size > 0 else None)

    def calculate_plane_coefficients(self, z):
        """Calculate plane equation coefficients for all unmasked triangles."""
        z = _array(z, np.float64)
        npoints = len(_native_tri.points(self._native)[0])
        if z.ndim != 1 or z.shape[0] != npoints:
            raise ValueError("z must be a 1D array with the same length as the "
                             "triangulation x and y arrays")
        return _native_tri.plane_coefficients(self._native, z)

    def get_edges(self):
        """Return edges array."""
        if self._edges is None:
            self._edges = _native_tri.edges(self._native).astype(np.int32)
        return self._edges

    def get_neighbors(self):
        """Return neighbors array."""
        if self._neighbors is None:
            self._neighbors = _native_tri.neighbors(self._native).astype(np.int32)
        return self._neighbors

    def set_mask(self, mask):
        """Set or clear the mask array."""
        self._set_native_mask(self._checked_mask(mask, self._ntri))
        self._edges = self._neighbors = None


class TriContourGenerator:
    """
    Create a new C++ TriContourGenerator object.
    This should not be called directly, use the functions
    matplotlib.axes.tricontour and tricontourf instead.
    """

    _signature = inspect.Signature([
        inspect.Parameter(name, inspect.Parameter.POSITIONAL_OR_KEYWORD)
        for name in ("triangulation", "z")])

    def __init__(self, *args, **kwargs):
        bound = _bind(type(self), self._signature, args, kwargs,
                      {"triangulation": Triangulation})
        triangulation = bound["triangulation"]
        z = _array(bound["z"], np.float64)
        npoints = len(_native_tri.points(triangulation._native)[0])
        if z.ndim != 1 or z.shape[0] != npoints:
            raise ValueError("z must be a 1D array with the same length as the "
                             "x and y arrays")
        self._triangulation = triangulation
        self._z = z

    def create_contour(self, level):
        """Create and return a non-filled contour."""
        vertices, codes = _native_tri.contour_lines(
            self._triangulation._native, self._z, float(level))
        if codes is None:  # no line, or one open line
            codes = np.full(len(vertices), 2, np.uint8)
            codes[:1] = 1
        starts = np.flatnonzero(codes == 1)
        if not len(starts):
            return [], []
        return (np.split(vertices, starts[1:]),
                [line.astype(np.uint8) for line in np.split(codes, starts[1:])])

    def create_filled_contour(self, lower_level, upper_level):
        """Create and return a filled contour."""
        if lower_level >= upper_level:
            raise ValueError("filled contour levels must be increasing")
        vertices, codes = _native_tri.filled_contour(
            self._triangulation._native, self._z, float(lower_level),
            float(upper_level))
        return [vertices], [np.zeros(0, np.uint8) if codes is None else codes]


class TrapezoidMapTriFinder:
    """
    Create a new C++ TrapezoidMapTriFinder object.
    This should not be called directly, use the python class
    matplotlib.tri.TrapezoidMapTriFinder instead.
    """

    _signature = inspect.Signature([
        inspect.Parameter("triangulation", inspect.Parameter.POSITIONAL_OR_KEYWORD)])

    def __init__(self, *args, **kwargs):
        bound = _bind(type(self), self._signature, args, kwargs,
                      {"triangulation": Triangulation})
        self._triangulation = bound["triangulation"]

    def find_many(self, x, y):
        """Find indices of triangles containing the point coordinates (x, y)."""
        x = _array(x, np.float64)
        y = _array(y, np.float64)
        if x.ndim != y.ndim or x.shape != y.shape:
            raise ValueError("x and y must be array-like with same shape")
        return (_native_tri.find(self._triangulation._native, x.ravel(), y.ravel())
                .astype(np.int32).reshape(x.shape))

    def initialize(self):
        """
        Initialize this object, creating the trapezoid map from the
        triangulation. pltrs (Rust) locates points in the triangulation
        as it is at every search, so there is nothing to build ahead.
        """
