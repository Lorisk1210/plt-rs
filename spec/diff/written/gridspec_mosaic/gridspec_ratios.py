# Scatter with marginal histograms built with GridSpec width and height ratios.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.gridspec import GridSpec

rng = np.random.default_rng(1)
x = rng.normal(0, 1, 500)
y = 0.6 * x + rng.normal(0, 0.8, 500)

fig = plt.figure(figsize=(6, 6))
gs = GridSpec(2, 2, figure=fig, width_ratios=[4, 1], height_ratios=[1, 4], wspace=0.05, hspace=0.05)
ax = fig.add_subplot(gs[1, 0])
top = fig.add_subplot(gs[0, 0], sharex=ax)
right = fig.add_subplot(gs[1, 1], sharey=ax)
ax.scatter(x, y, s=6, alpha=0.6)
top.hist(x, bins=30, color="gray")
right.hist(y, bins=30, orientation="horizontal", color="gray")
top.tick_params(labelbottom=False)
right.tick_params(labelleft=False)
ax.set_xlabel("x")
ax.set_ylabel("y")
print(gs.get_geometry(), gs.get_width_ratios(), gs.get_height_ratios())
fig.savefig("marginals.png")
