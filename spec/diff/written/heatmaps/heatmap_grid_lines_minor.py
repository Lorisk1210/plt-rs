# Cell borders via minor ticks and a white minor grid on an imshow heat map (a common recipe).
import numpy as np
import matplotlib.pyplot as plt

vals = np.random.default_rng(11).integers(0, 10, (6, 8))
fig, ax = plt.subplots(figsize=(6, 4.5))
ax.imshow(vals, cmap="PuBu")
ax.set_xticks(np.arange(vals.shape[1] + 1) - 0.5, minor=True)
ax.set_yticks(np.arange(vals.shape[0] + 1) - 0.5, minor=True)
ax.grid(which="minor", color="w", linestyle="-", linewidth=3)
ax.tick_params(which="minor", bottom=False, left=False)
ax.spines[:].set_visible(False)
print(vals.sum())
fig.savefig("bordered.png")
