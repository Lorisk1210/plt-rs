"""
`Axes3D.voxels`: cubes filling the cells of a 3-D grid, drawn by pltrs
(Rust) (``_native.mplot3d.voxels``) with matplotlib's signature.
"""

import numpy as np

from pltrs import _handles, colors as mcolors
from pltrs._native import mplot3d as _mplot3d

from . import art3d


def _cell_colors(color, filled, name):
    """
    The colour of every cell of *filled* as RGBA rows of shape
    ``filled.shape + (4,)``: one colour for all, a 3D array of colours or a
    4D array of RGB(A) values (matplotlib's ``_broadcast_color_arg``). Only
    the colours of filled cells are read, as matplotlib reads them (an
    empty cell's colour may be anything).
    """
    shape = filled.shape
    if np.ndim(color) in (0, 1):
        return np.broadcast_to(mcolors.to_rgba(color), shape + (4,))
    if np.ndim(color) in (3, 4):
        if np.shape(color)[:3] != shape:
            raise ValueError(
                f"When multidimensional, {name} must match the shape of filled")
        rows = np.zeros(shape + (4,))
        rows[filled] = mcolors.to_rgba_array(np.asarray(color)[filled])
        return rows
    raise ValueError(f"Invalid {name} argument")


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
            The coordinates of the corners of the voxels. This should
            broadcast to a shape one larger in every dimension than the shape
            of *filled*. They can only be passed positionally.
        facecolors, edgecolors : array-like, optional
            The color to draw the faces and edges of the voxels: one color,
            a 3D array of colors, or a 4D array of RGB/RGBA data.
        shade : bool, default: True
            Whether to shade the facecolors.
        lightsource : `~matplotlib.colors.LightSource`, optional
            The lightsource to use when *shade* is True.
        axlim_clip : bool, default: False
            Whether to hide voxels with points outside the axes view limits.
        **kwargs
            Additional keyword arguments to pass onto
            `~mpl_toolkits.mplot3d.art3d.Poly3DCollection`.

        Returns
        -------
        faces : dict
            A dictionary indexed by coordinate, where ``faces[i, j, k]`` is a
            `.Poly3DCollection` of the faces drawn for the voxel
            ``filled[i, j, k]``.
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
        cells = filled.astype(bool)
        faces = (None if facecolors is None
                 else _cell_colors(facecolors, cells, 'facecolors'))
        edges = (None if edgecolors is None
                 else _cell_colors(edgecolors, cells, 'edgecolors'))
        light = (None if lightsource is None
                 else (float(lightsource.azdeg), float(lightsource.altdeg)))
        made = _mplot3d.voxels(
            self._native, np.ascontiguousarray(filled, dtype=bool), corners,
            (None if faces is None else np.ascontiguousarray(faces, dtype=float),
             None if edges is None else np.ascontiguousarray(edges, dtype=float)),
            (bool(shade), light), bool(axlim_clip))
        polygons = {}
        for cell, native in made:
            poly = _handles.wrap(native, art3d.Poly3DCollection, self.figure)
            poly._internal_update(kwargs)
            polygons[cell] = poly
        return polygons
