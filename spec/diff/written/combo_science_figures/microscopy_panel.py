# Microscopy panel: three channels in false colour, a merged RGB, intensity profiles along a line and a scale bar.
import numpy as np
import matplotlib.pyplot as plt
from mpl_toolkits.axes_grid1.anchored_artists import AnchoredSizeBar

y, x = np.mgrid[0:1:128j, 0:1:128j]
ch = [np.exp(-((x - cx) ** 2 + (y - cy) ** 2) / 0.01) for cx, cy in [(0.3, 0.4), (0.6, 0.5), (0.45, 0.7)]]
fig, axs = plt.subplots(2, 3, figsize=(10, 7))
for ax, c, cmap, name in zip(axs[0], ch, ["Reds", "Greens", "Blues"], ["DAPI", "GFP", "RFP"]):
    ax.imshow(c, cmap=cmap)
    ax.set_title(name)
    ax.axis("off")
merged = np.dstack(ch)
axs[1, 0].imshow(merged / merged.max())
axs[1, 0].plot([10, 118], [64, 64], "w--")
axs[1, 0].add_artist(AnchoredSizeBar(axs[1, 0].transData, 32, "10 µm", "lower right", color="white", frameon=False))
axs[1, 0].axis("off")
for c, color in zip(ch, ["r", "g", "b"]):
    axs[1, 1].plot(c[64, 10:118], color=color)
axs[1, 1].set_title("profile")
axs[1, 2].hist([c.ravel() for c in ch], bins=20, color=["r", "g", "b"], log=True)
fig.tight_layout()
print([round(float(c.sum()), 3) for c in ch])
fig.savefig("microscopy.png", dpi=50)
