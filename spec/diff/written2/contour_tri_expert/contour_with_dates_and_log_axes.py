# Contours on log-scaled and date axes, contourf with LogNorm and its colorbar ticks; level choices and path extents.
import datetime
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import colors, ticker

x = np.logspace(0, 3, 40)
y = np.linspace(0, 10, 30)
X, Y = np.meshgrid(x, y)
Z = np.outer(np.exp(-y / 3), x) + 1
fig, axs = plt.subplots(1, 2, figsize=(8, 3), dpi=60)
axs[0].set_xscale("log")
cf = axs[0].contourf(X, Y, Z, locator=ticker.LogLocator(), norm=colors.LogNorm(), cmap="viridis")
cb = fig.colorbar(cf, ax=axs[0])
fig.canvas.draw()
print(np.round(np.log10(cf.levels), 3).tolist(), type(cb.locator).__name__, [t.get_text() for t in cb.ax.get_yticklabels()][:3])
print([round(v, 3) for v in axs[0].get_xlim()], axs[0].get_xscale())
days = np.array([np.datetime64("2024-01-01") + np.timedelta64(k, "D") for k in range(20)])
D, H = np.meshgrid(np.arange(20), np.arange(24))
temp = np.sin(D / 3) + np.cos(H / 4)
cs = axs[1].contour(days, np.arange(24), temp, levels=6)
fig.canvas.draw()
print(type(axs[1].xaxis.get_converter()).__name__, [t.get_text() for t in axs[1].get_xticklabels()][:3])
print(np.round(cs.levels, 3).tolist(), [round(v, 2) for v in axs[1].get_xlim()])
cl = axs[1].clabel(cs, fmt="%.1f")
print(len(cl))
fig.savefig("contour_scales.png")
axs[1].set_xlim(days[5], days[10])
fig.canvas.draw()
print([t.get_text() for t in axs[1].get_xticklabels()][:3])
