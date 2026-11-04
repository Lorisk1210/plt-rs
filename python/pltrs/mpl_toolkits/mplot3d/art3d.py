"""
Module containing 3D artist code and functions to convert 2D
artists into 3D versions which can be added to an Axes3D (matplotlib's
``mpl_toolkits.mplot3d.art3d``).

A 3-D artist is a 2-D artist of the projection plane of its `.Axes3D`
plus a 3-D geometry, which pltrs (Rust) keeps and projects at every draw:
the classes here read and replace that geometry.
"""

import numpy as np

from pltrs import _api, _values, cbook, collections as mcoll, colors as mcolors, lines
from pltrs import rcParams  # noqa: F401  (matplotlib's art3d has it)
from pltrs import patches as mpatches, text as mtext
from pltrs._native import artist as _artist, lines as _lines, mplot3d as _mplot3d
from pltrs._native import patches as _patches
from pltrs.axes import _program_locators


def _columns(arrays):
    """The coordinate arrays broadcast together, flattened to floats."""
    return tuple(np.asarray(c, dtype=float).ravel()
                 for c in np.broadcast_arrays(*arrays))


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
    columns = tuple(np.asarray(c, dtype=float).ravel().tolist()
                    for c in (xs, ys, zs))
    return _mplot3d.all_points_on_plane(columns, float(atol))


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


def _lifted(artist):
    """The 3-D axes in which *artist* has its 3-D geometry, or None (an
    artist outside any 3-D axes, or a 2-D one of its projection plane)."""
    axes = _axes3d(artist)
    if axes is None or _mplot3d.get_axlim_clip(axes._native,
                                               artist._native) is None:
        return None
    return axes


def _faces_3d(verts):
    """
    The (N, 3) float arrays of a sequence of polygons, refused as numpy
    refuses to put points of another dimension into matplotlib's array of
    faces.
    """
    faces = [np.asarray(vert, dtype=float) for vert in verts]
    for face in faces:
        if face.ndim == 2 and face.shape[1] != 3:
            raise ValueError(f"could not broadcast input array from shape "
                             f"({len(face)},{face.shape[1]}) into shape "
                             f"({len(face)},3)")
    return _vertices_3d(faces)


def _vertices_3d(verts):
    """
    The (N, 3) float arrays of a sequence of polygons or polylines; 2-D
    points lie in the plane z = 0.
    """
    vertices = []
    for vert in verts:
        vert = np.asarray(vert, dtype=float)
        if vert.ndim == 2 and vert.shape[1] == 2:
            vert = np.column_stack([vert, np.zeros(len(vert))])
        vertices.append(vert.reshape(-1, 3))
    return vertices


class Line3D(lines.Line2D):
    """
    3D line object.

    .. note:: Use `get_data_3d` to obtain the data associated with the line.
            `~.Line2D.get_data`, `~.Line2D.get_xdata`, and `~.Line2D.get_ydata`
            return the x- and y-coordinates of the projected 2D-line, not the
            x- and y-data of the 3D-line. Similarly, use `set_data_3d` to set
            the data, not `~.Line2D.set_data`, `~.Line2D.set_xdata`, and
            `~.Line2D.set_ydata`.
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

        Notes
        -----
        A line made this way, outside any axes, keeps its 3-D data until
        an `.Axes3D` adds it (`~.Axes.add_line`); meanwhile it serves as a
        legend handle.
        """
        super().__init__([], [], *args, **kwargs)
        self.set_data_3d(xs, ys, zs)
        self._axlim_clip = axlim_clip

    def _add_to_axes(self, axes, kind, autolim=True):
        # docstring inherited
        super()._add_to_axes(axes, kind, autolim)
        if _axes3d(self) is not None:
            _mplot3d.line_2d_to_3d(axes._native, self._native,
                                   self.__dict__.pop('_pending3d'),
                                   bool(self._axlim_clip))
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
        axlim_clip : bool, default: False
            Whether to hide lines with an endpoint outside the axes view limits.
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
        Get the current data.

        Returns
        -------
        verts3d : length-3 tuple or array-like
            The current data as a tuple or array-like.
        """
        axes = _axes3d(self)
        native = (self._pending3d if axes is None else tuple(
            np.array(c) for c in _mplot3d.line_data(axes._native, self._native)))
        # The data as given, as matplotlib keeps them.
        return _values.given(self, "data_3d", native)

    def set_data_3d(self, *args):
        """
        Set the x, y and z data.

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
        if axes is None:
            self._pending3d = _columns(args)
        else:
            _mplot3d.set_line_data(axes._native, self._native, _columns(args))
        _values.remember(self, "data_3d", args)
        self.stale = True

    def _drawn_data(self, getter):
        """The projected data *getter* reads off the native line, masked as
        matplotlib's `draw` gives them to the line, once a draw projected
        it; else None."""
        axes = _lifted(self)
        mask = (None if axes is None
                else _mplot3d.projected_line_mask(axes._native, self._native))
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
    axlim_clip : bool, default: False
        Whether to hide lines with an endpoint outside the axes view limits.
    """
    line.__class__ = Line3D
    line.set_3d_properties(zs, zdir, axlim_clip)


class _Loose3D:
    """
    What a 3-D collection made before an `.Axes3D` adds it holds: its 3-D
    vertices (the loose 2-D collection holds its properties), handed to
    pltrs (Rust) when a 3-D axes adds it (`.Axes3D.add_collection3d`,
    `~.Axes.add_collection`, `~.Axes.add_artist`).
    """

    _zsort = None
    _closed = True
    #: What a loose collection keeps for the axes that will add it: the
    #: position it is ordered by, and the codes of its 3-D polygons.
    _sort_zpos = None
    _codes3d = None

    def do_3d_projection(self):
        """Project the collection in the view of the last draw and return
        the depth of its nearest point (NaN without points)."""
        return _projected_depth(self)

    def set_sort_zpos(self, val):
        """Set the position to use for z-sorting."""
        axes = _axes3d(self)
        if axes is None:
            self._sort_zpos = val
        else:
            _mplot3d.set_sort_zpos(axes._native, self._native,
                                   None if val is None else float(val))
        self.stale = True

    def _add_to_axes(self, axes, kind, autolim=True):
        # docstring inherited
        if getattr(axes, 'name', None) != '3d':
            return super()._add_to_axes(axes, kind, autolim)
        return self._adopt_into(axes, kind == 'collection' and bool(autolim),
                                False)

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
            self.set_sort_zpos(pending.pop('_sort_zpos'))
        return self


class Collection3D(_ProjectedPaths, _Loose3D, mcoll.Collection):
    """A collection of 3D paths."""

    def _native_geometry(self):
        # docstring inherited
        return ("paths", (self.__dict__.pop('_verts3d_pending'),))


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
    col._axlim_clip = axlim_clip
    axes = _axes3d(col)
    if axes is None:
        col._verts3d_pending = verts
    else:
        _mplot3d.lift_paths(axes._native, col._native,
                            [np.asarray(v, dtype=float) for v in verts],
                            bool(axlim_clip))


class Line3DCollection(_ProjectedPaths, _Loose3D, mcoll.LineCollection):
    """A collection of 3D lines."""

    def __init__(self, lines, axlim_clip=False, **kwargs):
        """
        Parameters
        ----------
        lines : list of (N, 3) array-like
            A sequence ``[line0, line1, ...]`` where each line is a (N, 3)-shape
            array-like containing points.
        axlim_clip : bool, default: False
            Whether to hide lines with an endpoint outside the axes view limits.
        **kwargs
            Keyword arguments are passed onto `.LineCollection`.
        """
        super().__init__([], **kwargs)
        self._verts3d_pending = _vertices_3d(lines)
        self._axlim_clip = axlim_clip

    def set_segments(self, segments):
        """
        Set 3D segments.
        """
        axes = _lifted(self)
        if axes is None:
            self._verts3d_pending = _vertices_3d(segments)
        else:
            _mplot3d.set_segments3d(axes._native, self._native,
                                    _vertices_3d(segments))
        super().set_segments([])


def line_collection_2d_to_3d(col, zs=0, zdir='z', axlim_clip=False):
    """Convert a `.LineCollection` to a `.Line3DCollection` object."""
    col.__class__ = Line3DCollection
    col._verts3d_pending = _lifted_paths(col, zs, zdir, finite=True)
    col._axlim_clip = axlim_clip


def _lifted_paths(col, zs, zdir, finite=False):
    """
    The paths of the 2-D collection *col* placed at *zs* along *zdir*: one
    position for all, one per path, or one per vertex of each path;
    without their non-finite points with *finite* (matplotlib's
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
        path_zs = np.broadcast_to(np.asarray(path_zs, dtype=float), len(vertices))
        if finite:
            keep = np.isfinite(vertices).all(axis=1)
            vertices, path_zs = vertices[keep], path_zs[keep]
        lifted.append(np.column_stack(juggle_axes(
            vertices[:, 0], vertices[:, 1], path_zs, zdir)))
    return lifted


class _Lifted3DOffsets(_Loose3D):
    """
    What places the members of a 2-D collection at 3-D positions, its
    offsets lifted along an axis (matplotlib's `Patch3DCollection` and
    `Path3DCollection`): pltrs (Rust) projects the positions and shades
    the members with depth.
    """

    def __init__(self, *args, zs=0, zdir="z", depthshade=None,
                 depthshade_minalpha=None, axlim_clip=False, **kwargs):
        """
        Create a collection of flat 3D patches (or paths) with its normal
        vector pointed in *zdir* direction, and located at *zs* on the
        *zdir* axis. 'zs' can be a scalar or an array-like of the same
        length as the number of members in the collection.

        Constructor arguments are the same as for
        :class:`~matplotlib.collections.PatchCollection` (or
        :class:`~matplotlib.collections.PathCollection`). In addition,
        keywords *zs=0* and *zdir='z'* are available.

        The keyword argument *depthshade* is available to indicate whether
        or not to shade the members in order to give the appearance of
        depth (default is *True*). *depthshade_minalpha* sets the minimum
        alpha value applied by depth-shading.
        """
        super().__init__(*args, **kwargs)
        self._depthshade_pending = (depthshade, depthshade_minalpha)
        self.set_3d_properties(zs, zdir, axlim_clip)

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
        """
        Set the *z* positions and direction of the members.

        Parameters
        ----------
        zs : float or array of floats
            The location or locations to place the members in the collection
            along the *zdir* axis.
        zdir : {'x', 'y', 'z'}
            Plane to plot the members orthogonal to.
        axlim_clip : bool, default: False
            Whether to hide members with a vertex outside the axes view limits.
        """
        self._axlim_clip = axlim_clip
        axes = _lifted(self)
        if axes is None:
            self._offsets3d_pending = (zs, zdir)
        else:
            offsets = self.get_offsets()
            xs, ys = offsets.T if len(offsets) else ([], [])
            self._offsets3d = juggle_axes(xs, ys, np.atleast_1d(zs), zdir)
        self.stale = True

    @property
    def _offsets3d(self):
        """The 3-D positions of the members, as given."""
        unprojectable = self.__dict__.get("_unprojectable")
        if unprojectable is not None:
            return unprojectable
        return _values.given(self, "_offsets3d", tuple(
            np.array(c) for c in
            _mplot3d.scatter_offsets(self.axes._native, self._native)))

    @_offsets3d.setter
    def _offsets3d(self, offsets):
        try:
            columns = _columns(offsets)
        except ValueError:
            # Coordinates that do not broadcast together: matplotlib keeps
            # them, and fails to project them when it draws.
            self._keep_unprojectable(offsets)
        else:
            self.__dict__.pop("_unprojectable", None)
            self.__dict__.pop("_offsets_unprojected", None)
            _mplot3d.set_scatter_offsets(self.axes._native, self._native, columns)
            _values.remember(self, "_offsets3d", offsets)
        self.stale = True

    def _keep_unprojectable(self, offsets):
        """
        Keep *offsets*, 3-D positions whose coordinates do not broadcast
        together, as matplotlib keeps them: its draws fail to project them
        (raising as `_locate_by_program` does, before every layout of a
        draw), and the offsets stay those of the last projection.
        """
        if "_unprojectable" not in self.__dict__:
            self._offsets_unprojected = self.get_offsets()
        self._unprojectable = offsets
        _program_locators.follow(self, self.get_figure(root=True))

    def _locate_by_program(self, renderer):
        """Fail as matplotlib's ``do_3d_projection`` fails on the kept
        positions that do not broadcast together, when the members are
        drawn: gathering the masks of the coordinates outside the domains
        of the axes' scales raises numpy's error."""
        offsets = self.__dict__.get("_unprojectable")
        if (offsets is None or self.axes is None or not self.get_visible()
                or not self.axes.get_visible()):
            return
        axes = self.axes
        np.logical_or.reduce(tuple(
            np.logical_not(axis._scale.val_in_range(values))
            for axis, values in zip((axes.xaxis, axes.yaxis, axes.zaxis), offsets)))

    def get_offsets(self):
        # docstring inherited; those of the last projection while the
        # positions kept cannot be projected.
        kept = self.__dict__.get("_offsets_unprojected")
        return super().get_offsets() if kept is None else kept

    def get_depthshade(self):
        """Return whether the members fade with depth."""
        axes = _lifted(self)
        if axes is None:
            from pltrs import rcParams
            depthshade = self._depthshade_pending[0]
            return (rcParams['axes3d.depthshade'] if depthshade is None
                    else depthshade)
        return _mplot3d.get_depthshade(axes._native, self._native)

    def set_depthshade(self, depthshade, depthshade_minalpha=None):
        """
        Set whether depth shading is performed on collection members.

        Parameters
        ----------
        depthshade : bool
            Whether to shade the members in order to give the appearance of
            depth.
        depthshade_minalpha : float, default: :rc:`axes3d.depthshade_minalpha`
            Sets the minimum alpha value used by depth-shading.
        """
        axes = _lifted(self)
        if axes is None:
            self._depthshade_pending = (depthshade, depthshade_minalpha)
        else:
            _mplot3d.set_depthshade(axes._native, self._native,
                                    bool(depthshade),
                                    _depthshade_minalpha(depthshade_minalpha))
        self.stale = True


class Patch3DCollection(_Lifted3DOffsets, mcoll.PatchCollection):
    """
    A collection of 3D patches.
    """


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
        The location or locations to place the patches in the collection
        along the *zdir* axis. Default: 0.
    zdir : {'x', 'y', 'z'}
        The axis in which to place the patches. Default: "z".
    depthshade : bool, default: :rc:`axes3d.depthshade`
        Whether to shade the patches to give a sense of depth.
    axlim_clip : bool, default: False
        Whether to hide patches with a vertex outside the axes view limits.
    depthshade_minalpha : float, default: :rc:`axes3d.depthshade_minalpha`
        Sets the minimum alpha value used by depth-shading.
    """
    if isinstance(col, mcoll.PathCollection):
        col.__class__ = Path3DCollection
    elif isinstance(col, mcoll.PatchCollection):
        col.__class__ = Patch3DCollection
    col._depthshade_pending = (depthshade, depthshade_minalpha)
    col.set_3d_properties(zs, zdir, axlim_clip)


class Path3DCollection(_Lifted3DOffsets, mcoll.PathCollection):
    """A collection of 3D paths: the markers of a 3-D scatter plot."""

    # matplotlib's draw of the class is not decorated with
    # `allow_rasterization`: asking to rasterize it warns.
    _ignores_rasterization = True


class Poly3DCollection(_ProjectedPaths, _Loose3D, mcoll.PolyCollection):
    """
    A collection of 3D polygons.

    .. note::
        **Filling of 3D polygons**

        There is no simple definition of the enclosed surface of a 3D polygon
        unless the polygon is planar.
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
                 lightsource=None, axlim_clip=False, closed=True, **kwargs):
        """
        Parameters
        ----------
        verts : list of (N, 3) array-like
            The sequence of polygons [*verts0*, *verts1*, ...] where each
            element *verts_i* defines the vertices of polygon *i* as a 2D
            array-like of shape (N, 3).
        zsort : {'average', 'min', 'max'}, default: 'average'
            The calculation method for the z-order.
        shade : bool, default: False
            Whether to shade *facecolors* and *edgecolors*.
        lightsource : `~matplotlib.colors.LightSource`, optional
            The lightsource to use when *shade* is True.
        axlim_clip : bool, default: False
            Whether to hide polygons with a vertex outside the view limits.
        closed : bool, default: True
            Whether the polygons are closed.
        *args, **kwargs
            All other parameters are forwarded to `.PolyCollection`.
        """
        if isinstance(verts, np.ndarray):
            _api.check_shape((None, None, 3), segments3d=verts)
        elif any(len(np.shape(vert)) != 2 for vert in verts):
            raise ValueError('verts must be a list of (N, 3) array-like')
        faces = _faces_3d(verts)
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
        super().__init__([], *args, **kwargs)
        _api.check_in_list(['average', 'min', 'max'], zsort=zsort)
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
        """The face colours the polygons are drawn with, in polygon order."""
        return _mplot3d.poly_colors(self.axes._native, self._native)[0]

    @property
    def _edgecolor3d(self):
        """The edge colours the polygons are drawn with, in polygon order."""
        return _mplot3d.poly_colors(self.axes._native, self._native)[1]

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
        super().set_facecolor(colors)
        self._refresh_colors(True, False)

    def set_edgecolor(self, colors):
        # docstring inherited
        super().set_edgecolor(colors)
        self._refresh_colors(False, True)

    def set_alpha(self, alpha):
        # docstring inherited
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
        """Take the polygons *segments3d* (a list of ``(N, 3)`` arrays, or
        an ``(M, N, 3)`` array) as the 3-D vertices to project."""
        if isinstance(segments3d, np.ndarray):
            _api.check_shape((None, None, 3), segments3d=segments3d)
        self._set_faces(_vertices_3d(segments3d), self._closed)

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
        self._set_faces(_vertices_3d(verts), closed)

    def set_verts_and_codes(self, verts, codes):
        """Set 3D vertices with path codes."""
        faces = _vertices_3d(verts)
        axes = _axes3d(self)
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
        """Take the colours the collection has now as the colours of its
        polygons, ordered by the mean depth of their vertices."""
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


def poly_collection_2d_to_3d(col, zs=0, zdir='z', axlim_clip=False):
    """
    Convert a `.PolyCollection` into a `.Poly3DCollection` object.

    Parameters
    ----------
    col : `~matplotlib.collections.PolyCollection`
        The collection to convert.
    zs : float or array of floats
        The location or locations to place the polygons in the collection
        along the *zdir* axis. Default: 0.
    zdir : {'x', 'y', 'z'}
        The axis in which to place the patches. Default: 'z'.
    axlim_clip : bool, default: False
        Whether to hide patches with a vertex outside the axes view limits.
    """
    col.__class__ = Poly3DCollection
    col._verts3d_pending = _lifted_paths(col, zs, zdir)
    # Its alpha has faded its edges, as that of a 2-D collection does.
    col._lifted_from_2d = True
    col._zsort = 'average'
    col._closed = True
    col._axlim_clip = axlim_clip


def _get_patch_verts(patch):
    """Return a list of vertices for the path of a patch."""
    trans = patch.get_patch_transform()
    path = patch.get_path()
    polygons = path.to_polygons(trans)
    return polygons[0] if len(polygons) else np.array([])


class Patch3D(mpatches.Patch):
    """
    3D patch object.

    The patch follows its 3-D outline once a 3-D axes holds it; made
    outside one, it keeps the outline until a 3-D axes adds it.
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
        axlim_clip : bool, default: False
            Whether to hide patches with a vertex outside the axes view limits.
        """
        super().__init__(*args, **kwargs)
        self.set_3d_properties(zs, zdir, axlim_clip)

    def _add_to_axes(self, axes, kind, autolim=True):
        # docstring inherited
        super()._add_to_axes(axes, kind, autolim)
        if _axes3d(self) is not None:
            self._lift(*self.__dict__.pop('_pending3d'))
        return self

    def _lift(self, segment3d, axlim_clip):
        """Follow the outline *segment3d* in the 3-D axes holding the patch."""
        _mplot3d.patch_2d_to_3d(self.axes._native, self._native,
                                np.asarray(segment3d, dtype=float).reshape(-1, 3),
                                bool(axlim_clip))

    def do_3d_projection(self):
        """Project the patch in the view of the last draw and return the
        depth of its nearest point."""
        return _projected_depth(self)

    def _place(self, *geometry):
        """Lift the patch with *geometry*, or keep it for the 3-D axes
        that will add the patch."""
        if _axes3d(self) is None:
            self._pending3d = geometry
        else:
            self._lift(*geometry)
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
        axlim_clip : bool, default: False
            Whether to hide patches with a vertex outside the axes view limits.
        """
        zs = np.broadcast_to(zs, len(verts))
        segment3d = [juggle_axes(x, y, z, zdir)
                     for ((x, y), z) in zip(verts, zs)]
        self._place(segment3d, axlim_clip)

    def get_path(self):
        # docstring inherited
        from pltrs.path import Path
        return Path(*_mplot3d.patch_path(self.axes._native, self._native))

    def get_patch_transform(self):
        # docstring inherited
        from pltrs.transforms import IdentityTransform
        return IdentityTransform()


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
        axlim_clip : bool, default: False
            Whether to hide path patches with a point outside the axes view
            limits.
        """
        self._native = _patches.new_path_patch(path)
        mpatches.Patch.__init__(self, **kwargs)
        self.set_3d_properties(path, zs, zdir, axlim_clip)

    def _lift(self, path, segment3d, axlim_clip):
        """Follow *path*, its vertices at *segment3d*, in the 3-D axes
        holding the patch."""
        _mplot3d.pathpatch_2d_to_3d(
            self.axes._native, self._native, path,
            np.asarray(segment3d, dtype=float).reshape(-1, 3))

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
        axlim_clip : bool, default: False
            Whether to hide path patches with a point outside the axes view
            limits.
        """
        verts = path.vertices
        zs = np.broadcast_to(zs, len(verts))
        segment3d = [juggle_axes(x, y, z, zdir)
                     for ((x, y), z) in zip(verts, zs)]
        self._place(path, segment3d, axlim_clip)


def _change_class(patch, cls):
    """Make *patch* a *cls*, keeping the attributes matplotlib's instance of
    its former class holds (a `.Rectangle`'s ``angle``, ...), which
    outlive the change there."""
    kept = {name: getattr(patch, name) for klass in type(patch).__mro__
            for name in vars(klass).get("_instance_attributes", ())}
    patch.__class__ = cls
    patch.__dict__.update(kept)


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
        if _axes3d(self) is not None:
            self._lift(*self.__dict__.pop('_pending3d'))
        return self

    def _lift(self, position, direction, axlim_clip):
        """Anchor the text at *position*, its baseline along *direction*,
        in the 3-D axes holding it."""
        _mplot3d.text_2d_to_3d(
            self.axes._native, self._native,
            (tuple(map(float, position)), tuple(map(float, direction))),
            bool(axlim_clip))

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

    def set_position(self, xy):
        # docstring inherited; the x and y of the 3-D position.
        self.set_x(xy[0])
        self.set_y(xy[1])

    def set_x(self, x):
        # docstring inherited; the x of the 3-D position.
        if _lifted(self) is None:
            super().set_x(x)
        else:
            _, y, z = self.get_position_3d()
            self.set_position_3d((x, y, z))

    def set_y(self, y):
        # docstring inherited; the y of the 3-D position.
        if _lifted(self) is None:
            super().set_y(y)
        else:
            x, _, z = self.get_position_3d()
            self.set_position_3d((x, y, z))

    def get_unitless_position(self):
        # docstring inherited
        return tuple(map(float, self.get_position_3d()[:2]))

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
        direction = None if zdir is None else get_dir_vector(zdir)
        axes = _lifted(self)
        if axes is None:
            _, kept, axlim_clip = self._pending3d
            self._pending3d = (tuple(xyz), kept if direction is None
                               else direction, axlim_clip)
        else:
            _mplot3d.set_text_position(
                axes._native, self._native, tuple(map(float, xyz)),
                None if direction is None else tuple(map(float, direction)))
            _values.remember(self, "position_3d", tuple(xyz))
        self.stale = True

    def set_z(self, z):
        """
        Set the *z* position of the text.

        Parameters
        ----------
        z : float
        """
        x, y, _ = self.get_position_3d()
        self.set_position_3d((x, y, z))

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
            _values.remember(self, "position_3d", (x, y, z))
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
    """
    obj.__class__ = Text3D
    obj.set_3d_properties(z, zdir, axlim_clip)
