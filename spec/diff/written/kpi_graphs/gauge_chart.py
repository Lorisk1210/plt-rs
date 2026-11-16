# Speedometer-style gauge from wedges with a needle and the value in the centre.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Wedge

value = 72
fig, ax = plt.subplots(figsize=(5, 3))
bands = [(0, 50, "#d73027"), (50, 80, "#fee08b"), (80, 100, "#1a9850")]
for lo, hi, c in bands:
    ax.add_patch(Wedge((0, 0), 1, 180 - hi * 1.8, 180 - lo * 1.8, width=0.3, color=c))
angle = np.deg2rad(180 - value * 1.8)
ax.plot([0, 0.8 * np.cos(angle)], [0, 0.8 * np.sin(angle)], color="k", lw=3)
ax.add_patch(plt.Circle((0, 0), 0.05, color="k"))
ax.text(0, -0.2, f"{value}%", ha="center", fontsize=18, fontweight="bold")
ax.set_xlim(-1.1, 1.1)
ax.set_ylim(-0.35, 1.1)
ax.set_aspect("equal")
ax.axis("off")
print(round(float(np.rad2deg(angle)), 2))
fig.savefig("gauge.png")
