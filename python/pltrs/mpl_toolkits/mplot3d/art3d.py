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

from pltrs import _api, _unusable, _values, cbook, collections as mcoll, colors as mcolors, lines
from pltrs import rcParams  # noqa: F401  (matplotlib's art3d has it)
from pltrs import _docstring, _lazy_imports, patches as mpatches, text as mtext
from pltrs._native import artist as _artist, lines as _lines, mplot3d as _mplot3d
from pltrs._native import patches as _patches

_draw_problems = _lazy_imports.module(globals(), "._draw_problems")


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
            self.__dict__.pop("_kept_paths", None)
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
        """The x, y and z data."""
        return self.get_data_3d()

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
        axes = _axes3d(self)
        native = (self._pending3d if axes is None else tuple(
            np.array(c) for c in _mplot3d.line_data(axes._native, self._native)))
        # The data as given, as matplotlib keeps them.
        return _values.given(self, "data_3d", native)

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

    @_docstring.copy(lines.Line2D.set_data)
    def set_data(self, *args):
        # docstring inherited; the 2-D data, which the next draw replaces
        # with the projection of the 3-D data, as matplotlib's draw sets it.
        super().set_data(*args)
        self._data_set_at = _artist.draw_count(self._native)

    def _numbers(self):
        # docstring inherited; a line lifted in a 3-D axes keeps the numbers
        # of its projection: matplotlib converts the 2-D data of none.
        if _lifted(self) is None:
            return super()._numbers()
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

    _zsort = None
    _closed = True
    #: What a loose collection keeps for the axes that will add it: the
    #: position it is ordered by, and the codes of its 3-D polygons.
    _sort_zpos = None
    _codes3d = None

    def do_3d_projection(self):
        # Project the collection in the view of the last draw and return
        # the depth of its nearest point (NaN without points).
        return _projected_depth(self)

    def _place_sort_zpos(self, val):
        """`set_sort_zpos` of the collection classes: a position of no
        number is kept as given (the order stays as it was), and fails the
        draws of polygons, which matplotlib projects it for."""
        axes = _axes3d(self)
        if axes is None:
            self._sort_zpos = val
        elif val is None or (np.ndim(val) == 0
                             and np.asarray(val).dtype.kind in 'biuf'):
            _mplot3d.set_sort_zpos(axes._native, self._native,
                                   None if val is None else float(val))
            if self.__dict__.pop('_sort_zpos_fails', False):
                _draw_problems.record(self, None)
        elif isinstance(self, Poly3DCollection):
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
        return ("polygons", (verts, self._zsort, bool(self._closed),
                             self.__dict__.pop('_lifted_from_2d', False)))

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
        return self


class Collection3D(_ProjectedPaths, _Loose3D, mcoll.Collection):
    """A collection of 3D paths."""

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
        self._keep_segments(lines)

    def set_segments(self, segments):
        """
        Set 3D segments.
        """
        self._keep_segments(segments)
        super().set_segments([])

    def _keep_segments(self, segments):
        """Make *segments* the 3-D segments, kept as given
        (``_segments3d``): projected by pltrs (Rust) in a 3-D axes (else
        once one adds the collection), but for segments matplotlib's draw
        fails to project, on which the axes' draw raises as it does."""
        self._segments3d = segments
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
        _draw_problems.record(
            self, _draw_problems.segments_problem(self, self._segments3d))
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
    segments3d = _lifted_paths(col, zs, zdir, finite=True)
    col.__class__ = Line3DCollection
    _gains_projection(col)
    col._verts3d_pending = col._segments3d = segments3d
    col._axlim_clip = axlim_clip


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
class _Lifted3DOffsets(_Loose3D):

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
        return _values.given(self, "_offsets3d", tuple(
            np.array(c) for c in
            _mplot3d.scatter_offsets(self.axes._native, self._native)))

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
        return super().get_facecolor()

    def get_edgecolor(self):
        return super().get_edgecolor()


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


class Path3DCollection(_Lifted3DOffsets, mcoll.PathCollection):
    """
    A collection of 3D paths.
    """

    # matplotlib's draw of the class is not decorated with
    # `allow_rasterization`: asking to rasterize it warns.
    _ignores_rasterization = True

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
        return super().get_facecolor()

    def get_edgecolor(self):
        return super().get_edgecolor()


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
        changed in place, they colour the polygons (nilearn's contours of
        surface regions)."""
        from pltrs._changing_arrays import ChangingArray
        return ChangingArray(_mplot3d.poly_colors(self.axes._native, self._native)[0],
                             self.set_facecolor)

    @property
    def _edgecolor3d(self):
        """The edge colours the polygons are drawn with, in polygon order:
        changed in place, they colour the edges."""
        from pltrs._changing_arrays import ChangingArray
        return ChangingArray(_mplot3d.poly_colors(self.axes._native, self._native)[1],
                             self.set_edgecolor)

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
        # docstring inherited; the colours of the last projection.
        self._project_once()
        return super().get_facecolor()

    def get_edgecolor(self):
        # docstring inherited; the colours of the last projection.
        self._project_once()
        return super().get_edgecolor()

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
        if self.get_array() is not None:
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
            self._sort_zpos = None
        else:
            _mplot3d.set_zsort(axes._native, self._native, zsort)
        self.stale = True

    def _set_faces(self, faces, closed):
        """Make the polygons those of the 3-D vertices *faces*."""
        axes = _axes3d(self)
        if axes is None:
            self._verts3d_pending = faces
            self._closed = closed
        else:
            _mplot3d.set_poly_verts(axes._native, self._native, faces,
                                    bool(closed))
        self.stale = True

    @_api.deprecated("3.10")
    def get_vector(self, segments3d):
        if isinstance(segments3d, np.ndarray):
            _api.check_shape((None, None, 3), segments3d=segments3d)
        self._set_faces(_faces_3d(segments3d), self._closed)

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
        self._set_faces(_faces_3d(verts), closed)

    def set_verts_and_codes(self, verts, codes):
        """Set 3D vertices with path codes."""
        faces = _faces_3d(verts)
        axes = _axes3d(self)
        if codes is None or not len(codes) or len(codes) < len(faces):
            # matplotlib keeps the codes unchecked: its projection draws
            # without none, and fails indexing too few by face.
            self._set_faces(faces, False)
            self._codes3d = codes
            self._codes_problem = _draw_problems.codes_problem(codes, len(faces))
            self._record_projection_problem()
            return
        self.__dict__.pop("_codes_problem", None)
        codes = codes[:len(faces)]
        if axes is None:
            self._verts3d_pending = faces
            self._faces3d = faces
            self._codes3d = codes
            self._closed = False
        else:
            _mplot3d.set_poly_verts_and_codes(
                axes._native, self._native, faces,
                [np.asarray(c, np.uint8).tolist() for c in codes])
        self.stale = True

    def set_3d_properties(self, axlim_clip=False):
        self._axlim_clip = axlim_clip
        axes = _axes3d(self)
        if axes is None:
            self._zsort = 'average'
            self._sort_zpos = None
        else:
            self.update_scalarmappable()
            _mplot3d.set_poly_3d_properties(axes._native, self._native)
            _mplot3d.set_axlim_clip(axes._native, self._native, bool(axlim_clip))
        self.stale = True

    def set_sort_zpos(self, val):
        """Set the position to use for z-sorting."""
        self._place_sort_zpos(val)

    def do_3d_projection(self):
        """
        Perform the 3D projection for this object.
        """
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
    col._verts3d_pending = _lifted_paths(col, zs, zdir)
    # Its alpha has faded its edges, as that of a 2-D collection does.
    col._lifted_from_2d = True
    col._zsort = 'average'
    col._closed = True
    col._axlim_clip = axlim_clip


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

    def _add_to_axes(self, axes, kind, autolim=True):
        # docstring inherited
        super()._add_to_axes(axes, kind, autolim)
        if _axes3d(self) is not None and not _restored(self, '_pending3d'):
            self._settle(*self.__dict__.pop('_pending3d'))
        return self

    def _lift(self, segment3d, axlim_clip):
        """Follow the outline *segment3d* in the 3-D axes holding the patch."""
        _mplot3d.patch_2d_to_3d(self.axes._native, self._native,
                                _outline_points(segment3d), bool(axlim_clip))

    def _settle(self, *geometry):
        """Lift the patch with *geometry* (its outline last but one) in the
        3-D axes holding it, recording the problem matplotlib's draws meet
        projecting the outline."""
        problem = _draw_problems.patch_problem(self, geometry[-2])
        self._lift(*geometry)
        _draw_problems.record(self, problem)

    def do_3d_projection(self):
        return _projected_depth(self)

    def _place(self, *geometry):
        """Lift the patch with *geometry*, or keep it for the 3-D axes
        that will add the patch."""
        if _axes3d(self) is None:
            self._pending3d = geometry
        else:
            self._settle(*geometry)
        self.stale = True

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
        segment3d = [juggle_axes(x, y, z, zdir)
                     for ((x, y), z) in zip(verts, zs)]
        self._place(segment3d, axlim_clip)

    def get_path(self):
        # docstring inherited; matplotlib projects a patch never projected
        # in the view of now, which becomes that of the last draw (``M``).
        from pltrs.path import Path
        if not (self.__dict__.get("_projections")
                or _artist.draw_count(self._native)):
            self.axes.M = self.axes.get_proj()
            self.do_3d_projection()
        if self.axes is None:
            # Removed: the path of its last projection (``_path2d``).
            return Path(*_patches.get_path(self._native))
        return Path(*_mplot3d.patch_path(self.axes._native, self._native))

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
        mpatches.Patch.__init__(self, **kwargs)
        self.set_3d_properties(path, zs, zdir, axlim_clip)

    def _lift(self, path, segment3d, axlim_clip):
        """Follow *path*, its vertices at *segment3d*, in the 3-D axes
        holding the patch."""
        _mplot3d.pathpatch_2d_to_3d(self.axes._native, self._native, path,
                                    _outline_points(segment3d))

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
        verts = path.vertices
        zs = np.broadcast_to(zs, len(verts))
        segment3d = [juggle_axes(x, y, z, zdir)
                     for ((x, y), z) in zip(verts, zs)]
        self._place(path, segment3d, axlim_clip)

    def do_3d_projection(self):
        return super().do_3d_projection()


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
        if _lifted(self) is not None:
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
