"""
Module containing 3D artist code and functions to convert 2D
artists into 3D versions which can be added to an Axes3D (matplotlib's
``mpl_toolkits.mplot3d.art3d``).

A 3-D artist is a 2-D artist of the projection plane of its `.Axes3D`
plus a 3-D geometry, which pltrs (Rust) keeps and projects at every draw:
the classes here read and replace that geometry.
"""

from numbers import Real

import numpy as np

from pltrs import (_api, _program_classes, _unusable, _values, cbook, collections as mcoll,
                   colors as mcolors, lines)
from pltrs import rcParams  # noqa: F401  (matplotlib's art3d has it)
from pltrs import _docstring, _lazy_imports, _own_state, colorizer as mcolorizer
from pltrs import patches as mpatches
from pltrs import text as mtext
from pltrs._native import artist as _artist, lines as _lines, mplot3d as _mplot3d
from pltrs._native import patches as _patches

from . import _properties

_draw_problems = _lazy_imports.module(globals(), "._draw_problems")
_fields3d = _lazy_imports.module(globals(), "._fields3d")
_program_projection = _lazy_imports.module(globals(), "._program_projection")


def _columns(arrays):
    """The coordinate arrays broadcast together, flattened to floats."""
    return tuple(np.asarray(c, dtype=float).ravel()
                 for c in np.broadcast_arrays(*arrays))


def _three_columns(arrays):
    """The x, y and z coordinate arrays *arrays* broadcast together and
    flattened to floats, or None for coordinates of another number or
    that do not broadcast together."""
    try:
        columns = _columns(arrays)
    except ValueError:
        return None
    return columns if len(columns) == 3 else None


def get_dir_vector(zdir):
    """
    Return a direction vector.

    Parameters
    ----------
    zdir : {'x', 'y', 'z', None, 3-tuple}
        The direction. Possible values are:

        - 'x': equivalent to (1, 0, 0)
        - 'y': equivalent to (0, 1, 0)
        - 'z': equivalent to (0, 0, 1)
        - *None*: equivalent to (0, 0, 0)
        - an iterable (x, y, z) is converted to an array

    Returns
    -------
    x, y, z : array
        The direction vector.
    """
    if cbook._str_equal(zdir, 'x'):
        return np.array((1, 0, 0))
    elif cbook._str_equal(zdir, 'y'):
        return np.array((0, 1, 0))
    elif cbook._str_equal(zdir, 'z'):
        return np.array((0, 0, 1))
    elif zdir is None:
        return np.array((0, 0, 0))
    elif np.iterable(zdir) and len(zdir) == 3:
        return np.array(zdir)
    else:
        raise ValueError("'x', 'y', 'z', None or vector of length 3 expected")


def _all_points_on_plane(xs, ys, zs, atol=1e-8):
    """
    Check if all points are on the same plane. Note that NaN values are
    ignored.

    Parameters
    ----------
    xs, ys, zs : array-like
        The x, y, and z coordinates of the points.
    atol : float, default: 1e-8
        The tolerance for the equality check.
    """
    columns = tuple(np.asarray(c, dtype=float).ravel() for c in (xs, ys, zs))
    if any(np.isinf(c).any() for c in columns):
        # matplotlib's numpy arithmetic, which warns on infinite points.
        return _points_on_plane_with_numpy(*columns, atol)
    return _mplot3d.all_points_on_plane(tuple(c.tolist() for c in columns),
                                        float(atol))


def _points_on_plane_with_numpy(xs, ys, zs, atol):
    """`_all_points_on_plane` as matplotlib computes it."""
    points = np.column_stack([xs, ys, zs])
    points = points[~np.isnan(points).any(axis=1)]
    # Check for the case where we have less than 3 unique points
    points = np.unique(points, axis=0)
    if len(points) <= 3:
        return True
    # Calculate the vectors from the first point to all other points
    vs = (points - points[0])[1:]
    vs = vs / np.linalg.norm(vs, axis=1)[:, np.newaxis]
    # Filter out parallel vectors
    vs = np.unique(vs, axis=0)
    if len(vs) <= 2:
        return True
    # Filter out parallel and antiparallel vectors to the first vector
    cross_norms = np.linalg.norm(np.cross(vs[0], vs[1:]), axis=1)
    zero_cross_norms = np.where(np.isclose(cross_norms, 0, atol=atol))[0] + 1
    vs = np.delete(vs, zero_cross_norms, axis=0)
    if len(vs) <= 2:
        return True
    # Calculate the normal vector from the first three points
    n = np.cross(vs[0], vs[1])
    n = n / np.linalg.norm(n)
    # If the dot product of the normal vector and all other vectors is zero,
    # all points are on the same plane
    dots = np.dot(n, vs.transpose())
    return np.allclose(dots, 0, atol=atol)


def _generate_normals(polygons):
    """
    The normal of each polygon of *polygons* from three of its vertices
    spread around it (matplotlib's numpy arithmetic, which pltrs (Rust)
    shades faces with; run for its warnings on infinite vertices).
    """
    if isinstance(polygons, np.ndarray):
        n = polygons.shape[-2]
        i1, i2, i3 = 0, n//3, 2*n//3
        v1 = polygons[..., i1, :] - polygons[..., i2, :]
        v2 = polygons[..., i2, :] - polygons[..., i3, :]
    else:
        v1 = np.empty((len(polygons), 3))
        v2 = np.empty((len(polygons), 3))
        for poly_i, ps in enumerate(polygons):
            n = len(ps)
            ps = np.asarray(ps)
            i1, i2, i3 = 0, n//3, 2*n//3
            v1[poly_i, :] = ps[i1, :] - ps[i2, :]
            v2[poly_i, :] = ps[i2, :] - ps[i3, :]
    return np.cross(v1, v2)


def juggle_axes(xs, ys, zs, zdir):
    """
    Reorder coordinates so that 2D *xs*, *ys* can be plotted in the plane
    orthogonal to *zdir*. *zdir* is normally 'x', 'y' or 'z'. However, if
    *zdir* starts with a '-' it is interpreted as a compensation for
    `rotate_axes`.
    """
    if zdir == 'x':
        return zs, xs, ys
    elif zdir == 'y':
        return xs, zs, ys
    elif zdir[0] == '-':
        return rotate_axes(xs, ys, zs, zdir)
    else:
        return xs, ys, zs


def rotate_axes(xs, ys, zs, zdir):
    """
    Reorder coordinates so that the axes are rotated with *zdir* along
    the original z axis. Prepending the axis with a '-' does the
    inverse transform, so *zdir* can be 'x', '-x', 'y', '-y', 'z' or '-z'.
    """
    if zdir in ('x', '-y'):
        return ys, zs, xs
    elif zdir in ('-x', 'y'):
        return zs, xs, ys
    else:
        return xs, ys, zs


def _no_projection_error():
    """The error matplotlib raises computing through the projection matrix
    `.Axes3D.M` (or ``invM``) before the first draw sets it: numpy's for a
    product with None."""
    return TypeError("unsupported operand type(s) for *: 'NoneType' and 'float'")


def _projected_depth(artist):
    """Project *artist* through the projection matrix of the last draw of
    its axes (`.Axes3D.M`) and return its depth, as matplotlib's
    ``do_3d_projection`` does."""
    depth = _mplot3d.do_3d_projection(artist.axes._native, artist._native)
    if depth is None:
        raise _no_projection_error()
    artist._projections = artist.__dict__.get("_projections", 0) + 1
    # A numpy float, the minimum of the projected depths, but for the
    # Python floats matplotlib gives without points (NaN) or beyond its
    # cap (1e9).
    return depth if np.isnan(depth) or depth == 1e9 else np.float64(depth)


class _ProjectedPaths:
    """
    What the 3-D collections whose paths are their projection share:
    matplotlib makes new paths at each projection (a draw, or
    ``do_3d_projection``), so `get_paths` gives new objects after one.
    """

    def get_paths(self):
        # docstring inherited
        stamp = (_artist.draw_count(self._native),
                 self.__dict__.get("_projections", 0))
        if self.__dict__.get("_paths_stamp") != stamp:
            _own_state.drop(self, "paths")
            self._paths_stamp = stamp
        return super().get_paths()


def _axes3d(artist):
    """The 3-D axes holding *artist*, or None."""
    axes = artist.axes
    return axes if getattr(axes, 'name', None) == '3d' else None


def _depthshade_minalpha(minalpha):
    """The lowest opacity of depth shading: *minalpha*, or
    :rc:`axes3d.depthshade_minalpha` for None."""
    from pltrs import rcParams
    return float(rcParams['axes3d.depthshade_minalpha']
                 if minalpha is None else minalpha)


def _native_light(lightsource):
    """The light of a `~matplotlib.colors.LightSource` as pltrs (Rust)
    takes it, its azimuth and altitude, None for mplot3d's own (*lightsource*
    None): matplotlib shades with its direction, which an object of no
    light lacks."""
    if lightsource is None:
        return None
    lightsource.direction
    return float(lightsource.azdeg), float(lightsource.altdeg)


def _lifted(artist):
    """The 3-D axes in which *artist* has its 3-D geometry, or None (an
    artist outside any 3-D axes, or a 2-D one of its projection plane)."""
    axes = _axes3d(artist)
    if axes is None or _mplot3d.get_axlim_clip(axes._native,
                                               artist._native) is None:
        return None
    return axes


def _restore_removed(artist):
    """Give *artist*, added back to the 3-D axes it was removed from, the
    3-D geometry it had there, as matplotlib's 3-D artists keep their 3-D
    data."""
    _mplot3d.restore_removed(artist.axes._native, artist._native)


def _restored(artist, pending):
    """Whether *artist*, just added to a 3-D axes, was in one before (it
    keeps no 3-D geometry as *pending*, the attribute keeping it for the
    axes that will add it), and took back the geometry it had there."""
    if pending in artist.__dict__:
        return False
    _restore_removed(artist)
    return True


def _gains_projection(artist):
    """Forget the draw problem recorded for *artist*, an artist of a 3-D
    axes, lacking the 3-D projection the axes' draws ask the collections
    and patches for: the class a 2-D to 3-D conversion gave it has one."""
    if _axes3d(artist) is not None:
        _draw_problems.record(artist, None)


def _lift_added(col):
    """Have the 3-D axes that added *col* as a 2-D collection take it
    again, with the 3-D geometry a conversion has just given it: pltrs
    (Rust) lifts a collection as an axes takes it."""
    axes = _axes3d(col)
    if axes is not None and col._pending_key() in col.__dict__:
        col.remove()
        axes.add_collection(col, autolim=False)


def _faces_3d(verts):
    """
    The (N, 3) float arrays of a sequence of polygons, refused as numpy
    refuses to put points of another dimension into matplotlib's array of
    faces.
    """
    if not isinstance(verts, np.ndarray):
        # matplotlib counts the faces and the points of each first.
        len(verts)
        for vert in verts:
            len(vert)
    faces = [np.asarray(vert, dtype=float) for vert in verts]
    for face in faces:
        if face.ndim == 2 and face.shape[1] != 3:
            raise ValueError(f"could not broadcast input array from shape "
                             f"({len(face)},{face.shape[1]}) into shape "
                             f"({len(face)},3)")
    return [face.reshape(-1, 3) for face in faces]


def _segment_points(segments):
    """The (N, 3) float arrays of the points of the 3-D *segments*, which
    matplotlib projects (their first three coordinates), or None for
    segments without three coordinates or of numbers."""
    try:
        points = [np.asarray(segment, dtype=float).reshape(len(segment), -1)
                  for segment in segments]
    except (TypeError, ValueError):
        return None
    if any(segment.shape[1] < 3 for segment in points):
        return None
    return [segment[:, :3] for segment in points]


def _get_axlim_clip(artist):
    if "_axlim_clip" in artist.__dict__:
        return artist.__dict__["_axlim_clip"]
    axes = _axes3d(artist)
    clip = None if axes is None else _mplot3d.get_axlim_clip(axes._native, artist._native)
    if clip is None:
        raise AttributeError(
            f"'{type(artist).__name__}' object has no attribute '_axlim_clip'",
            name="_axlim_clip", obj=artist)
    return clip


def _set_axlim_clip(artist, axlim_clip):
    artist.__dict__["_axlim_clip"] = axlim_clip
    if "_native" in artist.__dict__ and _lifted(artist) is not None:
        _mplot3d.set_axlim_clip(artist.axes._native, artist._native, bool(axlim_clip))


#: Whether the points of a 3-D artist outside the view limits are hidden
#: (matplotlib's ``_axlim_clip``, on every `art3d` class): what the artist
#: was given, else what pltrs (Rust) holds for one it made; assigned, it
#: is pltrs (Rust)'s too.
_AXLIM_CLIP = property(_get_axlim_clip, _set_axlim_clip)


def _viewlim_mask(xs, ys, zs, axes):
    """The mask of the points outside the view limits of *axes*
    (matplotlib 3.11.2's, under the Matplotlib License)."""
    return np.logical_or.reduce((xs < axes.xy_viewLim.xmin,
                                 xs > axes.xy_viewLim.xmax,
                                 ys < axes.xy_viewLim.ymin,
                                 ys > axes.xy_viewLim.ymax,
                                 zs < axes.zz_viewLim.xmin,
                                 zs > axes.zz_viewLim.xmax))


def _scale_invalid_mask(xs, ys, zs, axes):
    """The mask of the points the scales of *axes* cannot place (at or
    below zero on a log axis; matplotlib 3.11.2's, under the Matplotlib
    License)."""
    return np.logical_or.reduce((
        np.logical_not(axes.xaxis._scale.val_in_range(xs)),
        np.logical_not(axes.yaxis._scale.val_in_range(ys)),
        np.logical_not(axes.zaxis._scale.val_in_range(zs))))


#: The 3-D data of a `Line3D` (matplotlib's ``_verts3d``) and the 3-D
#: positions of the members of a 3-D scatter (``_offsets3d``): tuples of
#: arrays the artists keep as their own (`pltrs._own_state`).
_DATA3D = _own_state.State(
    "data_3d", lambda line: line._read_data_3d(),
    lambda line, verts: line._take_data_3d(verts))
_OFFSETS3D = _own_state.State(
    "offsets3d", lambda collection: collection._read_offsets3d(),
    lambda collection, offsets: collection._take_offsets3d(offsets))


class Line3D(lines.Line2D):
    """
    3D line object.

    .. note:: Use `get_data_3d` to obtain the data associated with the line.
            `~.Line2D.get_data`, `~.Line2D.get_xdata`, and `~.Line2D.get_ydata` return
            the x- and y-coordinates of the projected 2D-line, not the x- and y-data of
            the 3D-line. Similarly, use `set_data_3d` to set the data, not
            `~.Line2D.set_data`, `~.Line2D.set_xdata`, and `~.Line2D.set_ydata`.
    """

    def __init__(self, xs, ys, zs, *args, axlim_clip=False, **kwargs):
        """

        Parameters
        ----------
        xs : array-like
            The x-data to be plotted.
        ys : array-like
            The y-data to be plotted.
        zs : array-like
            The z-data to be plotted.
        *args, **kwargs
            Additional arguments are passed to `~matplotlib.lines.Line2D`.
        """
        super().__init__([], [], *args, **kwargs)
        self.set_data_3d(xs, ys, zs)
        self._axlim_clip = axlim_clip

    _axlim_clip = _AXLIM_CLIP

    def _add_to_axes(self, axes, kind, autolim=True):
        # docstring inherited
        super()._add_to_axes(axes, kind, autolim)
        if _axes3d(self) is not None and not _restored(self, '_pending3d'):
            _mplot3d.line_2d_to_3d(axes._native, self._native,
                                   self.__dict__.pop('_pending3d'),
                                   bool(self._axlim_clip))
            data = self.__dict__.get('_unprojectable')
            if data is None and not all(
                    np.asarray(c).dtype.kind in "biuf" for c in self.get_data_3d()):
                # Data given before the line had axes to project in (dates,
                # say), which matplotlib's draw projects as given.
                data = self.get_data_3d()
            if data is not None:
                _draw_problems.record(self, _draw_problems.line_problem(self, data))
        return self

    @property
    def _verts3d(self):
        """The x, y and z data (matplotlib's attribute): the line's own
        tuple, of its own arrays."""
        return self.get_data_3d()

    @_verts3d.setter
    def _verts3d(self, verts):
        # Assigned, the data are drawn from the next draw on and are the
        # line's own object.
        self.set_data_3d(verts)

    def _drawn_column(self, name, column):
        """
        The x or y numbers drawn (*name* ``"_x"`` or ``"_y"``, *column* 0
        or 1). A line of a 3-D axes holds the projection of its last draw
        (new arrays after each, as matplotlib's draw sets the line's 2-D
        data anew), and until its first draw the 2-D data it was made
        with; what a program assigned since the last draw is read back
        until the next one replaces it.
        """
        draws = _artist.draw_count(self._native)
        values = self.__dict__
        assigned = values.get("_columns_assigned", {}).get(name)
        if assigned is not None and assigned[0] == draws:
            return assigned[1]
        if _lifted(self) is not None:
            if not draws and values.get("_verts3d_as") == "lists":
                # (matplotlib makes the caps of ``errorbar`` without 2-D data.)
                return values.setdefault("_columns_unmade", (np.array([]), np.array([])))[column]
            if values.get("_columns_drawn") != draws:
                # The arrays handed out before the draw are the line's no more.
                values["_columns_drawn"] = draws
                _own_state.drop(self, "xy")
                values.pop("_xy_views", None)
        return (lines.Line2D._y if column else lines.Line2D._x).fget(self)

    def _assign_column(self, name, values):
        self.__dict__.setdefault("_columns_assigned", {})[name] = (
            _artist.draw_count(self._native), values)

    _x = property(lambda self: self._drawn_column("_x", 0),
                  lambda self, values: self._assign_column("_x", values),
                  doc="The x numbers drawn: the projection of the last draw.")
    _y = property(lambda self: self._drawn_column("_y", 1),
                  lambda self, values: self._assign_column("_y", values),
                  doc="The y numbers drawn: the projection of the last draw.")

    def set_3d_properties(self, zs=0, zdir='z', axlim_clip=False):
        """
        Set the *z* position and direction of the line.

        Parameters
        ----------
        zs : float or array of floats
            The location along the *zdir* axis in 3D space to position the
            line.
        zdir : {'x', 'y', 'z'}
            Plane to plot line orthogonal to. Default: 'z'.
            See `.get_dir_vector` for a description of the values.
        axlim_clip : bool, default: False
            Whether to hide lines with an endpoint outside the axes view limits.

            .. versionadded:: 3.10
        """
        xs = self.get_xdata()
        ys = self.get_ydata()
        zs = cbook._to_unmasked_float_array(zs).ravel()
        zs = np.broadcast_to(zs, len(xs))
        self._axlim_clip = axlim_clip
        verts = juggle_axes(xs, ys, zs, zdir)
        self._set_verts3d(_columns(verts))
        _values.remember(self, "data_3d", verts)
        _own_state.drop(self, "data_3d")

    def _set_verts3d(self, columns):
        """Make *columns* the 3-D data: the native line's in a 3-D axes,
        else kept until one adds the line."""
        axes = _axes3d(self)
        if axes is None:
            self._pending3d = columns
        else:
            _mplot3d.line_2d_to_3d(axes._native, self._native, columns,
                                   bool(self._axlim_clip))
        self.stale = True

    def get_data_3d(self):
        """
        Get the current data

        Returns
        -------
        verts3d : length-3 tuple or array-like
            The current data as a tuple or array-like.
        """
        unprojectable = self.__dict__.get('_unprojectable')
        if unprojectable is not None:
            return unprojectable
        # The line's own tuple of its own arrays, as matplotlib's
        # ``_verts3d`` (`pltrs._own_state`): what a program changes in them
        # is projected at the next draw.
        return _own_state.hand_out(self, _DATA3D)

    def _read_data_3d(self):
        """The 3-D data as given, while pltrs (Rust) holds them, else as it
        holds them."""
        axes = _axes3d(self)
        if axes is None:
            native = self._pending3d
        else:
            native = tuple(np.array(c) for c in _mplot3d.line_data(axes._native, self._native))
            # A line a 3-D axes made, in the containers matplotlib's method
            # builds: lists of numbers (the caps of ``errorbar``), or
            # arrays whose z is a read-only view (``stem``'s lines, whose
            # z matplotlib broadcasts).
            container = self.__dict__.get("_verts3d_as")
            if container == "lists":
                native = tuple(list(column) for column in native)
            elif container == "broadcast":
                native[2].flags.writeable = False
        return _values.given(self, "data_3d", native)

    def _take_data_3d(self, verts):
        """Hand pltrs (Rust) the 3-D data handed out, *verts*, which the
        program changed in place."""
        columns = _three_columns(verts)
        if columns is None:
            return
        axes = _axes3d(self)
        if axes is None:
            self._pending3d = columns
        else:
            _mplot3d.set_line_data(axes._native, self._native, columns)

    def set_data_3d(self, *args):
        """
        Set the x, y and z data

        Parameters
        ----------
        x : array-like
            The x-data to be plotted.
        y : array-like
            The y-data to be plotted.
        z : array-like
            The z-data to be plotted.

        Notes
        -----
        Accepts x, y, z arguments or a single array-like (x, y, z)
        """
        if len(args) == 1:
            args = args[0]
        for name, xyz in zip('xyz', args):
            if not np.iterable(xyz):
                raise RuntimeError(f'{name} must be a sequence')
        axes = _axes3d(self)
        problem = (None if axes is None
                   else _draw_problems.line_problem(self, args))
        columns = None if problem is not None else _three_columns(args)
        if columns is None:
            # Data matplotlib's draw fails on: kept as given, the line
            # keeping its 3-D data and its draw raising.
            self._unprojectable = args
        else:
            self.__dict__.pop('_unprojectable', None)
            if axes is None:
                self._pending3d = columns
            else:
                _mplot3d.set_line_data(axes._native, self._native, columns)
            _values.remember(self, "data_3d", args)
        _own_state.drop(self, "data_3d")
        if axes is not None:
            _draw_problems.record(self, problem)
        self.stale = True

    def draw(self, renderer):
        # docstring inherited; matplotlib's projects the line through the
        # view of the last draw of its axes, failing as it does.
        axes = _lifted(self)
        if axes is not None:
            problem = _draw_problems.line_problem(self, self.get_data_3d(), axes.M)
            if problem is not None:
                raise problem
        super().draw(renderer)

    def _set_data(self, x, y):
        # docstring inherited; the 2-D data, which the next draw replaces
        # with the projection of the 3-D data, as matplotlib's draw sets it
        # (through `set_data`, `set_xdata` or `set_ydata`).
        before = self.__dict__.get('_data_set_at')
        self._data_set_at = _artist.draw_count(self._native)
        try:
            super()._set_data(x, y)
        except RuntimeError:
            self._data_set_at = before
            raise

    def _numbers(self):
        # docstring inherited; a line lifted in a 3-D axes keeps the numbers
        # of its projection, but for 2-D data it was given since its last
        # draw: matplotlib measures the line by them until the next draw
        # projects it again. Its draws convert no 2-D data, so data that
        # converts to no numbers leaves the projection.
        if _lifted(self) is None:
            return super()._numbers()
        if self.__dict__.get('_data_set_at') == _artist.draw_count(self._native):
            try:
                return super()._numbers()
            except (TypeError, ValueError):
                pass
        return _lines.get_xdata(self._native), _lines.get_ydata(self._native)

    def recache_always(self):
        # docstring inherited; when the units of its axes change, matplotlib
        # converts the 2-D data a line lifted in a 3-D axes was given (until
        # a draw replaced them with their projection) through them again,
        # which fails on values they do not take, though it draws the 3-D
        # data.
        if (_lifted(self) is not None and "_xorig" in self.__dict__
                and self._drawn_data(_lines.get_xdata) is None):
            self.convert_xunits(self._xorig)
            self.convert_yunits(self._yorig)
        super().recache_always()

    def _drawn_data(self, getter):
        """The projected data *getter* reads off the native line, masked as
        matplotlib's `draw` gives them to the line, once a draw projected
        it and while no 2-D data was set since; else None."""
        axes = _lifted(self)
        if (axes is None or self.__dict__.get('_data_set_at')
                == _artist.draw_count(self._native)):
            return None
        mask = _mplot3d.projected_line_mask(axes._native, self._native)
        return None if mask is None else np.ma.masked_array(getter(self._native), mask)

    def get_xdata(self, orig=True):
        # docstring inherited; a drawn line holds its projection.
        drawn = self._drawn_data(_lines.get_xdata) if orig else None
        return super().get_xdata(orig) if drawn is None else drawn

    def get_ydata(self, orig=True):
        # docstring inherited; a drawn line holds its projection.
        drawn = self._drawn_data(_lines.get_ydata) if orig else None
        return super().get_ydata(orig) if drawn is None else drawn


def line_2d_to_3d(line, zs=0, zdir='z', axlim_clip=False):
    """
    Convert a `.Line2D` to a `.Line3D` object.

    Parameters
    ----------
    zs : float
        The location along the *zdir* axis in 3D space to position the line.
    zdir : {'x', 'y', 'z'}
        Plane to plot line orthogonal to. Default: 'z'.
        See `.get_dir_vector` for a description of the values.
    axlim_clip : bool, default: False
        Whether to hide lines with an endpoint outside the axes view limits.

        .. versionadded:: 3.10
    """
    line.__class__ = Line3D
    line.set_3d_properties(zs, zdir, axlim_clip)


# What a 3-D collection made before an `.Axes3D` adds it holds: its 3-D
# vertices (the loose 2-D collection holds its properties), handed to pltrs
# (Rust) when a 3-D axes adds it (`.Axes3D.add_collection3d`,
# `~.Axes.add_collection`, `~.Axes.add_artist`). The collection classes
# define the public methods again as their own, as matplotlib's do. No
# docstring, nor for the mixins below: ``help`` shows those of the classes'
# bases, as for matplotlib's.
class _Loose3D:

    _axlim_clip = _AXLIM_CLIP
    _zsort = None
    _closed = True
    #: What a loose collection keeps for the axes that will add it: the
    #: codes of its 3-D polygons.
    _codes3d = None

    def do_3d_projection(self):
        # Project the collection in the view of the last draw and return
        # the depth of its nearest point (NaN without points).
        return _projected_depth(self)

    @property
    def _sort_zpos(self):
        # matplotlib's plain field, which programs assign as
        # `set_sort_zpos` sets it (qutip orders its bars so): a loose
        # collection's, or a position of no number, as given, else the
        # axes'.
        if '_sort_zpos' in self.__dict__:
            return self.__dict__['_sort_zpos']
        axes = _axes3d(self)
        return None if axes is None else _mplot3d.sort_zpos(axes._native, self._native)

    @_sort_zpos.setter
    def _sort_zpos(self, val):
        self._place_sort_zpos(val)

    def _place_sort_zpos(self, val):
        """`set_sort_zpos` of the collection classes: a position of no
        number is kept as given (the order stays as it was), and fails the
        draws of polygons, which matplotlib projects it for."""
        axes = _axes3d(self)
        if axes is None:
            self.__dict__['_sort_zpos'] = val
        elif val is None or (np.ndim(val) == 0
                             and np.asarray(val).dtype.kind in 'biuf'):
            self.__dict__.pop('_sort_zpos', None)
            _mplot3d.set_sort_zpos(axes._native, self._native,
                                   None if val is None else float(val))
            if self.__dict__.pop('_sort_zpos_fails', False):
                _draw_problems.record(self, None)
        else:
            self.__dict__['_sort_zpos'] = val
            if isinstance(self, Poly3DCollection):
                self._sort_zpos_fails = True
                _draw_problems.record(self, _draw_problems.sort_zpos_problem(val))
        self.stale = True

    def _add_to_axes(self, axes, kind, autolim=True):
        # docstring inherited
        if getattr(axes, 'name', None) != '3d':
            return super()._add_to_axes(axes, kind, autolim)
        if self._pending_key() in self.__dict__:
            return self._adopt_into(axes, kind == 'collection' and bool(autolim),
                                    False)
        super()._add_to_axes(axes, kind, autolim)
        _restore_removed(self)
        return self

    def _pending_key(self):
        """The attribute keeping the 3-D geometry of a collection no 3-D
        axes holds yet."""
        return '_verts3d_pending'

    def _native_geometry(self):
        """The 3-D geometry handed to pltrs (Rust): the polygons or the
        polylines through the 3-D vertices."""
        verts = self.__dict__.pop('_verts3d_pending')
        if self._zsort is None:
            return ("lines", (verts,))
        lifted = self.__dict__.pop('_lifted_from_2d', False)
        faded = self.__dict__.pop('_edges_faded', False)
        # (Polygons lifted from 2-D paths keep their codes, which close
        # them: matplotlib's ``_closed`` is False for them.)
        return ("polygons", (verts, self._zsort, lifted or bool(self._closed),
                             lifted or faded))

    def _adopt_into(self, axes, autolim, axlim_clip):
        """Join *axes* (a 3-D axes) with the 3-D geometry; return self."""
        from pltrs import _handles
        native = _mplot3d.adopt_collection3d(
            axes._native, self._native, self._native_geometry(), bool(autolim),
            bool(self._axlim_clip or axlim_clip))
        _handles.adopt(self, native, axes.figure)
        pending = self.__dict__
        if pending.get('_codes3d') is not None:
            _mplot3d.set_poly_verts_and_codes(
                axes._native, self._native, pending.pop('_faces3d'),
                [np.asarray(c, np.uint8).tolist() for c in pending.pop('_codes3d')])
        if pending.get('_sort_zpos') is not None:
            self._place_sort_zpos(pending.pop('_sort_zpos'))
        if _program_classes.takes_from_program(type(self), "do_3d_projection"):
            _program_projection.follow(axes, self)
        return self


def _lift_verts_codes(collection, verts_codes):
    """Have pltrs (Rust) place the paths of *collection* at the 3-D
    vertices of *verts_codes* (matplotlib's ``_3dverts_codes``), once a
    3-D axes holds it."""
    axes = _axes3d(collection)
    verts = [np.asarray(verts, dtype=float) for verts, _ in verts_codes]
    if axes is None:
        collection._verts3d_pending = verts
        return
    clip = bool(collection.__dict__.get("_axlim_clip", False))
    try:
        if not verts:
            # (matplotlib's projection joins the depths of the paths.)
            raise ValueError("need at least one array to concatenate")
        try:
            _mplot3d.lift_paths(axes._native, collection._native, verts, clip)
        except ValueError:
            # Other paths than the collection has: matplotlib's projection
            # makes its paths of the vertices and codes at each draw.
            from pltrs._native import collections as _collections
            from pltrs.path import Path
            _collections.set_paths(collection._native, [
                Path(np.zeros((len(part), 2)), codes)
                for part, (_, codes) in zip(verts, verts_codes)])
            _mplot3d.lift_paths(axes._native, collection._native, verts, clip)
    except (TypeError, ValueError) as problem:
        _draw_problems.record(collection, problem)
    else:
        _draw_problems.record(collection, None)


class Collection3D(_ProjectedPaths, _Loose3D, mcoll.Collection):
    """A collection of 3D paths."""

    #: The 3-D vertices and the codes of each path (matplotlib's
    #: attribute, a list of pairs): the collection's own list, which pltrs
    #: (Rust) follows in its vertices when a program changes it in place
    #: or assigns another.
    _3dverts_codes = _properties.Field(
        lambda collection: _fields3d.verts_codes(collection), _lift_verts_codes)

    def _native_geometry(self):
        # docstring inherited
        return ("paths", (self.__dict__.pop('_verts3d_pending'),))

    def do_3d_projection(self):
        """Project the points according to renderer matrix."""
        return super().do_3d_projection()


def collection_2d_to_3d(col, zs=0, zdir='z', axlim_clip=False):
    """Convert a `.Collection` to a `.Collection3D` object."""
    paths = col.get_paths()
    zs = np.broadcast_to(zs, len(paths))
    verts = [
        np.column_stack(juggle_axes(
            *np.column_stack([p.vertices, np.broadcast_to(z, len(p.vertices))]).T,
            zdir))
        for p, z in zip(paths, zs)]
    col.__class__ = cbook._make_class_factory(Collection3D, "{}3D")(type(col))
    _gains_projection(col)
    col._axlim_clip = axlim_clip
    axes = _axes3d(col)
    if axes is None:
        col._verts3d_pending = verts
    else:
        _mplot3d.lift_paths(axes._native, col._native,
                            [np.asarray(v, dtype=float) for v in verts],
                            bool(axlim_clip))


class Line3DCollection(_ProjectedPaths, _Loose3D, mcoll.LineCollection):
    """
    A collection of 3D lines.
    """

    def __init__(self, lines, axlim_clip=False, **kwargs):
        super().__init__([], **kwargs)
        self._axlim_clip = axlim_clip
        self._segments3d = lines

    #: The 3-D segments (matplotlib's attribute): kept as given, or as
    #: matplotlib's method builds them for a collection a 3-D Axes made;
    #: the collection's own object, which pltrs (Rust) follows when a
    #: program changes it in place or assigns another.
    _segments3d = _properties.Field(
        lambda collection: _fields3d.segments3d(collection),
        lambda collection, segments: collection._follow_segments(segments))

    def set_segments(self, segments):
        """
        Set 3D segments.
        """
        self._segments3d = segments
        super().set_segments([])

    def _follow_segments(self, segments):
        """Have pltrs (Rust) project *segments*, the 3-D segments the
        collection keeps, in a 3-D axes (else once one adds the
        collection), but for segments matplotlib's draw fails to project,
        on which the axes' draw raises as it does."""
        axes = _lifted(self)
        if axes is None:
            self._verts3d_pending = segments
            return
        problem = _draw_problems.segments_problem(self, segments)
        points = None if problem is not None else _segment_points(segments)
        if points is not None:
            _mplot3d.set_segments3d(axes._native, self._native, points)
        _draw_problems.record(self, problem)

    def _native_geometry(self):
        # docstring inherited; segments matplotlib fails to project are
        # recorded once the axes holds the collection (`_adopt_into`).
        points = _segment_points(self.__dict__.pop('_verts3d_pending'))
        return ("lines", ([] if points is None else points,))

    def _adopt_into(self, axes, autolim, axlim_clip):
        # docstring inherited
        super()._adopt_into(axes, autolim, axlim_clip)
        _draw_problems.record(self, _draw_problems.segments_problem(
            self, type(self)._segments3d.peek(self)))
        return self

    def do_3d_projection(self):
        """
        Project the points according to renderer matrix.
        """
        axes = _lifted(self)
        segments = self.__dict__.get('_segments3d')
        if axes is not None and segments is not None:
            problem = _draw_problems.segments_problem(self, segments, axes.M)
            if problem is not None:
                raise problem
        return _projected_depth(self)

    __init__.__doc__ = mcoll.LineCollection.__init__.__doc__

    def set_sort_zpos(self, val):
        """Set the position to use for z-sorting."""
        self._place_sort_zpos(val)


def line_collection_2d_to_3d(col, zs=0, zdir='z', axlim_clip=False):
    """Convert a `.LineCollection` to a `.Line3DCollection` object."""
    # (Lists of points as tuples, as matplotlib's ``_paths_to_3d_segments``
    # makes them.)
    segments3d = [list(map(tuple, segment.tolist()))
                  for segment in _lifted_paths(col, zs, zdir, finite=True)]
    col.__class__ = Line3DCollection
    _gains_projection(col)
    col._segments3d = segments3d
    col._axlim_clip = axlim_clip
    _lift_added(col)


def _lifted_paths(col, zs, zdir, finite=False):
    """
    The paths of the 2-D collection *col* placed at *zs* along *zdir*: one
    position for all, one per path, or one per vertex of each path, as
    given; without their non-finite points with *finite* (matplotlib's
    ``_paths_to_3d_segments``).
    """
    paths = col.get_paths()
    if not np.iterable(zs):
        zs = np.broadcast_to(zs, len(paths))
    elif len(zs) != len(paths):
        raise ValueError('Number of z-coordinates does not match paths.')
    lifted = []
    for path, path_zs in zip(paths, zs):
        vertices = path.vertices
        path_zs = np.broadcast_to(path_zs, len(vertices))
        if finite:
            keep = np.isfinite(vertices).all(axis=1)
            vertices, path_zs = vertices[keep], path_zs[keep]
        lifted.append(np.column_stack(juggle_axes(
            vertices[:, 0], vertices[:, 1], path_zs, zdir)))
    return lifted


# What places the members of a 2-D collection at 3-D positions, its offsets
# lifted along an axis (matplotlib's `Patch3DCollection` and
# `Path3DCollection`): pltrs (Rust) projects the positions and shades the
# members with depth.
def _projected_offsets(collection):
    """
    The 3-D positions of the members of *collection* through the
    projection of the last draw of its axes, as matplotlib's
    ``do_3d_projection`` computes them: masked where a position is, where
    a scale cannot place it and, with ``_axlim_clip``, outside the view
    limits. Computed once per projection of the collection, for what the
    projection leaves behind (`_properties.Left`).
    """
    now = _properties.stamp(collection)
    made = collection.__dict__.get("_projected3d")
    if made is not None and made[0] == now:
        return made[1]
    axes = collection.axes
    offsets = collection.__dict__.get("_unprojectable")
    if offsets is None:
        offsets = collection._read_offsets3d()
    mask = False
    for xyz in offsets:
        if np.ma.isMA(xyz):
            mask = mask | xyz.mask
    mask = mask | _scale_invalid_mask(*offsets, axes)
    if collection._axlim_clip:
        mask = mask | _viewlim_mask(*offsets, axes)
    if np.any(mask):
        mask = np.broadcast_to(mask, (len(offsets), *np.shape(offsets[0])))
        offsets = np.ma.array(offsets, mask=mask)
    projected = proj3d._scale_proj_transform_clip(*offsets, axes)[:3]
    collection.__dict__["_projected3d"] = (now, projected)
    return projected


class _Lifted3DOffsets(_Loose3D):

    #: Whether matplotlib's draw of the collection is running (its sizes
    #: and widths are then those sorted by depth): never, as pltrs (Rust)
    #: draws it.
    _in_draw = False

    #: The depths of the members as last projected, in their own order
    #: (matplotlib's attribute; None until the first projection).
    _vzs = _properties.Left(lambda collection: _projected_offsets(collection)[2], None)

    #: The members the colours are taken in: all but the last
    #: (matplotlib's attribute of a collection of patches, which keeps
    #: their order).
    _z_markers_idx = _properties.Left(lambda collection: slice(-1), slice(-1))

    @property
    def _depthshade_minalpha(self):
        """The alpha factor of the deepest members (matplotlib's
        attribute)."""
        pending = self.__dict__.get("_depthshade_pending")
        if pending is not None:
            return _depthshade_minalpha(pending[1])
        held = _fields3d.kept(self)
        if held is None or held[5] is None:
            raise AttributeError(
                f"'{type(self).__name__}' object has no attribute '_depthshade_minalpha'",
                name="_depthshade_minalpha", obj=self)
        return held[5]

    @_depthshade_minalpha.setter
    def _depthshade_minalpha(self, minalpha):
        _Lifted3DOffsets.set_depthshade(self, self.get_depthshade(), minalpha)

    _depthshade = property(
        lambda self: self.get_depthshade(),
        lambda self, depthshade: _Lifted3DOffsets.set_depthshade(
            self, depthshade, self._depthshade_minalpha),
        doc="Whether the members fade with depth (matplotlib's attribute).")

    def __init__(self, *args, zs=0, zdir="z", depthshade=None,
                 depthshade_minalpha=None, axlim_clip=False, **kwargs):
        super().__init__(*args, **kwargs)
        self._depthshade_pending = (depthshade, depthshade_minalpha)
        self.set_3d_properties(zs, zdir, axlim_clip)

    def _pending_key(self):
        # docstring inherited
        return '_offsets3d_pending'

    def _native_geometry(self):
        """The 3-D geometry handed to pltrs (Rust): the offsets placed in
        3-D, and the depth shading."""
        from pltrs import rcParams
        zs, zdir = self.__dict__.pop('_offsets3d_pending')
        depthshade, minalpha = self.__dict__.pop('_depthshade_pending')
        return ("patches", (
            np.atleast_1d(np.asarray(zs, dtype=float)).ravel().tolist(), zdir,
            bool(rcParams['axes3d.depthshade'] if depthshade is None
                 else depthshade),
            _depthshade_minalpha(minalpha)))

    def set_3d_properties(self, zs, zdir, axlim_clip=False):
        self._axlim_clip = axlim_clip
        offsets = self._lifted_offsets(zs, zdir)
        if _lifted(self) is None:
            self._offsets3d_pending = (zs, zdir)
        else:
            self._offsets3d = offsets
        self.stale = True

    def _lifted_offsets(self, zs, zdir):
        """The 3-D positions of the members: their 2-D offsets placed at
        *zs* along *zdir*, as given (matplotlib's ``set_3d_properties``)."""
        offsets = self.get_offsets()
        xs, ys = offsets.T if len(offsets) else ([], [])
        return juggle_axes(xs, ys, np.atleast_1d(zs), zdir)

    def _adopt_into(self, axes, autolim, axlim_clip):
        # docstring inherited; positions matplotlib's draws fail to project
        # (more z values than offsets, say) are recorded once the axes
        # holds the collection.
        zs, zdir = self.__dict__['_offsets3d_pending']
        offsets = self._lifted_offsets(zs, zdir)
        super()._adopt_into(axes, autolim, axlim_clip)
        problem = _draw_problems.offsets_problem(self, offsets)
        if problem is not None:
            self._keep_unprojectable(offsets)
        _draw_problems.record(self, problem or self._sizes_problem())
        return self

    def _sizes_problem(self):
        """The exception matplotlib's projection raises sorting the sizes
        of the members by depth once their positions are projected, or
        None (only `Path3DCollection` sorts them)."""
        return None

    @property
    def _offsets3d(self):
        """The 3-D positions of the members, as given."""
        unprojectable = self.__dict__.get("_unprojectable")
        if unprojectable is not None:
            return unprojectable
        pending = self.__dict__.get("_offsets3d_pending")
        if pending is not None:
            return self._lifted_offsets(*pending)
        # The collection's own tuple of its own arrays, as matplotlib's
        # (`pltrs._own_state`): what a program changes in them is projected
        # at the next draw.
        return _own_state.hand_out(self, _OFFSETS3D)

    def _read_offsets3d(self):
        """The 3-D positions as given, while pltrs (Rust) holds them, else
        as it holds them."""
        return _values.given(self, "_offsets3d", tuple(
            np.array(c) for c in
            _mplot3d.scatter_offsets(self.axes._native, self._native)))

    def _take_offsets3d(self, offsets):
        """Hand pltrs (Rust) the 3-D positions handed out, *offsets*, which
        the program changed in place."""
        columns = _three_columns(offsets)
        if columns is not None:
            _mplot3d.set_scatter_offsets(self.axes._native, self._native, columns)

    @_offsets3d.setter
    def _offsets3d(self, offsets):
        problem = _draw_problems.offsets_problem(self, offsets)
        columns = None if problem is not None else _three_columns(offsets)
        if columns is None:
            self._keep_unprojectable(offsets)
        else:
            self.__dict__.pop("_unprojectable", None)
            self.__dict__.pop("_offsets_unprojected", None)
            _mplot3d.set_scatter_offsets(self.axes._native, self._native, columns)
            _values.remember(self, "_offsets3d", offsets)
        _own_state.drop(self, "offsets3d")
        _draw_problems.record(self, problem or self._sizes_problem())
        self.stale = True

    def _keep_unprojectable(self, offsets):
        """
        Keep *offsets*, 3-D positions matplotlib's draws fail to project
        (their coordinates differ in length), as matplotlib keeps them: the
        offsets stay those of the last projection.
        """
        if "_unprojectable" not in self.__dict__:
            self._offsets_unprojected = self.get_offsets()
        self._unprojectable = offsets

    def do_3d_projection(self):
        axes = _lifted(self)
        if axes is not None:
            problem = (_draw_problems.offsets_problem(self, self._offsets3d, axes.M)
                       or self._sizes_problem())
            if problem is not None:
                raise problem
        return _projected_depth(self)

    def get_offsets(self):
        # docstring inherited; those of the last projection while the
        # positions kept cannot be projected.
        kept = self.__dict__.get("_offsets_unprojected")
        return super().get_offsets() if kept is None else kept

    def get_depthshade(self):
        axes = _lifted(self)
        if axes is None:
            from pltrs import rcParams
            depthshade = self._depthshade_pending[0]
            return (rcParams['axes3d.depthshade'] if depthshade is None
                    else depthshade)
        # As given, as matplotlib keeps it.
        return _values.given(self, "depthshade",
                             _mplot3d.get_depthshade(axes._native, self._native),
                             same=lambda given, native: bool(given) == native)

    def set_depthshade(self, depthshade, depthshade_minalpha=None):
        axes = _lifted(self)
        if axes is None:
            self._depthshade_pending = (depthshade, depthshade_minalpha)
        else:
            _mplot3d.set_depthshade(axes._native, self._native,
                                    bool(depthshade),
                                    _depthshade_minalpha(depthshade_minalpha))
            _values.remember(self, "depthshade", depthshade)
        self.stale = True


class Patch3DCollection(_Lifted3DOffsets, mcoll.PatchCollection):
    """
    A collection of 3D patches.
    """

    def __init__(
        self,
        *args,
        zs=0,
        zdir="z",
        depthshade=None,
        depthshade_minalpha=None,
        axlim_clip=False,
        **kwargs
    ):
        """
        Create a collection of flat 3D patches with its normal vector
        pointed in *zdir* direction, and located at *zs* on the *zdir*
        axis. 'zs' can be a scalar or an array-like of the same length as
        the number of patches in the collection.

        Constructor arguments are the same as for
        :class:`~matplotlib.collections.PatchCollection`. In addition,
        keywords *zs=0* and *zdir='z'* are available.

        The keyword argument *depthshade* is available to
        indicate whether or not to shade the patches in order to
        give the appearance of depth (default is *True*).
        This is typically desired in scatter plots.

        *depthshade_minalpha* sets the minimum alpha value applied by
        depth-shading.
        """
        super().__init__(*args, zs=zs, zdir=zdir, depthshade=depthshade,
                         depthshade_minalpha=depthshade_minalpha,
                         axlim_clip=axlim_clip, **kwargs)

    def get_depthshade(self):
        return super().get_depthshade()

    def set_depthshade(
        self,
        depthshade,
        depthshade_minalpha=None,
    ):
        """
        Set whether depth shading is performed on collection members.

        Parameters
        ----------
        depthshade : bool
            Whether to shade the patches in order to give the appearance of
            depth.
        depthshade_minalpha : float, default: :rc:`axes3d.depthshade_minalpha`
            Sets the minimum alpha value used by depth-shading.

            .. versionadded:: 3.11
        """
        super().set_depthshade(depthshade, depthshade_minalpha)

    def set_sort_zpos(self, val):
        """Set the position to use for z-sorting."""
        self._place_sort_zpos(val)

    def set_3d_properties(self, zs, zdir, axlim_clip=False):
        """
        Set the *z* positions and direction of the patches.

        Parameters
        ----------
        zs : float or array of floats
            The location or locations to place the patches in the collection
            along the *zdir* axis.
        zdir : {'x', 'y', 'z'}
            Plane to plot patches orthogonal to.
            All patches must have the same direction.
            See `.get_dir_vector` for a description of the values.
        axlim_clip : bool, default: False
            Whether to hide patches with a vertex outside the axes view limits.

            .. versionadded:: 3.10
        """
        super().set_3d_properties(zs, zdir, axlim_clip)

    def do_3d_projection(self):
        return super().do_3d_projection()

    def get_facecolor(self):
        # A new array at each call, as matplotlib's (the colours shaded
        # with depth).
        return np.array(self._read_facecolor())

    def get_edgecolor(self):
        return np.array(self._read_edgecolor())


def patch_collection_2d_to_3d(col, zs=0, zdir="z", depthshade=None,
                              axlim_clip=False, *args, depthshade_minalpha=None):
    """
    Convert a `.PatchCollection` into a `.Patch3DCollection` object
    (or a `.PathCollection` into a `.Path3DCollection` object).

    Parameters
    ----------
    col : `~matplotlib.collections.PatchCollection` or \
`~matplotlib.collections.PathCollection`
        The collection to convert.
    zs : float or array of floats
        The location or locations to place the patches in the collection along
        the *zdir* axis. Default: 0.
    zdir : {'x', 'y', 'z'}
        The axis in which to place the patches. Default: "z".
        See `.get_dir_vector` for a description of the values.
    depthshade : bool, default: :rc:`axes3d.depthshade`
        Whether to shade the patches to give a sense of depth.
    axlim_clip : bool, default: False
        Whether to hide patches with a vertex outside the axes view limits.

        .. versionadded:: 3.10

    depthshade_minalpha : float, default: :rc:`axes3d.depthshade_minalpha`
        Sets the minimum alpha value used by depth-shading.

        .. versionadded:: 3.11
    """
    if isinstance(col, mcoll.PathCollection):
        col.__class__ = Path3DCollection
    elif isinstance(col, mcoll.PatchCollection):
        col.__class__ = Patch3DCollection
    else:
        # matplotlib gives what is neither its depth shading, then asks it
        # for its 3-D properties.
        col._depthshade = depthshade
        col._depthshade_minalpha = depthshade_minalpha
        col._in_draw = False
        col.set_3d_properties(zs, zdir, axlim_clip)
        return
    _gains_projection(col)
    col._depthshade_pending = (depthshade, depthshade_minalpha)
    col.set_3d_properties(zs, zdir, axlim_clip)
    _lift_added(col)


#: The line widths of a 3-D scatter in the order of its markers
#: (matplotlib's ``_linewidths3d``, which its projections sort): the
#: collection's own array.
_LINEWIDTHS3D = _own_state.State(
    "linewidths3d", lambda collection: np.array(collection._read_linewidth()),
    lambda collection, widths: collection._take_linewidths3d(widths))


def _get_data_scale(X, Y, Z):
    """The scale of projected positions (masked arrays) that depth
    shading fades over: the root of the summed squares of their ranges, 0
    without any (matplotlib 3.11.2's, under the Matplotlib License)."""
    if not np.ma.count(X):
        return 0
    ptp_x = X.max() - X.min()
    ptp_y = Y.max() - Y.min()
    ptp_z = Z.max() - Z.min()
    return np.sqrt(ptp_x ** 2 + ptp_y ** 2 + ptp_z ** 2)


def _zordered_offsets(collection):
    """The projected positions of the markers of *collection*, deepest
    first (matplotlib's ``_offset_zordered``)."""
    vxs, vys, vzs = _projected_offsets(collection)
    order = np.ma.argsort(vzs)[::-1]
    return np.ma.column_stack((vxs[order], vys[order]))


class Path3DCollection(_Lifted3DOffsets, mcoll.PathCollection):
    """
    A collection of 3D paths.
    """

    # matplotlib's draw of the class is not decorated with
    # `allow_rasterization`: asking to rasterize it warns.
    _ignores_rasterization = True

    #: matplotlib's fields of the projected order, as its ``__init__`` sets
    #: them (its draws set them anew; pltrs (Rust) projects the collection
    #: itself): what programs read and assign back (ultraplot's previews).
    _offset_zordered = _properties.Left(
        lambda collection: _zordered_offsets(collection), None)
    _z_markers_idx = _properties.Left(
        lambda collection: np.ma.argsort(_projected_offsets(collection)[2])[::-1], slice(-1))

    #: The extent of the projected positions, over which the members fade
    #: with depth (matplotlib's attribute, there once the collection was
    #: projected).
    _data_scale = _properties.Left(
        lambda collection: _get_data_scale(*_projected_offsets(collection)))

    def _take_linewidths3d(self, widths):
        """Hand pltrs (Rust) the widths *widths* a program changed in the
        array of them: matplotlib's projection sorts them into the widths
        drawn only where there are several; one is kept, and read back."""
        if len(widths) > 1:
            self.set_linewidth(np.array(widths))
        else:
            self.__dict__["_linewidths3d"] = widths

    def __getattr__(self, name):
        # matplotlib's sizes and widths of the items in their given order,
        # which its projections sort.
        if name == "_sizes3d":
            return self.get_sizes()
        if name == "_linewidths3d":
            return _own_state.hand_out(self, _LINEWIDTHS3D)
        return super().__getattr__(name)

    def __init__(
        self,
        *args,
        zs=0,
        zdir="z",
        depthshade=None,
        depthshade_minalpha=None,
        axlim_clip=False,
        **kwargs
    ):
        """
        Create a collection of flat 3D paths with its normal vector
        pointed in *zdir* direction, and located at *zs* on the *zdir*
        axis. 'zs' can be a scalar or an array-like of the same length as
        the number of paths in the collection.

        Constructor arguments are the same as for
        :class:`~matplotlib.collections.PathCollection`. In addition,
        keywords *zs=0* and *zdir='z'* are available.

        Also, the keyword argument *depthshade* is available to
        indicate whether or not to shade the patches in order to
        give the appearance of depth (default is *True*).
        This is typically desired in scatter plots.

        *depthshade_minalpha* sets the minimum alpha value applied by
        depth-shading.
        """
        super().__init__(*args, zs=zs, zdir=zdir, depthshade=depthshade,
                         depthshade_minalpha=depthshade_minalpha,
                         axlim_clip=axlim_clip, **kwargs)

    def draw(self, renderer):
        super().draw(renderer)

    def set_sort_zpos(self, val):
        """Set the position to use for z-sorting."""
        self._place_sort_zpos(val)

    def set_3d_properties(self, zs, zdir, axlim_clip=False):
        """
        Set the *z* positions and direction of the paths.

        Parameters
        ----------
        zs : float or array of floats
            The location or locations to place the paths in the collection
            along the *zdir* axis.
        zdir : {'x', 'y', 'z'}
            Plane to plot paths orthogonal to.
            All paths must have the same direction.
            See `.get_dir_vector` for a description of the values.
        axlim_clip : bool, default: False
            Whether to hide paths with a vertex outside the axes view limits.

            .. versionadded:: 3.10
        """
        # matplotlib's projections sort the sizes the collection holds then.
        self.__dict__.pop("_sizes3d", None)
        super().set_3d_properties(zs, zdir, axlim_clip)

    def set_sizes(self, sizes, dpi=72.0):
        super().set_sizes(sizes, dpi)
        # Kept as given, as matplotlib's projections sort them by depth.
        self._sizes3d = sizes
        if _lifted(self) is not None:
            problem = _draw_problems.offsets_problem(self, self._offsets3d)
            _draw_problems.record(self, problem or self._sizes_problem())

    def _sizes_problem(self):
        # docstring inherited
        if "_sizes3d" not in self.__dict__:
            return None
        return _draw_problems.sizes_problem(self._sizes3d)

    def set_linewidth(self, lw):
        super().set_linewidth(lw)
        self.__dict__.pop("_linewidths3d", None)
        _own_state.drop(self, "linewidths3d")

    def get_depthshade(self):
        return super().get_depthshade()

    def set_depthshade(
        self,
        depthshade,
        depthshade_minalpha=None,
    ):
        """
        Set whether depth shading is performed on collection members.

        Parameters
        ----------
        depthshade : bool
            Whether to shade the patches in order to give the appearance of
            depth.
        depthshade_minalpha : float
            Sets the minimum alpha value used by depth-shading.

            .. versionadded:: 3.11
        """
        super().set_depthshade(depthshade, depthshade_minalpha)

    def do_3d_projection(self):
        return super().do_3d_projection()

    def get_facecolor(self):
        # A new array at each call, as matplotlib's (the colours shaded
        # with depth, in the order drawn).
        return np.array(self._read_facecolor())

    def get_edgecolor(self):
        return np.array(self._read_edgecolor())


#: The face and the edge colours of a `Poly3DCollection`, in polygon order
#: (matplotlib's ``_facecolor3d`` and ``_edgecolor3d``), which the collection
#: keeps as arrays of its own (`pltrs._own_state`): changed in place, they
#: are set as its colours again.
def _mapped_at(collection):
    """What tells the arrays of the colours of a collection mapping values
    apart: matplotlib's projection maps them anew, into new arrays, at
    each draw and each projection the program asks for (None for a
    collection that maps none: its arrays stay)."""
    if mcolorizer._array_of(collection) is None:
        return None
    return collection._mapping_draws()


_FACECOLOR3D = _own_state.State(
    "facecolor3d",
    lambda collection: _mplot3d.poly_colors(collection.axes._native, collection._native)[0],
    lambda collection, colors: collection.set_facecolor(np.array(colors)), version=_mapped_at)
_EDGECOLOR3D = _own_state.State(
    "edgecolor3d",
    lambda collection: _mplot3d.poly_colors(collection.axes._native, collection._native)[1],
    lambda collection, colors: collection.set_edgecolor(np.array(colors)), version=_mapped_at)


class Poly3DCollection(_ProjectedPaths, _Loose3D, mcoll.PolyCollection):
    """
    A collection of 3D polygons.

    .. note::
        **Filling of 3D polygons**

        There is no simple definition of the enclosed surface of a 3D polygon
        unless the polygon is planar.

        In practice, Matplotlib fills the 2D projection of the polygon. This
        gives a correct filling appearance only for planar polygons. For all
        other polygons, you'll find orientations in which the edges of the
        polygon intersect in the projection. This will lead to an incorrect
        visualization of the 3D area.

        If you need filled areas, it is recommended to create them via
        `~mpl_toolkits.mplot3d.axes3d.Axes3D.plot_trisurf`, which creates a
        triangulation and thus generates consistent surfaces.
    """

    def _mapping_draws(self):
        """How many draws mapped the collection's values to colours: in a
        3-D axes, each draw of the axes, whose projection of the polygons
        maps them (animated or not, as matplotlib's ``Axes3D.draw``
        projects them), and each projection the program asks for."""
        axes = _axes3d(self)
        if axes is None:
            return super()._mapping_draws()
        return _artist.draw_count(axes._native) + self.__dict__.get("_projections", 0)

    def __init__(self, verts, *args, zsort='average', shade=False,
                 lightsource=None, axlim_clip=False, **kwargs):
        """
        Parameters
        ----------
        verts : list of (N, 3) array-like
            The sequence of polygons [*verts0*, *verts1*, ...] where each
            element *verts_i* defines the vertices of polygon *i* as a 2D
            array-like of shape (N, 3).
        zsort : {'average', 'min', 'max'}, default: 'average'
            The calculation method for the z-order.
            See `~.Poly3DCollection.set_zsort` for details.
        shade : bool, default: False
            Whether to shade *facecolors* and *edgecolors*. When activating
            *shade*, *facecolors* and/or *edgecolors* must be provided.

            .. versionadded:: 3.7

        lightsource : `~matplotlib.colors.LightSource`, optional
            The lightsource to use when *shade* is True.

            .. versionadded:: 3.7

        axlim_clip : bool, default: False
            Whether to hide polygons with a vertex outside the view limits.

            .. versionadded:: 3.10

        *args, **kwargs
            All other parameters are forwarded to `.PolyCollection`.

        Notes
        -----
        Note that this class does a bit of magic with the _facecolors
        and _edgecolors properties.
        """
        # matplotlib's `set_verts` takes the faces first, then its
        # constructor checks that each is two-dimensional.
        if isinstance(verts, np.ndarray):
            _api.check_shape((None, None, 3), segments3d=verts)
        faces = _faces_3d(verts)
        if (not isinstance(verts, np.ndarray)
                and any(len(np.shape(vert)) != 2 for vert in verts)):
            raise ValueError('verts must be a list of (N, 3) array-like')
        if shade:
            light = (None if lightsource is None
                     else (float(lightsource.azdeg), float(lightsource.altdeg)))
            for key in ('facecolors', 'edgecolors'):
                if kwargs.get(key) is not None:
                    kwargs[key] = _mplot3d.shade_faces(
                        faces, mcolors.to_rgba_array(kwargs[key]), light)
            if (kwargs.get('facecolors') is None
                    and kwargs.get('edgecolors') is None):
                raise ValueError(
                    "You must provide facecolors, edgecolors, or both for "
                    "shade to work.")
        # matplotlib hands *closed* on to `.PolyCollection` in **kwargs.
        closed = kwargs.pop('closed', True)
        super().__init__([], *args, **kwargs)
        if zsort not in self._zsort_functions:
            raise KeyError(zsort)  # As matplotlib's `set_zsort` raises.
        self._verts3d_pending = faces
        self._zsort = zsort
        self._closed = closed
        self._axlim_clip = axlim_clip
        if self._program_setters:
            _program_classes.replay_setters(self, [("set_zsort", (zsort,))])

    def _refresh_colors(self, faces, edges):
        """Draw the polygons with the face (*faces*) and edge (*edges*)
        colours the collection has now (matplotlib's ``_facecolor3d`` and
        ``_edgecolor3d``), once a 3-D axes holds it."""
        axes = _axes3d(self)
        if axes is not None:
            _mplot3d.refresh_poly_colors(axes._native, self._native, faces, edges)

    @property
    def _facecolor3d(self):
        """The face colours the polygons are drawn with, in polygon order:
        the collection's own array (`pltrs._own_state`), so that changed in
        place they colour the polygons (nilearn's contours of surface
        regions)."""
        if (_properties.stamp(self) == (0, 0) and "_projection_asked" not in self.__dict__
                and mcolorizer._array_of(self) is not None and "_native" in self.__dict__):
            # matplotlib maps the values at the first projection: until
            # then the faces have the one colour of a patch.
            return self.__dict__.setdefault("_facecolor3d_unmapped", mcolors.to_rgba_array(
                rcParams['patch.facecolor'], self.get_alpha()))
        return _own_state.hand_out(self, _FACECOLOR3D)

    @_facecolor3d.setter
    def _facecolor3d(self, colors):
        # Assigned, the array colours the polygons from the next
        # projection on, and is the collection's own.
        _FACECOLOR3D.take(self, colors)
        _own_state.keep(self, _FACECOLOR3D, colors)

    @property
    def _edgecolor3d(self):
        """The edge colours the polygons are drawn with, in polygon order:
        the collection's own array, so that changed in place they colour
        the edges."""
        return _own_state.hand_out(self, _EDGECOLOR3D)

    @_edgecolor3d.setter
    def _edgecolor3d(self, colors):
        _EDGECOLOR3D.take(self, colors)
        _own_state.keep(self, _EDGECOLOR3D, colors)

    @property
    def _facecolors(self):
        """The face colours as they were set, in polygon order (matplotlib's
        ``_facecolors``; `get_facecolor` gives those of the last projection,
        in drawing order)."""
        return self._facecolor3d

    def _project_once(self):
        """Project the polygons in the current view unless they were
        projected, as matplotlib's getters of their colours do (through
        the axes, which a collection never added lacks)."""
        axes = _lifted(self)
        if axes is None:
            raise AttributeError("'NoneType' object has no attribute 'get_proj'")
        if not _mplot3d.poly_projected(axes._native, self._native):
            axes.M = axes.get_proj()
            self.do_3d_projection()

    def get_facecolor(self):
        # docstring inherited; the colours of the last projection, the
        # array it left (``_facecolors2d``).
        self._project_once()
        return np.asarray(self._facecolors2d)

    def get_edgecolor(self):
        # docstring inherited; the colours of the last projection, the
        # array it left (``_edgecolors2d``).
        self._project_once()
        return np.asarray(self._edgecolors2d)

    def set_facecolor(self, colors):
        # docstring inherited
        try:
            super().set_facecolor(colors)
        finally:
            self._record_projection_problem()
        self._refresh_colors(True, False)

    def set_edgecolor(self, colors):
        # docstring inherited
        try:
            super().set_edgecolor(colors)
        finally:
            self._record_projection_problem()
        if _axes3d(self) is None:
            # matplotlib's takes the edge colours with the alpha set by
            # then (``PolyCollection.get_edgecolor``), as no later
            # `set_alpha` does: faded when it was set before (a colour
            # given after ``alpha=`` in the constructor, pyroomacoustics'
            # walls), not when the constructor gave both.
            self.__dict__["_edges_faded"] = self.get_alpha() is not None
        self._refresh_colors(False, True)

    def _array_set(self):
        # docstring inherited
        super()._array_set()
        self._record_projection_problem()

    def _record_projection_problem(self):
        """Record with pltrs (Rust) the first exception matplotlib's
        projection of the polygons raises (`_draw_problems.record`), which
        an animated collection meets too: mapping the array it holds
        (``update_scalarmappable``: an array of more than one dimension,
        then an edge colour it refused, set again unless the faces are not
        mapped), then indexing the codes."""
        if _axes3d(self) is None:
            return
        problem = None
        if mcolorizer._array_of(self) is not None:
            try:
                _unusable.check_one(self, "array")
                refused = _unusable.kept(self, "refused edgecolor", None)
                if refused is not None and not cbook._str_equal(
                        self.__dict__.get("_original_facecolor"), "none"):
                    mcolors.to_rgba_array(refused)
            except (TypeError, ValueError) as err:
                problem = err
        _draw_problems.record(self, problem or self.__dict__.get("_codes_problem"))

    def set_alpha(self, alpha):
        # docstring inherited; matplotlib's checks the value as an
        # `.Artist`'s, whose message names its type.
        if alpha is not None and not isinstance(alpha, Real):
            raise TypeError(f'alpha must be numeric or None, not {type(alpha)}')
        super().set_alpha(alpha)
        self._refresh_colors(True, False)

    _zsort_functions = {
        'average': np.average,
        'min': np.min,
        'max': np.max,
    }

    def set_zsort(self, zsort):
        """
        Set the calculation method for the z-order.

        Parameters
        ----------
        zsort : {'average', 'min', 'max'}
            The function applied on the z-coordinates of the vertices in the
            viewer's coordinate system, to determine the z-order.
        """
        if zsort not in self._zsort_functions:
            raise KeyError(zsort)
        axes = _axes3d(self)
        if axes is None:
            self._zsort = zsort
        else:
            _mplot3d.set_zsort(axes._native, self._native, zsort)
        self._sort_zpos = None
        self.stale = True

    def _set_faces(self, faces, closed, given=None):
        """Make the polygons those of the 3-D vertices *faces*, of what
        *given* held: an array of all the faces is the collection's own
        array of them, as matplotlib keeps the one it is given."""
        values = self.__dict__
        for name in ("_faces", "_invalid_vertices"):
            values.pop(name, None)
            _own_state.drop(self, name.strip("_"))
        if isinstance(given, np.ndarray) and given.ndim == 3:
            values["_faces_as"] = "array"
            if not isinstance(given, np.ma.MaskedArray) and given.dtype.kind == "f":
                values["_faces"] = given
        else:
            values.pop("_faces_as", None)
        axes = _axes3d(self)
        if axes is None:
            self._verts3d_pending = faces
            values["_closed"] = closed
        else:
            values.pop("_closed", None)
            _mplot3d.set_poly_verts(axes._native, self._native, faces,
                                    bool(closed))
        self.stale = True

    def _pending_faces(self):
        """matplotlib's array of the faces and its mask, of the faces kept
        for the 3-D axes that will add the collection."""
        faces = self.__dict__.get("_verts3d_pending")
        if faces is None:
            return _properties.MISSING, _properties.MISSING
        counts = np.fromiter(map(len, faces), dtype=np.intp, count=len(faces))
        most = counts.max(initial=0)
        padded = np.empty((len(faces), most, 3))
        for index, face in enumerate(faces):
            padded[index, :len(face)] = face
        invalid = np.arange(most) >= counts[:, None]
        if self.__dict__.get("_faces_as") == "array" and not invalid.any():
            invalid = False
        return padded, invalid

    def _made_faces(self):
        """matplotlib's array of the faces and its mask of the vertices
        ragged faces lack (``_get_vector``), of the faces pltrs (Rust)
        holds, else of those kept for the axes that will add the
        collection."""
        made = _fields3d.faces(self)
        return self._pending_faces() if made is _properties.MISSING else made

    def _take_faces(self, faces):
        """Hand pltrs (Rust) the array of faces *faces*, which the program
        changed in place or assigned, without the vertices its mask
        leaves out."""
        invalid = type(self)._invalid_vertices.peek(self)
        if invalid is None:
            invalid = self._made_faces()[1]
        faces = np.asarray(faces, dtype=float)
        if invalid is not _properties.MISSING and np.ndim(invalid) == 0 and invalid:
            # (A mask of one True hides every vertex.)
            polygons = [face[:0] for face in faces]
        elif (invalid is False or invalid is _properties.MISSING
                or np.shape(invalid) != faces.shape[:2]):
            polygons = list(faces)
        else:
            polygons = [face[~hidden] for face, hidden in zip(faces, np.asarray(invalid))]
        axes = _axes3d(self)
        if axes is None:
            self._verts3d_pending = polygons
        else:
            _mplot3d.set_poly_verts(axes._native, self._native, polygons, bool(self._closed))

    #: The faces as matplotlib keeps them, an array ``(faces, vertices,
    #: 3)``: the collection's own (the array it was given, where it was
    #: given one), one object until the vertices are set again (libraries
    #: tell a changed geometry by it), which pltrs (Rust) follows when a
    #: program changes it in place or assigns another.
    _faces = _properties.Field(lambda collection: collection._made_faces()[0],
                             lambda collection, faces: collection._take_faces(faces))

    #: Which vertices of matplotlib's array of faces ragged faces lack:
    #: False, or a bool array ``(faces, vertices)``; pltrs (Rust) leaves
    #: out the vertices a program masks in it.
    _invalid_vertices = _properties.Field(
        lambda collection: collection._made_faces()[1],
        lambda collection, invalid: collection._take_faces(collection._faces))

    @property
    def _closed(self):
        """Whether the polygons made of the faces are closed (matplotlib's
        attribute)."""
        if "_closed" in self.__dict__:
            return self.__dict__["_closed"]
        held = _fields3d.kept(self)
        return True if held is None or held[4] is None else held[4]

    @_closed.setter
    def _closed(self, closed):
        self.__dict__["_closed"] = closed
        axes = _axes3d(self)
        if axes is not None and "_verts3d_pending" not in self.__dict__:
            held = _fields3d.kept(self)
            if held is not None and held[0] == "polys" and not held[2]:
                _mplot3d.set_poly_verts(axes._native, self._native, held[1], bool(closed))
                del self.__dict__["_closed"]

    @property
    def _zsortfunc(self):
        """The function reducing the depths of the vertices of a polygon
        to the depth it is sorted by (matplotlib's attribute, one of
        ``_zsort_functions``)."""
        name = self.__dict__.get("_zsort") or _fields3d.zsort(self) or type(self)._zsort
        if name is None:
            raise AttributeError(
                f"'{type(self).__name__}' object has no attribute '_zsortfunc'",
                name="_zsortfunc", obj=self)
        return self._zsort_functions[name]

    @_zsortfunc.setter
    def _zsortfunc(self, function):
        for name, known in self._zsort_functions.items():
            if function is known:
                self.set_zsort(name)
                return
        raise ValueError(
            "pltrs sorts the polygons of a 3-D collection by the mean, the least or the "
            f"greatest depth of their vertices (np.average, np.min, np.max), not by "
            f"{function!r}")

    #: The face and edge colours in the order the last projection drew the
    #: polygons in, deepest first (matplotlib's attributes, there once the
    #: polygons were projected).
    _facecolors2d = _properties.Left(lambda collection: collection._read_facecolor())
    _edgecolors2d = _properties.Left(lambda collection: collection._read_edgecolor())

    @_api.deprecated("3.10")
    def get_vector(self, segments3d):
        if isinstance(segments3d, np.ndarray):
            _api.check_shape((None, None, 3), segments3d=segments3d)
        self._set_faces(_faces_3d(segments3d), self._closed, segments3d)

    def set_verts(self, verts, closed=True):
        """
        Set 3D vertices.

        Parameters
        ----------
        verts : list of (N, 3) array-like
            The sequence of polygons [*verts0*, *verts1*, ...] where each
            element *verts_i* defines the vertices of polygon *i* as a 2D
            array-like of shape (N, 3).
        closed : bool, default: True
            Whether the polygon should be closed by adding a CLOSEPOLY
            connection at the end.
        """
        self._set_faces(_faces_3d(verts), closed, verts)

    def set_verts_and_codes(self, verts, codes):
        """Set 3D vertices with path codes."""
        faces = _faces_3d(verts)
        axes = _axes3d(self)
        values = self.__dict__
        _own_state.drop(self, "codes3d")
        if codes is None or not len(codes) or len(codes) < len(faces):
            # matplotlib keeps the codes unchecked: its projection draws
            # without none, and fails indexing too few by face.
            self._set_faces(faces, False, verts)
            values["_codes3d"] = codes
            self._codes_problem = _draw_problems.codes_problem(codes, len(faces))
            self._record_projection_problem()
            return
        values.pop("_codes_problem", None)
        for name in ("_faces", "_invalid_vertices"):
            values.pop(name, None)
            _own_state.drop(self, name.strip("_"))
        values.pop("_faces_as", None)
        codes = codes[:len(faces)]
        if axes is None:
            self._verts3d_pending = faces
            self._faces3d = faces
            values["_codes3d"] = codes
            values["_closed"] = False
        else:
            values.pop("_codes3d", None)
            values.pop("_closed", None)
            _mplot3d.set_poly_verts_and_codes(
                axes._native, self._native, faces,
                [np.asarray(c, np.uint8).tolist() for c in codes])
        self.stale = True

    def _take_codes(self, codes):
        """Hand pltrs (Rust) the codes *codes* of the polygons, which the
        program changed in place or assigned, with the faces it holds."""
        held = _fields3d.kept(self)
        if held is not None and held[0] == "polys":
            self.set_verts_and_codes(held[1], codes)

    def _read_codes(self):
        """The codes pltrs (Rust) holds, as matplotlib's list of them."""
        codes = _fields3d.codes3d(self)
        if codes is _properties.MISSING or codes is None:
            # Those of the 2-D paths the polygons were lifted from.
            return self.__dict__.get("_codes3d_lifted")
        return [list(np.asarray(path_codes, np.uint8)) for path_codes in codes]

    #: The codes of the polygons (matplotlib's attribute): None for
    #: polygons made of their vertices, a list of the codes of each for
    #: polygons lifted from 2-D paths; the collection's own list, which
    #: pltrs (Rust) follows when a program changes it in place or assigns
    #: another.
    _codes3d = _properties.Field(lambda collection: collection._read_codes(),
                               lambda collection, codes: collection._take_codes(codes))

    def set_3d_properties(self, axlim_clip=False):
        self._axlim_clip = axlim_clip
        self._alpha3d = mcoll.PolyCollection.get_alpha(self)
        axes = _axes3d(self)
        if axes is None:
            self._zsort = 'average'
        else:
            self.update_scalarmappable()
            _mplot3d.set_poly_3d_properties(axes._native, self._native)
            _mplot3d.set_axlim_clip(axes._native, self._native, bool(axlim_clip))
        self._sort_zpos = None
        self.stale = True

    def set_sort_zpos(self, val):
        """Set the position to use for z-sorting."""
        self._place_sort_zpos(val)

    def do_3d_projection(self):
        """
        Perform the 3D projection for this object.
        """
        # (matplotlib maps the values to colours first, also where the
        # projection then fails.)
        self.__dict__["_projection_asked"] = True
        return super().do_3d_projection()


def poly_collection_2d_to_3d(col, zs=0, zdir='z', axlim_clip=False):
    """
    Convert a `.PolyCollection` into a `.Poly3DCollection` object.

    Parameters
    ----------
    col : `~matplotlib.collections.PolyCollection`
        The collection to convert.
    zs : float or array of floats
        The location or locations to place the polygons in the collection along
        the *zdir* axis. Default: 0.
    zdir : {'x', 'y', 'z'}
        The axis in which to place the patches. Default: 'z'.
        See `.get_dir_vector` for a description of the values.
    """
    # matplotlib places one position or one per path on the paths first
    # (``_paths_to_3d_segments_with_codes``), then changes the class.
    zs = np.broadcast_to(zs, len(col.get_paths()))
    col.__class__ = Poly3DCollection
    _gains_projection(col)
    # The codes of the paths, a list per path, as matplotlib keeps them
    # (``_paths_to_3d_segments_with_codes``).
    col._codes3d_lifted = [
        [code for _, code in path.iter_segments(simplify=False, curves=False)]
        for path in col.get_paths()]
    col._verts3d_pending = _lifted_paths(col, zs, zdir)
    # Its alpha has faded its edges, as that of a 2-D collection does.
    col._lifted_from_2d = True
    col._zsort = 'average'
    col.__dict__["_closed"] = False
    col._alpha3d = mcoll.PolyCollection.get_alpha(col)
    col._axlim_clip = axlim_clip
    _lift_added(col)


def _outline_points(segment3d):
    """The (N, 3) float array of the outline *segment3d* of a 3-D patch,
    NaN for an outline of values that are not numbers, which matplotlib's
    draws fail to project (`_draw_problems.patch_problem`)."""
    try:
        return np.asarray(segment3d, dtype=float).reshape(-1, 3)
    except (TypeError, ValueError):
        return np.full((len(segment3d), 3), np.nan)


def _is_number(value):
    """Whether *value* converts to a float."""
    try:
        float(value)
    except (TypeError, ValueError):
        return False
    return True


def _text_floats(position):
    """The 3-D *position* of a text as floats, NaN for a coordinate that is
    no number (which matplotlib's draws fail on,
    `_draw_problems.text_problem`)."""
    return tuple(float(value) if _is_number(value) else np.nan
                 for value in position)


def _keep_text_position(text, position):
    """Keep the 3-D *position* of *text*, a text lifted in a 3-D axes, as
    given (`_values`): pltrs (Rust) holds it as floats, NaN for what is no
    number."""
    if all(_is_number(value) for value in position):
        _values.remember(text, "position_3d", tuple(position))
    else:
        _values.keep(text, "position_3d", tuple(position))


def _get_patch_verts(patch):
    """Return a list of vertices for the path of a patch."""
    trans = patch.get_patch_transform()
    path = patch.get_path()
    polygons = path.to_polygons(trans)
    return polygons[0] if len(polygons) else np.array([])


@_program_classes.class_cache
def _draws_itself(cls):
    """
    Whether patches of the class *cls* are drawn by another patch class
    mixed with the 3-D one (qiskit's ``Arrow3D`` is a `Patch3D` and a
    `~matplotlib.patches.FancyArrowPatch`, whose ``draw`` it calls with
    its ends where its 3-D points project). pltrs (Rust) then draws the
    patch that other class made and knows nothing of the 3-D outline: the
    patch keeps it in Python and projects it as matplotlib's code does.
    """
    after = cls.__mro__[cls.__mro__.index(Patch3D) + 1:]
    return any(klass is not mpatches.Patch and issubclass(klass, mpatches.Patch)
               for klass in after)


def _follows3d(patch):
    """Whether pltrs (Rust) follows the 3-D outline of *patch*: not that
    of a patch another class draws (`_draws_itself`)."""
    return not _draws_itself(type(patch))


def _no_field(artist):
    return _properties.MISSING


class Patch3D(mpatches.Patch):
    """
    3D patch object.
    """

    def __init__(self, *args, zs=(), zdir='z', axlim_clip=False, **kwargs):
        """
        Parameters
        ----------
        verts :
        zs : float
            The location along the *zdir* axis in 3D space to position the
            patch.
        zdir : {'x', 'y', 'z'}
            Plane to plot patch orthogonal to. Default: 'z'.
            See `.get_dir_vector` for a description of the values.
        axlim_clip : bool, default: False
            Whether to hide patches with a vertex outside the axes view limits.

            .. versionadded:: 3.10
        """
        super().__init__(*args, **kwargs)
        self.set_3d_properties(zs, zdir, axlim_clip)

    _axlim_clip = _AXLIM_CLIP

    #: The outline in 3-D, a list of points (matplotlib's attribute): the
    #: patch's own list, which pltrs (Rust) follows when a program changes
    #: it in place or assigns another.
    _segment3d = _properties.Field(_no_field, lambda patch, segment3d: patch._follow3d(),
                                 _follows3d)

    @property
    def _path2d(self):
        """The outline as last projected (matplotlib's attribute, there
        once the patch was projected): one path until the next projection
        makes another. A path a program assigned is read back until then."""
        made = self.__dict__.get("_path2d_made")
        if made is not None and (made[0] is None or made[0] == _properties.stamp(self)):
            return made[1]
        axes = self.axes
        if _draws_itself(type(self)) or not (
                self.__dict__.get("_projections") or _artist.draw_count(self._native)):
            raise AttributeError(
                f"'{type(self).__name__}' object has no attribute '_path2d'",
                name="_path2d", obj=self)
        from pltrs.path import Path
        if axes is None:
            # Removed: the path of its last projection.
            vertices, codes = _patches.get_path(self._native)
        else:
            vertices, codes = _mplot3d.patch_path(axes._native, self._native)
        kept = self.__dict__.get("_code3d")
        if kept is not None and len(kept) == len(vertices):
            # matplotlib's path is made of the codes the patch keeps.
            codes = kept
        path = Path(vertices, codes)
        self.__dict__["_path2d_made"] = (_properties.stamp(self), path)
        return path

    @_path2d.setter
    def _path2d(self, path):
        stamp = None if _draws_itself(type(self)) else _properties.stamp(self)
        self.__dict__["_path2d_made"] = (stamp, path)

    def _add_to_axes(self, axes, kind, autolim=True):
        # docstring inherited
        super()._add_to_axes(axes, kind, autolim)
        if _axes3d(self) is None:
            return self
        if _draws_itself(type(self)):
            _program_projection.follow_patch(self.axes, self)
        elif not _restored(self, '_pending3d'):
            del self.__dict__['_pending3d']
            self._follow3d()
        if _program_classes.takes_from_program(type(self), "do_3d_projection"):
            _program_projection.follow(self.axes, self)
        if kind == "patch" and "_segment3d" in self.__dict__:
            # matplotlib's `add_patch` reads the path of the patch for the
            # data limits, which projects a patch never projected.
            self.get_path()
        return self

    def _geometry3d(self):
        """What `_lift` takes: the outline and whether it is clipped to
        the view limits."""
        return self.__dict__["_segment3d"], self.__dict__.get("_axlim_clip", False)

    def _lift(self, segment3d, axlim_clip):
        """Follow the outline *segment3d* in the 3-D axes holding the patch."""
        _mplot3d.patch_2d_to_3d(self.axes._native, self._native,
                                _outline_points(segment3d), bool(axlim_clip))

    def _follow3d(self):
        """Have pltrs (Rust) follow the outline the patch keeps, in the
        3-D axes holding it (else once one adds it), recording the problem
        matplotlib's draws meet projecting the outline; nothing for a patch
        another class draws (`_draws_itself`)."""
        if _draws_itself(type(self)) or "_segment3d" not in self.__dict__:
            return
        if _axes3d(self) is None:
            self._pending3d = True
            return
        geometry = self._geometry3d()
        problem = _draw_problems.patch_problem(self, geometry[-2])
        self._lift(*geometry)
        _draw_problems.record(self, problem)

    def _projected_by_python(self, codes=None):
        """Project the outline through the projection matrix of the last
        draw of the axes as matplotlib's ``do_3d_projection`` does, keep
        the path projected (``_path2d``) and return the depth: for a patch
        pltrs (Rust) does not follow (`_draws_itself`)."""
        from pltrs.path import Path
        s = self._segment3d
        xs0, ys0, zs0 = zip(*s)
        mask = _scale_invalid_mask(xs0, ys0, zs0, self.axes)
        if self._axlim_clip:
            mask |= _viewlim_mask(xs0, ys0, zs0, self.axes)
        if np.any(mask):
            xs, ys, zs = np.ma.array(zip(*s), dtype=float, mask=mask).filled(np.nan)
        else:
            xs, ys, zs = xs0, ys0, zs0
        vxs, vys, vzs, vis = proj3d._scale_proj_transform_clip(xs, ys, zs, self.axes)
        self._path2d = Path(np.ma.column_stack([vxs, vys]), codes)
        return min(vzs)

    def do_3d_projection(self):
        if _draws_itself(type(self)):
            return self._projected_by_python()
        return _projected_depth(self)

    def set_3d_properties(self, verts, zs=0, zdir='z', axlim_clip=False):
        """
        Set the *z* position and direction of the patch.

        Parameters
        ----------
        verts :
        zs : float
            The location along the *zdir* axis in 3D space to position the
            patch.
        zdir : {'x', 'y', 'z'}
            Plane to plot patch orthogonal to. Default: 'z'.
            See `.get_dir_vector` for a description of the values.
        axlim_clip : bool, default: False
            Whether to hide patches with a vertex outside the axes view limits.

            .. versionadded:: 3.10
        """
        zs = np.broadcast_to(zs, len(verts))
        self.__dict__["_axlim_clip"] = axlim_clip
        self._segment3d = [juggle_axes(x, y, z, zdir)
                           for ((x, y), z) in zip(verts, zs)]

    def get_path(self):
        # docstring inherited; matplotlib projects a patch never projected
        # in the view of now, which becomes that of the last draw (``M``).
        if not hasattr(self, '_path2d'):
            self.axes.M = self.axes.get_proj()
            self.do_3d_projection()
        return self._path2d

    def get_patch_transform(self):
        # docstring inherited
        from pltrs.transforms import IdentityTransform
        return IdentityTransform()

    # matplotlib's class inherits `.Patch`'s (whose name its errors give):
    # this one leaves out the transform of the patch it was made from.
    get_patch_transform.__qualname__ = "Patch.get_patch_transform"


class PathPatch3D(Patch3D):
    """
    3D PathPatch object.
    """

    def __init__(self, path, *, zs=(), zdir='z', axlim_clip=False, **kwargs):
        """
        Parameters
        ----------
        path :
        zs : float
            The location along the *zdir* axis in 3D space to position the
            path patch.
        zdir : {'x', 'y', 'z', 3-tuple}
            Plane to plot path patch orthogonal to. Default: 'z'.
            See `.get_dir_vector` for a description of the values.
        axlim_clip : bool, default: False
            Whether to hide path patches with a point outside the axes view limits.

            .. versionadded:: 3.10
        """
        self._native = _patches.new_path_patch(path)
        # (A `.Patch` has no edge by default; the patch pltrs (Rust) makes
        # of a path has one.)
        self._edgeless = True
        mpatches.Patch.__init__(self, **kwargs)
        self.set_3d_properties(path, zs, zdir, axlim_clip)

    #: The codes of the path in 3-D (matplotlib's attribute): the array
    #: of the path the patch was given, which pltrs (Rust) follows when a
    #: program changes it in place or assigns another.
    _code3d = _properties.Field(_no_field, lambda patch, codes: patch._follow3d(), _follows3d)

    def _geometry3d(self):
        """What `_lift` takes: a path of the codes, the outline and
        whether it is clipped to the view limits."""
        from pltrs.path import Path
        segment3d = self.__dict__["_segment3d"]
        template = Path(np.zeros((len(segment3d), 2)), self.__dict__.get("_code3d"))
        return template, segment3d, self.__dict__.get("_axlim_clip", False)

    def _lift(self, path, segment3d, axlim_clip):
        """Follow *path*, its vertices at *segment3d*, in the 3-D axes
        holding the patch."""
        _mplot3d.pathpatch_2d_to_3d(self.axes._native, self._native, path,
                                    _outline_points(segment3d))
        if axlim_clip:
            _mplot3d.set_axlim_clip(self.axes._native, self._native, True)

    def _follow3d(self):
        # docstring inherited; codes that do not fit the outline fail
        # matplotlib's projection, which makes a path of both at the draw.
        codes = self.__dict__.get("_code3d")
        segment3d = self.__dict__.get("_segment3d")
        if (codes is not None and segment3d is not None and len(codes) != len(segment3d)
                and not _draws_itself(type(self)) and _axes3d(self) is not None):
            _draw_problems.record(self, ValueError(
                "'codes' must be a 1D list or array with the same length of 'vertices'. "
                f"Your vertices have shape ({len(segment3d)}, 2) but your codes have shape "
                f"({len(codes)},)"))
            return
        super()._follow3d()

    def set_3d_properties(self, path, zs=0, zdir='z', axlim_clip=False):
        """
        Set the *z* position and direction of the path patch.

        Parameters
        ----------
        path :
        zs : float
            The location along the *zdir* axis in 3D space to position the
            path patch.
        zdir : {'x', 'y', 'z', 3-tuple}
            Plane to plot path patch orthogonal to. Default: 'z'.
            See `.get_dir_vector` for a description of the values.
        axlim_clip : bool, default: False
            Whether to hide path patches with a point outside the axes view limits.

            .. versionadded:: 3.10
        """
        # (matplotlib reads the vertices first: what is no path says so.)
        vertices = path.vertices
        self.__dict__["_code3d"] = path.codes
        Patch3D.set_3d_properties(self, vertices, zs=zs, zdir=zdir,
                                  axlim_clip=axlim_clip)

    def do_3d_projection(self):
        if _draws_itself(type(self)):
            return self._projected_by_python(self._code3d)
        return _projected_depth(self)


def _change_class(patch, cls):
    """Make *patch* a *cls*, keeping the attributes matplotlib's instance of
    its former class holds (a `.Rectangle`'s ``angle``, ...), which
    outlive the change there."""
    kept = {name: getattr(patch, name) for klass in type(patch).__mro__
            for name in vars(klass).get("_instance_attributes", ())}
    patch.__class__ = cls
    patch.__dict__.update(kept)
    _gains_projection(patch)


def patch_2d_to_3d(patch, z=0, zdir='z', axlim_clip=False):
    """Convert a `.Patch` to a `.Patch3D` object."""
    verts = _get_patch_verts(patch)
    _change_class(patch, Patch3D)
    patch.set_3d_properties(verts, z, zdir, axlim_clip)


def pathpatch_2d_to_3d(pathpatch, z=0, zdir='z'):
    """Convert a `.PathPatch` to a `.PathPatch3D` object."""
    path = pathpatch.get_path()
    trans = pathpatch.get_patch_transform()

    mpath = trans.transform_path(path)
    _change_class(pathpatch, PathPatch3D)
    pathpatch.set_3d_properties(mpath, z, zdir)


class Text3D(mtext.Text):
    """
    Text object with 3D position and direction.

    Parameters
    ----------
    x, y, z : float
        The position of the text.
    text : str
        The text string to display.
    zdir : {'x', 'y', 'z', None, 3-tuple}
        The direction of the text. See `.get_dir_vector` for a description of
        the values.
    axlim_clip : bool, default: False
        Whether to hide text outside the axes view limits.

        .. versionadded:: 3.10

    Other Parameters
    ----------------
    **kwargs
         All other parameters are passed on to `~matplotlib.text.Text`.
    """

    def __init__(self, x=0, y=0, z=0, text='', zdir='z', axlim_clip=False,
                 **kwargs):
        if 'rotation' in kwargs:
            _api.warn_external(
                "The `rotation` parameter has not yet been implemented "
                "and is currently ignored."
            )
        if 'rotation_mode' in kwargs:
            _api.warn_external(
                "The `rotation_mode` parameter has not yet been implemented "
                "and is currently ignored."
            )
        mtext.Text.__init__(self, x, y, text, **kwargs)
        self.set_3d_properties(z, zdir, axlim_clip)

    _axlim_clip = _AXLIM_CLIP

    @property
    def _z(self):
        """The z of the 3-D position (matplotlib's attribute)."""
        if not self._has_3d():
            raise AttributeError(f"'{type(self).__name__}' object has no attribute '_z'",
                                 name="_z", obj=self)
        return self.get_position_3d()[2]

    @_z.setter
    def _z(self, z):
        self._set_coordinate(2, z)

    def _read_direction(self):
        """The direction of the baseline as given, while pltrs (Rust)
        holds it, else as it holds it (pending, for the axes that will add
        the text)."""
        pending = self.__dict__.get("_pending3d")
        if pending is not None:
            return pending[1]
        held = _fields3d.direction(self)
        if held is _properties.MISSING:
            return held
        return _values.given(self, "dir_vec", held, same=np.array_equal)

    def _take_direction(self, direction):
        self._place3d(self.get_position_3d(), direction)
        _values.remember(self, "dir_vec", direction)

    #: The direction of the baseline (matplotlib's attribute, an array of
    #: three): the text's own array, which pltrs (Rust) follows when a
    #: program changes it in place or assigns another.
    _dir_vec = _properties.Field(lambda text: text._read_direction(),
                               lambda text, direction: text._take_direction(direction))

    def _add_to_axes(self, axes, kind, autolim=True):
        # docstring inherited
        super()._add_to_axes(axes, kind, autolim)
        if _axes3d(self) is not None and not _restored(self, '_pending3d'):
            self._lift(*self.__dict__.pop('_pending3d'))
        return self

    def draw(self, renderer):
        # docstring inherited; matplotlib's projects the text's position
        # through the view of the last draw of its axes.
        axes = _lifted(self)
        if axes is not None and axes.M is None:
            raise _no_projection_error()
        super().draw(renderer)

    def _lift(self, position, direction, axlim_clip):
        """Anchor the text at *position*, its baseline along *direction*,
        in the 3-D axes holding it."""
        _mplot3d.text_2d_to_3d(
            self.axes._native, self._native,
            (_text_floats(position), tuple(map(float, direction))),
            bool(axlim_clip))
        _values.remember(self, "dir_vec", direction)
        self._settle(position)

    def _settle(self, position):
        """Keep the 3-D *position* of the text as given, as matplotlib
        keeps it unchecked, recording the problem its draws meet with it."""
        _keep_text_position(self, position)
        _draw_problems.record(self, _draw_problems.text_problem(self, position))

    def _has_3d(self):
        """Whether the text has its 3-D geometry yet (`set_3d_properties`)."""
        return _lifted(self) is not None or '_pending3d' in self.__dict__

    def _place3d(self, position, direction=None):
        """Move the text to the 3-D *position* (kept as given), its
        baseline along *direction* (None: as it is)."""
        axes = _lifted(self)
        if axes is None:
            _, kept, axlim_clip = self._pending3d
            self._pending3d = (tuple(position), kept if direction is None
                               else direction, axlim_clip)
        else:
            _mplot3d.set_text_position(
                axes._native, self._native, _text_floats(position),
                None if direction is None else tuple(map(float, direction)))
            self._settle(position)
        self.stale = True

    def _set_coordinate(self, index, value):
        """Set the coordinate *index* (0, 1, 2 for x, y, z) of the 3-D
        position to *value*."""
        position = list(self.get_position_3d())
        position[index] = value
        self._place3d(position)

    def get_position_3d(self):
        """Return the (x, y, z) position of the text."""
        axes = _lifted(self)
        if axes is None:
            return self._pending3d[0]
        # As given, as matplotlib keeps it.
        return _values.given(self, "position_3d",
                             _mplot3d.text_position(axes._native, self._native))

    def get_position(self):
        # docstring inherited; matplotlib projects the position only while
        # it draws the text.
        return tuple(self.get_position_3d()[:2])

    @_docstring.copy(mtext.Text.set_position)
    def set_position(self, xy):
        # docstring inherited; the x and y of the 3-D position, set one
        # after the other as matplotlib's sets them.
        self.set_x(xy[0])
        self.set_y(xy[1])

    @_docstring.copy(mtext.Text.set_x)
    def set_x(self, x):
        # docstring inherited; the x of the 3-D position.
        if self._has_3d():
            self._set_coordinate(0, x)
        else:
            super().set_x(x)

    @_docstring.copy(mtext.Text.set_y)
    def set_y(self, y):
        # docstring inherited; the y of the 3-D position.
        if self._has_3d():
            self._set_coordinate(1, y)
        else:
            super().set_y(y)

    def get_unitless_position(self):
        # docstring inherited
        return tuple(map(float, self.get_position_3d()[:2]))

    @_docstring.copy(mtext.Text.set_rotation)
    def set_rotation(self, s):
        # docstring inherited; the rotation set, which the projection
        # replaces while the text is drawn.
        super().set_rotation(s)
        self._rotation3d = super().get_rotation()

    def get_rotation(self):
        # docstring inherited
        return self.__dict__.get("_rotation3d", 0.0)

    @property
    def _rotation(self):
        # matplotlib's field: the rotation as set, which its draw replaces
        # with the projected angle only while it draws the text.
        return self.__dict__.get("_rotation3d", 0.0)

    @_rotation.setter
    def _rotation(self, rotation):
        Text3D.set_rotation(self, rotation)

    def set_position_3d(self, xyz, zdir=None):
        """
        Set the (*x*, *y*, *z*) position of the text.

        Parameters
        ----------
        xyz : (float, float, float)
            The position in 3D space.
        zdir : {'x', 'y', 'z', None, 3-tuple}
            The direction of the text. If unspecified, the *zdir* will not be
            changed. See `.get_dir_vector` for a description of the values.
        """
        self.set_position(xyz[:2])
        self.set_z(xyz[2])
        if zdir is not None:
            self._place3d(self.get_position_3d(), get_dir_vector(zdir))

    def set_z(self, z):
        """
        Set the *z* position of the text.

        Parameters
        ----------
        z : float
        """
        self._set_coordinate(2, z)

    def set_3d_properties(self, z=0, zdir='z', axlim_clip=False):
        """
        Set the *z* position and direction of the text.

        Parameters
        ----------
        z : float
            The z-position in 3D space.
        zdir : {'x', 'y', 'z', 3-tuple}
            The direction of the text. Default: 'z'.
            See `.get_dir_vector` for a description of the values.
        axlim_clip : bool, default: False
            Whether to hide text outside the axes view limits.

            .. versionadded:: 3.10
        """
        axes = _axes3d(self)
        if self._has_3d():
            # (Lifted, or pending: set_position moved the pending x and y.)
            x, y = self.get_position_3d()[:2]
        else:
            x, y = mtext.Text.get_position(self)
        geometry = ((x, y, z), get_dir_vector(zdir), axlim_clip)
        if axes is None:
            self._pending3d = geometry
        else:
            self._lift(*geometry)
        self.stale = True

    def get_tightbbox(self, renderer=None):
        # Overwriting the 2d Text behavior which is not valid for 3d.
        # For now, just return None to exclude from layout calculation.
        return None

    def _text_extent(self):
        # matplotlib measures the 2-D text at the x and y of its 3-D
        # position, at its own rotation: the projection moves and turns it
        # only while drawing it.
        axes = _lifted(self)
        if axes is None:
            return super()._text_extent()
        points = _mplot3d.unprojected_text_extent(
            axes._native, self._native, float(self.get_rotation()))
        return None if points is None else tuple(np.ravel(points))


def text_2d_to_3d(obj, z=0, zdir='z', axlim_clip=False):
    """
    Convert a `.Text` to a `.Text3D` object.

    Parameters
    ----------
    z : float
        The z-position in 3D space.
    zdir : {'x', 'y', 'z', 3-tuple}
        The direction of the text. Default: 'z'.
        See `.get_dir_vector` for a description of the values.
    axlim_clip : bool, default: False
        Whether to hide text outside the axes view limits.

        .. versionadded:: 3.10
    """
    obj.__class__ = Text3D
    obj.set_3d_properties(z, zdir, axlim_clip)


# What matplotlib's module also has from its imports, which programs
# import from it.
import math  # noqa: E402, F401
from contextlib import contextmanager  # noqa: E402, F401
from pltrs import artist, path as mpath  # noqa: E402, F401
from pltrs.collections import (  # noqa: E402, F401
    Collection, LineCollection, PatchCollection, PathCollection, PolyCollection)
from pltrs.mpl_toolkits.mplot3d import proj3d  # noqa: E402, F401
from pltrs.patches import Patch  # noqa: E402, F401
