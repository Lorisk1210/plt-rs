# Square map-like scatter: the figure is made square and the axes equal so circles look round.
import numpy as np
import matplotlib.pyplot as plt

theta = np.linspace(0, 2 * np.pi, 9)[:-1]
fig, ax = plt.subplots(figsize=(4, 4))
ax.scatter(np.cos(theta), np.sin(theta), s=200, c=range(8), cmap="tab10")
circle = plt.Circle((0, 0), 1, fill=False, ls="--")
ax.add_patch(circle)
ax.set_aspect("equal", adjustable="box")
ax.set_xlim(-1.5, 1.5)
ax.set_ylim(-1.5, 1.5)
print(ax.get_aspect(), ax.get_position().bounds)
fig.savefig("square.png")
ax.grid(True, ls=":")
ax.set_title("Eight points on the unit circle")
fig.savefig("square_grid.png", dpi=50)
