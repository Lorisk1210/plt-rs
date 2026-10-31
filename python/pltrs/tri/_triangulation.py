"""
The `Triangulation` class (matplotlib's ``tri._triangulation``): an
unstructured triangular grid, kept as a pltrs (Rust) triangulation.
"""

import numpy as np

from .. import _api
from .._native import tri as _tri


class Triangulation:
    """
    An unstructured triangular grid consisting of npoints points and
    ntri triangles.  The triangles can either be specified by the user
    or automatically generated using a Delaunay triangulation.

    Parameters
    ----------
    x, y : (npoints,) array-like
        Coordinates of grid points.
    triangles : (ntri, 3) array-like of int, optional
        For each triangle, the indices of the three points that make
        up the triangle, ordered in an anticlockwise manner.  If not
        specified, the Delaunay triangulation is calculated.
    mask : (ntri,) array-like of bool, optional
        Which triangles are masked out.

    Attributes
    ----------
    triangles : (ntri, 3) array of int
        For each triangle, the indices of the three points that make
        up the triangle.
    mask : (ntri, 3) array of bool or None
        Masked out triangles.
    is_delaunay : bool
        Whether the Triangulation is a calculated Delaunay
        triangulation (where *triangles* was not specified) or not.
    """

    def __init__(self, x, y, triangles=None, mask=None):
        self.x = np.asarray(x, dtype=np.float64)
        self.y = np.asarray(y, dtype=np.float64)
        if self.x.shape != self.y.shape or self.x.ndim != 1:
            raise ValueError("x and y must be equal-length 1D arrays, but "
                             f"found shapes {self.x.shape!r} and "
                             f"{self.y.shape!r}")
        native_triangles = None
        if triangles is not None:
            try:
                native_triangles = np.array(triangles, dtype=np.int64, order='C')
            except ValueError as e:
                raise ValueError('triangles must be a (N, 3) int array, not '
                                 f'{triangles!r}') from e
            if native_triangles.ndim != 2 or native_triangles.shape[1] != 3:
                raise ValueError(
                    'triangles must be a (N, 3) int array, but found shape '
                    f'{native_triangles.shape!r}')
            if native_triangles.max() >= len(self.x):
                raise ValueError(
                    'triangles are indices into the points and must be in the '
                    f'range 0 <= i < {len(self.x)} but found value '
                    f'{native_triangles.max()}')
            if native_triangles.min() < 0:
                raise ValueError(
                    'triangles are indices into the points and must be in the '
                    f'range 0 <= i < {len(self.x)} but found value '
                    f'{native_triangles.min()}')
        self._native = _tri.new(self.x, self.y, native_triangles, None)
        self.is_delaunay = native_triangles is None
        self.triangles = _tri.triangles(self._native).astype(np.int32)
        self.mask = None
        self.set_mask(mask)

    @classmethod
    def _from_native(cls, native):
        """The triangulation around the native triangulation *native*."""
        triangulation = cls.__new__(cls)
        triangulation._native = native
        triangulation.x, triangulation.y = _tri.points(native)
        triangulation.triangles = _tri.triangles(native).astype(np.int32)
        triangulation.is_delaunay = _tri.is_delaunay(native)
        mask = _tri.mask(native)
        triangulation.mask = None if mask is None else np.asarray(mask, dtype=bool)
        return triangulation

    def calculate_plane_coefficients(self, z):
        """
        Calculate plane equation coefficients for all unmasked triangles from
        the point (x, y) coordinates and specified z-array of shape (npoints).
        The returned array has shape (npoints, 3) and allows z-value at (x, y)
        position in triangle tri to be calculated using
        ``z = array[tri, 0] * x  + array[tri, 1] * y + array[tri, 2]``.
        """
        return _tri.plane_coefficients(self._native, np.asarray(z, dtype=float))

    def get_cpp_triangulation(self):
        """
        Return the underlying C++ Triangulation object (of the ``_tri``
        extension), creating it if necessary.
        """
        from .. import _tri
        if getattr(self, "_cpp_triangulation", None) is None:
            self._cpp_triangulation = _tri.Triangulation(
                # For unset arrays use empty tuple which has size of zero.
                self.x, self.y, self.triangles,
                self.mask if self.mask is not None else (),
                (), (), not self.is_delaunay)
        return self._cpp_triangulation

    @property
    def edges(self):
        """
        Return integer array of shape (nedges, 2) containing all edges of
        non-masked triangles.

        Each row defines an edge by its start point index and end point
        index.  Each edge appears only once, i.e. for an edge between points
        *i*  and *j*, there will only be either *(i, j)* or *(j, i)*.
        """
        return _tri.edges(self._native).astype(np.int32)

    def get_masked_triangles(self):
        """
        Return an array of triangles taking the mask into account.
        """
        return _tri.masked_triangles(self._native).astype(np.int32)

    @staticmethod
    def get_from_args_and_kwargs(*args, **kwargs):
        """
        Return a Triangulation object from the args and kwargs, and
        the remaining args and kwargs with the consumed values removed.

        There are two alternatives: either the first argument is a
        Triangulation object, in which case it is returned, or the args
        and kwargs are sufficient to create a new Triangulation to
        return.  In the latter case, see Triangulation.__init__ for
        the possible args and kwargs.
        """
        if isinstance(args[0], Triangulation):
            triangulation, *args = args
            if 'triangles' in kwargs:
                _api.warn_external(
                    "Passing the keyword 'triangles' has no effect when also "
                    "passing a Triangulation")
            if 'mask' in kwargs:
                _api.warn_external(
                    "Passing the keyword 'mask' has no effect when also "
                    "passing a Triangulation")
        else:
            x, y, triangles, mask, args, kwargs = \
                Triangulation._extract_triangulation_params(args, kwargs)
            triangulation = Triangulation(x, y, triangles, mask)
        return triangulation, args, kwargs

    @staticmethod
    def _extract_triangulation_params(args, kwargs):
        x, y, *args = args
        # Check triangles in kwargs then args.
        triangles = kwargs.pop('triangles', None)
        from_args = False
        if triangles is None and args:
            triangles = args[0]
            from_args = True
        if triangles is not None:
            try:
                triangles = np.asarray(triangles, dtype=np.int32)
            except ValueError:
                triangles = None
        if triangles is not None and (triangles.ndim != 2 or
                                      triangles.shape[1] != 3):
            triangles = None
        if triangles is not None and from_args:
            args = args[1:]  # Consumed first item in args.
        # Check for mask in kwargs.
        mask = kwargs.pop('mask', None)
        return x, y, triangles, mask, args, kwargs

    def get_trifinder(self):
        """
        Return the default `matplotlib.tri.TriFinder` of this
        triangulation, creating it if necessary.  This allows the same
        TriFinder object to be easily shared.
        """
        if getattr(self, "_trifinder", None) is None:
            from ._trifinder import TrapezoidMapTriFinder
            self._trifinder = TrapezoidMapTriFinder(self)
        return self._trifinder

    @property
    def neighbors(self):
        """
        Return integer array of shape (ntri, 3) containing neighbor triangles.

        For each triangle, the indices of the three triangles that
        share the same edges, or -1 if there is no such neighboring
        triangle.  ``neighbors[i, j]`` is the triangle that is the neighbor
        to the edge from point index ``triangles[i, j]`` to point index
        ``triangles[i, (j+1)%3]``.
        """
        return _tri.neighbors(self._native).astype(np.int32)

    def set_mask(self, mask):
        """
        Set or clear the mask array.

        Parameters
        ----------
        mask : None or bool array of length ntri
        """
        if mask is None:
            self.mask = None
        else:
            self.mask = np.asarray(mask, dtype=bool)
            if self.mask.shape != (self.triangles.shape[0],):
                raise ValueError('mask array must have same length as '
                                 'triangles array')
        # The trifinder, which locates points natively on the current
        # triangles, stays the same object (matplotlib reinitialises it).
        _tri.set_mask(self._native, None if self.mask is None else self.mask.tolist())
        if getattr(self, "_cpp_triangulation", None) is not None:
            self._cpp_triangulation.set_mask(
                self.mask if self.mask is not None else ())
