"""
The computations matplotlib's 3-D artists make of their data before they
project them (``Line3D.draw`` and the ``do_3d_projection`` of
``Line3DCollection``, ``Patch3DCollection`` and ``Path3DCollection``),
kept from matplotlib's ``art3d`` and ``proj3d`` (Matplotlib License) for
the exceptions they raise.

matplotlib accepts data these computations fail on (a line through points
of two coordinates, segments of 2-D points, positions whose coordinates
differ in length) and raises only when its draw projects them. pltrs
(Rust) projects and draws the data itself; the package runs these
computations when it is given the data, to know whether and how
matplotlib's draw fails, and records the exception with pltrs (Rust)
(`record`), whose draw of the `.Axes3D` raises it where matplotlib's
does. Where the program projects or draws an artist itself, they run
through the axes' projection of the last draw (``M``), and raise what
matplotlib raises there (before the first draw, numpy's error for a
product with None).
"""

import numpy as np

from pltrs._native import mplot3d as _mplot3d

#: The projection the computations run through when they look for the
#: problems of the data alone: the identity, which any data multiply.
ANY_PROJECTION = np.eye(4)


def _scale_invalid_mask(xs, ys, zs, axes):
    """The mask of the points whose coordinates are invalid for the axis
    scales they live on (matplotlib's)."""
    return np.logical_or.reduce((
        np.logical_not(axes.xaxis._scale.val_in_range(xs)),
        np.logical_not(axes.yaxis._scale.val_in_range(ys)),
        np.logical_not(axes.zaxis._scale.val_in_range(zs))))


def _viewlim_mask(xs, ys, zs, axes):
    """The mask of the points outside the axes view limits (matplotlib's)."""
    return np.logical_or.reduce((xs < axes.xy_viewLim.xmin,
                                 xs > axes.xy_viewLim.xmax,
                                 ys < axes.xy_viewLim.ymin,
                                 ys > axes.xy_viewLim.ymax,
                                 zs < axes.zz_viewLim.xmin,
                                 zs > axes.zz_viewLim.xmax))


def _apply_scale_transforms(xs, ys, zs, axes):
    """The coordinates through the axis scales (matplotlib's)."""
    def transform_coord(coord, axis):
        coord = np.asanyarray(coord)
        data = np.ma.getdata(coord).ravel()
        return axis.get_transform().transform(data).reshape(coord.shape)

    return (transform_coord(xs, axes.xaxis), transform_coord(ys, axes.yaxis),
            transform_coord(zs, axes.zaxis))


def _vec_pad_ones(xs, ys, zs):
    """The homogeneous coordinates of the points (matplotlib's)."""
    if np.ma.isMA(xs) or np.ma.isMA(ys) or np.ma.isMA(zs):
        return np.ma.array([xs, ys, zs, np.ones_like(xs)])
    return np.array([xs, ys, zs, np.ones_like(xs)])


def _project_columns(xs, ys, zs, axes, M):
    """The points through the scales and *M* (``_scale_proj_transform_clip``)."""
    vec = _vec_pad_ones(*_apply_scale_transforms(xs, ys, zs, axes))
    np.dot(M, vec.data)


def _clips(artist):
    """Whether *artist* hides its points outside the view limits
    (matplotlib's ``_axlim_clip``): as set from Python, else as pltrs
    (Rust) made it."""
    clips = artist.__dict__.get("_axlim_clip")
    if clips is None:
        clips = _mplot3d.get_axlim_clip(artist.axes._native, artist._native)
    return bool(clips)


def _problem(compute):
    """The exception *compute* raises, or None."""
    try:
        compute()
    except (AttributeError, TypeError, ValueError, IndexError) as error:
        return error
    return None


def line_problem(line, verts, M=ANY_PROJECTION):
    """The exception ``Line3D.draw`` raises for the data *verts* of *line*
    (in its 3-D axes) projected through *M*, or None."""
    def compute():
        axes = line.axes
        mask = _scale_invalid_mask(*verts, axes)
        if _clips(line):
            mask |= _viewlim_mask(*verts, axes)
        if np.any(mask):
            mask = np.broadcast_to(mask, (len(verts), *verts[0].shape))
            xs, ys, zs = np.ma.array(verts, dtype=float, mask=mask).filled(np.nan)
        else:
            xs, ys, zs = verts
        _project_columns(xs, ys, zs, axes, M)
    return _problem(compute)


def segments_problem(collection, segments, M=ANY_PROJECTION):
    """The exception ``Line3DCollection.do_3d_projection`` raises for the
    *segments* of *collection* projected through *M*, or None."""
    def compute():
        lengths = [len(segment) for segment in segments]
        if len(set(lengths)) > 1:
            if any(np.ma.isMA(segment) for segment in segments):
                points = np.ma.concatenate(segments)
            else:
                points = np.concatenate(segments)
        else:
            points = np.asanyarray(segments)
        if points.size == 0:
            return
        axes = collection.axes
        columns = points[..., 0], points[..., 1], points[..., 2]
        _scale_invalid_mask(*columns, axes)
        if _clips(collection):
            _viewlim_mask(*columns, axes)
        xs, ys, zs = _apply_scale_transforms(*columns, axes)
        np.dot(M, _vec_pad_ones(xs.ravel(), ys.ravel(), zs.ravel()))
    return _problem(compute)


def offsets_problem(collection, offsets, M=ANY_PROJECTION):
    """The exception the ``do_3d_projection`` of ``Patch3DCollection`` and
    ``Path3DCollection`` raises for the 3-D positions *offsets* of
    *collection* projected through *M*, or None."""
    def compute():
        axes = collection.axes
        mask = False
        for xyz in offsets:
            if np.ma.isMA(xyz):
                mask = mask | xyz.mask
        mask = mask | _scale_invalid_mask(*offsets, axes)
        if _clips(collection):
            mask = mask | _viewlim_mask(*offsets, axes)
        if np.any(mask):
            mask = np.broadcast_to(mask, (len(offsets), *offsets[0].shape))
            xyzs = np.ma.array(offsets, mask=mask)
        else:
            xyzs = offsets
        _project_columns(*xyzs, axes, M)
    return _problem(compute)


def record(artist, problem):
    """Record *problem*, the exception matplotlib's draw raises for
    *artist*, an artist of a 3-D axes (None: its draw meets none), with
    pltrs (Rust), whose draw raises it where matplotlib's does."""
    if problem is None and not artist.__dict__.pop("_draw_problem", False):
        return
    if problem is not None:
        artist._draw_problem = True
    _mplot3d.set_draw_failure(
        artist.axes._native, artist._native,
        None if problem is None else (type(problem).__name__, str(problem)))


def missing_projection(artist):
    """The exception matplotlib's draw of a 3-D axes raises for *artist*, a
    2-D collection or patch it asks for its 3-D projection."""
    return AttributeError(f"{type(artist).__name__!r} object has no "
                          f"attribute 'do_3d_projection'")
