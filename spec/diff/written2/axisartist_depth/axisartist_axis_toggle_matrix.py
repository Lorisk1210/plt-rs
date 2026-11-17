# Toggle ticks, ticklabels and label on each side of an axisartist Axes; print the visibility matrix and the tight extents.
import itertools
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from mpl_toolkits import axisartist

fig = plt.figure(figsize=(4, 3), dpi=80)
ax = fig.add_subplot(axes_class=axisartist.Axes)
ax.set_xlabel("x")
ax.set_ylabel("y")
for side in ("top", "right"):
    ax.axis[side].label.set_text(side)
r = fig.canvas.get_renderer()
for ticks, labels, label in itertools.product((True, False), repeat=3):
    ax.axis[:].toggle(ticks=ticks, ticklabels=labels, label=label)
    fig.canvas.draw()
    matrix = [(ax.axis[s].major_ticks.get_visible(), ax.axis[s].major_ticklabels.get_visible(),
               ax.axis[s].label.get_visible()) for s in ("left", "bottom", "right", "top")]
    tb = ax.get_tightbbox(r)
    print(ticks, labels, label, matrix[0], matrix[2], round(tb.width, 1), round(tb.height, 1))
ax.axis["bottom"].toggle(all=True)
ax.axis["left"].toggle(all=False, ticks=True)
fig.canvas.draw()
print([(ax.axis[s].major_ticks.get_visible(), ax.axis[s].major_ticklabels.get_visible()) for s in ("left", "bottom")])
fig.savefig("toggles.png")
ax.axis["right"].set_visible(False)
fig.canvas.draw()
print([s for s in ("left", "bottom", "right", "top") if ax.axis[s].get_visible()], round(ax.get_tightbbox(r).width, 1))
print(sorted(ax.axis.keys()))
