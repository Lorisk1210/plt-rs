# Draw a floor plan with Rectangle, Circle, Ellipse, Polygon, FancyBboxPatch and Wedge patches.
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle, Circle, Ellipse, Polygon, FancyBboxPatch, Wedge, Arc

fig, ax = plt.subplots(figsize=(7, 5))
ax.add_patch(Rectangle((0, 0), 10, 6, fill=False, lw=2))
ax.add_patch(Rectangle((0.5, 0.5), 3, 2, fc="burlywood", ec="k", label="table"))
ax.add_patch(Circle((7, 4), 0.8, fc="lightgreen", ec="darkgreen", label="plant"))
ax.add_patch(Ellipse((5, 1.5), 2.5, 1.2, angle=20, fc="lightblue", ec="navy", label="rug"))
ax.add_patch(Polygon([[8, 0.5], [9.5, 0.5], [9.5, 2.5]], closed=True, fc="salmon", label="shelf"))
ax.add_patch(FancyBboxPatch((1, 4), 2.5, 1.2, boxstyle="round,pad=0.1", fc="khaki", ec="k", label="sofa"))
ax.add_patch(Wedge((10, 6), 1.5, 180, 270, fc="0.85", ec="k", label="door swing"))
ax.add_patch(Arc((0, 3), 1.5, 1.5, theta1=270, theta2=90, color="k"))
ax.set_xlim(-0.5, 10.5)
ax.set_ylim(-0.5, 6.5)
ax.set_aspect("equal")
ax.legend(loc="upper left", bbox_to_anchor=(1, 1), fontsize=8)
print(len(ax.patches))
fig.savefig("floor.png", bbox_inches="tight")
