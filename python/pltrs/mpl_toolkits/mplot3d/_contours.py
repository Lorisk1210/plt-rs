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

from pltrs import _lazy_imports
from . import _properties

_draw_problems = _lazy_imports.module(globals(), "._draw_problems")


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


def _plane_dir(zdir):
    """The axis normal to the planes of the contours of *zdir*:
    matplotlib turns the data for ``'x'`` and ``'y'`` (``rotate_axes``) and
    turns the contours back (``'-x'``, ``'-y'``), any other string leaving
    them along z."""
    return zdir if zdir in ('x', 'y') else 'z'


def _placement(zdir, offset, stride, axlim_clip):
    """The native placement of contours: along *zdir*, at *offset* if
    given, extended into ribbons every *stride* points if not None."""
    return (_plane_dir(zdir), None if offset is None else float(offset),
            None if stride is None else int(stride), bool(axlim_clip))


def _left_2d(cset, cls):
    """Leave the contour set *cset* in its 3-D axes as the 2-D set of *cls*
    matplotlib's failed placement leaves there, whose draws fail on it."""
    cset.__class__ = cls
    _draw_problems.record(cset, _draw_problems.missing_projection(cset))


def _lift_contour_set(cset, zs, zdir, axlim_clip):
    """
    Place the paths of *cset*, a contour set of a 3-D axes, at *zs* (one
    position for all, or one per path) along *zdir* (``'-'`` and the
    direction), as given (matplotlib's ``collection_2d_to_3d``): positions
    that do not broadcast to the paths raise, positions matplotlib's draws
    fail to project are recorded.
    """
    from .art3d import Collection3D
    positions, verts = _lifted_verts(cset, zs, zdir)
    if not isinstance(cset, Collection3D):
        cset.__class__ = cbook._make_class_factory(Collection3D, "{}3D")(type(cset))
    cset._axlim_clip = axlim_clip
    problem = _draw_problems.paths_problem(cset, verts)
    _mplot3d.lift_contour_paths(
        cset.axes._native, cset._contour,
        (_plane_dir(zdir.removeprefix('-')), bool(axlim_clip)),
        [np.nan if problem is not None else float(z) for z in positions])
    _draw_problems.record(cset, problem)


def _in_units(axes, x, y, kwargs):
    """The grids *x* and *y* converted with the units of the x and y axes
    of *axes*, which take their units first (and those *kwargs* gives as
    ``xunits``, ``yunits`` and ``zunits``, popped), as matplotlib's 2-D
    contour converts them (``_process_unit_info``)."""
    datasets = [(axes.xaxis, x), (axes.yaxis, y)]
    for axis, data in datasets:
        if data is not None and not axis.have_units():
            axis.update_units(data)
    for name in axes._axis_names:
        axis = getattr(axes, f"{name}axis")
        units = kwargs.pop(f"{name}units", axis.units)
        if units != axis.units and units is not None:
            axis.set_units(units)
            for data in (data for given, data in datasets if given is axis):
                if data is not None:
                    axis.update_units(data)
    return [data if data is None else axis.convert_units(data)
            for axis, data in datasets]


def _lifted_verts(cset, zs, zdir):
    """The positions *zs* broadcast to the paths of *cset* and its 3-D
    vertices, each path at its position along *zdir* (matplotlib's
    ``collection_2d_to_3d``)."""
    from .art3d import juggle_axes
    paths = cset.get_paths()
    positions = np.broadcast_to(zs, len(paths))
    return positions, [np.column_stack(juggle_axes(*np.column_stack(
        [p.vertices, np.broadcast_to(z, len(p.vertices))]).T, zdir))
        for p, z in zip(paths, positions)]


def _record_lift_problem(cset, zdir):
    """Record the problem matplotlib's draws meet projecting the paths of
    *cset*, placed by pltrs (Rust) at its levels (the middles of its bands,
    for filled contours) along *zdir* (data invalid on the axis scales,
    which matplotlib's mask fails to broadcast)."""
    levels = _band_middles(cset) if cset.filled else cset.levels
    _draw_problems.record(cset, _draw_problems.paths_problem(
        cset, _lifted_verts(cset, levels, '-' + _plane_dir(zdir))[1]))


def _band_middles(cset):
    """The positions of the bands of the filled contour set *cset*: their
    middles, an extended end half a band beyond the outer level (matplotlib's
    ``_add_contourf_set``)."""
    levels = cset.levels
    midpoints = levels[:-1] + np.diff(levels) / 2
    if cset.extend in ('both', 'min'):
        midpoints = np.insert(midpoints, 0, levels[0] - np.diff(levels[:2]) / 2)
    if cset.extend in ('both', 'max'):
        midpoints = np.append(midpoints, levels[-1] + np.diff(levels[-2:]) / 2)
    return midpoints


def _placed(make, cls, zdir, offset, stride, axlim_clip):
    """
    The contour set *make* makes at a placement (`_placement`), of the
    class *cls* in 2-D. matplotlib makes the 2-D set, then places it in
    3-D (``'-' + zdir``) at its levels, or at *offset* broadcast to the
    set's paths, unless *stride* extends it into ribbons: a *zdir* of no
    string, or an offset that does not broadcast, raises there, leaving the
    2-D set in the axes, whose draws then fail on it.
    """
    if not isinstance(zdir, str):
        _left_2d(make(_placement('z', None, None, axlim_clip)), cls)
        raise TypeError(f'can only concatenate str (not "{type(zdir).__name__}") '
                        f'to str')
    if stride is not None or isinstance(offset, (int, float)):
        return make(_placement(zdir, offset, stride, axlim_clip))
    if offset is None:
        cset = make(_placement(zdir, offset, stride, axlim_clip))
        _record_lift_problem(cset, zdir)
        return cset
    cset = make(_placement(zdir, None, None, axlim_clip))
    try:
        _lift_contour_set(cset, offset, '-' + zdir, axlim_clip)
    except ValueError:
        _left_2d(cset, cls)
        raise
    return cset


class ContoursMixin:
    """`contour`, `contourf`, `tricontour`, `tricontourf` and `clabel`, and the
    contour sets of the projection plane placed in 3-D
    (`add_contour_set`, `add_contourf_set`)."""

    def add_contour_set(
            self, cset, extend3d=False, stride=5, zdir='z', offset=None,
            axlim_clip=False):
        zdir = '-' + zdir
        if extend3d:
            _mplot3d.add_contour_set(self._native, cset._contour,
                                     ('z', None, bool(axlim_clip)), int(stride))
        else:
            _lift_contour_set(cset, offset if offset is not None else cset.levels,
                              zdir, axlim_clip)

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
        zdir = '-' + zdir
        midpoints = _band_middles(cset)
        _lift_contour_set(cset, offset if offset is not None else midpoints,
                          zdir, axlim_clip)
        return midpoints

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
        if 2 < len(args) <= 4:
            x, y, *rest = args
            args = (*_in_units(self, x, y, kwargs), *rest)
        z, _mask, x, y, levels = mcontour._contour_args(fn, args, kwargs)
        if x.ndim == 1:
            x, y = np.meshgrid(x, y)
        look = _contour_look(self, kwargs)
        grids = tuple(np.ascontiguousarray(a, dtype=float) for a in (x, y, z))
        _properties.check_known(mcontour.QuadContourSet, kwargs)
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
        _properties.check_known(TriContourSet, kwargs)
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
