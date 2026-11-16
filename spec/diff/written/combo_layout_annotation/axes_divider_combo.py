# make_axes_locatable combo: image with colour bar right, marginal histograms top and right, all sized with the divider.
import numpy as np
import matplotlib.pyplot as plt
from mpl_toolkits.axes_grid1 import make_axes_locatable

rng = np.random.default_rng(124)
x, y = rng.normal(0, 1, (2, 2000))
fig, ax = plt.subplots(figsize=(6, 6))
h = ax.hist2d(x, y, bins=40, cmap="Blues")
div = make_axes_locatable(ax)
top = div.append_axes("top", 1.0, pad=0.1, sharex=ax)
right = div.append_axes("right", 1.0, pad=0.1, sharey=ax)
cax = div.append_axes("right", 0.15, pad=0.1)
top.hist(x, bins=40, color="C0")
right.hist(y, bins=40, orientation="horizontal", color="C0")
top.tick_params(labelbottom=False)
right.tick_params(labelleft=False)
fig.colorbar(h[3], cax=cax)
print(len(fig.axes), int(h[0].sum()))
fig.savefig("divider.png", dpi=50)
