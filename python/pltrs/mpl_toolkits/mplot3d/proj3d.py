"""
Various transforms used for by the 3D code (matplotlib's
``mpl_toolkits.mplot3d.proj3d``): homogeneous projection matrices, which
pltrs (Rust) computes with numpy's rounding.
"""

import numpy as np

from pltrs import _api
from pltrs._native import mplot3d as _mplot3d


def _vec3(v):
    """The three numbers of the sequence *v*."""
    x, y, z = np.asarray(v, dtype=float)
    return float(x), float(y), float(z)


def world_transformation(xmin, xmax,
                         ymin, ymax,
                         zmin, zmax, pb_aspect=None):
    """
    Produce a matrix that scales homogeneous coords in the specified ranges
    to [0, 1], or [0, pb_aspect[i]] if the plotbox aspect ratio is specified.
    """
    dx = xmax - xmin
    dy = ymax - ymin
    dz = zmax - zmin
    if pb_aspect is not None:
        ax, ay, az = pb_aspect
        dx /= ax
        dy /= ay
        dz /= az

    return np.array([[1/dx,    0,    0, -xmin/dx],
                     [   0, 1/dy,    0, -ymin/dy],
                     [   0,    0, 1/dz, -zmin/dz],
                     [   0,    0,    0,        1]])


def _view_axes(E, R, V, roll):
    """
    Get the unit viewing axes in data coordinates.

    Parameters
    ----------
    E : 3-element numpy array
        The coordinates of the eye/camera.
    R : 3-element numpy array
        The coordinates of the center of the view box.
    V : 3-element numpy array
        Unit vector in the direction of the vertical axis.
    roll : float
        The roll angle in radians.

    Returns
    -------
    u : 3-element numpy array
        Unit vector pointing towards the right of the screen.
    v : 3-element numpy array
        Unit vector pointing towards the top of the screen.
    w : 3-element numpy array
        Unit vector pointing out of the screen.
    """
    u, v, w = _mplot3d.view_axes(_vec3(E), _vec3(R), _vec3(V), float(roll))
    return np.array(u), np.array(v), np.array(w)


def _view_transformation_uvw(u, v, w, E):
    """
    Return the view transformation matrix.

    Parameters
    ----------
    u : 3-element numpy array
        Unit vector pointing towards the right of the screen.
    v : 3-element numpy array
        Unit vector pointing towards the top of the screen.
    w : 3-element numpy array
        Unit vector pointing out of the screen.
    E : 3-element numpy array
        The coordinates of the eye/camera.
    """
    return _mplot3d.view_transformation((_vec3(u), _vec3(v), _vec3(w)),
                                        _vec3(E))


def _persp_transformation(zfront, zback, focal_length):
    """The perspective projection between the planes *zfront* and
    *zback*."""
    return _mplot3d.persp_transformation(float(zfront), float(zback),
                                         float(focal_length))


def _ortho_transformation(zfront, zback):
    """The orthographic projection between the planes *zfront* and
    *zback*."""
    # Of the type of the planes, as matplotlib's matrix holds them with
    # integers: integer planes give an integer matrix.
    return _mplot3d.ortho_transformation(float(zfront), float(zback)).astype(
        np.array([zfront, zback, 0]).dtype)


def _vec_pad_ones(xs, ys, zs):
    """The points in homogeneous coordinates, a row per coordinate, as
    numpy makes the array of them (of coordinates of other lengths, none)."""
    if np.ma.isMA(xs) or np.ma.isMA(ys) or np.ma.isMA(zs):
        return np.ma.array([xs, ys, zs, np.ones_like(xs)])
    else:
        return np.array([xs, ys, zs, np.ones_like(xs)])


def _native_takes(vec, M):
    """Whether pltrs (Rust) projects the padded points *vec* through *M*:
    numbers through a 4 x 4 matrix of numbers, neither masked, the points
    finite (numpy's product warns on infinite ones)."""
    return (not np.ma.isMA(vec) and not np.ma.isMA(M) and np.shape(M) == (4, 4)
            and np.asarray(M).dtype.kind in "biuf" and vec.dtype.kind in "biuf"
            and vec.ndim <= 2 and not np.isinf(vec).any())


def _native_rows(vec, M, through):
    """The rows x, y and z of the padded points *vec* (numbers, see
    `_native_takes`) through *M* by the native function *through*."""
    columns = tuple(np.ravel(row).astype(float).tolist() for row in vec[:3])
    return np.reshape(through(np.asarray(M, dtype=float), columns),
                      (3,) + vec.shape[1:])


def _projected_rows(vec, M):
    """The rows x/w, y/w and z/w of the padded points *vec* through *M*
    (matplotlib's ``np.dot(M, vec.data)`` divided by its last row): by
    pltrs (Rust), else as numpy computes them, raising what it raises (for
    a matrix of no 4 x 4 numbers, the matrix of an axes not drawn yet)."""
    if _native_takes(vec, M):
        return _native_rows(vec, M, _mplot3d.transform)
    vecw = np.dot(M, vec.data)
    return vecw[0:3] / vecw[3]


def inv_transform(xs, ys, zs, invM):
    """
    Transform the points by the inverse of the projection matrix, *invM*.
    """
    vec = _vec_pad_ones(xs, ys, zs)
    if _native_takes(vec, invM):
        # matplotlib divides each point by its w unless that is zero.
        rows = _native_rows(vec.reshape((4, -1)), invM, _mplot3d.inverse_transform)
        return rows[0], rows[1], rows[2]
    vecr = np.dot(invM, vec)
    if vecr.shape == (4,):
        vecr = vecr.reshape((4, 1))
    for i in range(vecr.shape[1]):
        if vecr[3][i] != 0:
            vecr[:, i] = vecr[:, i] / vecr[3][i]
    return vecr[0], vecr[1], vecr[2]


def proj_transform(xs, ys, zs, M):
    """
    Transform the points by the projection matrix *M*.
    """
    vec = _vec_pad_ones(xs, ys, zs)
    ts = _projected_rows(vec, M)
    if np.ma.isMA(vec):
        ts = np.ma.array(ts, mask=vec.mask)
    return ts[0], ts[1], ts[2]


@_api.deprecated("3.10")
def proj_transform_clip(xs, ys, zs, M):
    vec = _vec_pad_ones(xs, ys, zs)
    txs, tys, tzs = _projected_rows(vec, M)
    tis = np.ones(txs.shape, dtype=bool)
    for row in vec[:3]:
        if np.ma.isMA(row):
            tis = tis & ~row.mask
    return (np.ma.masked_array(txs, ~tis), np.ma.masked_array(tys, ~tis),
            np.ma.masked_array(tzs, ~tis), tis)


def _proj_trans_points(points, M):
    """The *points* (rows of x, y, z) projected by *M* (matplotlib 3.11.2's,
    under the Matplotlib License)."""
    points = np.asanyarray(points)
    xs, ys, zs = points[:, 0], points[:, 1], points[:, 2]
    return proj_transform(xs, ys, zs, M)
