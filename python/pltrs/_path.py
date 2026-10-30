"""
The geometry of paths (matplotlib's C++ ``_path`` module), computed by
pltrs (Rust) (``_native.path`` and ``_native.transforms``).

The functions keep the C++ module's signatures: paths are `.Path` objects,
transforms are anything with a 3×3 matrix (an affine `.Transform`, an
array) or None, boxes are `.Bbox` objects or their 2×2 points.
"""

import numpy as np

from ._native import path as _path
from ._native import transforms as _transforms


def _matrix(trans):
    """The 3×3 matrix of *trans* (None stays None)."""
    return None if trans is None else np.asarray(trans, dtype=float)


def _rect(rect):
    """A rectangle (a box or its points) as ``(x0, y0, x1, y1)``, or None."""
    if rect is None:
        return None
    (x0, y0), (x1, y1) = np.asarray(rect, dtype=float).reshape((2, 2))
    return x0, y0, x1, y1


def affine_transform(points, trans):
    """Transform the ``(N, 2)`` (or ``(2,)``) *points* by the matrix *trans*."""
    return _transforms.affine_transform(np.asarray(points, float), _matrix(trans))


def count_bboxes_overlapping_bbox(bbox, bboxes):
    """Count how many of *bboxes* overlap *bbox*."""
    bbox = np.asarray(bbox, float).reshape((2, 2))
    others = np.array([np.asarray(b, float).reshape((2, 2)) for b in bboxes])
    return _transforms.count_overlaps(bbox, others.reshape((-1, 2, 2)))


def cleanup_path(path, trans, remove_nans, clip_rect, snap_mode, stroke_width,
                 simplify, return_curves, sketch):
    """
    Transform, clean (of NaNs), clip, snap and simplify *path*; return its
    vertices and codes, ending with a ``STOP``.
    """
    clip = _rect(clip_rect) if clip_rect is not None and len(clip_rect) else None
    options = {
        "remove_nans": bool(remove_nans), "clip": clip, "snap": snap_mode,
        "stroke_width": float(stroke_width), "simplify": simplify,
        "curves": bool(return_curves),
        "sketch": None if sketch is None else tuple(map(float, sketch)),
    }
    return _path.cleanup_path(path, _matrix(trans), options)


def point_in_path(x, y, radius, path, trans):
    """Whether the point (*x*, *y*) is inside *path* grown by *radius*."""
    return _path.point_in_path(x, y, radius, path, _matrix(trans))


def points_in_path(points, radius, path, trans):
    """Whether each of the ``(N, 2)`` *points* is inside *path*."""
    points = np.asarray(points, float).reshape((-1, 2))
    return _path.points_in_path(points, radius, path, _matrix(trans))


def path_in_path(a, atrans, b, btrans):
    """Whether path *a* (by *atrans*) contains path *b* (by *btrans*)."""
    return _path.path_in_path(a, _matrix(atrans), b, _matrix(btrans))


def path_intersects_path(p1, p2, filled=True):
    """Whether paths *p1* and *p2* intersect."""
    return _path.path_intersects_path(p1, p2, bool(filled))


def path_intersects_rectangle(path, rect_x1, rect_y1, rect_x2, rect_y2,
                              filled=False):
    """Whether *path* intersects the rectangle."""
    return _path.path_intersects_rectangle(
        path, (rect_x1, rect_y1, rect_x2, rect_y2), bool(filled))


def convert_path_to_polygons(path, trans, width=0, height=0, closed_only=False):
    """*path* (by *trans*) as a list of ``(N, 2)`` polygons."""
    return _path.convert_path_to_polygons(
        path, _matrix(trans), width, height, bool(closed_only))


def clip_path_to_rect(path, rect, inside):
    """The polygons of *path* clipped to the box *rect*."""
    return _path.clip_path_to_rect(
        path, np.asarray(rect, float).reshape((2, 2)), bool(inside))


def get_path_collection_extents(master_transform, paths, transforms, offsets,
                                offset_transform):
    """The extents (2×2 points) and minimum positive coordinates of the
    paths of a collection."""
    return _path.get_path_collection_extents(
        _matrix(master_transform), list(paths),
        [_matrix(t) for t in transforms],
        np.asarray(offsets, float).reshape((-1, 2)), _matrix(offset_transform))


def point_in_path_collection(x, y, radius, master_transform, paths, transforms,
                             offsets, offset_trans, filled):
    """The indices of the items of a collection containing (*x*, *y*)."""
    return _path.point_in_path_collection(
        (x, y, radius), _matrix(master_transform), list(paths),
        [_matrix(t) for t in transforms],
        (np.asarray(offsets, float).reshape((-1, 2)), _matrix(offset_trans)),
        bool(filled))


def convert_to_string(path, trans, clip_rect, simplify, sketch, precision,
                      codes, postfix):
    """*path* as text: one command a line, with the five tokens *codes*."""
    clip = _rect(clip_rect) if clip_rect is not None and len(clip_rect) else None
    return _path.convert_to_string(
        path, _matrix(trans), clip, simplify,
        None if sketch is None else tuple(map(float, sketch)), int(precision),
        [c.decode() if isinstance(c, bytes) else c for c in codes], bool(postfix))


def is_sorted_and_has_non_nan(array):
    """Whether the non-NaN values of *array*, a 1D array of numbers, are
    increasing, and there is at least one."""
    array = np.asarray(array)
    if array.ndim != 1:
        raise ValueError("array must be 1D")
    try:
        array = array.astype(float)
    except (TypeError, ValueError) as err:
        raise ValueError(f"array of type {array.dtype} is not numeric") from err
    return _path.is_sorted_and_has_non_nan(array)
