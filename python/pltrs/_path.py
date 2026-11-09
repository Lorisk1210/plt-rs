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
    """The 3×3 matrix of *trans* (None stays None), refused with the error
    of pybind11's conversion to ``agg::trans_affine`` for what is none."""
    if trans is None:
        return None
    matrix = _array(trans)
    if matrix is None or matrix.shape != (3, 3):
        raise ValueError("Invalid affine transformation matrix")
    return matrix


def _affine(trans):
    """The 3×3 matrix of *trans*, the identity for None (as pybind11 makes
    an ``agg::trans_affine`` of None)."""
    return np.eye(3) if trans is None else _matrix(trans)


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


def _trailing_shape(array, name, *sizes):
    """Raise matplotlib's errors for an array that is not ``(N, *sizes)``
    (``check_trailing_shape``; an empty array of as many dimensions
    passes)."""
    ndim = len(sizes) + 1
    if array.ndim != ndim:
        raise ValueError(f"Expected {ndim}-dimensional array, got {array.ndim}")
    if array.size and array.shape[1:] != sizes:
        raise ValueError(f"{name} must have shape (N, {', '.join(map(str, sizes))}), "
                         f"got {array.shape}")


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


@_pybind.binds(
    "(master_transform: trans_affine, paths: PathGenerator, transforms: "
    "typing.Annotated[numpy.typing.ArrayLike, numpy.float64], offsets: "
    "typing.Annotated[numpy.typing.ArrayLike, numpy.float64], offset_transform: "
    "trans_affine) -> tuple", method=False, paths=_pybind.ACCEPTS["object"])
def get_path_collection_extents(master_transform, paths, transforms, offsets,
                                offset_transform):
    """The extents (2×2 points) and minimum positive coordinates of the
    paths of a collection."""
    transforms = np.asarray(transforms, dtype=float)
    offsets = np.asarray(offsets, dtype=float)
    _trailing_shape(transforms, "transforms", 3, 3)
    _trailing_shape(offsets, "points", 2)
    # Each path as matplotlib's ``PathGenerator`` takes it when it reaches
    # it, which is in order.
    for path in paths:
        _pybind.path_iterator(path)
    return _path.get_path_collection_extents(
        _affine(master_transform), [_iterated(path) for path in paths],
        list(transforms), offsets.reshape((-1, 2)), _affine(offset_transform))


def point_in_path_collection(x, y, radius, master_transform, paths, transforms,
                             offsets, offset_trans, filled):
    """The indices of the items of a collection containing (*x*, *y*)."""
    return _path.point_in_path_collection(
        (x, y, radius), _matrix(master_transform), list(paths),
        [np.asarray(t, dtype=float) for t in transforms],
        (np.asarray(offsets, float).reshape((-1, 2)), _matrix(offset_trans)),
        bool(filled))


#: matplotlib's signature of ``convert_to_string``, as its docstring gives it.
_CONVERT_TO_STRING = (
    "(path: PathIterator, trans: trans_affine, clip_rect: rect_d, simplify: bool | None, "
    "sketch: SketchParams, precision: typing.SupportsInt | typing.SupportsIndex, codes: "
    "typing.Annotated[collections.abc.Sequence[str], \"FixedSize(5)\"], postfix: bool) "
    "-> object")


def _converted(name, signature, args):
    """Convert the arguments *args* of the function *name* (its
    *signature*) to the C++ types of matplotlib's ``_path`` module, in
    order, as pybind11 does, for its errors: a path reads its vertices and
    codes (None is an empty path), a transform must be a 3×3 matrix and a box 2×2 or 4 values
    (``ValueError``), a flag a truth value or None (else no call binds),
    sketch parameters three numbers (``RuntimeError``)."""
    for kind, value in args:
        if kind == "path" and value is not None:
            value.vertices, value.codes  # noqa: B018
        elif kind == "trans":
            _matrix(value)
        elif kind == "rect" and value is not None:
            rect = _array(value)
            if rect is None or rect.shape not in ((2, 2), (4,)):
                raise ValueError("Invalid bounding box")
        elif kind == "flag" and not (value is None or isinstance(value, np.bool_)
                                     or hasattr(type(value), "__bool__")):
            raise _pybind.incompatible(name, [signature], [v for _, v in args], {})
        elif kind == "sketch" and value is not None and not (
                isinstance(value, (tuple, list)) and len(value) == 3
                and all(_pybind.double(v) for v in value)):
            raise _pybind.cast_error(value)


def _array(value):
    """*value* as pybind11's ``py::array_t<double>::ensure`` makes it an
    array of doubles, or None for one it cannot."""
    try:
        return np.asarray(value, dtype=float)
    except (TypeError, ValueError):
        return None


def convert_to_string(path, trans, clip_rect, simplify, sketch, precision,
                      codes, postfix):
    """*path* as text: one command a line, with the five tokens *codes*."""
    _converted("convert_to_string", _CONVERT_TO_STRING, [
        ("path", path), ("trans", trans), ("rect", clip_rect), ("flag", simplify),
        ("sketch", sketch), ("precision", precision), ("codes", codes),
        ("flag", postfix)])
    if path is None:
        return b''  # The empty path pybind11 makes of None.
    clip = _rect(clip_rect) if clip_rect is not None and len(clip_rect) else None
    return _path.convert_to_string(
        path, _matrix(trans), clip, None if simplify is None else bool(simplify),
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
