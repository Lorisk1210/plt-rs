# Image with NaN holes and a masked region shown in a "bad" colour; vmin/vmax clipping with extremes.
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[0:1:50j, 0:1:50j]
z = np.sin(6 * x) * np.cos(4 * y)
z[10:15, 10:30] = np.nan
zm = np.ma.masked_where((x - 0.7) ** 2 + (y - 0.7) ** 2 < 0.02, z)
cmap = plt.get_cmap("RdBu").with_extremes(bad="black", over="yellow", under="lime")
fig, ax = plt.subplots(figsize=(5, 4))
im = ax.imshow(zm, cmap=cmap, vmin=-0.8, vmax=0.8, origin="lower")
fig.colorbar(im, extend="both")
print(int(np.ma.count_masked(zm)), int(np.isnan(z).sum()))
fig.savefig("holes.png")
