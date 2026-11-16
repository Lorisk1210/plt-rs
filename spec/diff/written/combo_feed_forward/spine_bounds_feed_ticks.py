# Read spines' positions and bounds after setting them, and choose ticks that fall within the bounds only.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 4))
ax.plot(np.linspace(0, 10, 50), np.sin(np.linspace(0, 10, 50)))
ax.spines["bottom"].set_bounds(2, 8)
ax.spines["left"].set_bounds(-0.5, 0.5)
ax.spines[["top", "right"]].set_visible(False)
fig.canvas.draw()
lo, hi = ax.spines["bottom"].get_bounds()
ax.set_xticks([t for t in ax.get_xticks() if lo <= t <= hi])
ylo, yhi = ax.spines["left"].get_bounds()
ax.set_yticks([t for t in ax.get_yticks() if ylo <= t <= yhi])
print(ax.spines["bottom"].get_bounds(), ax.get_xticks().tolist(), ax.get_yticks().tolist())
fig.savefig("spine_ticks.png", dpi=50)
