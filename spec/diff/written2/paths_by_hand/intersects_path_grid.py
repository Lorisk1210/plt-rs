# intersects_path / intersects_bbox on a grid of segments and shapes, filled and unfilled, printed as a matrix.
import numpy as np
from matplotlib.path import Path
from matplotlib.transforms import Bbox

shapes = {
    "square": Path.unit_rectangle(),
    "circle": Path.circle((0.5, 0.5), 0.5),
    "line": Path([(0, 0), (1, 1)]),
    "vline": Path([(0.5, -1), (0.5, 2)]),
    "far": Path([(5, 5), (6, 6)]),
    "inner": Path.circle((0.5, 0.5), 0.1),
    "touch": Path([(1, 0), (2, 0)]),
}
names = list(shapes)
print(" " * 8 + " ".join(n[:6].rjust(6) for n in names))
for a in names:
    row = []
    for b in names:
        r1 = shapes[a].intersects_path(shapes[b], filled=True)
        r2 = shapes[a].intersects_path(shapes[b], filled=False)
        row.append(f"{int(r1)}{int(r2)}".rjust(6))
    print(a.ljust(8) + " ".join(row))
boxes = [Bbox([[0.2, 0.2], [0.3, 0.3]]), Bbox([[2, 2], [3, 3]]), Bbox([[0.9, 0.9], [1.5, 1.5]]), Bbox([[-1, -1], [2, 2]])]
for name in names:
    print(name, [int(shapes[name].intersects_bbox(b)) for b in boxes],
          [int(shapes[name].intersects_bbox(b, filled=False)) for b in boxes])
star = Path.unit_regular_star(6, 0.5)
print([int(star.intersects_path(Path([(x, -2), (x, 2)]))) for x in np.linspace(-1.5, 1.5, 7)])
seg = Path([(0.2, 0.2), (0.8, 0.8)])
print(int(shapes["inner"].intersects_path(seg)), int(seg.intersects_path(shapes["inner"], filled=False)))
