# Colour bar placed inside the axes via inset_axes (a common trick for maps).
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[0:1:80j, 0:1:120j]
fig, ax = plt.subplots(figsize=(6, 4))
im = ax.imshow(np.sin(8 * x) * y, cmap="cividis", origin="lower", extent=[0, 12, 0, 8])
cax = ax.inset_axes([0.6, 0.08, 0.35, 0.05])
cb = fig.colorbar(im, cax=cax, orientation="horizontal")
cb.ax.tick_params(labelsize=7, colors="white")
cb.set_label("value", color="white", fontsize=8)
print([round(v, 3) for v in cax.get_position().bounds])
fig.savefig("inner_cbar.png")
