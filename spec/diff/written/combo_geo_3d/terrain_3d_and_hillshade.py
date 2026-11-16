# Terrain: 3-D surface coloured by LightSource shading, a 2-D hillshade map with contours, and an elevation histogram.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import LightSource

y, x = np.mgrid[0:1:80j, 0:1:80j]
z = 500 * np.exp(-((x - 0.4) ** 2 + (y - 0.6) ** 2) / 0.03) + 300 * np.exp(-((x - 0.7) ** 2 + (y - 0.3) ** 2) / 0.05) + 100 * x
ls = LightSource(azdeg=300, altdeg=40)
rgb = ls.shade(z, cmap=plt.cm.gist_earth, vert_exag=0.01, blend_mode="soft")
fig = plt.figure(figsize=(12, 4))
a = fig.add_subplot(1, 3, 1, projection="3d")
a.plot_surface(x, y, z, facecolors=rgb, rstride=2, cstride=2, linewidth=0, antialiased=False, shade=False)
a.view_init(40, -120)
b = fig.add_subplot(1, 3, 2)
b.imshow(ls.hillshade(z, vert_exag=0.01), cmap="gray", origin="lower", extent=(0, 1, 0, 1))
cs = b.contour(x, y, z, levels=8, colors="k", linewidths=0.5)
b.clabel(cs, fontsize=6, fmt="%.0f")
c = fig.add_subplot(1, 3, 3)
c.hist(z.ravel(), bins=30, color="sienna")
c.set_xlabel("elevation [m]")
fig.tight_layout()
print(rgb.shape, round(float(z.max()), 2), len(cs.levels))
fig.savefig("terrain.png", dpi=50)
