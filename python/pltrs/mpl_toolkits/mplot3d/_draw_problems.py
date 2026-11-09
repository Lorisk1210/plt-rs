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

import warnings

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


def _problem(compute, M=ANY_PROJECTION):
    """The exception *compute* raises projecting through *M*, or None. Run
    for the problems of the data alone (through `ANY_PROJECTION`), it
    issues none of the warnings matplotlib's computation issues where its
    draw projects them."""
    if M is not ANY_PROJECTION:
        return _raised(compute)
    with warnings.catch_warnings(), np.errstate(all="ignore"):
        warnings.simplefilter("ignore")
        return _raised(compute)


def _raised(compute):
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
    return _problem(compute, M)


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
    return _problem(compute, M)


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
    return _problem(compute, M)


def sizes_problem(sizes):
    """The exception ``Path3DCollection.do_3d_projection`` raises sorting
    the sizes *sizes*, as given to ``set_sizes``, by the depth order of
    its markers (an index array), or None: what has no length, and
    several sizes in what is no array. Sizes in an array too short for
    the markers fail with numpy's ``IndexError`` for the first marker in
    depth order beyond them, which pltrs (Rust) raises."""
    def compute():
        if len(sizes) > 1 and not isinstance(sizes, np.ndarray):
            try:
                sizes[np.zeros(1, dtype=np.intp)]
            except LookupError:
                pass
    return _problem(compute)


def patch_problem(patch, segment3d, M=ANY_PROJECTION):
    """The exception the ``do_3d_projection`` of ``Patch3D`` and
    ``PathPatch3D`` raises for the outline *segment3d* of *patch* (in its
    3-D axes) projected through *M*, or None: none for no point, a
    ``float()`` of a ``zip`` for points invalid on their axis scales, which
    matplotlib means to mask. (Its mask of the points outside the view
    limits, with ``axlim_clip``, depends on the limits of the draw.)"""
    def compute():
        axes = patch.axes
        xs0, ys0, zs0 = zip(*segment3d)
        mask = _scale_invalid_mask(xs0, ys0, zs0, axes)
        if np.any(mask):
            xs, ys, zs = np.ma.array(zip(*segment3d), dtype=float,
                                     mask=mask).filled(np.nan)
        else:
            xs, ys, zs = xs0, ys0, zs0
        _project_columns(xs, ys, zs, axes, M)
    return _problem(compute, M)


def paths_problem(collection, verts, M=ANY_PROJECTION):
    """The exception ``Collection3D.do_3d_projection`` raises for the 3-D
    vertices *verts* (an (N, 3) array per path) of *collection* projected
    through *M*, or None: matplotlib masks the invalid points of a path
    with a mask it fails to broadcast to the path's vertices."""
    def compute():
        axes = collection.axes
        masks = [_scale_invalid_mask(*vs.T, axes) for vs in verts]
        if _clips(collection):
            masks = [m | _viewlim_mask(*vs.T, axes) for m, vs in zip(masks, verts)]
        for vs, m in zip(verts, masks):
            if np.any(m):
                vs = np.ma.array(vs, mask=np.broadcast_to(m, vs.shape))
            _project_columns(vs[:, 0], vs[:, 1], vs[:, 2], axes, M)
    return _problem(compute, M)


def codes_problem(codes, count):
    """The exception ``Poly3DCollection.do_3d_projection`` raises reading
    the path *codes* (kept unchecked) of each of its *count* polygons, or
    None: none for no codes, which it draws without."""
    if codes is None or not len(codes):
        return None
    return _problem(lambda: [codes[i] for i in range(count)])


def text_problem(text, position):
    """The exception ``Text3D.draw`` raises for the 3-D *position* of
    *text* (in its 3-D axes), or None: coordinates that are no numbers
    (None, strings), on which numpy's ``isfinite`` fails."""
    def compute():
        axes = text.axes
        mask = _scale_invalid_mask(*position, axes)
        if _clips(text):
            mask |= _viewlim_mask(*position, axes)
        if np.any(mask):
            np.ma.array(position, mask=mask, dtype=float)
        else:
            np.array(position, dtype=float)
    return _problem(compute)


def _norm_angle(a):
    """The angle *a* normalized to -180 < *a* <= 180 degrees (matplotlib's
    ``art3d._norm_angle``)."""
    a = (a + 360) % 360
    if a > 180:
        a = a - 360
    return a


def view_problem(axes, elev, azim, roll):
    """The exception matplotlib's ``get_proj`` of *axes* raises computing
    with the view angles *elev*, *azim* and *roll* (degrees, kept
    unchecked), or None: its computations of the eye and of the view
    axes."""
    def compute():
        R = np.full(3, 0.5)
        elev_rad = np.deg2rad(elev)
        azim_rad = np.deg2rad(azim)
        ps = np.roll([np.cos(elev_rad) * np.cos(azim_rad),
                      np.cos(elev_rad) * np.sin(azim_rad),
                      np.sin(elev_rad)], axes._vertical_axis - 2)
        eye = R + 10 * ps
        elev_rad = np.deg2rad(_norm_angle(elev))
        roll_rad = np.deg2rad(_norm_angle(roll))
        V = np.zeros(3)
        V[axes._vertical_axis] = -1 if abs(elev_rad) > np.pi / 2 else 1
        w = eye - R
        w = w / np.linalg.norm(w)
        u = np.cross(V, w)
        u = u / np.linalg.norm(u)
        v = np.cross(w, u)
        if roll_rad != 0:
            angle = -roll_rad
            vx, vy, vz = w / np.linalg.norm(w)
            s, c = np.sin(angle), np.cos(angle)
            t = 2 * np.sin(angle / 2) ** 2
            rotation = np.array([
                [t * vx * vx + c, t * vx * vy - vz * s, t * vx * vz + vy * s],
                [t * vy * vx + vz * s, t * vy * vy + c, t * vy * vz - vx * s],
                [t * vz * vx - vy * s, t * vz * vy + vx * s, t * vz * vz + c]])
            u = np.dot(rotation, u)
            v = np.dot(rotation, v)
        Mr, Mt = np.eye(4), np.eye(4)
        Mr[:3, :3] = [u, v, w]
        Mt[:3, -1] = -eye

    with np.errstate(all="ignore"):
        return _problem(compute)


def zorder_problem(axes):
    """The exception matplotlib's draw of the 3-D *axes* raises computing
    the z-order its collections and patches start at from the z-orders of
    its axes (kept unchecked, a string say), or None."""
    return _problem(lambda: max(axis.get_zorder() for axis in _axes_of(axes)) + 1)


def axes_problem(axes):
    """The exception matplotlib's draw of the 3-D *axes* raises drawing its
    axes (`.Axis3D`), or None: calling an image filter of an axis that is
    no function, which its draws of the gridlines (when shown) call first,
    then computing with the label pad of an axis (kept unchecked) as it
    draws the axis, which calls the filter again."""
    filters = [_filter_problem(axis.get_agg_filter()) for axis in _axes_of(axes)]
    if _mplot3d.get_grid(axes._native):
        problem = next((problem for problem in filters if problem is not None), None)
        if problem is not None:
            return problem
    for axis, filter_problem in zip(_axes_of(axes), filters):
        problem = _labelpad_problem(axis) or filter_problem
        if problem is not None:
            return problem
    return None


def _labelpad_problem(axis):
    """The exception matplotlib's draw of the 3-D *axis* raises offsetting
    its label by its label pad (kept unchecked), or None."""
    return _problem(lambda: axis.labelpad + 21.)


def _axes_of(axes):
    """The x, y and z axes of the 3-D *axes* (its ``_axis_map``)."""
    return axes.xaxis, axes.yaxis, axes.zaxis


def _filter_problem(agg_filter):
    """The exception calling the image filter *agg_filter* (None for none)
    with what an artist drew raises when it is no function, else None."""
    if agg_filter is None or callable(agg_filter):
        return None
    return TypeError(f"{type(agg_filter).__name__!r} object is not callable")


def sort_zpos_problem(val, M=ANY_PROJECTION):
    """The exception ``Poly3DCollection.do_3d_projection`` raises projecting
    the position *val* it orders the polygons by (``set_sort_zpos``), or
    None."""
    return _problem(lambda: np.dot(M, np.array([[0], [0], [val], [1]])))


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
