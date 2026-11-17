# GridHelperCurveLinear with a pair of plain functions (a shear), new_floating_axis, and the gridlines it produces.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from mpl_toolkits.axisartist import Axes, GridHelperCurveLinear


def tr(x, y):
    return x + 0.5 * y, y


def inv_tr(x, y):
    return x - 0.5 * y, y


helper = GridHelperCurveLinear((tr, inv_tr))
fig = plt.figure(figsize=(5, 4), dpi=80)
ax = fig.add_subplot(axes_class=Axes, grid_helper=helper)
ax.set_xlim(0, 10)
ax.set_ylim(0, 10)
ax.plot(*tr(np.array([1, 4, 4, 1, 1]), np.array([1, 1, 6, 6, 1])), "r-")
ax.axis["t"] = ax.new_floating_axis(0, 3)
ax.axis["t2"] = ax.new_floating_axis(1, 7)
ax.grid(True, zorder=0)
fig.canvas.draw()
print(sorted(ax.axis.keys()))
gl = helper.get_gridlines(which="major", axis="both")
print(len(gl), [len(l[0]) for l in gl[:3]])
print(np.round(np.array(gl[0]).T[:2], 3).tolist())
print([round(v, 2) for v in ax.axis["t"].get_tightbbox(fig.canvas.get_renderer()).bounds])
x, y = tr(np.array([2.0]), np.array([4.0]))
print(x.tolist(), y.tolist(), [v.tolist() for v in inv_tr(x, y)])
fig.savefig("curvilinear.png")
ax.set_xlim(-5, 5)
fig.canvas.draw()
print(len(helper.get_gridlines(which="major", axis="both")))
