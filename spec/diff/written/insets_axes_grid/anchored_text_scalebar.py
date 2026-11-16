# AnchoredText label and an AnchoredSizeBar scale bar on a microscopy-like image.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.offsetbox import AnchoredText
from mpl_toolkits.axes_grid1.anchored_artists import AnchoredSizeBar

rng = np.random.default_rng(2)
img = rng.random((200, 300))
fig, ax = plt.subplots(figsize=(6, 4))
ax.imshow(img, cmap="gray")
ax.add_artist(AnchoredText("sample 7, 40x", loc="upper left", prop=dict(size=9), frameon=True))
bar = AnchoredSizeBar(ax.transData, 50, "10 µm", "lower right", pad=0.5, color="white", frameon=False, size_vertical=3)
ax.add_artist(bar)
ax.set_axis_off()
print(len(ax.artists))
fig.savefig("microscope.png")
