# matplotlib.bezier helpers: BezierSegment evaluation, split_bezier_intersecting_with_closedpath, get_parallels, path lengths by sampling.
import numpy as np
from matplotlib import bezier
from matplotlib.path import Path

seg = bezier.BezierSegment(np.array([[0, 0], [1, 2], [3, 2], [4, 0]]))
ts = np.linspace(0, 1, 5)
print(seg.degree, seg.dimension, np.round(seg(ts), 4).tolist())
print(np.round(seg.polynomial_coefficients, 4).tolist())
print([np.round(r, 4).tolist() for r in seg.axis_aligned_extrema()])
print(np.round(seg.point_at_t(0.25), 4).tolist())


def inside_circle(xy, c=(4, 0), r=1.5):
    return (xy[0] - c[0]) ** 2 + (xy[1] - c[1]) ** 2 < r ** 2


left, right = bezier.split_bezier_intersecting_with_closedpath(seg.control_points, inside_circle, tolerance=0.001)
print(np.round(left, 3).tolist())
print(np.round(right, 3).tolist())
p1, p2 = bezier.get_parallels([(0, 0), (1, 1), (2, 0)], 0.1)
print(np.round(p1, 4).tolist(), np.round(p2, 4).tolist())
print(np.round(bezier.get_cos_sin(0, 0, 3, 4), 4).tolist())
print(bezier.check_if_parallel(1, 0, 2, 0), bezier.check_if_parallel(1, 0, 0, 1))
pts = seg(np.linspace(0, 1, 2001))
print(round(float(np.sum(np.hypot(*np.diff(pts, axis=0).T))), 4))
path = Path([(0, 0), (1, 2), (3, 2), (4, 0)], [Path.MOVETO, Path.CURVE4, Path.CURVE4, Path.CURVE4])
inner, outer = bezier.split_path_inout(path, lambda xy: xy[0] < 2)
print(len(inner.vertices), len(outer.vertices), np.round(inner.vertices[-1], 3).tolist())
print(np.round(bezier.find_bezier_t_intersecting_with_closedpath(seg.point_at_t, inside_circle), 4).tolist())
sl = bezier.make_wedged_bezier2([(0, 0), (1, 1), (2, 0)], 0.2, 1.0, 0.5, 0.0)
print(np.round(sl[0], 4).tolist())
