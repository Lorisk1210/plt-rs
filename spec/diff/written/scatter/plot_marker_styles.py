# Showcase of marker styles users look up: filled, unfilled, custom path, mathtext marker.
import matplotlib.pyplot as plt
from matplotlib.markers import MarkerStyle

markers = ["o", "v", "^", "<", ">", "s", "p", "*", "h", "H", "+", "x", "D", "d", "|", "_", "1", "2", "3", "4"]
fig, ax = plt.subplots(figsize=(8, 3))
for i, m in enumerate(markers):
    ax.plot(i, 1, marker=m, ms=10, color="k", ls="")
    ax.text(i, 1.3, repr(m), ha="center", fontsize=7)
ax.plot(range(4), [0] * 4, marker="$\\clubsuit$", ms=14, ls="", color="green")
ax.plot(range(4, 8), [0] * 4, marker=MarkerStyle("o", fillstyle="left"), ms=12, ls="", color="C1")
ax.plot(range(8, 12), [0] * 4, marker=(5, 1, 0), ms=12, ls="", color="C2")
ax.set_ylim(-0.6, 1.8)
ax.axis("off")
fig.savefig("markers.png")
print(len(ax.lines))
