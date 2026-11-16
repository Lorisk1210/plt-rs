# Overlapping transparent shapes and lines to show alpha blending and zorder.
import matplotlib.pyplot as plt
from matplotlib.patches import Circle

fig, ax = plt.subplots(figsize=(5, 4.5))
for (x, y), c in zip([(0, 0), (1, 0), (0.5, 0.85)], ["red", "green", "blue"]):
    ax.add_patch(Circle((x, y), 0.8, color=c, alpha=0.35))
ax.plot([-1, 2], [-1, 2], color="k", lw=6, alpha=0.5, zorder=0)
ax.plot([-1, 2], [2, -1], color="orange", lw=6, alpha=0.8, zorder=3)
ax.set_xlim(-1, 2)
ax.set_ylim(-1, 2)
ax.set_aspect("equal")
fig.savefig("venn.png")
print([round(p.get_alpha(), 2) for p in ax.patches])
