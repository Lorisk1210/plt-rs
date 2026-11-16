# Heat map of categorical states (ok, warn, fail) over time with a legend of patches instead of colour bar.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import ListedColormap
from matplotlib.patches import Patch

rng = np.random.default_rng(8)
status = rng.choice([0, 1, 2], size=(6, 24), p=[0.8, 0.15, 0.05])
cmap = ListedColormap(["#2ca02c", "#ffbf00", "#d62728"])
fig, ax = plt.subplots(figsize=(9, 3))
ax.imshow(status, cmap=cmap, vmin=-0.5, vmax=2.5, aspect="auto", interpolation="nearest")
ax.set_yticks(range(6), [f"server {i}" for i in range(6)])
ax.set_xlabel("hour")
ax.legend(handles=[Patch(color=c, label=l) for c, l in zip(cmap.colors, ["ok", "warn", "fail"])],
          loc="upper left", bbox_to_anchor=(1, 1))
fig.tight_layout()
print(np.bincount(status.ravel()).tolist())
fig.savefig("status.png")
