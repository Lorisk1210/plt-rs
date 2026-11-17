# A non-affine Transform subclass (a fisheye) with its own inverted(), used for a line, a grid and contains_point.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.transforms import Transform
from matplotlib.path import Path


class Fisheye(Transform):
    input_dims = output_dims = 2
    has_inverse = True

    def __init__(self, strength=0.5):
        super().__init__()
        self.strength = strength

    def transform_non_affine(self, values):
        v = np.asarray(values, dtype=float)
        r = np.hypot(v[:, 0], v[:, 1])
        f = 1 + self.strength * r
        return v / f[:, None]

    def inverted(self):
        return InverseFisheye(self.strength)


class InverseFisheye(Transform):
    input_dims = output_dims = 2
    has_inverse = True

    def __init__(self, strength):
        super().__init__()
        self.strength = strength

    def transform_non_affine(self, values):
        v = np.asarray(values, dtype=float)
        rho = np.hypot(v[:, 0], v[:, 1])
        with np.errstate(divide="ignore", invalid="ignore"):
            r = np.where(rho > 0, rho / (1 - self.strength * rho), 0.0)
            f = np.where(rho > 0, r / rho, 1.0)
        return v * f[:, None]

    def inverted(self):
        return Fisheye(self.strength)


fish = Fisheye(0.3)
pts = np.array([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [-2.0, 0.5]])
out = fish.transform(pts)
print(np.round(out, 6).tolist())
print(np.allclose(fish.inverted().transform(out), pts), fish.is_affine, fish.has_inverse)
fig, ax = plt.subplots(figsize=(4, 4))
t = fish + ax.transData
for k in np.linspace(-2, 2, 9):
    ax.plot(np.full(50, k), np.linspace(-2, 2, 50), color="0.6", lw=0.5, transform=t)
    ax.plot(np.linspace(-2, 2, 50), np.full(50, k), color="0.6", lw=0.5, transform=t)
ax.set_xlim(-1.5, 1.5)
ax.set_ylim(-1.5, 1.5)
path = Path([(0, 0), (1.5, 0), (1.5, 1.5), (0, 1.5), (0, 0)], closed=True)
tp = fish.transform_path(path)
print(np.round(tp.vertices, 6).tolist(), tp.contains_point((0.5, 0.5)))
fig.savefig("fisheye.png")
print(type(t).__name__, t.is_affine, (fish + fish.inverted()).transform([[1.0, 1.0]]).round(9).tolist())
