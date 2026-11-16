# Remove a colour bar and add a new one after changing the image's colormap (users fix stale bars).
import numpy as np
import matplotlib.pyplot as plt

z = np.outer(np.arange(10), np.arange(10))
fig, ax = plt.subplots(figsize=(5, 4))
im = ax.imshow(z, cmap="Greys")
cb = fig.colorbar(im)
fig.savefig("before.png")
cb.remove()
im.set_cmap("magma")
im.set_clim(0, 50)
cb2 = fig.colorbar(im, extend="max")
print(len(fig.axes), cb2.extend)
fig.savefig("after.png")
