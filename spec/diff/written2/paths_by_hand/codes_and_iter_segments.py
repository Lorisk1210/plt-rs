# Build paths with every code by hand and walk them with iter_segments under simplify, curves and snapping options.
import numpy as np
from matplotlib.path import Path
from matplotlib.transforms import Affine2D

verts = [(0, 0), (1, 0), (1, 1), (0.5, 1.5), (0, 1), (0.2, 0.2), (0.4, 0.8), (0.8, 0.8), (0.9, 0.2), (0, 0)]
codes = [Path.MOVETO, Path.LINETO, Path.LINETO, Path.CURVE3, Path.CURVE3,
         Path.MOVETO, Path.CURVE4, Path.CURVE4, Path.CURVE4, Path.CLOSEPOLY]
p = Path(verts, codes)
print(p.codes.tolist(), p.vertices.shape, p.should_simplify, p.simplify_threshold)
for v, c in p.iter_segments():
    print(c, np.round(v, 4).tolist())
print("--- no curves")
for v, c in p.iter_segments(curves=False):
    print(c, np.round(v, 4).tolist())
print("--- transformed and clipped")
segs = list(p.iter_segments(transform=Affine2D().scale(100), clip=(0, 0, 60, 60), simplify=False))
print(len(segs), [c for _, c in segs])
print("--- bezier")
for curve, code in p.iter_bezier():
    print(code, curve.degree, np.round(curve.control_points, 3).tolist(), np.round(curve([0.5]), 4).tolist())
dense = Path(np.c_[np.linspace(0, 1, 500), 0.001 * np.sin(np.linspace(0, 40, 500))])
simple = list(dense.iter_segments(transform=Affine2D().scale(200), simplify=True))
print(len(simple), dense.should_simplify)
print(Path.NUM_VERTICES_FOR_CODE, Path.code_type)
p2 = Path(verts[:5], codes[:5])
print(len(p2), [round(x, 4) for x in p2.get_extents().bounds])
print([round(x, 4) for x in p.get_extents().bounds])
try:
    Path([(0, 0), (1, 1)], [Path.MOVETO])
except ValueError as e:
    print("bad codes:", e)
