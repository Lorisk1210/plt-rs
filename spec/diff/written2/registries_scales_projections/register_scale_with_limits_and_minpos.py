# A "sqrt" ScaleBase whose limit_range_for_scale clamps negatives; check autoscaling with negative data and minpos.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import scale as mscale
from matplotlib.transforms import Transform
from matplotlib.ticker import AutoLocator, ScalarFormatter, AutoMinorLocator


class Sqrt(Transform):
    input_dims = output_dims = 1

    def transform_non_affine(self, a):
        return np.sqrt(np.clip(np.asarray(a, dtype=float), 0, None))

    def inverted(self):
        return Square()


class Square(Transform):
    input_dims = output_dims = 1

    def transform_non_affine(self, a):
        return np.asarray(a, dtype=float) ** 2

    def inverted(self):
        return Sqrt()


class SqrtScale(mscale.ScaleBase):
    name = "usr2sqrt"

    def get_transform(self):
        return Sqrt()

    def set_default_locators_and_formatters(self, axis):
        axis.set_major_locator(AutoLocator())
        axis.set_major_formatter(ScalarFormatter())
        axis.set_minor_locator(AutoMinorLocator())

    def limit_range_for_scale(self, vmin, vmax, minpos):
        return max(0.0, vmin), vmax


mscale.register_scale(SqrtScale)
fig, ax = plt.subplots(figsize=(4, 3))
ax.plot([-5, 0, 10, 100], [-5, 0, 10, 100], "o-")
ax.set_yscale("usr2sqrt")
fig.canvas.draw()
print([round(v, 6) for v in ax.get_ylim()], ax.get_yticks().tolist()[:6])
ax.set_ylim(-10, 50)
print([round(v, 6) for v in ax.get_ylim()])
print(np.round(ax.transData.transform([[0, 0], [0, 25]])[:, 1], 3).tolist())
fig.savefig("sqrt.png")
