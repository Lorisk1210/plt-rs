"""
Module containing 3D artist code and functions to convert 2D
artists into 3D versions which can be added to an Axes3D (matplotlib's
``mpl_toolkits.mplot3d.art3d``).

A 3-D artist is a 2-D artist of the projection plane of its `.Axes3D`
plus a 3-D geometry, which pltrs (Rust) keeps and projects at every draw:
the classes here read and replace that geometry.
"""

import numpy as np

from pltrs import _api, cbook, collections as mcoll, lines, text as mtext
from pltrs._native import mplot3d as _mplot3d


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
        an `.Axes3D` projects it; it serves as a legend handle.
        """
        super().__init__(xs, ys, *args, **kwargs)
        self._verts3d = _columns((xs, ys, zs))
        self._axlim_clip = axlim_clip

    def get_data_3d(self):
        """
        Get the current data.

        Returns
        -------
        verts3d : length-3 tuple or array-like
            The current data as a tuple or array-like.
        """
        if self.axes is None:
            return self._verts3d
        return tuple(np.array(c) for c in
                     _mplot3d.line_data(self.axes._native, self._native))

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
        if self.axes is None:
            self._verts3d = _columns(args)
            return
        _mplot3d.set_line_data(self.axes._native, self._native, _columns(args))
        self.stale = True


def _vertices_3d(verts):
    """The (N, 3) float arrays of a sequence of polygons or polylines."""
    return [np.asarray(v, dtype=float).reshape(-1, 3) for v in verts]


class _Loose3D:
    """
    What a 3-D collection made before an `.Axes3D` adds it holds: its 3-D
    vertices (the loose 2-D collection holds its properties), handed to
    pltrs (Rust) by `.Axes3D.add_collection3d`.
    """

    _zsort = None
    _closed = True

    def _adopt_into(self, axes, autolim, axlim_clip):
        """Join *axes* (a 3-D axes) with the 3-D vertices; return self."""
        from pltrs import _handles
        native = _mplot3d.adopt_collection3d(
            axes._native, self._native, self._verts3d_pending,
            (self._zsort, bool(self._closed)), bool(autolim),
            bool(self._axlim_clip or axlim_clip))
        del self._verts3d_pending
        return _handles.adopt(self, native, axes.figure)


class Line3DCollection(_Loose3D, mcoll.LineCollection):
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


class Path3DCollection(mcoll.PathCollection):
    """A collection of 3D paths: the markers of a 3-D scatter plot."""

    @property
    def _offsets3d(self):
        """The 3-D positions of the markers."""
        return tuple(np.array(c) for c in
                     _mplot3d.scatter_offsets(self.axes._native, self._native))

    @_offsets3d.setter
    def _offsets3d(self, offsets):
        _mplot3d.set_scatter_offsets(self.axes._native, self._native,
                                     _columns(offsets))
        self.stale = True


    def get_depthshade(self):
        """Return whether the markers fade with depth."""
        return _mplot3d.get_depthshade(self.axes._native, self._native)

    def set_depthshade(self, depthshade, depthshade_minalpha=None):
        """
        Set whether depth shading is performed on collection members.

        Parameters
        ----------
        depthshade : bool
            Whether to shade the patches in order to give the appearance of
            depth.
        depthshade_minalpha : float, default: None
            Sets the minimum alpha value used by depth-shading.
        """
        if depthshade_minalpha is not None:
            raise NotImplementedError(
                "pltrs: set_depthshade(depthshade_minalpha=...)")
        _mplot3d.set_depthshade(self.axes._native, self._native,
                                bool(depthshade))
        self.stale = True


class Poly3DCollection(_Loose3D, mcoll.PolyCollection):
    """
    A collection of 3D polygons.

    .. note::
        **Filling of 3D polygons**

        There is no simple definition of the enclosed surface of a 3D polygon
        unless the polygon is planar.
    """

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
        axlim_clip : bool, default: False
            Whether to hide polygons with a vertex outside the view limits.
        closed : bool, default: True
            Whether the polygons are closed.
        *args, **kwargs
            All other parameters are forwarded to `.PolyCollection`.
        """
        if shade:
            if (kwargs.get('facecolors') is None
                    and kwargs.get('edgecolors') is None):
                raise ValueError(
                    "You must provide facecolors, edgecolors, or both for "
                    "shade to work.")
            raise NotImplementedError("pltrs: Poly3DCollection(shade=True)")
        if lightsource is not None:
            raise NotImplementedError("pltrs: Poly3DCollection(lightsource=...)")
        if isinstance(verts, np.ndarray):
            if verts.ndim != 3:
                raise ValueError('verts must be a list of (N, 3) array-like')
        elif any(len(np.shape(vert)) != 2 for vert in verts):
            raise ValueError('verts must be a list of (N, 3) array-like')
        super().__init__([], *args, **kwargs)
        _api.check_in_list(['average', 'min', 'max'], zsort=zsort)
        self._verts3d_pending = _vertices_3d(verts)
        self._zsort = zsort
        self._closed = closed
        self._axlim_clip = axlim_clip


class Text3D(mtext.Text):
    """Text object with 3D position and direction."""

    def get_position_3d(self):
        """Return the (x, y, z) position of the text."""
        return _mplot3d.text_position(self.axes._native, self._native)
