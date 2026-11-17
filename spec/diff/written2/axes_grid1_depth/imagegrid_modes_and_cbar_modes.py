# ImageGrid with cbar_mode each/single/edge, cbar_location sides and label_mode; positions and colorbar axes read back.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from mpl_toolkits.axes_grid1 import ImageGrid

data = np.add.outer(np.arange(10), np.arange(10))
for cbar_mode, loc, label_mode in [("each", "right", "L"), ("single", "top", "1"), ("edge", "bottom", "all"),
                                   (None, "right", "keep")]:
    fig = plt.figure(figsize=(6, 5), dpi=60)
    grid = ImageGrid(fig, 111, nrows_ncols=(2, 3), axes_pad=0.3, cbar_mode=cbar_mode, cbar_location=loc,
                     cbar_pad=0.1, cbar_size="7%", label_mode=label_mode, share_all=True)
    for k, ax in enumerate(grid):
        im = ax.imshow(data * (k + 1), cmap="viridis")
        if cbar_mode is not None:
            grid.cbar_axes[k].colorbar(im)
    fig.canvas.draw()
    print(cbar_mode, loc, label_mode, len(grid), grid.get_geometry(), grid.ngrids)
    print([round(v, 3) for v in grid[0].get_position().bounds], [round(v, 3) for v in grid[-1].get_position().bounds])
    print([a.get_visible() for a in grid.cbar_axes])
    print([a.xaxis.get_ticklabels()[0].get_visible() if a.xaxis.get_ticklabels() else None for a in grid])
    fig.savefig(f"imagegrid_{cbar_mode}.png")
    plt.close(fig)
fig = plt.figure(figsize=(5, 4))
grid = ImageGrid(fig, (0.1, 0.1, 0.8, 0.8), nrows_ncols=(1, 3), axes_pad=(0.1, 0.4), aspect=False, direction="column")
print(len(grid.axes_all), len(grid.axes_row), len(grid.axes_column), grid.get_aspect())
grid.set_label_mode("all")
grid.set_axes_pad((0.5, 0.5))
fig.canvas.draw()
print([round(v, 3) for v in grid[1].get_position().bounds], grid.get_axes_pad())
