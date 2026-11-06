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

from . import _draw_problems


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


def _placed(make, cls, zdir, offset, stride, axlim_clip):
    """
    The contour set *make* makes at a placement (`_placement`), of the
    class *cls* in 2-D. matplotlib places the set its 2-D method made in
    3-D at *offset* broadcast to the set's paths, unless *stride* extends
    it into ribbons: an offset that does not broadcast raises there,
    leaving the 2-D set in the axes, whose draws then fail on it.
    """
    if stride is not None or offset is None or np.ndim(offset) == 0:
        return make(_placement(zdir, offset, stride, axlim_clip))
    cset = make(_placement(zdir, None, None, axlim_clip))
    try:
        positions = np.broadcast_to(offset, len(cset.get_paths()))
    except ValueError:
        cset.__class__ = cls
        _draw_problems.record(cset, _draw_problems.missing_projection(cset))
        raise
    cset.remove()
    if len(np.unique(positions)) > 1:
        raise NotImplementedError("pltrs: contour(offset=...) with a "
                                  "position per contour")
    return make(_placement(zdir, positions[0] if len(positions) else None,
                           None, axlim_clip))


class ContoursMixin:
    """`contour`, `contourf`, `tricontour`, `tricontourf` and `clabel`, and the
    contour sets of the projection plane placed in 3-D
    (`add_contour_set`, `add_contourf_set`)."""

    def add_contour_set(
            self, cset, extend3d=False, stride=5, zdir='z', offset=None,
            axlim_clip=False):
        _mplot3d.add_contour_set(
            self._native, cset._contour,
            (zdir, None if offset is None else float(offset), bool(axlim_clip)),
            int(stride) if extend3d else None)

    def add_contourf_set(self, cset, zdir='z', offset=None, *, axlim_clip=False):
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
        zdir = placement[0]
        if zdir != 'z':
            # matplotlib contours the arrays turned for *zdir*, which its
            # checks see (pltrs (Rust) turns the contours instead).
            from .art3d import rotate_axes
            mcontour._contour_args(fn, (*rotate_axes(*args[:3], zdir), *args[3:]),
                                   dict(kwargs))
        z, _mask, x, y, levels = mcontour._contour_args(fn, args, kwargs)
        if x.ndim == 1:
            x, y = np.meshgrid(x, y)
        look = _contour_look(self, kwargs)
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
        look = _contour_look(self, kwargs)
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

            .. versionadded:: 3.10
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER

        *args, **kwargs
            Other arguments are forwarded to `matplotlib.axes.Axes.contour`.

        Returns
        -------
        matplotlib.contour.QuadContourSet
        """
        return _placed(
            lambda placement: self._contour3d(False, (X, Y, Z, *args),
                                              dict(kwargs), placement),
            mcontour.QuadContourSet, zdir, offset, stride if extend3d else None,
            axlim_clip)

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

            .. versionadded:: 3.10
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        *args, **kwargs
            Other arguments are forwarded to `matplotlib.axes.Axes.contourf`.

        Returns
        -------
        matplotlib.contour.QuadContourSet
        """
        return _placed(
            lambda placement: self._contour3d(True, (X, Y, Z, *args),
                                              dict(kwargs), placement),
            mcontour.QuadContourSet, zdir, offset, None, axlim_clip)

    contourf3D = contourf

    def clabel(self, *args, **kwargs):
        """Currently not implemented for 3D Axes, and returns *None*."""
        return None

    @_preprocess_data()
    def tricontour(self, *args,
                   extend3d=False, stride=5, zdir='z', offset=None, axlim_clip=False,
                   **kwargs):
        """
        Create a 3D contour plot.

        .. note::
            This method currently produces incorrect output due to a
            longstanding bug in 3D PolyCollection rendering.

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

            .. versionadded:: 3.10
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        *args, **kwargs
            Other arguments are forwarded to `matplotlib.axes.Axes.tricontour`.

        Returns
        -------
        matplotlib.tri._tricontour.TriContourSet
        """
        from pltrs.tri import TriContourSet
        return _placed(
            lambda placement: self._tricontour3d(False, args, dict(kwargs),
                                                 placement),
            TriContourSet, zdir, offset, stride if extend3d else None, axlim_clip)

    @_preprocess_data()
    def tricontourf(self, *args, zdir='z', offset=None, axlim_clip=False, **kwargs):
        """
        Create a 3D filled contour plot.

        .. note::
            This method currently produces incorrect output due to a
            longstanding bug in 3D PolyCollection rendering.

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

            .. versionadded:: 3.10
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        *args, **kwargs
            Other arguments are forwarded to
            `matplotlib.axes.Axes.tricontourf`.

        Returns
        -------
        matplotlib.tri._tricontour.TriContourSet
        """
        from pltrs.tri import TriContourSet
        return _placed(
            lambda placement: self._tricontour3d(True, args, dict(kwargs),
                                                 placement),
            TriContourSet, zdir, offset, None, axlim_clip)
