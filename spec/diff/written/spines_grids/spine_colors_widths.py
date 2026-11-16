# Coloured, thick spines and matching tick colours for a dual-colour scheme.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 3.5))
ax.plot(np.arange(10), np.random.default_rng(1).normal(size=10).cumsum(), color="darkorange")
for side, color in [("left", "darkorange"), ("bottom", "navy")]:
    ax.spines[side].set_color(color)
    ax.spines[side].set_linewidth(2.5)
ax.spines[["top", "right"]].set_linestyle("--")
ax.spines[["top", "right"]].set_alpha(0.3)
ax.tick_params(axis="y", colors="darkorange", width=2)
ax.tick_params(axis="x", colors="navy", width=2)
print(ax.spines["left"].get_linewidth(), ax.spines["top"].get_linestyle())
fig.savefig("spines.png")
