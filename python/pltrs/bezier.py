"""
A module providing some utility functions regarding Bézier path manipulation
(matplotlib's ``bezier``), computed by pltrs (Rust).

pltrs (Rust) computes with finite Python numbers. Other arguments (numpy
scalars, which matplotlib's arithmetic keeps, infinities, NaN, what is no
number) are computed with matplotlib 3.11.2's arithmetic (under the
Matplotlib License), for its result types, errors and warnings.
"""

import numpy as np

from . import _api
from ._native import bezier as _bezier
from ._native import register as _register


class NonIntersectingPathException(ValueError):
    pass


_register("NonIntersectingPathException", NonIntersectingPathException)


def _plain(*values):
    """Whether pltrs (Rust) computes with *values* as matplotlib's
    arithmetic does: finite Python ints and floats."""
    return all(type(value) in (int, float) and math.isfinite(value)
               for value in values)


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
    if _plain(cx1, cy1, cos_t1, sin_t1, cx2, cy2, cos_t2, sin_t2):
        return _bezier.intersection((cx1, cy1), (cos_t1, sin_t1),
                                    (cx2, cy2), (cos_t2, sin_t2))
    # line1 => sin_t1 * (x - cx1) - cos_t1 * (y - cy1) = 0.
    # line1 => sin_t1 * x + cos_t1 * y = sin_t1*cx1 - cos_t1*cy1
    line1_rhs = sin_t1 * cx1 - cos_t1 * cy1
    line2_rhs = sin_t2 * cx2 - cos_t2 * cy2

    # rhs matrix
    a, b = sin_t1, -cos_t1
    c, d = sin_t2, -cos_t2

    ad_bc = a * d - b * c
    if abs(ad_bc) < 1e-12:
        raise ValueError("Given lines do not intersect. Please verify that "
                         "the angles are not equal or differ by 180 degrees.")

    # rhs_inverse
    a_, b_ = d, -b
    c_, d_ = -c, a
    a_, b_, c_, d_ = (k / ad_bc for k in [a_, b_, c_, d_])

    x = a_ * line1_rhs + b_ * line2_rhs
    y = c_ * line1_rhs + d_ * line2_rhs

    return x, y


def get_normal_points(cx, cy, cos_t, sin_t, length):
    """
    For a line passing through (*cx*, *cy*) and having an angle *t*, return
    locations of the two points located along its perpendicular line at the
    distance of *length*.
    """
    # matplotlib 3.11.2's arithmetic, which keeps the types of its operands.
    if length == 0.:
        return cx, cy, cx, cy

    cos_t1, sin_t1 = sin_t, -cos_t
    cos_t2, sin_t2 = -sin_t, cos_t

    x1, y1 = length * cos_t1 + cx, length * sin_t1 + cy
    x2, y2 = length * cos_t2 + cx, length * sin_t2 + cy

    return x1, y1, x2, y2


def split_de_casteljau(beta, t):
    """
    Split a Bézier segment defined by its control points *beta* into two
    separate segments divided at *t* and return their control points.
    """
    if not _plain(t):
        beta = np.asarray(beta)
        beta_list = [beta]
        while True:
            beta = _de_casteljau1(beta, t)
            beta_list.append(beta)
            if len(beta) == 1:
                break
        left_beta = [beta[0] for beta in beta_list]
        right_beta = [beta[-1] for beta in reversed(beta_list)]
        return left_beta, right_beta
    beta = np.asarray(beta, float)
    left, right = _bezier.split_de_casteljau(beta.reshape((len(beta), -1)), t)
    # Lists of control points, as matplotlib's.
    return list(left), list(right)


def _de_casteljau1(beta, t):
    next_beta = beta[:-1] * (1 - t) + beta[1:] * t
    return next_beta


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
    if _plain(t0, t1, tolerance):
        return _bezier.find_t_intersecting(
            lambda t: tuple(bezier_point_at_t(t)), inside_closedpath, (t0, t1),
            tolerance)
    start = bezier_point_at_t(t0)
    end = bezier_point_at_t(t1)

    start_inside = inside_closedpath(start)
    end_inside = inside_closedpath(end)

    if start_inside == end_inside and start != end:
        raise NonIntersectingPathException(
            "Both points are on the same side of the closed path")

    while True:

        # return if the distance is smaller than the tolerance
        if np.hypot(start[0] - end[0], start[1] - end[1]) < tolerance:
            return t0, t1

        # calculate the middle point
        middle_t = 0.5 * (t0 + t1)
        middle = bezier_point_at_t(middle_t)
        middle_inside = inside_closedpath(middle)

        if start_inside ^ middle_inside:
            t1 = middle_t
            if end == middle:
                # Edge case where infinite loop is possible
                # Caused by large numbers relative to tolerance
                return t0, t1
            end = middle
        else:
            t0 = middle_t
            if start == middle:
                # Edge case where infinite loop is possible
                # Caused by large numbers relative to tolerance
                return t0, t1
            start = middle
            start_inside = middle_inside


class BezierSegment:
    """
    A d-dimensional Bézier segment.

    A BezierSegment can be called with an argument, either a scalar or an array-like
    object, to evaluate the curve at that/those location(s).

    Parameters
    ----------
    control_points : (N, d) array
        Location of the *N* control points.
    """

    def __init__(self, control_points):
        self._N, self._d = np.asarray(control_points).shape
        self._cpoints = np.asarray(control_points, float)
        # pltrs (Rust) computes with curves of one control point or more;
        # matplotlib's formulas make a curve of none 0 everywhere, without
        # extrema, and fail to expand it into a polynomial.
        self._native = _bezier.segment(self._cpoints) if self._N else None

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
        t = np.asarray(t)
        # matplotlib's first arithmetic on the points, which fails as it
        # does on what are no numbers.
        1 - t  # noqa: B018
        t = t.astype(float)
        if self._native is None:
            return np.zeros(t.shape + (self._d,))
        if not np.isfinite(t).all():
            # matplotlib's polynomial, as numpy evaluates (and warns about)
            # it beyond the curve.
            orders = np.arange(self._N)
            coefficients = [math.comb(self._N - 1, i) for i in range(self._N)]
            return (np.power.outer(1 - t, orders[::-1]) * np.power.outer(t, orders)) \
                @ (np.asarray(coefficients)[:, None] * self._cpoints)
        values = _bezier.segment_eval(self._native, np.atleast_1d(t))
        # One point per value of *t*, as matplotlib's ``np.power.outer``.
        return values.reshape(t.shape + (self._d,))

    @_api.deprecated(
        "3.11", alternative="Call the BezierSegment object with an argument.")
    def point_at_t(self, t):
        """
        Evaluate the curve at a single point, returning a tuple of *d* floats.
        """
        return tuple(self(t))

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

        Notes
        -----
        The coefficients are calculated as

        .. math::

            {n \choose j} \sum_{i=0}^j (-1)^{i+j} {j \choose i} P_i

        where :math:`P_i` are the control points of the curve.
        """
        if self._native is None:
            # numpy's error, which matplotlib's binomials meet first.
            raise ValueError('cannot call `vectorize` on size 0 inputs '
                             'unless `otypes` is set')
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
        if self._native is None:
            return np.array([]), np.array([])
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
    if not _plain(tolerance):
        bz = BezierSegment(bezier)
        t0, t1 = find_bezier_t_intersecting_with_closedpath(
            lambda t: tuple(bz(t)), inside_closedpath, tolerance=tolerance)
        _left, _right = split_de_casteljau(bezier, (t0 + t1) / 2.)
        return _left, _right
    left, right = _bezier.split_intersecting(
        np.asarray(bezier, float), inside_closedpath, tolerance)
    # Lists of control points, as matplotlib's `split_de_casteljau`.
    return list(left), list(right)


def split_path_inout(path, inside, tolerance=0.01, reorder_inout=False):
    """
    Divide a path into two segments at the point where ``inside(x, y)`` becomes
    False.
    """
    from .path import Path
    if _plain(tolerance):
        # matplotlib walks the path's segments first.
        path.iter_segments  # noqa: B018
        first, second = _bezier.split_inout(path, inside, tolerance, reorder_inout)
        return Path(*first), Path(*second)
    path_iter = path.iter_segments()

    ctl_points, command = next(path_iter)
    begin_inside = inside(ctl_points[-2:])  # true if begin point is inside

    ctl_points_old = ctl_points

    iold = 0
    i = 1

    for ctl_points, command in path_iter:
        iold = i
        i += len(ctl_points) // 2
        if inside(ctl_points[-2:]) != begin_inside:
            bezier_path = np.concatenate([ctl_points_old[-2:], ctl_points])
            break
        ctl_points_old = ctl_points
    else:
        raise ValueError("The path does not intersect with the patch")

    bp = bezier_path.reshape((-1, 2))
    left, right = split_bezier_intersecting_with_closedpath(
        bp, inside, tolerance)
    if len(left) == 2:
        codes_left = [Path.LINETO]
        codes_right = [Path.MOVETO, Path.LINETO]
    elif len(left) == 3:
        codes_left = [Path.CURVE3, Path.CURVE3]
        codes_right = [Path.MOVETO, Path.CURVE3, Path.CURVE3]
    elif len(left) == 4:
        codes_left = [Path.CURVE4, Path.CURVE4, Path.CURVE4]
        codes_right = [Path.MOVETO, Path.CURVE4, Path.CURVE4, Path.CURVE4]
    else:
        raise AssertionError("This should never be reached")

    verts_left = left[1:]
    verts_right = right[:]

    if path.codes is None:
        path_in = Path(np.concatenate([path.vertices[:i], verts_left]))
        path_out = Path(np.concatenate([verts_right, path.vertices[i:]]))

    else:
        path_in = Path(np.concatenate([path.vertices[:iold], verts_left]),
                       np.concatenate([path.codes[:iold], codes_left]))

        path_out = Path(np.concatenate([verts_right, path.vertices[i:]]),
                        np.concatenate([codes_right, path.codes[i:]]))

    if reorder_inout and not begin_inside:
        path_in, path_out = path_out, path_in

    return path_in, path_out


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
    if _plain(x0, y0, x1, y1):
        return _bezier.cos_sin((x0, y0), (x1, y1))
    dx, dy = x1 - x0, y1 - y0
    d = (dx * dx + dy * dy) ** .5
    # Account for divide by zero
    if d == 0:
        return 0.0, 0.0
    return dx / d, dy / d


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
    if not _plain(dx1, dy1, dx2, dy2, tolerance):
        theta1 = np.arctan2(dx1, dy1)
        theta2 = np.arctan2(dx2, dy2)
        dtheta = abs(theta1 - theta2)
        if dtheta < tolerance:
            return 1
        elif abs(dtheta - np.pi) < tolerance:
            return -1
        else:
            return False
    result = _bezier.parallel((dx1, dy1), (dx2, dy2), tolerance)
    return False if result is None else result


def get_parallels(bezier2, width):
    """
    Given the quadratic Bézier control points *bezier2*, returns
    control points of quadratic Bézier lines roughly parallel to given
    one separated by *width*.
    """
    if not (_plain(width) and _plain_points(bezier2, 3)):
        return _parallels(bezier2, width)
    left, right = _bezier.parallels(tuple(map(tuple, bezier2)), width)
    return list(left), list(right)


def _plain_points(points, count):
    """Whether *points* are *count* pairs of plain numbers (`_plain`)."""
    try:
        return len(points) == count and all(
            len(point) == 2 and _plain(*point) for point in points)
    except TypeError:
        return False


def _parallels(bezier2, width):
    """`get_parallels` as matplotlib computes it."""
    c1x, c1y = bezier2[0]
    cmx, cmy = bezier2[1]
    c2x, c2y = bezier2[2]

    parallel_test = check_if_parallel(c1x - cmx, c1y - cmy,
                                      cmx - c2x, cmy - c2y)

    if parallel_test == -1:
        _api.warn_external(
            "Lines do not intersect. A straight line is used instead.")
        cos_t1, sin_t1 = get_cos_sin(c1x, c1y, c2x, c2y)
        cos_t2, sin_t2 = cos_t1, sin_t1
    else:
        # t1 and t2 is the angle between c1 and cm, cm, c2.  They are
        # also an angle of the tangential line of the path at c1 and c2
        cos_t1, sin_t1 = get_cos_sin(c1x, c1y, cmx, cmy)
        cos_t2, sin_t2 = get_cos_sin(cmx, cmy, c2x, c2y)

    # find c1_left, c1_right which are located along the lines
    # through c1 and perpendicular to the tangential lines of the
    # Bezier path at a distance of width. Same thing for c2_left and
    # c2_right with respect to c2.
    c1x_left, c1y_left, c1x_right, c1y_right = (
        get_normal_points(c1x, c1y, cos_t1, sin_t1, width)
    )
    c2x_left, c2y_left, c2x_right, c2y_right = (
        get_normal_points(c2x, c2y, cos_t2, sin_t2, width)
    )

    # find cm_left which is the intersecting point of a line through
    # c1_left with angle t1 and a line through c2_left with angle
    # t2. Same with cm_right.
    try:
        cmx_left, cmy_left = get_intersection(c1x_left, c1y_left, cos_t1,
                                              sin_t1, c2x_left, c2y_left,
                                              cos_t2, sin_t2)
        cmx_right, cmy_right = get_intersection(c1x_right, c1y_right, cos_t1,
                                                sin_t1, c2x_right, c2y_right,
                                                cos_t2, sin_t2)
    except ValueError:
        # Special case straight lines, i.e., angle between two lines is
        # less than the threshold used by get_intersection (we don't use
        # check_if_parallel as the threshold is not the same).
        cmx_left, cmy_left = (
            0.5 * (c1x_left + c2x_left), 0.5 * (c1y_left + c2y_left)
        )
        cmx_right, cmy_right = (
            0.5 * (c1x_right + c2x_right), 0.5 * (c1y_right + c2y_right)
        )

    # the parallel Bezier lines are created with control points of
    # [c1_left, cm_left, c2_left] and [c1_right, cm_right, c2_right]
    path_left = [(c1x_left, c1y_left),
                 (cmx_left, cmy_left),
                 (c2x_left, c2y_left)]
    path_right = [(c1x_right, c1y_right),
                  (cmx_right, cmy_right),
                  (c2x_right, c2y_right)]

    return path_left, path_right


def find_control_points(c1x, c1y, mmx, mmy, c2x, c2y):
    """
    Find control points of the Bézier curve passing through (*c1x*, *c1y*),
    (*mmx*, *mmy*), and (*c2x*, *c2y*), at parametric values 0, 0.5, and 1.
    """
    # matplotlib 3.11.2's arithmetic, which keeps the types of its operands.
    cmx = .5 * (4 * mmx - (c1x + c2x))
    cmy = .5 * (4 * mmy - (c1y + c2y))
    return [(c1x, c1y), (cmx, cmy), (c2x, c2y)]


def make_wedged_bezier2(bezier2, width, w1=1., wm=0.5, w2=0.):
    """
    Being similar to `get_parallels`, returns control points of two quadratic
    Bézier lines having a width roughly parallel to given one separated by
    *width*.
    """
    # matplotlib 3.11.2's function, as it is.

    # c1, cm, c2
    c1x, c1y = bezier2[0]
    cmx, cmy = bezier2[1]
    c3x, c3y = bezier2[2]

    # t1 and t2 is the angle between c1 and cm, cm, c3.
    # They are also an angle of the tangential line of the path at c1 and c3
    cos_t1, sin_t1 = get_cos_sin(c1x, c1y, cmx, cmy)
    cos_t2, sin_t2 = get_cos_sin(cmx, cmy, c3x, c3y)

    # find c1_left, c1_right which are located along the lines
    # through c1 and perpendicular to the tangential lines of the
    # Bezier path at a distance of width. Same thing for c3_left and
    # c3_right with respect to c3.
    c1x_left, c1y_left, c1x_right, c1y_right = (
        get_normal_points(c1x, c1y, cos_t1, sin_t1, width * w1)
    )
    c3x_left, c3y_left, c3x_right, c3y_right = (
        get_normal_points(c3x, c3y, cos_t2, sin_t2, width * w2)
    )

    # find c12, c23 and c123 which are middle points of c1-cm, cm-c3 and
    # c12-c23
    c12x, c12y = (c1x + cmx) * .5, (c1y + cmy) * .5
    c23x, c23y = (cmx + c3x) * .5, (cmy + c3y) * .5
    c123x, c123y = (c12x + c23x) * .5, (c12y + c23y) * .5

    # tangential angle of c123 (angle between c12 and c23)
    cos_t123, sin_t123 = get_cos_sin(c12x, c12y, c23x, c23y)

    c123x_left, c123y_left, c123x_right, c123y_right = (
        get_normal_points(c123x, c123y, cos_t123, sin_t123, width * wm)
    )

    path_left = find_control_points(c1x_left, c1y_left,
                                    c123x_left, c123y_left,
                                    c3x_left, c3y_left)
    path_right = find_control_points(c1x_right, c1y_right,
                                     c123x_right, c123y_right,
                                     c3x_right, c3y_right)

    return path_left, path_right


# What matplotlib's module also has from its imports, which programs
# import from it.
import math  # noqa: E402, F401
import warnings  # noqa: E402, F401
from functools import lru_cache  # noqa: E402, F401
