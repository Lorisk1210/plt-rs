# Two-set Venn diagram by hand with transparent circles and count labels.
import matplotlib.pyplot as plt
from matplotlib.patches import Circle

a_only, both, b_only = 25, 12, 18
fig, ax = plt.subplots(figsize=(5, 3.5))
ax.add_patch(Circle((0.38, 0.5), 0.3, alpha=0.4, color="C0"))
ax.add_patch(Circle((0.62, 0.5), 0.3, alpha=0.4, color="C1"))
for x, n in [(0.25, a_only), (0.5, both), (0.75, b_only)]:
    ax.text(x, 0.5, str(n), ha="center", va="center", fontsize=14)
ax.text(0.2, 0.88, "Python users", ha="center")
ax.text(0.8, 0.88, "R users", ha="center")
ax.set_xlim(0, 1)
ax.set_ylim(0.1, 0.95)
ax.set_aspect("equal")
ax.axis("off")
print(a_only + both + b_only)
fig.savefig("venn.png")
