# A reciprocal scale (1/x) as a ScaleBase subclass with its transforms, locators and formatters; registered and used by name.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import scale as mscale
from matplotlib.transforms import Transform
from matplotlib.ticker import FixedLocator, FuncFormatter, NullLocator


class ReciprocalTransform(Transform):
    input_dims = output_dims = 1

    def transform_non_affine(self, a):
        a = np.asarray(a, dtype=float)
        with np.errstate(divide="ignore"):
            return np.where(a > 0, 1.0 / a, np.nan)

    def inverted(self):
        return ReciprocalTransform()


class ReciprocalScale(mscale.ScaleBase):
    name = "reciprocal"

    def __init__(self, axis, *, ticks=(1, 2, 5, 10, 20, 50)):
        super().__init__(axis)
        self.ticks = ticks

    def get_transform(self):
        return ReciprocalTransform()

    def set_default_locators_and_formatters(self, axis):
        axis.set_major_locator(FixedLocator(self.ticks))
        axis.set_major_formatter(FuncFormatter(lambda v, _: f"1/{v:g}"))
        axis.set_minor_locator(NullLocator())

    def limit_range_for_scale(self, vmin, vmax, minpos):
        return max(vmin, minpos), max(vmax, minpos)


mscale.register_scale(ReciprocalScale)
print("reciprocal" in mscale.get_scale_names())
fig, ax = plt.subplots(figsize=(4, 3))
x = np.linspace(1, 50, 200)
ax.plot(x, np.log(x))
ax.set_xscale("reciprocal", ticks=(1, 2, 5, 10, 50))
fig.canvas.draw()
print(ax.get_xscale(), type(ax.xaxis.get_transform()).__name__)
print([t.get_text() for t in ax.get_xticklabels()], [round(v, 6) for v in ax.get_xlim()])
print(np.round(ax.transData.transform([[1, 0], [50, 0]])[:, 0], 3).tolist())
fig.savefig("reciprocal.png")
ax.set_xscale("linear")
print(ax.get_xscale(), [t.get_text() for t in ax.get_xticklabels()][:3])
