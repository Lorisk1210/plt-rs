# Contour lines drawn on top of an imshow image with matching extent and origin.
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[0:4:80j, 0:6:120j]
z = np.sin(x) + np.cos(y * 1.5)
fig, ax = plt.subplots(figsize=(6, 4))
ax.imshow(z, extent=[0, 6, 0, 4], origin="lower", cmap="Greys", alpha=0.8)
cs = ax.contour(x, y, z, levels=6, cmap="autumn")
ax.clabel(cs, fontsize=7)
ax.set_title("contours over image")
print(np.round(cs.levels, 3))
fig.savefig("image_contours.png")
fig.savefig("contour_on_image_small.png", dpi=50)
print(len(fig.axes))
