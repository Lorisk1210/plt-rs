# Mark missing cells of a heat map with hatching behind a masked image.
import numpy as np
import matplotlib.pyplot as plt

data = np.arange(30, dtype=float).reshape(5, 6)
data[1, 2] = data[3, 4] = data[4, 0] = np.nan
fig, ax = plt.subplots(figsize=(5, 4))
ax.add_patch(plt.Rectangle((-0.5, -0.5), 6, 5, fill=False, hatch="///", ec="0.6", zorder=0))
im = ax.imshow(np.ma.masked_invalid(data), cmap="viridis", zorder=1)
fig.colorbar(im)
ax.set_title("missing values hatched")
print(int(np.isnan(data).sum()))
fig.savefig("missing.png")
