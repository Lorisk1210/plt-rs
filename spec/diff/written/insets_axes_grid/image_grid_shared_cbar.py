# ImageGrid of 2x3 images with a single colour bar and labels, axes_pad set.
import numpy as np
import matplotlib.pyplot as plt
from mpl_toolkits.axes_grid1 import ImageGrid

fig = plt.figure(figsize=(8, 5))
grid = ImageGrid(fig, 111, nrows_ncols=(2, 3), axes_pad=0.15, share_all=True, cbar_location="right",
                 cbar_mode="single", cbar_size="5%", cbar_pad=0.1)
for k, ax in enumerate(grid):
    im = ax.imshow(np.fromfunction(lambda i, j: np.sin((i + k) / 3) * np.cos(j / 4), (20, 20)), vmin=-1, vmax=1, cmap="RdBu")
    ax.set_title(f"t={k}", fontsize=8)
grid.cbar_axes[0].colorbar(im)
print(len(grid), grid.get_geometry())
fig.savefig("image_grid.png")
