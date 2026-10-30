"""
Various transforms used for by the 3D code (matplotlib's
``mpl_toolkits.mplot3d.proj3d``): homogeneous projection matrices, which
pltrs (Rust) computes with numpy's rounding.
"""

import numpy as np

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
    aspect = (1.0, 1.0, 1.0) if pb_aspect is None else _vec3(pb_aspect)
    limits = tuple(float(v) for v in (xmin, xmax, ymin, ymax, zmin, zmax))
    return _mplot3d.world_transformation(limits, aspect)


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
    return _mplot3d.ortho_transformation(float(zfront), float(zback))


def _transform(xs, ys, zs, M):
    """The points through the homogeneous matrix *M*, divided by ``w``."""
    columns = [np.asarray(c, dtype=float).ravel().tolist()
               for c in np.broadcast_arrays(xs, ys, zs)]
    txs, tys, tzs = _mplot3d.transform(np.asarray(M, dtype=float),
                                       tuple(columns))
    return np.array(txs), np.array(tys), np.array(tzs)


def inv_transform(xs, ys, zs, invM):
    """
    Transform the points by the inverse of the projection matrix, *invM*.
    """
    return _transform(xs, ys, zs, invM)


def proj_transform(xs, ys, zs, M):
    """
    Transform the points by the projection matrix *M*.
    """
    return _transform(xs, ys, zs, M)
