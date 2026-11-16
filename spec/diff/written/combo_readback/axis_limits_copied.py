# Copy the autoscaled limits and ticks of one axes to others drawn later, plus the aspect.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(21)
fig, axs = plt.subplots(1, 3, figsize=(10, 3.5))
axs[0].scatter(rng.normal(0, 2, 100), rng.normal(5, 1, 100), s=8)
lims = axs[0].get_xlim(), axs[0].get_ylim()
ticks = axs[0].get_xticks(), axs[0].get_yticks()
print([tuple(round(v, 4) for v in l) for l in lims])
for ax in axs[1:]:
    ax.hexbin(rng.normal(0, 2, 500), rng.normal(5, 1, 500), gridsize=15)
    ax.set_xlim(*lims[0])
    ax.set_ylim(*lims[1])
    ax.set_xticks(ticks[0])
    ax.set_yticks(ticks[1])
axs[2].set_aspect(axs[0].get_aspect())
print([round(v, 4) for v in axs[2].get_xlim()], axs[2].get_xticks().tolist() == ticks[0].tolist())
fig.savefig("copied_limits.png", dpi=50)
