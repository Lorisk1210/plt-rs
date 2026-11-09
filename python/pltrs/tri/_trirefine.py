"""
Mesh refinement for triangular grids (matplotlib's ``tri._trirefine``);
pltrs (Rust) refines.
"""

import numpy as np

from .. import _api
from .._native import tri as _tri
from ._triangulation import Triangulation
from ._triinterpolate import CubicTriInterpolator, TriInterpolator


class TriRefiner:
    """
    Abstract base class for classes implementing mesh refinement.

    A TriRefiner encapsulates a Triangulation object and provides tools for
    mesh refinement and interpolation.

    Derived classes must implement:

    - ``refine_triangulation(return_tri_index=False, **kwargs)`` , where
      the optional keyword arguments *kwargs* are defined in each
      TriRefiner concrete implementation, and which returns:

      - a refined triangulation,
      - optionally (depending on *return_tri_index*), for each
        point of the refined triangulation: the index of
        the initial triangulation triangle to which it belongs.

    - ``refine_field(z, triinterpolator=None, **kwargs)``, where:

      - *z* array of field values (to refine) defined at the base
        triangulation nodes,
      - *triinterpolator* is an optional `~matplotlib.tri.TriInterpolator`,
      - the other optional keyword arguments *kwargs* are defined in
        each TriRefiner concrete implementation;

      and which returns (as a tuple) a refined triangular mesh and the
      interpolated values of the field at the refined triangulation nodes.
    """

    def __init__(self, triangulation):
        _api.check_isinstance(Triangulation, triangulation=triangulation)
        self._triangulation = triangulation


class UniformTriRefiner(TriRefiner):
    """
    Uniform mesh refinement by recursive subdivisions.

    Parameters
    ----------
    triangulation : `~matplotlib.tri.Triangulation`
        The encapsulated triangulation (to be refined)
    """

    def __init__(self, triangulation):
        super().__init__(triangulation)

    def refine_triangulation(self, return_tri_index=False, subdiv=3):
        """
        Compute a uniformly refined triangulation *refi_triangulation* of
        the encapsulated :attr:`!triangulation`.

        This function refines the encapsulated triangulation by splitting each
        father triangle into 4 child sub-triangles built on the edges midside
        nodes, recursing *subdiv* times.  In the end, each triangle is hence
        divided into ``4**subdiv`` child triangles.

        Parameters
        ----------
        return_tri_index : bool, default: False
            Whether an index table indicating the father triangle index of each
            point is returned.
        subdiv : int, default: 3
            Recursion level for the subdivision.
            Each triangle is divided into ``4**subdiv`` child triangles;
            hence, the default results in 64 refined subtriangles for each
            triangle of the initial triangulation.

        Returns
        -------
        refi_triangulation : `~matplotlib.tri.Triangulation`
            The refined triangulation.
        found_index : int array
            Index of the initial triangulation containing triangle, for each
            point of *refi_triangulation*.
            Returned only if *return_tri_index* is set to True.
        """
        subdiv = _refinements(subdiv)
        refined, found_index = _tri.refine(self._triangulation._native, subdiv)
        # Refined no times, it is the triangulation itself, as matplotlib's.
        refi_triangulation = (Triangulation._from_native(refined) if subdiv
                              else self._triangulation)
        if return_tri_index:
            return refi_triangulation, found_index.astype(np.int32)
        return refi_triangulation

    def refine_field(self, z, triinterpolator=None, subdiv=3):
        """
        Refine a field defined on the encapsulated triangulation.

        Parameters
        ----------
        z : (npoints,) array-like
            Values of the field to refine, defined at the nodes of the
            encapsulated triangulation. (``n_points`` is the number of points
            in the initial triangulation)
        triinterpolator : `~matplotlib.tri.TriInterpolator`, optional
            Interpolator used for field interpolation. If not specified,
            a `~matplotlib.tri.CubicTriInterpolator` will be used.
        subdiv : int, default: 3
            Recursion level for the subdivision.
            Each triangle is divided into ``4**subdiv`` child triangles.

        Returns
        -------
        refi_tri : `~matplotlib.tri.Triangulation`
             The returned refined triangulation.
        refi_z : 1D array of length: *refi_tri* node count.
             The returned interpolated field (at *refi_tri* nodes).
        """
        if triinterpolator is None:
            interp = CubicTriInterpolator(self._triangulation, z)
        else:
            _api.check_isinstance(TriInterpolator,
                                  triinterpolator=triinterpolator)
            interp = triinterpolator
        subdiv = _refinements(subdiv)
        refined, refi_z = _tri.refine_field(
            self._triangulation._native, np.asarray(interp._z, dtype=float),
            interp._native_kind, subdiv)
        # A masked array, as matplotlib's interpolators return values.
        return (Triangulation._from_native(refined) if subdiv
                else self._triangulation,
                np.ma.masked_invalid(refi_z, copy=False))


def _refinements(subdiv):
    """How many times matplotlib's refiner refines for *subdiv*: it loops
    ``for _ in range(subdiv)``, failing as `range` fails on what is no
    integer, and not at all for one below one."""
    return len(range(subdiv))
