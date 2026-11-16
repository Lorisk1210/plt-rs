# Annotated photo-like image: scale bar, north arrow, a text box with acquisition parameters and a colour bar for depth.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.offsetbox import AnchoredText
from mpl_toolkits.axes_grid1.anchored_artists import AnchoredSizeBar

y, x = np.mgrid[0:1:150j, 0:1:200j]
depth = 50 * np.sin(4 * x) * np.cos(3 * y) + 100
fig, ax = plt.subplots(figsize=(7, 5))
im = ax.imshow(depth, cmap="ocean_r", extent=(0, 2000, 0, 1500), origin="lower")
fig.colorbar(im, ax=ax, label="depth [m]", shrink=0.8)
ax.add_artist(AnchoredSizeBar(ax.transData, 500, "500 m", "lower left", pad=0.4, color="k", frameon=True, size_vertical=15))
ax.annotate("N", xy=(0.93, 0.92), xytext=(0.93, 0.78), xycoords="axes fraction", ha="center", fontsize=14,
            arrowprops=dict(arrowstyle="-|>", lw=2))
ax.add_artist(AnchoredText("sonar survey\n2024-06-03\nres 10 m", loc="upper left", prop=dict(size=8)))
ax.set_xlabel("easting [m]")
ax.set_ylabel("northing [m]")
print(round(float(depth.min()), 3), round(float(depth.max()), 3), len(ax.artists))
fig.savefig("survey_map.png", dpi=50)
