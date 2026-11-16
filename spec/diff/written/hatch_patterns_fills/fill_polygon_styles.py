# ax.fill with several polygons in one call, edge styles and alpha, plus fill with a hatch.
import numpy as np
import matplotlib.pyplot as plt

t = np.linspace(0, 2 * np.pi, 7)[:-1]
fig, ax = plt.subplots(figsize=(6, 4))
polys = ax.fill(np.cos(t), np.sin(t), "C0", np.cos(t) + 2.5, np.sin(t), "C1", alpha=0.6)
ax.fill(np.cos(t[::2]) + 5, np.sin(t[::2]), facecolor="none", edgecolor="k", hatch="xx", linestyle="--")
ax.set_aspect("equal")
print(len(polys), len(ax.patches))
fig.savefig("polygons.png")
fig.savefig("fill_polygon_styles_small.png", dpi=40)
print(fig.get_size_inches().tolist())
