# Take the norm and colormap from one image (get_norm/get_cmap/get_clim) and apply them to a second image and contour.
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[0:1:40j, 0:1:40j]
a = np.sin(6 * x) * y * 3
b = np.cos(5 * y) * x * 2
fig, axs = plt.subplots(1, 3, figsize=(11, 3.5))
im1 = axs[0].imshow(a, cmap="cividis")
norm, cmap = im1.norm, im1.get_cmap()
print(round(norm.vmin, 4), round(norm.vmax, 4), cmap.name, im1.get_clim() == (norm.vmin, norm.vmax))
im2 = axs[1].imshow(b, norm=norm, cmap=cmap)
cs = axs[2].contourf(b, levels=8, norm=norm, cmap=cmap)
fig.colorbar(im1, ax=axs, shrink=0.8)
print(im2.norm is norm, round(im2.get_clim()[1], 4), cs.norm is norm, np.round(cs.levels[:3], 3).tolist())
fig.savefig("shared_norm.png", dpi=50)
