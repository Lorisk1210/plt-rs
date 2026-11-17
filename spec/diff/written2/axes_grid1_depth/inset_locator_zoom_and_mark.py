# inset_axes and zoomed_inset_axes with bbox_to_anchor in several transforms, mark_inset connectors and their corners.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from mpl_toolkits.axes_grid1.inset_locator import inset_axes, zoomed_inset_axes, mark_inset

fig, ax = plt.subplots(figsize=(6, 4), dpi=80)
x = np.linspace(0, 10, 400)
ax.plot(x, np.sin(5 * x) * np.exp(-x / 5))
a1 = inset_axes(ax, width="30%", height="25%", loc="upper right", borderpad=1)
a2 = inset_axes(ax, width=1.2, height=0.8, loc="lower left", bbox_to_anchor=(0.1, 0.1), bbox_transform=ax.transAxes)
a3 = zoomed_inset_axes(ax, zoom=4, loc="center right", bbox_to_anchor=(0.95, 0.4), bbox_transform=ax.transAxes)
a3.plot(x, np.sin(5 * x) * np.exp(-x / 5))
a3.set_xlim(2, 2.6)
a3.set_ylim(-0.7, 0.7)
patch, c1, c2 = mark_inset(ax, a3, loc1=2, loc2=4, fc="none", ec="0.5")
fig.canvas.draw()
for a in (a1, a2, a3):
    print([round(v, 4) for v in a.get_position().bounds], type(a.get_axes_locator()).__name__)
print(type(patch).__name__, c1.loc1, c1.loc2, c2.loc1, c2.loc2)
print([round(v, 2) for v in patch.get_path().get_extents().bounds])
print(np.round(c1.get_path().vertices, 1).tolist())
ax.set_xlim(0, 5)
fig.canvas.draw()
print([round(v, 4) for v in a3.get_position().bounds])
print(np.round(c1.get_path().vertices, 1).tolist())
fig.savefig("insets.png")
a4 = inset_axes(ax, width="40%", height="40%", bbox_to_anchor=(0.5, 0.5, 0.5, 0.5), bbox_transform=ax.transAxes,
                loc="lower left", borderpad=0)
fig.canvas.draw()
print([round(v, 4) for v in a4.get_position().bounds])
