# Affine2D construction variants (from_values, skew, rotate_around, set_matrix) and transform of deltas vs points.
import numpy as np
from matplotlib.transforms import Affine2D, Affine2DBase

a = Affine2D.from_values(1, 0.5, -0.5, 1, 3, 4)
print(np.round(a.get_matrix(), 6).tolist())
b = Affine2D().rotate_deg_around(1, 1, 90)
print(np.round(b.transform([[1, 1], [2, 1], [1, 3]]), 6).tolist())
c = Affine2D().skew_deg(30, 0)
print(np.round(c.transform([[0, 1], [1, 1]]), 6).tolist())
d = Affine2D(np.array([[2, 0, 1], [0, 3, -1], [0, 0, 1]], dtype=float))
print(np.round(d.transform_point((1, 1)), 6).tolist(), np.round(d.inverted().transform_point((3, 2)), 6).tolist())
m = d.get_matrix().copy()
m[0, 2] = 100
d.set_matrix(m)
print(np.round(d.transform([[0, 0]]), 6).tolist())
print(isinstance(d, Affine2DBase), d.is_affine, d.is_separable, Affine2D().rotate_deg(10).is_separable)
e = Affine2D().scale(2).rotate_deg(45).translate(1, 1)
f = Affine2D().translate(1, 1)
f.rotate_deg(45)
print(e == e.frozen(), np.round((e + f).get_matrix(), 6).tolist())
print(np.round(e.transform_affine(np.array([[1.0, 0.0]])), 6).tolist())
print(str(Affine2D().scale(2, 3)))
g = Affine2D().scale(0)
try:
    g.inverted().get_matrix()
    print("singular inverted")
except Exception as err:
    print(type(err).__name__)
h = Affine2D().scale(2, 3).translate(5, 5)
print(np.round(h.transform([[1, 1]]) - h.transform([[0, 0]]), 6).tolist())
