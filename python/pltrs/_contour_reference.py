"""
matplotlib's code of the contour labeller (``contour.ContourLabeler``) for
arguments pltrs (Rust) does not take as they are: values that are no
numbers, points that are not finite, lines that are no ``(N, 2)`` arrays of
numbers. It computes with them as matplotlib does, raising what
matplotlib raises and warning as numpy warns; the plain arguments go to
pltrs (Rust), which computes the same answers (see `.contour`).
"""

from numbers import Real

import numpy as np


def is_point(*values):
    """Whether *values* are finite real numbers."""
    return all(isinstance(value, Real) and np.isfinite(value) for value in values)


def is_line(linecontour):
    """Whether *linecontour* is a non-empty ``(N, 2)`` array of numbers."""
    return (isinstance(linecontour, np.ndarray) and linecontour.ndim == 2
            and linecontour.shape[1] == 2 and len(linecontour) > 0
            and linecontour.dtype.kind in "iuf")


def too_close(label_positions, x, y, lw):
    """matplotlib's ``too_close``: whether a label is already near the
    point, of those at *label_positions*."""
    thresh = (1.2 * lw) ** 2
    return any((x - loc[0]) ** 2 + (y - loc[1]) ** 2 < thresh
               for loc in label_positions)


def print_label(linecontour, labelwidth):
    """matplotlib's ``print_label``: whether a contour is long enough to
    hold a label."""
    return (len(linecontour) > 10 * labelwidth
            or (len(linecontour)
                and (np.ptp(linecontour, axis=0) > 1.2 * labelwidth).any()))


def locate_label(labeler, linecontour, labelwidth):
    """matplotlib's ``locate_label``: a good place to draw a label (a
    relatively flat part of the contour) away from *labeler*'s labels."""
    ctr_size = len(linecontour)
    n_blocks = int(np.ceil(ctr_size / labelwidth)) if labelwidth > 1 else 1
    block_size = ctr_size if n_blocks == 1 else int(labelwidth)
    # Split contour into blocks of length ``block_size``, filling the last
    # block by cycling the contour start (per `np.resize` semantics).  (Due
    # to cycling, the index returned is taken modulo ctr_size.)
    xx = np.resize(linecontour[:, 0], (n_blocks, block_size))
    yy = np.resize(linecontour[:, 1], (n_blocks, block_size))
    yfirst = yy[:, :1]
    ylast = yy[:, -1:]
    xfirst = xx[:, :1]
    xlast = xx[:, -1:]
    s = (yfirst - yy) * (xlast - xfirst) - (xfirst - xx) * (ylast - yfirst)
    l = np.hypot(xlast - xfirst, ylast - yfirst)  # noqa: E741  (matplotlib's name)
    # Ignore warning that divide by zero throws, as this is a valid option
    with np.errstate(divide='ignore', invalid='ignore'):
        distances = (abs(s) / l).sum(axis=-1)
    # Labels are drawn in the middle of the block (``hbsize``) where the
    # contour is the closest (per ``distances``) to a straight line, but
    # not `too_close()` to a preexisting label.
    hbsize = block_size // 2
    adist = np.argsort(distances)
    # If all candidates are `too_close()`, go back to the straightest part
    # (``adist[0]``).
    for idx in np.append(adist, adist[0]):
        x, y = xx[idx, hbsize], yy[idx, hbsize]
        if not labeler.too_close(x, y, labelwidth):
            break
    return x, y, (idx * block_size + hbsize) % ctr_size


def find_closest_point_on_path(xys, p):
    """
    matplotlib's ``_find_closest_point_on_path``: the minimum square
    distance of *p* to the polyline *xys*, the projection of *p* onto it,
    and the consecutive indices of the vertices of the segment it lies on.
    """
    if len(xys) == 1:
        return (((p - xys[0]) ** 2).sum(), xys[0], (0, 0))
    dxys = xys[1:] - xys[:-1]  # Individual segment vectors.
    norms = (dxys ** 2).sum(axis=1)
    norms[norms == 0] = 1  # For zero-length segment, replace 0/0 by 0/1.
    rel_projs = np.clip(  # Project onto each segment in relative 0-1 coords.
        ((p - xys[:-1]) * dxys).sum(axis=1) / norms,
        0, 1)[:, None]
    projs = xys[:-1] + rel_projs * dxys  # Projs. onto each segment, in (x, y).
    d2s = ((projs - p) ** 2).sum(axis=1)  # Squared distances.
    imin = np.argmin(d2s)
    return (d2s[imin], projs[imin], (imin, imin+1))


def nearest_contour(paths, transform, xy, indices):
    """
    matplotlib's ``_find_nearest_contour`` over the line *paths* drawn
    with *transform*: the index of the level of the line closest to *xy*,
    the index of the vertex after the closest point, and that point.
    """
    if indices is None:
        indices = range(len(paths))
    d2min = np.inf
    idx_level_min = idx_vtx_min = proj_min = None
    for idx_level in indices:
        path = paths[idx_level]
        idx_vtx_start = 0
        for subpath in path._iter_connected_components():
            if not len(subpath.vertices):
                continue
            lc = transform.transform(subpath.vertices)
            d2, proj, leg = find_closest_point_on_path(lc, xy)
            if d2 < d2min:
                d2min = d2
                idx_level_min = idx_level
                idx_vtx_min = leg[1] + idx_vtx_start
                proj_min = proj
            idx_vtx_start += len(subpath)
    return idx_level_min, idx_vtx_min, proj_min


def find_nearest_contour(paths, transform, x, y, indices):
    """matplotlib's ``find_nearest_contour`` over the line *paths* drawn
    with *transform* (an identity one to measure in axes space)."""
    segment = index = d2 = None
    i_level, i_vtx, (xmin, ymin) = nearest_contour(paths, transform, (x, y), indices)
    if i_level is not None:
        cc_cumlens = np.cumsum(
            [*map(len, paths[i_level]._iter_connected_components())])
        segment = cc_cumlens.searchsorted(i_vtx, "right")
        index = i_vtx if segment == 0 else i_vtx - cc_cumlens[segment - 1]
        d2 = (xmin-x)**2 + (ymin-y)**2
    return (i_level, segment, index, xmin, ymin, d2)
