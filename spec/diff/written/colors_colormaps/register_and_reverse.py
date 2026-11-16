# Register a colormap by name, use it by string, and use reversed and resampled versions.
import numpy as np
import matplotlib as mpl
import matplotlib.pyplot as plt
from matplotlib.colors import LinearSegmentedColormap

cmap = LinearSegmentedColormap.from_list("sunset", ["#2c105c", "#c2185b", "#ff9800", "#fff59d"])
mpl.colormaps.register(cmap)
y, x = np.mgrid[0:1:50j, 0:1:50j]
fig, axs = plt.subplots(1, 3, figsize=(9, 3))
axs[0].imshow(x + y, cmap="sunset")
try:
    axs[1].imshow(x + y, cmap="sunset_r")
except ValueError as err:
    print("ValueError:", str(err)[:60])
    axs[1].imshow(x + y, cmap=cmap.reversed())
axs[2].imshow(x + y, cmap=mpl.colormaps["sunset"].resampled(5))
for ax, t in zip(axs, ["sunset", "sunset_r", "resampled(5)"]):
    ax.set_title(t)
    ax.axis("off")
print("sunset" in mpl.colormaps, cmap.reversed().name)
fig.savefig("sunset.png")
