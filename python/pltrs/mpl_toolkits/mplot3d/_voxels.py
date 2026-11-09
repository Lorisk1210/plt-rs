"""
`Axes3D.voxels`: cubes filling the cells of a 3-D grid, drawn by pltrs
(Rust) (``_native.mplot3d.voxels``) with matplotlib's signature. Colours
some voxel takes other than one of (none, several, or one that is no
colour) go through matplotlib's own loop, which makes the voxels one by
one and fails as the first that cannot be made fails.
"""

from collections import defaultdict
import itertools

import numpy as np

from pltrs import _handles, colors as mcolors
from pltrs._native import mplot3d as _mplot3d

from . import _properties, art3d


def _broadcast_color_arg(color, filled, name):
    """*color* for every cell of *filled*: one colour for all, a 3D array
    of colours or a 4D array of RGB(A) values (matplotlib's
    ``_broadcast_color_arg``)."""
    if np.ndim(color) in (0, 1):
        return np.broadcast_to(color, filled.shape + np.shape(color))
    if np.ndim(color) in (3, 4):
        if np.shape(color)[:3] != filled.shape:
            raise ValueError(
                f"When multidimensional, {name} must match the shape of filled")
        return color
    raise ValueError(f"Invalid {name} argument")


def _cell_colors(color, cells):
    """
    The colour of every cell of *cells* (a boolean array) as RGBA rows of
    shape ``cells.shape + (4,)``, from the colour argument *color*, which
    `_broadcast_color_arg` takes; None when some filled cell takes other
    than exactly one colour from it, as matplotlib's ``to_rgba_array`` of
    the cell's entry reads it. Only the colours of filled cells are read,
    as matplotlib reads them (an empty cell's colour may be anything).
    """
    shape = cells.shape
    try:
        if np.ndim(color) in (0, 1):
            # Every cell's entry is the argument itself, as an array entry.
            entry = np.broadcast_to(color, (1,) + np.shape(color))[0]
            rgba = mcolors.to_rgba_array(entry)
            return None if len(rgba) != 1 else np.broadcast_to(rgba[0], shape + (4,))
        values = np.asarray(color)
        if values.ndim == 4 and (values.shape[3] not in (3, 4)
                                 or values.dtype.kind not in "biuf"):
            return None
        rgba = mcolors.to_rgba_array(values[cells])
    except (TypeError, ValueError):
        return None
    if rgba.shape != (np.count_nonzero(cells), 4):
        return None
    rows = np.zeros(shape + (4,))
    rows[cells] = rgba
    return rows


def _permutation_matrices(n):
    """Cyclic permutation matrices (matplotlib's)."""
    mat = np.eye(n, dtype=np.intp)
    for _ in range(n):
        yield mat
        mat = np.roll(mat, 1, axis=0)


def _voxel_faces(filled):
    """The faces of each filled cell of *filled* drawn between it and an
    empty cell or the outside, as corner indices, keyed by cell in the
    order matplotlib's loop finds them."""
    size = np.array(filled.shape, dtype=np.intp)
    # points lying on corners of a square
    square = np.array([[0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0]],
                      dtype=np.intp)
    voxel_faces = defaultdict(list)
    for permute in _permutation_matrices(3):
        pc, qc, rc = permute.T.dot(size)
        square_rot_pos = square.dot(permute.T)
        square_rot_neg = square_rot_pos[::-1]
        for p in np.arange(pc):
            for q in np.arange(qc):
                p0 = permute.dot([p, q, 0])
                i0 = tuple(p0)
                if filled[i0]:
                    voxel_faces[i0].append(p0 + square_rot_neg)
                for r1, r2 in itertools.pairwise(np.arange(rc)):
                    p1 = permute.dot([p, q, r1])
                    p2 = permute.dot([p, q, r2])
                    i1, i2 = tuple(p1), tuple(p2)
                    if filled[i1] and not filled[i2]:
                        voxel_faces[i1].append(p2 + square_rot_pos)
                    elif not filled[i1] and filled[i2]:
                        voxel_faces[i2].append(p2 + square_rot_neg)
                pk = permute.dot([p, q, rc - 1])
                pk2 = permute.dot([p, q, rc])
                ik = tuple(pk)
                if filled[ik]:
                    voxel_faces[ik].append(pk2 + square_rot_pos)
    return voxel_faces


def _voxels_as_matplotlib(ax, xyz, filled, colors, options, kwargs):
    """matplotlib's voxels from its colour arguments broadcast over the
    cells, ``colors = (facecolors, edgecolors)``, and ``options = (shade,
    lightsource, axlim_clip)``: a `.Poly3DCollection` per voxel, in its
    order, after the view takes in every corner."""
    facecolors, edgecolors = colors
    shade, lightsource, axlim_clip = options
    coord_shape = tuple(np.array(filled.shape, dtype=np.intp) + 1)
    if xyz is None:
        x, y, z = np.indices(coord_shape)
    else:
        x, y, z = (np.broadcast_to(c, coord_shape) for c in xyz)
    ax.auto_scale_xyz(x, y, z)
    polygons = {}
    for coord, faces_inds in _voxel_faces(filled).items():
        if xyz is None:
            faces = faces_inds
        else:
            faces = []
            for face_inds in faces_inds:
                ind = face_inds[:, 0], face_inds[:, 1], face_inds[:, 2]
                face = np.empty(face_inds.shape)
                face[:, 0] = x[ind]
                face[:, 1] = y[ind]
                face[:, 2] = z[ind]
                faces.append(face)
        poly = art3d.Poly3DCollection(
            faces, facecolors=facecolors[coord], edgecolors=edgecolors[coord],
            shade=shade, lightsource=lightsource, axlim_clip=axlim_clip,
            **kwargs)
        ax.add_collection3d(poly)
        polygons[coord] = poly
    return polygons


class VoxelsMixin:
    """`voxels`."""

    def voxels(self, *args, facecolors=None, edgecolors=None, shade=True,
               lightsource=None, axlim_clip=False, **kwargs):
        """
        ax.voxels([x, y, z,] /, filled, facecolors=None, edgecolors=None, \
**kwargs)

        Plot a set of filled voxels

        All voxels are plotted as 1x1x1 cubes on the axis, with
        ``filled[0, 0, 0]`` placed with its lower corner at the origin.
        Occluded faces are not plotted.

        Parameters
        ----------
        filled : 3D np.array of bool
            A 3D array of values, with truthy values indicating which voxels
            to fill

        x, y, z : 3D np.array, optional
            The coordinates of the corners of the voxels. This should broadcast
            to a shape one larger in every dimension than the shape of
            *filled*.  These can be used to plot non-cubic voxels.

            If not specified, defaults to increasing integers along each axis,
            like those returned by :func:`~numpy.indices`.
            As indicated by the ``/`` in the function signature, these
            arguments can only be passed positionally.

        facecolors, edgecolors : array-like, optional
            The color to draw the faces and edges of the voxels. Can only be
            passed as keyword arguments.
            These parameters can be:

            - A single color value, to color all voxels the same color. This
              can be either a string, or a 1D RGB/RGBA array
            - ``None``, the default, to use a single color for the faces, and
              the style default for the edges.
            - A 3D `~numpy.ndarray` of color names, with each item the color
              for the corresponding voxel. The size must match the voxels.
            - A 4D `~numpy.ndarray` of RGB/RGBA data, with the components
              along the last axis.

        shade : bool, default: True
            Whether to shade the facecolors.

        lightsource : `~matplotlib.colors.LightSource`, optional
            The lightsource to use when *shade* is True.

        axlim_clip : bool, default: False
            Whether to hide voxels with points outside the axes view limits.

            .. versionadded:: 3.10

        **kwargs
            Additional keyword arguments to pass onto
            `~mpl_toolkits.mplot3d.art3d.Poly3DCollection`.

        Returns
        -------
        faces : dict
            A dictionary indexed by coordinate, where ``faces[i, j, k]`` is a
            `.Poly3DCollection` of the faces drawn for the voxel
            ``filled[i, j, k]``. If no faces were drawn for a given voxel,
            either because it was not asked to be drawn, or it is fully
            occluded, then ``(i, j, k) not in faces``.

        Examples
        --------
        .. plot:: gallery/mplot3d/voxels.py
        .. plot:: gallery/mplot3d/voxels_rgb.py
        .. plot:: gallery/mplot3d/voxels_torus.py
        .. plot:: gallery/mplot3d/voxels_numpy_logo.py
        """
        # The name must be voxels for the right error messages.
        if len(args) >= 3:
            def voxels(__x, __y, __z, filled, **kwargs):
                return (__x, __y, __z), filled, kwargs
        else:
            def voxels(filled, **kwargs):
                return None, filled, kwargs

        xyz, filled, kwargs = voxels(*args, **kwargs)
        filled = np.asarray(filled)
        if filled.ndim != 3:
            raise ValueError("Argument filled must be 3-dimensional")
        corners = None
        if xyz is not None:
            coord_shape = tuple(np.array(filled.shape) + 1)
            corners = tuple(np.ascontiguousarray(
                np.broadcast_to(c, coord_shape), dtype=float) for c in xyz)
        if facecolors is None:
            facecolors = self._get_patches_for_fill.get_next_color()
        broadcast = (_broadcast_color_arg(facecolors, filled, 'facecolors'),
                     _broadcast_color_arg(edgecolors, filled, 'edgecolors'))
        cells = filled.astype(bool)
        faces = _cell_colors(facecolors, cells)
        edges = None if edgecolors is None else _cell_colors(edgecolors, cells)
        if faces is None or (edges is None and edgecolors is not None):
            return _voxels_as_matplotlib(
                self, xyz, filled, broadcast, (shade, lightsource, axlim_clip),
                kwargs)
        light = art3d._native_light(lightsource)
        _properties.check_known(art3d.Poly3DCollection, kwargs)
        made = _mplot3d.voxels(
            self._native, np.ascontiguousarray(filled, dtype=bool), corners,
            (np.ascontiguousarray(faces, dtype=float),
             None if edges is None else np.ascontiguousarray(edges, dtype=float)),
            (bool(shade), light), bool(axlim_clip))
        polygons = {}
        for cell, native in made:
            poly = _handles.wrap(native, art3d.Poly3DCollection, self.figure)
            poly._internal_update(kwargs)
            polygons[cell] = poly
        return polygons
