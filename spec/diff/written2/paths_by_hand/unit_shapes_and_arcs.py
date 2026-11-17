# The Path class constructors: unit_regular_polygon/star/asterisk, arc and wedge at many angles, circle with readius, readonly checks.
import numpy as np
from matplotlib.path import Path

for n in (3, 5, 8):
    print(n, len(Path.unit_regular_polygon(n).vertices), len(Path.unit_regular_star(n, 0.3).vertices),
          len(Path.unit_regular_asterisk(n).vertices))
print(np.round(Path.unit_regular_polygon(4).vertices, 4).tolist())
for t1, t2 in [(0, 90), (0, 360), (45, 135), (-30, 30), (350, 10), (0, 1)]:
    a = Path.arc(t1, t2)
    w = Path.wedge(t1, t2)
    a8 = Path.arc(t1, t2, n=8)
    print(t1, t2, len(a.vertices), len(w.vertices), len(a8.vertices), np.round(a.vertices[-1], 5).tolist())
c = Path.circle((1, 2), 3)
print(len(c.vertices), np.round(c.get_extents().bounds, 4).tolist())
cr = Path.circle((0, 0), 1, readonly=True)
try:
    cr.vertices = np.zeros((2, 2))
except AttributeError as e:
    print("readonly:", e)
uc = Path.unit_circle()
print(uc is Path.unit_circle(), uc.readonly, len(Path.unit_circle_righthalf().vertices))
copy = uc.copy()
dcopy = uc.deepcopy()
print(copy.readonly, dcopy.readonly, copy.vertices is uc.vertices, dcopy.vertices is uc.vertices)
print(np.round(Path.arc(90, 0).vertices[:4], 4).tolist(), len(Path.arc(90, 0).vertices))
print(np.round(Path.unit_rectangle().vertices, 2).tolist(), Path.unit_rectangle().codes.tolist())
star = Path.unit_regular_star(5, 0.5)
print(star.codes[0], star.codes[-1], np.round(star.vertices[:3], 4).tolist())
print(np.round(Path.unit_regular_asterisk(4).vertices, 4).tolist())
