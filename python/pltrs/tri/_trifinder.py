"""
Point location on triangular grids (matplotlib's ``tri._trifinder``).
"""

import numpy as np

from .. import _api
from .._native import tri as _tri
from ._triangulation import Triangulation


class TriFinder:
    """
    Abstract base class for classes used to find the triangles of a
    Triangulation in which (x, y) points lie.

    Rather than instantiate an object of a class derived from TriFinder, it is
    usually better to use the function `.Triangulation.get_trifinder`.

    Derived classes implement __call__(x, y) where x and y are array-like point
    coordinates of the same shape.
    """

    def __init__(self, triangulation):
        _api.check_isinstance(Triangulation, triangulation=triangulation)
        self._triangulation = triangulation

    def __call__(self, x, y):
        raise NotImplementedError


class TrapezoidMapTriFinder(TriFinder):
    """
    `~matplotlib.tri.TriFinder` class implemented using the trapezoid
    map algorithm from the book "Computational Geometry, Algorithms and
    Applications", second edition, by M. de Berg, M. van Kreveld, M. Overmars
    and O. Schwarzkopf.

    The triangulation must be valid, i.e. it must not have duplicate points,
    triangles formed from colinear points, or overlapping triangles.  The
    algorithm has some tolerance to triangles formed from colinear points, but
    this should not be relied upon.
    """

    def __init__(self, triangulation):
        super().__init__(triangulation)

    def __call__(self, x, y):
        """
        Return an array containing the indices of the triangles in which the
        specified *x*, *y* points lie, or -1 for points that do not lie within
        a triangle.

        *x*, *y* are array-like x and y coordinates of the same shape and any
        number of dimensions.

        Returns integer array with the same shape and *x* and *y*.
        """
        x = np.asarray(x, dtype=np.float64)
        y = np.asarray(y, dtype=np.float64)
        if x.shape != y.shape:
            raise ValueError("x and y must be array-like with the same shape")
        return (_tri.find(self._triangulation._native, x.ravel(), y.ravel())
                .astype(np.int32).reshape(x.shape))
