"""
A module providing some utility functions regarding Bézier path manipulation
(matplotlib's ``bezier``), computed by plt-rs.
"""

import numpy as np

from ._native import bezier as _bezier
from ._native import register as _register


class NonIntersectingPathException(ValueError):
    """Both ends of a curve lie on the same side of a closed path."""


_register("NonIntersectingPathException", NonIntersectingPathException)


def _real_roots_in_01(coeffs):
    """
    Find real roots of a polynomial in the interval [0, 1].

    Parameters
    ----------
    coeffs : array-like
        Polynomial coefficients in increasing order:
        ``[c0, c1, c2, ...]`` represents ``c0 + c1*t + c2*t**2 + ...``.

    Returns
    -------
    roots : ndarray
        Sorted array of real roots in [0, 1].
    """
    return _bezier.real_roots_in_01(np.asarray(coeffs, float).ravel().tolist())


def get_intersection(cx1, cy1, cos_t1, sin_t1,
                     cx2, cy2, cos_t2, sin_t2):
    """
    Return the intersection between the line through (*cx1*, *cy1*) at angle
    *t1* and the line through (*cx2*, *cy2*) at angle *t2*.
    """
    return _bezier.intersection((cx1, cy1), (cos_t1, sin_t1),
                                (cx2, cy2), (cos_t2, sin_t2))


def get_normal_points(cx, cy, cos_t, sin_t, length):
    """
    For a line passing through (*cx*, *cy*) and having an angle *t*, return
    locations of the two points located along its perpendicular line at the
    distance of *length*.
    """
    (x1, y1), (x2, y2) = _bezier.normal_points((cx, cy), (cos_t, sin_t), length)
    return x1, y1, x2, y2


def split_de_casteljau(beta, t):
    """
    Split a Bézier segment defined by its control points *beta* into two
    separate segments divided at *t* and return their control points.
    """
    beta = np.asarray(beta, float)
    return _bezier.split_de_casteljau(beta.reshape((len(beta), -1)), t)


def find_bezier_t_intersecting_with_closedpath(
        bezier_point_at_t, inside_closedpath, t0=0., t1=1., tolerance=0.01):
    """
    Find the intersection of the Bézier curve with a closed path.

    The intersection point *t* is approximated by two parameters *t0*, *t1*
    such that *t0* <= *t* <= *t1*.

    Search starts from *t0* and *t1* and uses a simple bisecting algorithm
    therefore one of the end points must be inside the path while the other
    doesn't. The search stops when the distance of the points parametrized by
    *t0* and *t1* gets smaller than the given *tolerance*.

    Parameters
    ----------
    bezier_point_at_t : callable
        A function returning x, y coordinates of the Bézier at parameter *t*.
        It must have the signature::

            bezier_point_at_t(t: float) -> tuple[float, float]

    inside_closedpath : callable
        A function returning True if a given point (x, y) is inside the
        closed path. It must have the signature::

            inside_closedpath(point: tuple[float, float]) -> bool

    t0, t1 : float
        Start parameters for the search.

    tolerance : float
        Maximal allowed distance between the final points.

    Returns
    -------
    t0, t1 : float
        The Bézier path parameters.
    """
    return _bezier.find_t_intersecting(
        lambda t: tuple(bezier_point_at_t(t)), inside_closedpath, (t0, t1), tolerance)


class BezierSegment:
    """
    A d-dimensional Bézier segment.

    Parameters
    ----------
    control_points : (N, d) array
        Location of the *N* control points.
    """

    def __init__(self, control_points):
        self._cpoints = np.asarray(control_points, float)
        self._N, self._d = self._cpoints.shape
        self._native = _bezier.segment(self._cpoints)

    def __call__(self, t):
        """
        Evaluate the Bézier curve at point(s) *t* in [0, 1].

        Parameters
        ----------
        t : (k,) array-like
            Points at which to evaluate the curve.

        Returns
        -------
        (k, d) array
            Value of the curve for each point in *t*.
        """
        return _bezier.segment_eval(self._native, np.atleast_1d(np.asarray(t, float)))

    def point_at_t(self, t):
        """
        Evaluate the curve at a single point, returning a tuple of *d* floats.
        """
        return tuple(self(t)[0])

    @property
    def control_points(self):
        """The control points of the curve."""
        return self._cpoints

    @property
    def dimension(self):
        """The dimension of the curve."""
        return self._d

    @property
    def degree(self):
        """Degree of the polynomial. One less the number of control points."""
        return self._N - 1

    @property
    def polynomial_coefficients(self):
        r"""
        The polynomial coefficients of the Bézier curve.

        .. warning:: Follows opposite convention from `numpy.polyval`.

        Returns
        -------
        (n+1, d) array
            Coefficients after expanding in polynomial basis, where :math:`n`
            is the degree of the Bézier curve and :math:`d` its dimension.
            These are the numbers (:math:`C_j`) such that the curve can be
            written :math:`\sum_{j=0}^n C_j t^j`.
        """
        return _bezier.segment_coefficients(self._native)

    def axis_aligned_extrema(self):
        """
        Return the dimension and location of the curve's interior extrema.

        The extrema are the points along the curve where one of its partial
        derivatives is zero.

        Returns
        -------
        dims : array of int
            Index :math:`i` of the partial derivative which is zero at each
            interior extrema.
        dzeros : array of float
            Of same size as dims. The :math:`t` such that :math:`d/dx_i B(t) =
            0`
        """
        return _bezier.segment_extrema(self._native)


def split_bezier_intersecting_with_closedpath(
        bezier, inside_closedpath, tolerance=0.01):
    """
    Split a Bézier curve into two at the intersection with a closed path.

    Parameters
    ----------
    bezier : (N, 2) array-like
        Control points of the Bézier segment. See `.BezierSegment`.
    inside_closedpath : callable
        A function returning True if a given point (x, y) is inside the
        closed path. See also `.find_bezier_t_intersecting_with_closedpath`.
    tolerance : float
        The tolerance for the intersection. See also
        `.find_bezier_t_intersecting_with_closedpath`.

    Returns
    -------
    left, right
        Lists of control points for the two Bézier segments.
    """
    return _bezier.split_intersecting(
        np.asarray(bezier, float), inside_closedpath, tolerance)


def split_path_inout(path, inside, tolerance=0.01, reorder_inout=False):
    """
    Divide a path into two segments at the point where ``inside(x, y)`` becomes
    False.
    """
    from .path import Path
    first, second = _bezier.split_inout(path, inside, tolerance, reorder_inout)
    return Path(*first), Path(*second)


def inside_circle(cx, cy, r):
    """
    Return a function that checks whether a point is in a circle with center
    (*cx*, *cy*) and radius *r*.

    The returned function has the signature::

        f(xy: tuple[float, float]) -> bool
    """
    r2 = r ** 2

    def _f(xy):
        x, y = xy
        return (x - cx) ** 2 + (y - cy) ** 2 < r2
    return _f


def get_cos_sin(x0, y0, x1, y1):
    """The cosine and sine of the direction from (*x0*, *y0*) to (*x1*,
    *y1*); zeros for coincident points."""
    return _bezier.cos_sin((x0, y0), (x1, y1))


def check_if_parallel(dx1, dy1, dx2, dy2, tolerance=1.e-5):
    """
    Check if two lines are parallel.

    Parameters
    ----------
    dx1, dy1, dx2, dy2 : float
        The gradients *dy*/*dx* of the two lines.
    tolerance : float
        The angular tolerance in radians up to which the lines are considered
        parallel.

    Returns
    -------
    is_parallel
        - 1 if two lines are parallel in same direction.
        - -1 if two lines are parallel in opposite direction.
        - False otherwise.
    """
    result = _bezier.parallel((dx1, dy1), (dx2, dy2), tolerance)
    return False if result is None else result


def get_parallels(bezier2, width):
    """
    Given the quadratic Bézier control points *bezier2*, returns
    control points of quadratic Bézier lines roughly parallel to given
    one separated by *width*.
    """
    left, right = _bezier.parallels(tuple(map(tuple, bezier2)), width)
    return list(left), list(right)


def find_control_points(c1x, c1y, mmx, mmy, c2x, c2y):
    """
    Find control points of the Bézier curve passing through (*c1x*, *c1y*),
    (*mmx*, *mmy*), and (*c2x*, *c2y*), at parametric values 0, 0.5, and 1.
    """
    return list(_bezier.control_points((c1x, c1y), (mmx, mmy), (c2x, c2y)))


def make_wedged_bezier2(bezier2, width, w1=1., wm=0.5, w2=0.):
    """
    Being similar to `get_parallels`, returns control points of two quadratic
    Bézier lines having a width roughly parallel to given one separated by
    *width*.
    """
    left, right = _bezier.wedged(tuple(map(tuple, bezier2)), width, (w1, wm, w2))
    return list(left), list(right)
