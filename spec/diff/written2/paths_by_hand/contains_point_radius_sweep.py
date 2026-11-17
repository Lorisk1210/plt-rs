# Path.contains_point with signed radius on clockwise and counter-clockwise paths and through a transform.
import numpy as np
from matplotlib.path import Path
from matplotlib.transforms import Affine2D

ccw = Path([(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)], closed=True)
cw = Path(ccw.vertices[::-1], closed=True)
edge_points = [(0, 0.5), (1, 0.5), (0.5, 0), (0.5, 1), (0.5, 0.5), (1.05, 0.5), (-0.05, 0.5)]
for r in (-0.1, 0.0, 0.1):
    print(r, [int(ccw.contains_point(p, radius=r)) for p in edge_points],
          [int(cw.contains_point(p, radius=r)) for p in edge_points])
t = Affine2D().scale(10).rotate_deg(45)
print([int(ccw.contains_point(p, transform=t)) for p in [(0, 7), (0, 1), (5, 5), (-3, 3)]])
grid = np.c_[np.repeat(np.linspace(-0.5, 1.5, 9), 9), np.tile(np.linspace(-0.5, 1.5, 9), 9)]
for r in (-0.2, 0.2):
    a = ccw.contains_points(grid, radius=r).reshape(9, 9)
    b = cw.contains_points(grid, radius=r).reshape(9, 9)
    print(r, int(a.sum()), int(b.sum()))
open_path = Path([(0, 0), (1, 0), (1, 1)])
print(open_path.contains_point((0.8, 0.2)), open_path.contains_point((0.2, 0.8)))
curved = Path([(0, 0), (0.5, 2), (1, 0), (0, 0)], [Path.MOVETO, Path.CURVE3, Path.CURVE3, Path.CLOSEPOLY])
print([int(curved.contains_point((0.5, y))) for y in np.linspace(0.1, 1.1, 6)])
print(int(curved.contains_point((0.5, 0.95), radius=0.1)), int(curved.contains_point((0.5, 0.95), radius=-0.1)))
empty = Path(np.zeros((0, 2)))
print(empty.contains_points(np.array([[0, 0]])).tolist(), len(empty))
nanp = Path([(0, 0), (1, np.nan), (1, 1), (0, 1)], closed=True)
print(nanp.contains_point((0.5, 0.5)))
big = Path.circle((0, 0), 1)
print([int(big.contains_point((x, 0), radius=0.05)) for x in (0.94, 0.97, 1.0, 1.03, 1.06)])
print([int(big.contains_point((x, 0), radius=-0.05)) for x in (0.94, 0.97, 1.0, 1.03, 1.06)])
print(bool(cw.contains_point((0.5, 0.5))), bool(ccw.contains_point((0.5, 0.5))))
