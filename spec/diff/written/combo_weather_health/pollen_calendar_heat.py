# Pollen calendar: species x week heat map with category colours, a legend of levels and a current-week marker.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import ListedColormap
from matplotlib.patches import Patch

species = ["birch", "grass", "alder", "hazel", "ragweed", "mugwort"]
peaks = [16, 24, 8, 6, 34, 30]
weeks = np.arange(1, 53)
levels = np.array([np.clip(np.round(3 * np.exp(-((weeks - p) / 4) ** 2)), 0, 3) for p in peaks]).astype(int)
cmap = ListedColormap(["#f0f0f0", "#fee391", "#fe9929", "#cc4c02"])
fig, ax = plt.subplots(figsize=(11, 3.5))
ax.imshow(levels, cmap=cmap, aspect="auto", vmin=-0.5, vmax=3.5, extent=(0.5, 52.5, len(species) - 0.5, -0.5))
ax.set_yticks(range(len(species)), species)
ax.axvline(20, color="k", lw=2)
ax.text(20.3, -0.7, "now", fontsize=8)
ax.legend(handles=[Patch(color=cmap(i), label=l) for i, l in enumerate(["none", "low", "mid", "high"])],
          loc="upper left", bbox_to_anchor=(1, 1))
ax.set_xlabel("week")
fig.tight_layout()
print(levels.sum(axis=1).tolist())
fig.savefig("pollen.png", dpi=50)
