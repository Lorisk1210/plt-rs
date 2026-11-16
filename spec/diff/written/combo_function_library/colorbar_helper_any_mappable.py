# A add_cbar(ax, mappable, label) helper using make_axes_locatable for images, meshes, scatter and contours.
import numpy as np
import matplotlib.pyplot as plt
from mpl_toolkits.axes_grid1 import make_axes_locatable


def add_cbar(ax, mappable, label, size="5%", pad=0.05):
    cax = make_axes_locatable(ax).append_axes("right", size=size, pad=pad)
    cb = ax.figure.colorbar(mappable, cax=cax)
    cb.set_label(label)
    return cb


y, x = np.mgrid[-2:2:50j, -2:2:50j]
z = np.exp(-(x ** 2 + y ** 2)) * np.cos(2 * x)
fig, axs = plt.subplots(2, 2, figsize=(9, 8))
cbs = [add_cbar(axs[0, 0], axs[0, 0].imshow(z, cmap="viridis"), "imshow"),
       add_cbar(axs[0, 1], axs[0, 1].pcolormesh(x, y, z, cmap="magma"), "mesh"),
       add_cbar(axs[1, 0], axs[1, 0].scatter(x[::5, ::5], y[::5, ::5], c=z[::5, ::5], cmap="coolwarm"), "scatter"),
       add_cbar(axs[1, 1], axs[1, 1].contourf(x, y, z, levels=8, cmap="PuOr"), "contourf")]
fig.tight_layout()
print([(cb.ax.get_ylabel(), round(cb.vmin, 3), round(cb.vmax, 3)) for cb in cbs])
fig.savefig("cbars.png", dpi=50)
