"""
The plotting methods of `Axes3D` drawing error bars, stems, flat bars and
areas between curves (matplotlib's ``Axes3D.errorbar``, ``stem``, ``bar``
and ``fill_between``), drawn by pltrs (Rust) with matplotlib's
signatures.
"""

import numpy as np

from pltrs import _api, cbook, colors as mcolors, lines as mlines
from pltrs._native import mplot3d as _mplot3d
from pltrs._preprocess import _preprocess_data
from pltrs.axes import Axes
from pltrs.axes._plotting._helpers import axis_values, line_kwargs, wrap, wrap_all
from pltrs.axes._plotting.errorbar import _errorevery_to_mask
from pltrs.container import ErrorbarContainer, StemContainer

from . import art3d


def _error_bounds(err, n):
    """The errors below and above each of *n* points: one size for all,
    one per point, or ``(2, n)`` rows of lower and upper sizes."""
    if not np.iterable(err):
        err = [err] * n
    err = np.ma.filled(np.ma.asarray(np.atleast_1d(err), dtype=float), np.nan)
    lower, upper = err if err.ndim == 2 else (err, err)
    return (np.ascontiguousarray(np.broadcast_to(lower, n), dtype=float),
            np.ascontiguousarray(np.broadcast_to(upper, n), dtype=float))


def _flags(lims, n):
    """Limit flags, one per point."""
    return np.broadcast_to(lims, n).astype(bool).tolist()


class ChartsMixin:
    """`errorbar`, `stem`, `bar` and `fill_between`."""

    @_preprocess_data(replace_names=["x", "y", "z", "xerr", "yerr", "zerr"])
    def errorbar(self, x, y, z, zerr=None, yerr=None, xerr=None, fmt='',
                 barsabove=False, errorevery=1, ecolor=None, elinewidth=None,
                 capsize=None, capthick=None, xlolims=False, xuplims=False,
                 ylolims=False, yuplims=False, zlolims=False, zuplims=False,
                 axlim_clip=False, **kwargs):
        """
        Plot lines and/or markers with errorbars around them.

        *x*/*y*/*z* define the data locations, and *xerr*/*yerr*/*zerr* define
        the errorbar sizes. By default, this draws the data markers/lines as
        well the errorbars. Use fmt='none' to draw errorbars only.

        Parameters
        ----------
        x, y, z : float or array-like
            The data positions.

        xerr, yerr, zerr : float or array-like, shape (N,) or (2, N), optional
            The errorbar sizes:

            - scalar: Symmetric +/- values for all data points.
            - shape(N,): Symmetric +/-values for each data point.
            - shape(2, N): Separate - and + values for each bar. First row
              contains the lower errors, the second row contains the upper
              errors.
            - *None*: No errorbar.

            Note that all error arrays should have *positive* values.

        fmt : str, default: ''
            The format for the data points / data lines. See `.plot` for
            details.

            Use 'none' (case-insensitive) to plot errorbars without any data
            markers.

        ecolor : :mpltype:`color`, optional
            The color of the errorbar lines. If not given, use the color of the
            line connecting the markers.

        elinewidth : float, optional
            The linewidth of the errorbar lines. If not given, the linewidth of
            the current style is used.

        capsize : float, default: :rc:`errorbar.capsize`
            The length of the error bar caps in points.

        capthick : float, default: None
            An alias to the keyword argument *markeredgewidth* (a.k.a. *mew*).
            This setting is a more sensible name for the property that
            controls the thickness of the error bar cap in points. For
            backwards compatibility, if *mew* or *markeredgewidth* are given,
            then they will over-ride *capthick*. This may change in future
            releases.

        barsabove : bool, default: False
            If True, will plot the errorbars above the plot
            symbols. Default is below.

        xlolims, ylolims, zlolims : bool, default: False
            These arguments can be used to indicate that a value gives only
            lower limits. In that case a caret symbol is used to indicate
            this. *lims*-arguments may be scalars, or array-likes of the same
            length as the errors. To use limits with inverted axes,
            `~.set_xlim`, `~.set_ylim`, or `~.set_zlim` must be
            called before `errorbar`. Note the tricky parameter names: setting
            e.g. *ylolims* to True means that the y-value is a *lower* limit of
            the True value, so, only an *upward*-pointing arrow will be drawn!

        xuplims, yuplims, zuplims : bool, default: False
            Same as above, but for controlling the upper limits.

        errorevery : int or (int, int), default: 1
            draws error bars on a subset of the data. *errorevery* =N draws
            error bars on the points (x[::N], y[::N], z[::N]).
            *errorevery* =(start, N) draws error bars on the points
            (x[start::N], y[start::N], z[start::N]). e.g. *errorevery* =(6, 3)
            adds error bars to the data at (x[6], x[9], x[12], x[15], ...).
            Used to avoid overlapping error bars when two series share x-axis
            values.

        axlim_clip : bool, default: False
            Whether to hide error bars that are outside the axes limits.

            .. versionadded:: 3.10

        Returns
        -------
        errlines : list
            List of `~mpl_toolkits.mplot3d.art3d.Line3DCollection` instances
            each containing an errorbar line.
        caplines : list
            List of `~mpl_toolkits.mplot3d.art3d.Line3D` instances each
            containing a capline object.
        limmarks : list
            List of `~mpl_toolkits.mplot3d.art3d.Line3D` instances each
            containing a marker with an upper or lower limit.

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER

        **kwargs
            All other keyword arguments for styling errorbar lines are passed
            `~mpl_toolkits.mplot3d.art3d.Line3DCollection`.

        Examples
        --------
        .. plot:: gallery/mplot3d/errorbar3d.py
        """
        kwargs = cbook.normalize_kwargs(kwargs, mlines.Line2D)
        # Drop anything that comes in as None to use the default instead.
        kwargs = {k: v for k, v in kwargs.items() if v is not None}
        x = x if np.iterable(x) else [x]
        y = y if np.iterable(y) else [y]
        z = z if np.iterable(z) else [z]
        if not len(x) == len(y) == len(z):
            raise ValueError("'x', 'y', and 'z' must have the same size")
        n = len(x)
        everymask = _errorevery_to_mask(np.asarray(x, dtype=object), errorevery)
        errors = tuple(None if err is None else _error_bounds(err, n)
                       for err in (xerr, yerr, zerr))
        lims = ((_flags(xlolims, n), _flags(ylolims, n), _flags(zlolims, n)),
                (_flags(xuplims, n), _flags(yuplims, n), _flags(zuplims, n)))
        data_line, errlines, caplines = _mplot3d.errorbar(
            self._native, (axis_values(x), axis_values(y), axis_values(z)),
            errors, (fmt, line_kwargs(kwargs)),
            (ecolor, elinewidth, capsize, capthick, bool(barsabove)), lims,
            (everymask.tolist(), bool(axlim_clip)))
        data_line = wrap(self, data_line, art3d.Line3D)
        errlines = wrap_all(self, errlines, art3d.Line3DCollection)
        caplines = wrap_all(self, caplines, art3d.Line3D)
        self.containers.append(ErrorbarContainer(
            (data_line, tuple(caplines), tuple(errlines)),
            has_xerr=(xerr is not None or yerr is not None),
            has_yerr=(zerr is not None), label=kwargs.get('label')))
        return errlines, caplines, []

    @_preprocess_data()
    def stem(self, x, y, z, *, linefmt='C0-', markerfmt='C0o', basefmt='C3-',
             bottom=0, label=None, orientation='z', axlim_clip=False):
        """
        Create a 3D stem plot.

        A stem plot draws lines perpendicular to a baseline, and places markers
        at the heads. By default, the baseline is defined by *x* and *y*, and
        stems are drawn vertically from *bottom* to *z*.

        Parameters
        ----------
        x, y, z : array-like
            The positions of the heads of the stems. The stems are drawn along
            the *orientation*-direction from the baseline at *bottom* (in the
            *orientation*-coordinate) to the heads. By default, the *x* and *y*
            positions are used for the baseline and *z* for the head position,
            but this can be changed by *orientation*.

        linefmt : str, default: 'C0-'
            A string defining the properties of the vertical lines. Usually,
            this will be a color or a color and a linestyle:

            =========  =============
            Character  Line Style
            =========  =============
            ``'-'``    solid line
            ``'--'``   dashed line
            ``'-.'``   dash-dot line
            ``':'``    dotted line
            =========  =============

            Note: While it is technically possible to specify valid formats
            other than color or color and linestyle (e.g. 'rx' or '-.'), this
            is beyond the intention of the method and will most likely not
            result in a reasonable plot.

        markerfmt : str, default: 'C0o'
            A string defining the properties of the markers at the stem heads.

        basefmt : str, default: 'C3-'
            A format string defining the properties of the baseline.

        bottom : float, default: 0
            The position of the baseline, in *orientation*-coordinates.

        label : str, optional
            The label to use for the stems in legends.

        orientation : {'x', 'y', 'z'}, default: 'z'
            The direction along which stems are drawn.

        axlim_clip : bool, default: False
            Whether to hide stems that are outside the axes limits.

            .. versionadded:: 3.10

        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER

        Returns
        -------
        `.StemContainer`
            The container may be treated like a tuple
            (*markerline*, *stemlines*, *baseline*)

        Examples
        --------
        .. plot:: gallery/mplot3d/stem3d_demo.py
        """
        _api.check_in_list(['x', 'y', 'z'], orientation=orientation)
        markerline, stemlines, baseline = _mplot3d.stem(
            self._native, (axis_values(x), axis_values(y), axis_values(z)),
            (linefmt, markerfmt, basefmt), float(bottom), orientation,
            bool(axlim_clip))
        container = StemContainer(
            (wrap(self, markerline, art3d.Line3D),
             wrap(self, stemlines, art3d.Line3DCollection),
             wrap(self, baseline, art3d.Line3D)), label=label)
        self.add_container(container)
        return container

    stem3D = stem

    @_preprocess_data()
    def bar(self, left, height, zs=0, zdir='z', *args,
            axlim_clip=False, **kwargs):
        """
        Add 2D bar(s).

        Parameters
        ----------
        left : 1D array-like
            The x coordinates of the left sides of the bars.
        height : 1D array-like
            The height of the bars.
        zs : float or 1D array-like, default: 0
            Z coordinate of bars; if a single value is specified, it will be
            used for all bars.
        zdir : {'x', 'y', 'z'}, default: 'z'
            When plotting 2D data, the direction to use as z ('x', 'y' or 'z').
        axlim_clip : bool, default: False
            Whether to hide bars with points outside the axes view limits.

            .. versionadded:: 3.10
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        **kwargs
            Other keyword arguments are forwarded to
            `matplotlib.axes.Axes.bar`.

        Returns
        -------
        mpl_toolkits.mplot3d.art3d.Patch3DCollection
        """
        had_data = self.has_data()
        kept = _mplot3d.plane_data_lim(self._native)
        patches = Axes.bar(self, left, height, *args, **kwargs)
        # 2D `bar` extends the data limits through `update_datalim`, which
        # does nothing on 3D Axes.
        _mplot3d.restore_plane_data_lim(self._native, kept)
        zs = np.broadcast_to(zs, len(left), subok=True)
        verts = []
        verts_zs = []
        for p, z in zip(patches, zs):
            vs = art3d._get_patch_verts(p)
            verts += vs.tolist()
            verts_zs += [z] * len(vs)
            art3d.patch_2d_to_3d(p, z, zdir, axlim_clip)
            if 'alpha' in kwargs:
                p.set_alpha(kwargs['alpha'])
        xs, ys = zip(*verts) if verts else ([], [])
        xs, ys, verts_zs = art3d.juggle_axes(xs, ys, verts_zs, zdir)
        self.auto_scale_xyz(xs, ys, verts_zs, had_data)
        return patches

    def fill_between(self, x1, y1, z1, x2, y2, z2, *,
                     where=None, mode='auto', facecolors=None, shade=None,
                     axlim_clip=False, **kwargs):
        """
        Fill the area between two 3D curves.

        The curves are defined by the points (*x1*, *y1*, *z1*) and
        (*x2*, *y2*, *z2*). This creates one or multiple quadrangle
        polygons that are filled. All points must be the same length N, or a
        single value to be used for all points.

        Parameters
        ----------
        x1, y1, z1 : float or 1D array-like
            x, y, and z  coordinates of vertices for 1st line.

        x2, y2, z2 : float or 1D array-like
            x, y, and z coordinates of vertices for 2nd line.

        where : array of bool (length N), optional
            Define *where* to exclude some regions from being filled. The
            filled regions are defined by the coordinates ``pts[where]``,
            for all x, y, and z pts. More precisely, fill between ``pts[i]``
            and ``pts[i+1]`` if ``where[i] and where[i+1]``. Note that this
            definition implies that an isolated *True* value between two
            *False* values in *where* will not result in filling. Both sides of
            the *True* position remain unfilled due to the adjacent *False*
            values.

        mode : {'quad', 'polygon', 'auto'}, default: 'auto'
            The fill mode. One of:

            - 'quad':  A separate quadrilateral polygon is created for each
              pair of subsequent points in the two lines.
            - 'polygon': The two lines are connected to form a single polygon.
              This is faster and can render more cleanly for simple shapes
              (e.g. for filling between two lines that lie within a plane).
            - 'auto': If the points all lie on the same 3D plane, 'polygon' is
              used. Otherwise, 'quad' is used.

        facecolors : :mpltype:`color` or list of :mpltype:`color`, optional
            Colors of each individual patch, or a single color to be used for
            all patches. If not given, the next color from the patch color
            cycle is used.

        shade : bool, default: None
            Whether to shade the facecolors. If *None*, then defaults to *True*
            for 'quad' mode and *False* for 'polygon' mode.

        axlim_clip : bool, default: False
            Whether to hide data that is outside the axes view limits.

            .. versionadded:: 3.10

        **kwargs
            All other keyword arguments are passed on to `.Poly3DCollection`.

        Returns
        -------
        `.Poly3DCollection`
            A `.Poly3DCollection` containing the plotted polygons.

        """
        _api.check_in_list(['auto', 'quad', 'polygon'], mode=mode)
        x1, y1, z1, x2, y2, z2 = cbook._broadcast_with_masks(x1, y1, z1, x2, y2, z2)
        if where is not None:
            where = np.asarray(where, dtype=bool)
            if where.size != x1.size:
                raise ValueError(f"where size ({where.size}) does not match "
                                 f"size ({x1.size})")
            where = np.broadcast_to(where, x1.shape).ravel().tolist()
        faces = (None if facecolors is None
                 else np.ascontiguousarray(mcolors.to_rgba_array(facecolors)))
        # Shading applies to the face colours, and to edge colours given as
        # `edgecolors` (matplotlib's `Poly3DCollection` keyword).
        edges = kwargs.pop('edgecolors', None)
        edges = (None if edges is None
                 else np.ascontiguousarray(mcolors.to_rgba_array(edges)))
        light = kwargs.pop('lightsource', None)
        light = (None if light is None
                 else (float(light.azdeg), float(light.altdeg)))
        native = _mplot3d.fill_between(
            self._native,
            tuple(tuple(axis_values(c) for c in curve)
                  for curve in ((x1, y1, z1), (x2, y2, z2))),
            (where, mode), (faces, edges),
            (None if shade is None else bool(shade), light), bool(axlim_clip))
        collection = wrap(self, native, art3d.Poly3DCollection)
        collection._internal_update(kwargs)
        return collection
