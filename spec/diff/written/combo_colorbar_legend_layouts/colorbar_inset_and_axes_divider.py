# Three ways of placing colour bars in one figure: ax=, cax via inset_axes, and cax via make_axes_locatable.
import numpy as np
import matplotlib.pyplot as plt
from mpl_toolkits.axes_grid1 import make_axes_locatable

y, x = np.mgrid[0:1:40j, 0:1:40j]
fig, axs = plt.subplots(1, 3, figsize=(13, 4))
ims = [ax.imshow(np.cos((k + 3) * x) * y, cmap="PuOr") for k, ax in enumerate(axs)]
cb1 = fig.colorbar(ims[0], ax=axs[0])
cax2 = axs[1].inset_axes([1.03, 0.0, 0.05, 1.0])
cb2 = fig.colorbar(ims[1], cax=cax2)
cax3 = make_axes_locatable(axs[2]).append_axes("right", size="5%", pad=0.05)
cb3 = fig.colorbar(ims[2], cax=cax3)
fig.tight_layout()
fig.canvas.draw()
print([[round(v, 3) for v in cb.ax.get_position().bounds] for cb in (cb1, cb2, cb3)])
fig.savefig("three_ways.png", dpi=50)
