# axes_grid1 inset_locator: inset_axes with a fixed size in inches and mark_inset.
import numpy as np
import matplotlib.pyplot as plt
from mpl_toolkits.axes_grid1.inset_locator import inset_axes, mark_inset, zoomed_inset_axes

x = np.linspace(-3, 3, 400)
y = np.tanh(3 * x)
fig, ax = plt.subplots(figsize=(6, 4))
ax.plot(x, y)
ins = inset_axes(ax, width="35%", height=1.0, loc="lower right", borderpad=2)
ins.hist(np.diff(y), bins=20)
ins.set_title("steps", fontsize=8)
zoom = zoomed_inset_axes(ax, zoom=4, loc="upper left")
zoom.plot(x, y)
zoom.set_xlim(-0.2, 0.2)
zoom.set_ylim(-0.5, 0.5)
mark_inset(ax, zoom, loc1=2, loc2=4, fc="none", ec="0.5")
zoom.set_xticks([])
zoom.set_yticks([])
print(type(ins).__name__)
fig.savefig("toolkit_insets.png")
