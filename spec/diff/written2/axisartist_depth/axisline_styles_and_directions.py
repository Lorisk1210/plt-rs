# axisartist Axes: axisline_style arrows, set_axis_direction, ticks inside/outside, label padding and toggles per axis.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from mpl_toolkits.axisartist.axislines import AxesZero

fig = plt.figure(figsize=(5, 4), dpi=80)
ax = fig.add_subplot(axes_class=AxesZero)
ax.plot(np.linspace(-2, 2, 20), np.linspace(-2, 2, 20) ** 3 / 4)
for d in ("xzero", "yzero"):
    ax.axis[d].set_axisline_style("-|>", size=1.5)
    ax.axis[d].set_visible(True)
for d in ("left", "right", "bottom", "top"):
    ax.axis[d].set_visible(False)
ax.axis["xzero"].set_axis_direction("top")
ax.axis["yzero"].set_axis_direction("right")
ax.axis["xzero"].major_ticks.set_tick_out(True)
ax.axis["yzero"].major_ticklabels.set_pad(8)
ax.axis["xzero"].label.set_text("x")
ax.axis["xzero"].label.set_visible(True)
ax.axis["yzero"].toggle(ticklabels=False, label=True)
ax.axis["yzero"].label.set_text("y")
fig.canvas.draw()
print(sorted(ax.axis.keys()))
for d in ("xzero", "yzero"):
    a = ax.axis[d]
    print(d, type(a).__name__, a.get_visible(), a.major_ticks.get_tick_out(), a.major_ticklabels.get_visible(),
          a.label.get_visible(), a.major_ticklabels.get_pad(), a.label.get_text())
print(type(ax.axis["xzero"].line).__name__, type(ax.axis["xzero"]._axisline_style).__name__)
print([round(v, 1) for v in ax.axis["xzero"].get_tightbbox(fig.canvas.get_renderer()).bounds])
ax.axis["xzero"].toggle(all=False)
fig.canvas.draw()
print(ax.axis["xzero"].major_ticks.get_visible(), ax.axis["xzero"].line.get_visible())
fig.savefig("axislines.png")
