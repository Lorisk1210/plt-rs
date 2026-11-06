"""
Interpolation inside triangular grids (matplotlib's
``tri._triinterpolate``); pltrs (Rust) interpolates.

The gradient estimators of the cubic interpolator (`_DOF_estimator` and
its ``user`` and ``geom`` kinds) and the sparse conjugate gradient solver
(`_Sparse_Matrix_coo`, `_cg`) are matplotlib 3.11.2's pure-Python
internals, under the Matplotlib License, kept as they are for programs
that use them; pltrs (Rust) has its own for the interpolation.
"""

import numpy as np

from .. import _api
from .._native import tri as _tri
from ._trifinder import TriFinder
from ._triangulation import Triangulation
from ._tritools import TriAnalyzer

__all__ = ('TriInterpolator', 'LinearTriInterpolator', 'CubicTriInterpolator')


class TriInterpolator:
    """
    Abstract base class for classes used to interpolate on a triangular grid.

    Derived classes implement the following methods:

    - ``__call__(x, y)``,
      where x, y are array-like point coordinates of the same shape, and
      that returns a masked array of the same shape containing the
      interpolated z-values.

    - ``gradient(x, y)``,
      where x, y are array-like point coordinates of the same
      shape, and that returns a list of 2 masked arrays of the same shape
      containing the 2 derivatives of the interpolator (derivatives of
      interpolated z values with respect to x and y).
    """

    #: How pltrs (Rust) interpolates: ``(kind, gradients)``.
    _native_kind = ("linear", None)

    def __init__(self, triangulation, z, trifinder=None):
        _api.check_isinstance(Triangulation, triangulation=triangulation)
        self._triangulation = triangulation

        self._z = np.asarray(z)
        if self._z.shape != self._triangulation.x.shape:
            raise ValueError("z array must have same length as triangulation x"
                             " and y arrays")

        _api.check_isinstance((TriFinder, None), trifinder=trifinder)
        self._trifinder = trifinder or self._triangulation.get_trifinder()
        # The values pltrs (Rust) interpolates, one per point (the cubic
        # interpolator renumbers ``_z`` as matplotlib's does).
        self._native_z = self._z.astype(np.float64)

    # __call__ and gradient docstrings are shared by all subclasses
    # (except, if needed, relevant additions).
    # However these methods are only implemented in subclasses to avoid
    # confusion in the documentation.
    _docstring__call__ = """
        Returns a masked array containing interpolated values at the specified
        (x, y) points.

        Parameters
        ----------
        x, y : array-like
            x and y coordinates of the same shape and any number of
            dimensions.

        Returns
        -------
        np.ma.array
            Masked array of the same shape as *x* and *y*; values corresponding
            to (*x*, *y*) points outside of the triangulation are masked out.

        """

    _docstringgradient = r"""
        Returns a list of 2 masked arrays containing interpolated derivatives
        at the specified (x, y) points.

        Parameters
        ----------
        x, y : array-like
            x and y coordinates of the same shape and any number of
            dimensions.

        Returns
        -------
        dzdx, dzdy : np.ma.array
            2 masked arrays of the same shape as *x* and *y*; values
            corresponding to (x, y) points outside of the triangulation
            are masked out.
            The first returned array contains the values of
            :math:`\frac{\partial z}{\partial x}` and the second those of
            :math:`\frac{\partial z}{\partial y}`.

        """

    def _interpolate_multikeys(self, x, y, tri_index=None,
                               return_keys=('z',)):
        """
        Versatile (private) method defined for all TriInterpolators: the
        interpolated values and derivatives *return_keys* names at the
        points *x*, *y* (of any shape), masked outside the grid. pltrs
        (Rust) interpolates.

        Parameters
        ----------
        x, y : array-like
            x and y coordinates where interpolated values are requested.
        tri_index : array-like of int, optional
            Array of the containing triangle indices, same shape as
            *x* and *y*. Defaults to None. If None, these indices
            will be computed by a TriFinder instance.
            (Note: For point outside the grid, tri_index[ipt] shall be -1).
        return_keys : tuple of keys from {'z', 'dzdx', 'dzdy'}
            Defines the interpolation arrays to return, and in which order.

        Returns
        -------
        list of arrays
            Each array-like contains the expected interpolated values in the
            order defined by *return_keys* parameter.
        """
        x = np.asarray(x, dtype=np.float64)
        y = np.asarray(y, dtype=np.float64)
        sh_ret = x.shape
        if x.shape != y.shape:
            raise ValueError("x and y shall have same shapes."
                             f" Given: {x.shape} and {y.shape}")
        if tri_index is not None:
            if tri_index.shape != sh_ret:
                raise ValueError(
                    "tri_index array is provided and shall"
                    " have same shape as x and y. Given: "
                    f"{tri_index.shape} and {sh_ret}")
            tri_index = np.ravel(tri_index).astype(np.int64)
        indices = []
        for return_key in return_keys:
            try:
                indices.append({'z': 0, 'dzdx': 1, 'dzdy': 2}[return_key])
            except KeyError as err:
                raise ValueError("return_keys items shall take values in"
                                 " {'z', 'dzdx', 'dzdy'}") from err
        computed = _tri.interpolate(
            self._triangulation._native, self._native_z, self._native_kind,
            (np.ravel(x), np.ravel(y)), tri_index)
        return [np.ma.masked_invalid(computed[index].reshape(sh_ret), copy=False)
                for index in indices]



class LinearTriInterpolator(TriInterpolator):
    """
    Linear interpolator on a triangular grid.

    Each triangle is represented by a plane so that an interpolated value at
    point (x, y) lies on the plane of the triangle containing (x, y).
    Interpolated values are therefore continuous across the triangulation, but
    their first derivatives are discontinuous at edges between triangles.

    Parameters
    ----------
    triangulation : `~matplotlib.tri.Triangulation`
        The triangulation to interpolate over.
    z : (npoints,) array-like
        Array of values, defined at grid points, to interpolate between.
    trifinder : `~matplotlib.tri.TriFinder`, optional
        If this is not specified, the Triangulation's default TriFinder will
        be used by calling `.Triangulation.get_trifinder`.

    Methods
    -------
    `__call__` (x, y) : Returns interpolated values at (x, y) points.
    `gradient` (x, y) : Returns interpolated derivatives at (x, y) points.

    """

    def __call__(self, x, y):
        return self._interpolate_multikeys(x, y, tri_index=None,
                                           return_keys=('z',))[0]
    __call__.__doc__ = TriInterpolator._docstring__call__

    def gradient(self, x, y):
        return self._interpolate_multikeys(x, y, tri_index=None,
                                           return_keys=('dzdx', 'dzdy'))
    gradient.__doc__ = TriInterpolator._docstringgradient


class CubicTriInterpolator(TriInterpolator):
    r"""
    Cubic interpolator on a triangular grid.

    In one-dimension - on a segment - a cubic interpolating function is
    defined by the values of the function and its derivative at both ends.
    This is almost the same in 2D inside a triangle, except that the values
    of the function and its 2 derivatives have to be defined at each triangle
    node.

    The CubicTriInterpolator takes the value of the function at each node -
    provided by the user - and internally computes the value of the
    derivatives, resulting in a smooth interpolation.
    (As a special feature, the user can also impose the value of the
    derivatives at each node, but this is not supposed to be the common
    usage.)

    Parameters
    ----------
    triangulation : `~matplotlib.tri.Triangulation`
        The triangulation to interpolate over.
    z : (npoints,) array-like
        Array of values, defined at grid points, to interpolate between.
    kind : {'min_E', 'geom', 'user'}, optional
        Choice of the smoothing algorithm, in order to compute
        the interpolant derivatives (defaults to 'min_E'):

        - if 'min_E': (default) The derivatives at each node is computed
          to minimize a bending energy.
        - if 'geom': The derivatives at each node is computed as a
          weighted average of relevant triangle normals. To be used for
          speed optimization (large grids).
        - if 'user': The user provides the argument *dz*, no computation
          is hence needed.

    trifinder : `~matplotlib.tri.TriFinder`, optional
        If not specified, the Triangulation's default TriFinder will
        be used by calling `.Triangulation.get_trifinder`.
    dz : tuple of array-likes (dzdx, dzdy), optional
        Used only if  *kind* ='user'. In this case *dz* must be provided as
        (dzdx, dzdy) where dzdx, dzdy are arrays of the same shape as *z* and
        are the interpolant first derivatives at the *triangulation* points.

    Methods
    -------
    `__call__` (x, y) : Returns interpolated values at (x, y) points.
    `gradient` (x, y) : Returns interpolated derivatives at (x, y) points.

    Notes
    -----
    This note is a bit technical and details how the cubic interpolation is
    computed.

    The interpolation is based on a Clough-Tocher subdivision scheme of
    the *triangulation* mesh (to make it clearer, each triangle of the
    grid will be divided in 3 child-triangles, and on each child triangle
    the interpolated function is a cubic polynomial of the 2 coordinates).
    This technique originates from FEM (Finite Element Method) analysis;
    the element used is a reduced Hsieh-Clough-Tocher (HCT)
    element. Its shape functions are described in [1]_.
    The assembled function is guaranteed to be C1-smooth, i.e. it is
    continuous and its first derivatives are also continuous (this
    is easy to show inside the triangles but is also true when crossing the
    edges).

    In the default case (*kind* ='min_E'), the interpolant minimizes a
    curvature energy on the functional space generated by the HCT element
    shape functions - with imposed values but arbitrary derivatives at each
    node. The minimized functional is the integral of the so-called total
    curvature (implementation based on an algorithm from [2]_ - PCG sparse
    solver):

    .. math::

        E(z) = \frac{1}{2} \int_{\Omega} \left(
            \left( \frac{\partial^2{z}}{\partial{x}^2} \right)^2 +
            \left( \frac{\partial^2{z}}{\partial{y}^2} \right)^2 +
            2\left( \frac{\partial^2{z}}{\partial{y}\partial{x}} \right)^2
        \right) dx\,dy

    If the case *kind* ='geom' is chosen by the user, a simple geometric
    approximation is used (weighted average of the triangle normal
    vectors), which could improve speed on very large grids.

    References
    ----------
    .. [1] Michel Bernadou, Kamal Hassan, "Basis functions for general
        Hsieh-Clough-Tocher triangles, complete or reduced.",
        International Journal for Numerical Methods in Engineering,
        17(5):784 - 789. 2.01.
    .. [2] C.T. Kelley, "Iterative Methods for Optimization".

    """

    def __init__(self, triangulation, z, kind='min_E', trifinder=None,
                 dz=None):
        super().__init__(triangulation, z, trifinder)
        # The unmasked triangles and the points they use, renumbered and
        # rescaled as matplotlib keeps them for its gradient estimators.
        tri_analyzer = TriAnalyzer(self._triangulation)
        (compressed_triangles, compressed_x, compressed_y, tri_renum,
         node_renum) = tri_analyzer._get_compressed_triangulation()
        self._triangles = compressed_triangles
        self._tri_renum = tri_renum
        valid_node = (node_renum != -1)
        self._z[node_renum[valid_node]] = self._z[valid_node]
        self._unit_x = np.ptp(compressed_x)
        self._unit_y = np.ptp(compressed_y)
        self._pts = np.column_stack([compressed_x / self._unit_x,
                                     compressed_y / self._unit_y])
        self._tris_pts = self._pts[self._triangles]
        _api.check_in_list(['user', 'geom', 'min_E'], kind=kind)
        if kind == 'user':
            if dz is None:
                raise ValueError("For a CubicTriInterpolator with "
                                 "*kind*='user', a valid *dz* "
                                 "argument is expected.")
            dzdx, dzdy = (np.asarray(d, dtype=float) for d in dz)
            self._native_kind = (kind, (dzdx, dzdy))
        else:
            self._native_kind = (kind, None)

    @staticmethod
    def _get_jacobian(tris_pts):
        """
        Fast (vectorized) function to compute triangle jacobian matrix.

        Parameters
        ----------
        tris_pts : array like of dim 3 (shape: (nx, 3, 2))
            Coordinates of the containing triangles apexes.

        Returns
        -------
        array of dim 3 (shape (nx, 2, 2))
            Barycentric coordinates of the points inside the containing
            triangles.
            J[itri, :, :] is the jacobian matrix at apex 0 of the triangle
            itri, so that the following (matrix) relationship holds:
               [dz/dksi] = [J] x [dz/dx]
            with x: global coordinates
                 ksi: element parametric coordinates in triangle first apex
                 local basis.
        """
        a = np.array(tris_pts[:, 1, :] - tris_pts[:, 0, :])
        b = np.array(tris_pts[:, 2, :] - tris_pts[:, 0, :])
        J = _to_matrix_vectorized([[a[:, 0], a[:, 1]],
                                   [b[:, 0], b[:, 1]]])
        return J

    def __call__(self, x, y):
        return self._interpolate_multikeys(x, y, tri_index=None,
                                           return_keys=('z',))[0]
    __call__.__doc__ = TriInterpolator._docstring__call__

    def gradient(self, x, y):
        return self._interpolate_multikeys(x, y, tri_index=None,
                                           return_keys=('dzdx', 'dzdy'))
    gradient.__doc__ = TriInterpolator._docstringgradient


class _DOF_estimator:
    """
    Abstract base class for classes used to estimate a function's first
    derivatives, and deduce the dofs for a CubicTriInterpolator using a
    reduced HCT element formulation.

    Derived classes implement ``compute_df(self, **kwargs)``, returning
    ``np.vstack([dfx, dfy]).T`` where ``dfx, dfy`` are the estimation of the 2
    gradient coordinates.
    """

    #: The rotations taking the jacobian at apex 0 to those at apexes 1
    #: and 2 (matplotlib keeps them on its reduced HCT element, whose work
    #: pltrs (Rust) does).
    _J0_to_J1 = np.array([[-1., 1.], [-1., 0.]])
    _J0_to_J2 = np.array([[0., -1.], [1., -1.]])

    def __init__(self, interpolator, **kwargs):
        _api.check_isinstance(CubicTriInterpolator, interpolator=interpolator)
        self._pts = interpolator._pts
        self._tris_pts = interpolator._tris_pts
        self.z = interpolator._z
        self._triangles = interpolator._triangles
        (self._unit_x, self._unit_y) = (interpolator._unit_x,
                                        interpolator._unit_y)
        self.dz = self.compute_dz(**kwargs)
        self.compute_dof_from_df()

    def compute_dz(self, **kwargs):
        """The ``(npoints, 2)`` gradients estimated at the points."""
        raise NotImplementedError

    def compute_dof_from_df(self):
        """
        Compute reduced-HCT elements degrees of freedom, from the gradient.
        """
        J = CubicTriInterpolator._get_jacobian(self._tris_pts)
        tri_z = self.z[self._triangles]
        tri_dz = self.dz[self._triangles]
        tri_dof = self.get_dof_vec(tri_z, tri_dz, J)
        return tri_dof

    @staticmethod
    def get_dof_vec(tri_z, tri_dz, J):
        """
        Compute the dof vector of a triangle, from the value of f, df and
        of the local Jacobian at each node.

        Parameters
        ----------
        tri_z : shape (3,) array
            f nodal values.
        tri_dz : shape (3, 2) array
            df/dx, df/dy nodal values.
        J
            Jacobian matrix in local basis of apex 0.

        Returns
        -------
        dof : shape (9,) array
            For each apex ``iapex``::

                dof[iapex*3+0] = f(Ai)
                dof[iapex*3+1] = df(Ai).(AiAi+)
                dof[iapex*3+2] = df(Ai).(AiAi-)
        """
        npt = tri_z.shape[0]
        dof = np.zeros([npt, 9], dtype=np.float64)
        J1 = _DOF_estimator._J0_to_J1 @ J
        J2 = _DOF_estimator._J0_to_J2 @ J

        col0 = J @ np.expand_dims(tri_dz[:, 0, :], axis=2)
        col1 = J1 @ np.expand_dims(tri_dz[:, 1, :], axis=2)
        col2 = J2 @ np.expand_dims(tri_dz[:, 2, :], axis=2)

        dfdksi = _to_matrix_vectorized([
            [col0[:, 0, 0], col1[:, 0, 0], col2[:, 0, 0]],
            [col0[:, 1, 0], col1[:, 1, 0], col2[:, 1, 0]]])
        dof[:, 0:7:3] = tri_z
        dof[:, 1:8:3] = dfdksi[:, 0]
        dof[:, 2:9:3] = dfdksi[:, 1]
        return dof


class _DOF_estimator_user(_DOF_estimator):
    """dz is imposed by user; accounts for scaling if any."""

    def compute_dz(self, dz):
        (dzdx, dzdy) = dz
        dzdx = dzdx * self._unit_x
        dzdy = dzdy * self._unit_y
        return np.vstack([dzdx, dzdy]).T


class _DOF_estimator_geom(_DOF_estimator):
    """Fast 'geometric' approximation, recommended for large arrays."""

    def compute_dz(self):
        """
        self.df is computed as weighted average of _triangles sharing a common
        node. On each triangle itri f is first assumed linear (= ~f), which
        allows to compute d~f[itri]
        Then the following approximation of df nodal values is then proposed:
            f[ipt] = SUM ( w[itri] x d~f[itri] , for itri sharing apex ipt)
        The weighted coeff. w[itri] are proportional to the angle of the
        triangle itri at apex ipt
        """
        el_geom_w = self.compute_geom_weights()
        el_geom_grad = self.compute_geom_grads()

        # Sum of weights coeffs
        w_node_sum = np.bincount(np.ravel(self._triangles),
                                 weights=np.ravel(el_geom_w))

        # Sum of weighted df = (dfx, dfy)
        dfx_el_w = np.empty_like(el_geom_w)
        dfy_el_w = np.empty_like(el_geom_w)
        for iapex in range(3):
            dfx_el_w[:, iapex] = el_geom_w[:, iapex]*el_geom_grad[:, 0]
            dfy_el_w[:, iapex] = el_geom_w[:, iapex]*el_geom_grad[:, 1]
        dfx_node_sum = np.bincount(np.ravel(self._triangles),
                                   weights=np.ravel(dfx_el_w))
        dfy_node_sum = np.bincount(np.ravel(self._triangles),
                                   weights=np.ravel(dfy_el_w))

        # Estimation of df
        dfx_estim = dfx_node_sum/w_node_sum
        dfy_estim = dfy_node_sum/w_node_sum
        return np.vstack([dfx_estim, dfy_estim]).T

    def compute_geom_weights(self):
        """
        Build the (nelems, 3) weights coeffs of _triangles angles,
        renormalized so that np.sum(weights, axis=1) == np.ones(nelems)
        """
        weights = np.zeros([np.size(self._triangles, 0), 3])
        tris_pts = self._tris_pts
        for ipt in range(3):
            p0 = tris_pts[:, ipt % 3, :]
            p1 = tris_pts[:, (ipt+1) % 3, :]
            p2 = tris_pts[:, (ipt-1) % 3, :]
            alpha1 = np.arctan2(p1[:, 1]-p0[:, 1], p1[:, 0]-p0[:, 0])
            alpha2 = np.arctan2(p2[:, 1]-p0[:, 1], p2[:, 0]-p0[:, 0])
            # In the below formula we could take modulo 2. but
            # modulo 1. is safer regarding round-off errors (flat triangles).
            angle = np.abs(((alpha2-alpha1) / np.pi) % 1)
            # Weight proportional to angle up np.pi/2; null weight for
            # degenerated cases 0 and np.pi (note that *angle* is normalized
            # by np.pi).
            weights[:, ipt] = 0.5 - np.abs(angle-0.5)
        return weights

    def compute_geom_grads(self):
        """
        Compute the (global) gradient component of f assumed linear (~f).
        returns array df of shape (nelems, 2)
        df[ielem].dM[ielem] = dz[ielem] i.e. df = dz x dM = dM.T^-1 x dz
        """
        tris_pts = self._tris_pts
        tris_f = self.z[self._triangles]

        dM1 = tris_pts[:, 1, :] - tris_pts[:, 0, :]
        dM2 = tris_pts[:, 2, :] - tris_pts[:, 0, :]
        dM = np.dstack([dM1, dM2])
        # Here we try to deal with the simplest colinear cases: a null
        # gradient is assumed in this case.
        dM_inv = _safe_inv22_vectorized(dM)

        dZ1 = tris_f[:, 1] - tris_f[:, 0]
        dZ2 = tris_f[:, 2] - tris_f[:, 0]
        dZ = np.vstack([dZ1, dZ2]).T
        df = np.empty_like(dZ)

        # With np.einsum: could be ej,eji -> ej
        df[:, 0] = dZ[:, 0]*dM_inv[:, 0, 0] + dZ[:, 1]*dM_inv[:, 1, 0]
        df[:, 1] = dZ[:, 0]*dM_inv[:, 0, 1] + dZ[:, 1]*dM_inv[:, 1, 1]
        return df


# The following private :class:_Sparse_Matrix_coo and :func:_cg provide
# a PCG sparse solver for (symmetric) elliptic problems.
class _Sparse_Matrix_coo:
    """A sparse matrix in COO format."""

    def __init__(self, vals, rows, cols, shape):
        """
        Create a sparse matrix in COO format.
        *vals*: arrays of values of non-null entries of the matrix
        *rows*: int arrays of rows of non-null entries of the matrix
        *cols*: int arrays of cols of non-null entries of the matrix
        *shape*: 2-tuple (n, m) of matrix shape
        """
        self.n, self.m = shape
        self.vals = np.asarray(vals, dtype=np.float64)
        self.rows = np.asarray(rows, dtype=np.int32)
        self.cols = np.asarray(cols, dtype=np.int32)

    def dot(self, V):
        """
        Dot product of self by a vector *V* in sparse-dense to dense format
        *V* dense vector of shape (self.m,).
        """
        assert V.shape == (self.m,)
        return np.bincount(self.rows,
                           weights=self.vals*V[self.cols],
                           minlength=self.m)

    def compress_csc(self):
        """
        Compress rows, cols, vals / summing duplicates. Sort for csc format.
        """
        _, unique, indices = np.unique(
            self.rows + self.n*self.cols,
            return_index=True, return_inverse=True)
        self.rows = self.rows[unique]
        self.cols = self.cols[unique]
        self.vals = np.bincount(indices, weights=self.vals)

    def compress_csr(self):
        """
        Compress rows, cols, vals / summing duplicates. Sort for csr format.
        """
        _, unique, indices = np.unique(
            self.m*self.rows + self.cols,
            return_index=True, return_inverse=True)
        self.rows = self.rows[unique]
        self.cols = self.cols[unique]
        self.vals = np.bincount(indices, weights=self.vals)

    def to_dense(self):
        """
        Return a dense matrix representing self, mainly for debugging purposes.
        """
        ret = np.zeros([self.n, self.m], dtype=np.float64)
        nvals = self.vals.size
        for i in range(nvals):
            ret[self.rows[i], self.cols[i]] += self.vals[i]
        return ret

    def __str__(self):
        return self.to_dense().__str__()

    @property
    def diag(self):
        """Return the (dense) vector of the diagonal elements."""
        in_diag = (self.rows == self.cols)
        diag = np.zeros(min(self.n, self.n), dtype=np.float64)  # default 0.
        diag[self.rows[in_diag]] = self.vals[in_diag]
        return diag


def _cg(A, b, x0=None, tol=1.e-10, maxiter=1000):
    """
    Use Preconditioned Conjugate Gradient iteration to solve A x = b
    A simple Jacobi (diagonal) preconditioner is used.

    Parameters
    ----------
    A : _Sparse_Matrix_coo
        *A* must have been compressed before by compress_csc or
        compress_csr method.
    b : array
        Right hand side of the linear system.
    x0 : array, optional
        Starting guess for the solution. Defaults to the zero vector.
    tol : float, optional
        Tolerance to achieve. The algorithm terminates when the relative
        residual is below tol. Default is 1e-10.
    maxiter : int, optional
        Maximum number of iterations.  Iteration will stop after *maxiter*
        steps even if the specified tolerance has not been achieved. Defaults
        to 1000.

    Returns
    -------
    x : array
        The converged solution.
    err : float
        The absolute error np.linalg.norm(A.dot(x) - b)
    """
    n = b.size
    assert A.n == n
    assert A.m == n
    b_norm = np.linalg.norm(b)

    # Jacobi pre-conditioner
    kvec = A.diag
    # For diag elem < 1e-6 we keep 1e-6.
    kvec = np.maximum(kvec, 1e-6)

    # Initial guess
    if x0 is None:
        x = np.zeros(n)
    else:
        x = x0

    r = b - A.dot(x)
    w = r/kvec

    p = np.zeros(n)
    beta = 0.0
    rho = np.dot(r, w)
    k = 0

    # Following C. T. Kelley
    while (np.sqrt(abs(rho)) > tol*b_norm) and (k < maxiter):
        p = w + beta*p
        z = A.dot(p)
        alpha = rho/np.dot(p, z)
        r = r - alpha*z
        w = r/kvec
        rhoold = rho
        rho = np.dot(r, w)
        x = x + alpha*p
        beta = rho/rhoold
        k += 1
    err = np.linalg.norm(A.dot(x) - b)
    return x, err


def _safe_inv22_vectorized(M):
    """
    Inversion of arrays of (2, 2) matrices, returns 0 for rank-deficient
    matrices.

    *M* : array of (2, 2) matrices to inverse, shape (n, 2, 2)
    """
    _api.check_shape((None, 2, 2), M=M)
    M_inv = np.empty_like(M)
    prod1 = M[:, 0, 0]*M[:, 1, 1]
    delta = prod1 - M[:, 0, 1]*M[:, 1, 0]

    # We set delta_inv to 0. in case of a rank deficient matrix; a
    # rank-deficient input matrix *M* will lead to a null matrix in output
    rank2 = (np.abs(delta) > 1e-8*np.abs(prod1))
    if np.all(rank2):
        # Normal 'optimized' flow.
        delta_inv = 1./delta
    else:
        # 'Pathologic' flow.
        delta_inv = np.zeros(M.shape[0])
        delta_inv[rank2] = 1./delta[rank2]

    M_inv[:, 0, 0] = M[:, 1, 1]*delta_inv
    M_inv[:, 0, 1] = -M[:, 0, 1]*delta_inv
    M_inv[:, 1, 0] = -M[:, 1, 0]*delta_inv
    M_inv[:, 1, 1] = M[:, 0, 0]*delta_inv
    return M_inv


def _to_matrix_vectorized(M):
    """
    Build an array of matrices from individuals np.arrays of identical shapes.

    Parameters
    ----------
    M
        ncols-list of nrows-lists of shape sh.

    Returns
    -------
    M_res : np.array of shape (sh, nrow, ncols)
        *M_res* satisfies ``M_res[..., i, j] = M[i][j]``.
    """
    assert isinstance(M, (tuple, list))
    assert all(isinstance(item, (tuple, list)) for item in M)
    c_vec = np.asarray([len(item) for item in M])
    assert np.all(c_vec-c_vec[0] == 0)
    r = len(M)
    c = c_vec[0]
    M00 = np.asarray(M[0][0])
    dt = M00.dtype
    sh = [M00.shape[0], r, c]
    M_ret = np.empty(sh, dtype=dt)
    for irow in range(r):
        for icol in range(c):
            M_ret[:, irow, icol] = np.asarray(M[irow][icol])
    return M_ret
