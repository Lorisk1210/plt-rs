# Waffle chart of 100 squares coloured by category share, with a legend of patches.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Patch

shares = {"yes": 58, "no": 30, "unsure": 12}
colors = {"yes": "#1b9e77", "no": "#d95f02", "unsure": "#7570b3"}
cells = np.concatenate([[k] * v for k, (name, v) in enumerate(shares.items())]).reshape(10, 10)
cmap = plt.matplotlib.colors.ListedColormap(list(colors.values()))
fig, ax = plt.subplots(figsize=(5, 5))
ax.pcolormesh(cells, cmap=cmap, edgecolors="white", linewidth=2)
ax.set_aspect("equal")
ax.axis("off")
ax.legend(handles=[Patch(color=c, label=f"{n} ({shares[n]})") for n, c in colors.items()], loc="upper center",
          bbox_to_anchor=(0.5, 0), ncols=3)
print(np.bincount(cells.ravel()).tolist())
fig.savefig("waffle.png", bbox_inches="tight")
