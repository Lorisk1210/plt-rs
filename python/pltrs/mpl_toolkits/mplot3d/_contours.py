"""
The contours of `Axes3D` (matplotlib's ``Axes3D.contour``, ``contourf``,
``tricontour`` and ``tricontourf``): the levels and the look are those of
2-D contours, and pltrs (Rust) places the contours in 3-D
(``_native.mplot3d.contour``).
"""

import numpy as np

from pltrs import cbook, contour as mcontour
from pltrs._native import mplot3d as _mplot3d
from pltrs._preprocess import _preprocess_data
from pltrs.axes._plotting.contours import _contour_look, _finish, _levels


def _in_3d(cls, placement):
    """The class of a contour set of *cls* placed in 3-D as *placement*
    says: the one matplotlib's ``art3d.collection_2d_to_3d`` makes it
    (``QuadContourSet3D``), but for line contours extended into ribbons,
    which matplotlib leaves of *cls*."""
    from . import art3d
    _, _, stride, _ = placement
    if stride is not None:
        return cls
    return cbook._make_class_factory(art3d.Collection3D, "{}3D")(cls)


def _placement(zdir, offset, stride, axlim_clip):
    """The native placement of contours: along *zdir*, at *offset* if
    given, extended into ribbons every *stride* points if not None."""
    return (zdir, None if offset is None else float(offset),
            None if stride is None else int(stride), bool(axlim_clip))


class ContoursMixin:
    """`contour`, `contourf`, `tricontour` and `tricontourf`, and the
    contour sets of the projection plane placed in 3-D
    (`add_contour_set`, `add_contourf_set`)."""

    def add_contour_set(
            self, cset, extend3d=False, stride=5, zdir='z', offset=None,
            axlim_clip=False):
        """Place the line contours *cset*, drawn on this axes, in 3-D: each
        at its level along *zdir*, or at *offset*, or extended into
        ribbons every *stride* points with *extend3d*."""
        _mplot3d.add_contour_set(
            self._native, cset._contour,
            (zdir, None if offset is None else float(offset), bool(axlim_clip)),
            int(stride) if extend3d else None)

    def add_contourf_set(self, cset, zdir='z', offset=None, *, axlim_clip=False):
        """Place the filled contours *cset*, drawn on this axes, in 3-D at
        the middles of their bands along *zdir*, or at *offset*."""
        self._add_contourf_set(cset, zdir=zdir, offset=offset,
                               axlim_clip=axlim_clip)

    def _add_contourf_set(self, cset, zdir='z', offset=None, axlim_clip=False):
        """
        Returns
        -------
        levels : `numpy.ndarray`
            Levels at which the filled contours are added.
        """
        return np.array(_mplot3d.add_contourf_set(
            self._native, cset._contour,
            (zdir, None if offset is None else float(offset), bool(axlim_clip))))

    def _contour3d(self, filled, args, kwargs, placement):
        """The `.QuadContourSet` of ``contour`` (or, *filled*,
        ``contourf``) of ``X, Y, Z [, levels]`` in *args*."""
        fn = 'contourf' if filled else 'contour'
        z, _mask, x, y, levels = mcontour._contour_args(fn, args, kwargs)
        if x.ndim == 1:
            x, y = np.meshgrid(x, y)
        look = _contour_look(kwargs)
        grids = tuple(np.ascontiguousarray(a, dtype=float) for a in (x, y, z))
        native = _mplot3d.contour(self._native, bool(filled), grids, placement,
                                  _levels(levels), *look[:2])
        return _finish(_in_3d(mcontour.QuadContourSet, placement), native, self,
                       look, kwargs)

    def _tricontour3d(self, filled, args, kwargs, placement):
        """The `.TriContourSet` of ``tricontour`` (or, *filled*,
        ``tricontourf``)."""
        from pltrs.tri import Triangulation, TriContourSet
        tri, args, kwargs = Triangulation.get_from_args_and_kwargs(*args, **kwargs)
        if 'Z' in kwargs:
            z = kwargs.pop('Z')
        else:
            z, *args = args
        levels = kwargs.pop("levels", None)
        if levels is None and args:
            levels = args[0]
        look = _contour_look(kwargs)
        z = np.ma.filled(np.ma.asarray(z, dtype=float), np.nan).ravel()
        native = _mplot3d.tricontour(self._native, bool(filled), (tri._native, z),
                                     placement, _levels(levels), *look[:2])
        return _finish(_in_3d(TriContourSet, placement), native, self, look, kwargs)

    @_preprocess_data()
    def contour(self, X, Y, Z, *args,
                extend3d=False, stride=5, zdir='z', offset=None, axlim_clip=False,
                **kwargs):
        """
        Create a 3D contour plot.

        Parameters
        ----------
        X, Y, Z : array-like,
            Input data. See `.Axes.contour` for supported data shapes.
        extend3d : bool, default: False
            Whether to extend contour in 3D.
        stride : int, default: 5
            Step size for extending contour.
        zdir : {'x', 'y', 'z'}, default: 'z'
            The direction to use.
        offset : float, optional
            If specified, plot a projection of the contour lines at this
            position in a plane normal to *zdir*.
        axlim_clip : bool, default: False
            Whether to hide lines with a vertex outside the axes view limits.
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        *args, **kwargs
            Other arguments are forwarded to `matplotlib.axes.Axes.contour`.

        Returns
        -------
        matplotlib.contour.QuadContourSet
        """
        placement = _placement(zdir, offset, stride if extend3d else None,
                               axlim_clip)
        return self._contour3d(False, (X, Y, Z, *args), kwargs, placement)

    contour3D = contour

    @_preprocess_data()
    def contourf(self, X, Y, Z, *args,
                 zdir='z', offset=None, axlim_clip=False, **kwargs):
        """
        Create a 3D filled contour plot.

        Parameters
        ----------
        X, Y, Z : array-like
            Input data. See `.Axes.contourf` for supported data shapes.
        zdir : {'x', 'y', 'z'}, default: 'z'
            The direction to use.
        offset : float, optional
            If specified, plot a projection of the contour lines at this
            position in a plane normal to *zdir*.
        axlim_clip : bool, default: False
            Whether to hide lines with a vertex outside the axes view limits.
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        *args, **kwargs
            Other arguments are forwarded to `matplotlib.axes.Axes.contourf`.

        Returns
        -------
        matplotlib.contour.QuadContourSet
        """
        placement = _placement(zdir, offset, None, axlim_clip)
        return self._contour3d(True, (X, Y, Z, *args), kwargs, placement)

    contourf3D = contourf

    @_preprocess_data()
    def tricontour(self, *args,
                   extend3d=False, stride=5, zdir='z', offset=None, axlim_clip=False,
                   **kwargs):
        """
        Create a 3D contour plot.

        Parameters
        ----------
        X, Y, Z : array-like
            Input data. See `.Axes.tricontour` for supported data shapes.
        extend3d : bool, default: False
            Whether to extend contour in 3D.
        stride : int, default: 5
            Step size for extending contour.
        zdir : {'x', 'y', 'z'}, default: 'z'
            The direction to use.
        offset : float, optional
            If specified, plot a projection of the contour lines at this
            position in a plane normal to *zdir*.
        axlim_clip : bool, default: False
            Whether to hide lines with a vertex outside the axes view limits.
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        *args, **kwargs
            Other arguments are forwarded to `matplotlib.axes.Axes.tricontour`.

        Returns
        -------
        matplotlib.tri._tricontour.TriContourSet
        """
        placement = _placement(zdir, offset, stride if extend3d else None,
                               axlim_clip)
        return self._tricontour3d(False, args, kwargs, placement)

    @_preprocess_data()
    def tricontourf(self, *args, zdir='z', offset=None, axlim_clip=False, **kwargs):
        """
        Create a 3D filled contour plot.

        Parameters
        ----------
        X, Y, Z : array-like
            Input data. See `.Axes.tricontourf` for supported data shapes.
        zdir : {'x', 'y', 'z'}, default: 'z'
            The direction to use.
        offset : float, optional
            If specified, plot a projection of the contour lines at this
            position in a plane normal to zdir.
        axlim_clip : bool, default: False
            Whether to hide lines with a vertex outside the axes view limits.
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        *args, **kwargs
            Other arguments are forwarded to
            `matplotlib.axes.Axes.tricontourf`.

        Returns
        -------
        matplotlib.tri._tricontour.TriContourSet
        """
        placement = _placement(zdir, offset, None, axlim_clip)
        return self._tricontour3d(True, args, kwargs, placement)
