"""
The geometry of paths (matplotlib's C++ ``_path`` module), computed by
pltrs (Rust) (``_native.path`` and ``_native.transforms``).

The functions keep the C++ module's signatures: paths are `.Path` objects,
transforms are anything with a 3×3 matrix (an affine `.Transform`, an
array) or None, boxes are `.Bbox` objects or their 2×2 points.
"""

import numpy as np

from . import _pybind
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


def _rect_points(rect):
    """A box (its points, or its bounds) as its 2x2 points; None is the
    empty box at the origin, as matplotlib's ``rect_d`` takes it."""
    if rect is None:
        return np.zeros((2, 2))
    return np.asarray(rect, dtype=float).reshape((2, 2))


def _iterated(path):
    """*path* as matplotlib's ``PathIterator`` takes it: None as an empty
    path."""
    if path is None:
        from .path import Path
        return Path(np.empty((0, 2)))
    return path


def _trailing_shape(array, name, size):
    """Raise matplotlib's errors for an array that is not ``(N, size)``
    (``check_trailing_shape``; an empty 2-D array passes)."""
    if array.ndim != 2:
        raise ValueError(f"Expected 2-dimensional array, got {array.ndim}")
    if array.size and array.shape[1] != size:
        raise ValueError(f"{name} must have shape (N, {size}), got "
                         f"({array.shape[0]}, {array.shape[1]})")


def affine_transform(points, trans):
    """Transform the ``(N, 2)`` (or ``(2,)``) *points* by the matrix *trans*
    (``_native.transforms`` reads *points* as ``np.asarray(points, float)``)."""
    return _transforms.affine_transform(points, _matrix(trans))


def count_bboxes_overlapping_bbox(bbox, bboxes):
    """Count how many of *bboxes* overlap *bbox*."""
    bbox = np.asarray(bbox, float).reshape((2, 2))
    others = np.array([np.asarray(b, float).reshape((2, 2)) for b in bboxes])
    return _transforms.count_overlaps(bbox, others.reshape((-1, 2, 2)))


@_pybind.binds(
    "(path: PathIterator, trans: trans_affine, remove_nans: bool, clip_rect: rect_d, "
    "snap_mode: e_snap_mode, stroke_width: typing.SupportsFloat | typing.SupportsIndex, "
    "simplify: bool | None, return_curves: bool, sketch: SketchParams) -> tuple",
    method=False)
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
    return _path.cleanup_path(_iterated(path), _matrix(trans), options)


@_pybind.binds(
    "(x: typing.SupportsFloat | typing.SupportsIndex, y: typing.SupportsFloat | "
    "typing.SupportsIndex, radius: typing.SupportsFloat | typing.SupportsIndex, "
    "path: PathIterator, trans: trans_affine) -> bool", method=False)
def point_in_path(x, y, radius, path, trans):
    """Whether the point (*x*, *y*) is inside *path* grown by *radius*."""
    return _path.point_in_path(x, y, radius, _iterated(path), _matrix(trans))


@_pybind.binds(
    "(points: typing.Annotated[numpy.typing.ArrayLike, numpy.float64], radius: "
    "typing.SupportsFloat | typing.SupportsIndex, path: PathIterator, trans: "
    "trans_affine) -> numpy.typing.NDArray[numpy.float64]", method=False)
def points_in_path(points, radius, path, trans):
    """Whether each of the ``(N, 2)`` *points* is inside *path*."""
    points = np.asarray(points, float)
    _trailing_shape(points, "points", 2)
    inside = _path.points_in_path(points.reshape((-1, 2)), radius, _iterated(path),
                                  _matrix(trans))
    return np.asarray(inside, dtype=float)


@_pybind.binds(
    "(path_a: PathIterator, trans_a: trans_affine, path_b: PathIterator, "
    "trans_b: trans_affine) -> bool", method=False)
def path_in_path(path_a, trans_a, path_b, trans_b):
    """Whether *path_a* (by *trans_a*) contains *path_b* (by *trans_b*)."""
    return _path.path_in_path(_iterated(path_a), _matrix(trans_a), _iterated(path_b),
                              _matrix(trans_b))


@_pybind.binds("(path1: PathIterator, path2: PathIterator, filled: bool = False) -> bool",
               method=False)
def path_intersects_path(path1, path2, filled=False):
    """Whether paths *path1* and *path2* intersect."""
    return _path.path_intersects_path(_iterated(path1), _iterated(path2), bool(filled))


@_pybind.binds(
    "(path: PathIterator, rect_x1: typing.SupportsFloat | typing.SupportsIndex, "
    "rect_y1: typing.SupportsFloat | typing.SupportsIndex, rect_x2: "
    "typing.SupportsFloat | typing.SupportsIndex, rect_y2: typing.SupportsFloat | "
    "typing.SupportsIndex, filled: bool = False) -> bool", method=False)
def path_intersects_rectangle(path, rect_x1, rect_y1, rect_x2, rect_y2,
                              filled=False):
    """Whether *path* intersects the rectangle."""
    return _path.path_intersects_rectangle(
        _iterated(path), (rect_x1, rect_y1, rect_x2, rect_y2), bool(filled))


@_pybind.binds(
    "(path: PathIterator, trans: trans_affine, width: typing.SupportsFloat | "
    "typing.SupportsIndex = 0.0, height: typing.SupportsFloat | typing.SupportsIndex = "
    "0.0, closed_only: bool = False) -> list", method=False)
def convert_path_to_polygons(path, trans, width=0.0, height=0.0, closed_only=False):
    """*path* (by *trans*) as a list of ``(N, 2)`` polygons."""
    return _path.convert_path_to_polygons(
        _iterated(path), _matrix(trans), width, height, bool(closed_only))


@_pybind.binds("(path: PathIterator, rect: rect_d, inside: bool) -> list", method=False)
def clip_path_to_rect(path, rect, inside):
    """The polygons of *path* clipped to the box *rect*."""
    return _path.clip_path_to_rect(_iterated(path), _rect_points(rect), bool(inside))


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
