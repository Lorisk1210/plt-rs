# Patches with dashed and dotted edges, capstyle/joinstyle options and fill=False.
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle, Circle, RegularPolygon

fig, ax = plt.subplots(figsize=(6, 3))
ax.add_patch(Rectangle((0, 0), 1, 1, fill=False, ls="--", lw=2))
ax.add_patch(Circle((2, 0.5), 0.5, fill=False, ls=":", lw=3, ec="C1"))
ax.add_patch(RegularPolygon((3.5, 0.5), 5, radius=0.55, fill=False, ls=(0, (8, 3, 2, 3)), lw=2, ec="C2",
                            joinstyle="round", capstyle="round"))
ax.set_xlim(-0.5, 4.5)
ax.set_ylim(-0.3, 1.3)
ax.set_aspect("equal")
print([p.get_linestyle() for p in ax.patches])
fig.savefig("dashed_edges.png")
