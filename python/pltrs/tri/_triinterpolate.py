"""
Interpolation inside triangular grids (matplotlib's
``tri._triinterpolate``); pltrs (Rust) interpolates.
"""

import numpy as np

from .. import _api
from .._native import tri as _tri
from ._trifinder import TriFinder
from ._triangulation import Triangulation

__all__ = ('TriInterpolator', 'LinearTriInterpolator', 'CubicTriInterpolator')


class TriInterpolator:
    """
    Abstract base class for classes used to interpolate on a triangular grid.

    Derived classes implement the following methods:

    - ``__call__(x, y)``,
      where x, y are array-like point coordinates of the same shape, and
      that returns a masked array of the same shape containing the
      interpolated z-values.

    - ``gradient(x, y)``,
      where x, y are array-like point coordinates of the same
      shape, and that returns a list of 2 masked arrays of the same shape
      containing the 2 derivatives of the interpolator (derivatives of
      interpolated z values with respect to x and y).
    """

    #: How pltrs (Rust) interpolates: ``(kind, gradients)``.
    _native_kind = ("linear", None)

    def __init__(self, triangulation, z, trifinder=None):
        _api.check_isinstance(Triangulation, triangulation=triangulation)
        self._triangulation = triangulation

        self._z = np.asarray(z)
        if self._z.shape != self._triangulation.x.shape:
            raise ValueError("z array must have same length as triangulation x"
                             " and y arrays")

        _api.check_isinstance((TriFinder, None), trifinder=trifinder)
        self._trifinder = trifinder or self._triangulation.get_trifinder()

    def _interpolate(self, x, y):
        """The values and gradients at *x*, *y*, masked outside the grid."""
        x = np.asarray(x, dtype=np.float64)
        y = np.asarray(y, dtype=np.float64)
        sh_ret = x.shape
        if x.shape != y.shape:
            raise ValueError("x and y shall have same shapes."
                             f" Given: {x.shape} and {y.shape}")
        z, dzdx, dzdy = _tri.interpolate(
            self._triangulation._native, self._z.astype(float), self._native_kind,
            (np.ravel(x), np.ravel(y)))
        return [np.ma.masked_invalid(values.reshape(sh_ret), copy=False)
                for values in (z, dzdx, dzdy)]

    def __call__(self, x, y):
        """
        Returns a masked array containing interpolated values at the
        specified (x, y) points.

        Parameters
        ----------
        x, y : array-like
            x and y coordinates of the same shape and any number of
            dimensions.

        Returns
        -------
        np.ma.array
            Masked array of the same shape as *x* and *y*; values
            corresponding to (*x*, *y*) points outside of the triangulation
            are masked out.
        """
        return self._interpolate(x, y)[0]

    def gradient(self, x, y):
        r"""
        Returns a list of 2 masked arrays containing interpolated derivatives
        at the specified (x, y) points.

        Parameters
        ----------
        x, y : array-like
            x and y coordinates of the same shape and any number of
            dimensions.

        Returns
        -------
        dzdx, dzdy : np.ma.array
            2 masked arrays of the same shape as *x* and *y*; values
            corresponding to (x, y) points outside of the triangulation
            are masked out.
            The first returned array contains the values of
            :math:`\frac{\partial z}{\partial x}` and the second those of
            :math:`\frac{\partial z}{\partial y}`.
        """
        return self._interpolate(x, y)[1:]


class LinearTriInterpolator(TriInterpolator):
    """
    Linear interpolator on a triangular grid.

    Each triangle is represented by a plane so that an interpolated value at
    point (x, y) lies on the plane of the triangle containing (x, y).
    Interpolated values are therefore continuous across the triangulation, but
    their first derivatives are discontinuous at edges between triangles.

    Parameters
    ----------
    triangulation : `~matplotlib.tri.Triangulation`
        The triangulation to interpolate over.
    z : (npoints,) array-like
        Array of values, defined at grid points, to interpolate between.
    trifinder : `~matplotlib.tri.TriFinder`, optional
        If this is not specified, the Triangulation's default TriFinder will
        be used by calling `.Triangulation.get_trifinder`.
    """


class CubicTriInterpolator(TriInterpolator):
    r"""
    Cubic interpolator on a triangular grid.

    In one-dimension - on a segment - a cubic interpolating function is
    defined by the values of the function and its derivative at both ends.
    This is almost the same in 2D inside a triangle, except that the values
    of the function and its 2 derivatives have to be defined at each triangle
    node.

    The CubicTriInterpolator takes the value of the function at each node -
    provided by the user - and internally computes the value of the
    derivatives, resulting in a smooth interpolation.

    Parameters
    ----------
    triangulation : `~matplotlib.tri.Triangulation`
        The triangulation to interpolate over.
    z : (npoints,) array-like
        Array of values, defined at grid points, to interpolate between.
    kind : {'min_E', 'geom', 'user'}, optional
        Choice of the smoothing algorithm, in order to compute
        the interpolant derivatives (defaults to 'min_E').
    trifinder : `~matplotlib.tri.TriFinder`, optional
        If not specified, the Triangulation's default TriFinder will
        be used by calling `.Triangulation.get_trifinder`.
    dz : tuple of array-likes (dzdx, dzdy), optional
        Used only if  *kind* ='user'. In this case *dz* must be provided as
        (dzdx, dzdy) where dzdx, dzdy are arrays of the same shape as *z* and
        are the interpolant first derivatives at the *triangulation* points.
    """

    def __init__(self, triangulation, z, kind='min_E', trifinder=None,
                 dz=None):
        super().__init__(triangulation, z, trifinder)
        _api.check_in_list(['user', 'geom', 'min_E'], kind=kind)
        if kind == 'user':
            if dz is None:
                raise ValueError("For a CubicTriInterpolator with "
                                 "*kind*='user', a valid *dz* "
                                 "argument is expected.")
            dzdx, dzdy = (np.asarray(d, dtype=float) for d in dz)
            self._native_kind = (kind, (dzdx, dzdy))
        else:
            self._native_kind = (kind, None)
